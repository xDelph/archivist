use crate::StoreError;
use domain::{ChannelKind, EventPayload, Message};
use sqlx::{PgPool, Row, Transaction, postgres::PgRow};

pub(crate) async fn count_rows(pool: &PgPool, table: &str) -> Result<usize, StoreError> {
    let query = format!("SELECT COUNT(*)::bigint AS count FROM {table}");
    let count = sqlx::query_scalar::<_, i64>(&query)
        .fetch_one(pool)
        .await
        .map_err(StoreError::Sqlx)?;
    Ok(count.max(0) as usize)
}

pub(crate) fn map_message_row(row: PgRow) -> Message {
    let ts: String = row.get("ts");
    let root_ts: String = row.get("root_ts");

    Message {
        channel_id: row.get("channel_id"),
        ts: ts.clone(),
        thread_ts: (root_ts != ts).then_some(root_ts),
        user_id: row.get("user_id"),
        text: row.get("text"),
    }
}

pub(crate) fn event_type(payload: &EventPayload) -> &'static str {
    match payload {
        EventPayload::Message { .. } => "message",
        EventPayload::ReactionAdded { .. } => "reaction_added",
        EventPayload::ChannelUpdated { .. } => "channel_updated",
    }
}

pub(crate) fn thread_root_ts(ts: &str, thread_ts: Option<&str>) -> String {
    thread_ts.unwrap_or(ts).to_owned()
}

pub(crate) fn slack_ts_seconds(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or_default()
}

pub(crate) fn normalize_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

pub(crate) fn parse_channel_kind(value: String) -> ChannelKind {
    match value.as_str() {
        "public" => ChannelKind::Public,
        "private" => ChannelKind::Private,
        "direct" => ChannelKind::Direct,
        _ => ChannelKind::Unknown,
    }
}

pub(crate) async fn upsert_channel(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    channel_id: &str,
    channel_kind: ChannelKind,
    name: Option<String>,
    is_archived: bool,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO channels (id, kind, name, is_archived)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO UPDATE
        SET kind = EXCLUDED.kind,
            name = COALESCE(EXCLUDED.name, channels.name),
            is_archived = EXCLUDED.is_archived,
            updated_at = now()
        "#,
    )
    .bind(channel_id)
    .bind(channel_kind.as_str())
    .bind(name)
    .bind(is_archived)
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(())
}

pub(crate) async fn message_root_ts(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    channel_id: &str,
    message_ts: &str,
) -> Result<Option<String>, StoreError> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT root_ts
        FROM messages
        WHERE channel_id = $1 AND ts = $2
        "#,
    )
    .bind(channel_id)
    .bind(message_ts)
    .fetch_optional(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)
}
