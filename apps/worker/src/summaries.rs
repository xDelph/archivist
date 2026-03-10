use super::{AppState, ErrorResponse};
use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct RefreshThreadSummariesResponse {
    ok: bool,
    refreshed: usize,
}

pub(crate) async fn refresh_thread_summaries(
    State(state): State<AppState>,
    _body: Bytes,
) -> Result<Json<RefreshThreadSummariesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let refreshed = state.store.refresh_thread_summaries().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "store_failed",
            }),
        )
    })?;

    Ok(Json(RefreshThreadSummariesResponse {
        ok: true,
        refreshed,
    }))
}
