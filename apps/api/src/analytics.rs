use crate::AppState;
use crate::analytics_store::AnalyticsEventRecord;
use crate::auth::SessionClaims;
use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Deserialize)]
pub(crate) struct RecordEventRequest {
    event_type: String,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecordEventResponse {
    ok: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct EventCountEntry {
    event_type: String,
    count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct MetricsResponse {
    total_events: usize,
    by_type: Vec<EventCountEntry>,
    recent: Vec<RecentEventEntry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecentEventEntry {
    event_type: String,
    user_id: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn record_event(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Json(payload): Json<RecordEventRequest>,
) -> Result<Json<RecordEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    let event = AnalyticsEventRecord {
        event_type: payload.event_type,
        user_id: Some(claims.slack_user_id),
        metadata: if payload.metadata.is_null() {
            serde_json::json!({})
        } else {
            payload.metadata
        },
        created_at: now_iso8601(),
    };

    state
        .analytics_store
        .record_event(event)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "failed to record analytics event",
                }),
            )
        })?;

    Ok(Json(RecordEventResponse { ok: true }))
}

pub(crate) async fn metrics(
    State(state): State<AppState>,
    Extension(_claims): Extension<SessionClaims>,
) -> Json<MetricsResponse> {
    let counts = state.analytics_store.count_by_type().await;
    let total_events: usize = counts.iter().map(|(_, count)| count).sum();
    let recent = state
        .analytics_store
        .list_events(None, 20)
        .await
        .into_iter()
        .map(|e| RecentEventEntry {
            event_type: e.event_type,
            user_id: e.user_id,
            created_at: e.created_at,
        })
        .collect();

    Json(MetricsResponse {
        total_events,
        by_type: counts
            .into_iter()
            .map(|(event_type, count)| EventCountEntry { event_type, count })
            .collect(),
        recent,
    })
}

/// Records an analytics event from within a handler.
/// Fire-and-forget — errors are logged but not propagated.
pub(crate) fn record_analytics(
    state: &AppState,
    event_type: &str,
    user_id: Option<&str>,
    metadata: serde_json::Value,
) {
    let store = state.analytics_store.clone();
    let event = AnalyticsEventRecord {
        event_type: event_type.to_owned(),
        user_id: user_id.map(|id| id.to_owned()),
        metadata,
        created_at: now_iso8601(),
    };

    tokio::spawn(async move {
        if let Err(error) = store.record_event(event).await {
            tracing::warn!(%error, "failed to record analytics event");
        }
    });
}

fn now_iso8601() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    format!("{secs}")
}

#[cfg(test)]
#[path = "analytics_tests.rs"]
mod tests;
