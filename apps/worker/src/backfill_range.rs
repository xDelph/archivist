use super::ErrorResponse;
use axum::{Json, body::Bytes, http::StatusCode};
use db::EventStore;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct BackfillChannelRequest {
    pub(crate) channel_id: Option<String>,
    pub(crate) cursor: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    pub(crate) oldest_ts: Option<String>,
    #[serde(default)]
    pub(crate) resume_from_last_message_ts: bool,
}

pub(crate) async fn resolve_oldest_ts(
    store: &EventStore,
    request: &BackfillChannelRequest,
    channel_id: &str,
) -> Result<Option<String>, (StatusCode, Json<ErrorResponse>)> {
    if request.resume_from_last_message_ts {
        let latest_message_ts = store
            .latest_message_ts(channel_id)
            .await
            .map_err(store_failed)?;
        if let Some(latest_message_ts) = latest_message_ts.as_deref() {
            tracing::info!(
                channel_id,
                oldest_ts = latest_message_ts,
                "resolved incremental backfill start from last stored message"
            );
        } else {
            tracing::info!(
                channel_id,
                "no stored messages found for resume mode, falling back to full channel backfill"
            );
        }
        return Ok(latest_message_ts);
    }

    Ok(request.oldest_ts.clone())
}

pub(crate) async fn resolve_user_sync_oldest_ts(
    store: &EventStore,
    request: &BackfillChannelRequest,
    channel_ids: &[String],
) -> Result<Option<String>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(oldest_ts) = request.oldest_ts.clone() {
        return Ok(Some(oldest_ts));
    }
    if !request.resume_from_last_message_ts {
        return Ok(None);
    }

    let mut earliest_latest_message_ts = None::<String>;
    for channel_id in channel_ids {
        let latest_message_ts = store
            .latest_message_ts(channel_id)
            .await
            .map_err(store_failed)?;
        let Some(latest_message_ts) = latest_message_ts else {
            continue;
        };
        if earliest_latest_message_ts
            .as_deref()
            .is_none_or(|current| compare_slack_timestamps(&latest_message_ts, current).is_lt())
        {
            earliest_latest_message_ts = Some(latest_message_ts);
        }
    }

    if let Some(oldest_ts) = earliest_latest_message_ts.as_deref() {
        tracing::info!(
            oldest_ts,
            channels = channel_ids.len(),
            "resolved workspace user sync cutoff from earliest stored channel message"
        );
    } else {
        tracing::info!(
            channels = channel_ids.len(),
            "no stored channel messages found for workspace resume mode, falling back to full user sync"
        );
    }

    Ok(earliest_latest_message_ts)
}

pub(crate) fn should_sync_user(updated_at: Option<i64>, oldest_ts: Option<&str>) -> bool {
    let Some(oldest_ts) = oldest_ts.and_then(slack_timestamp_value) else {
        return true;
    };
    let Some(updated_at) = updated_at else {
        return true;
    };

    (updated_at as f64) > oldest_ts
}

pub(crate) fn parse_backfill_request(
    body: &Bytes,
) -> Result<BackfillChannelRequest, (StatusCode, Json<ErrorResponse>)> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(BackfillChannelRequest::default());
    }

    let request: BackfillChannelRequest = serde_json::from_slice(body).map_err(|error| {
        tracing::warn!(?error, "received invalid backfill request body");
        invalid_backfill_request()
    })?;
    if request.resume_from_last_message_ts && request.oldest_ts.is_some() {
        tracing::warn!(
            "received conflicting backfill range options: resume_from_last_message_ts and oldest_ts"
        );
        return Err(invalid_backfill_request());
    }

    Ok(request)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TimestampValue {
    String(String),
    Number(f64),
}

fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<TimestampValue>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    let normalized = match value {
        TimestampValue::String(value) => value.trim().to_owned(),
        TimestampValue::Number(value) => {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
    };
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.parse::<f64>().is_err() {
        return Err(serde::de::Error::custom("invalid slack timestamp"));
    }

    Ok(Some(normalized))
}

fn compare_slack_timestamps(left: &str, right: &str) -> std::cmp::Ordering {
    match (slack_timestamp_value(left), slack_timestamp_value(right)) {
        (Some(left), Some(right)) => left
            .partial_cmp(&right)
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => left.cmp(right),
    }
}

fn slack_timestamp_value(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn invalid_backfill_request() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_backfill_request",
        }),
    )
}

#[cfg(test)]
#[path = "backfill_range_tests.rs"]
mod tests;

fn store_failed(_error: db::StoreError) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "store_failed",
        }),
    )
}
