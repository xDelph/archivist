use crate::{
    AppState,
    highlights::{
        DeleteHighlightResponse, ErrorResponse, HighlightMutationResponse, HighlightsResponse,
        ListHighlightsQuery, delete_highlight_by_id, list_highlights_response,
        pin_highlight_for_user, require_admin_user_id,
    },
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct SlackHighlightThreadRequest {
    slack_user_id: String,
    thread_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlackHighlightListRequest {
    slack_user_id: String,
}

pub(crate) async fn pin_highlight_from_slack_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SlackHighlightThreadRequest>,
) -> Result<Json<HighlightMutationResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_slack_command(&state, &headers, payload.slack_user_id.trim()).await?;
    pin_highlight_for_user(&state, payload.slack_user_id.trim(), &payload.thread_id).await
}

pub(crate) async fn list_highlights_from_slack_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SlackHighlightListRequest>,
) -> Result<Json<HighlightsResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_slack_command(&state, &headers, payload.slack_user_id.trim()).await?;
    Ok(Json(
        list_highlights_response(&state, &ListHighlightsQuery { channel_id: None }).await?,
    ))
}

pub(crate) async fn unpin_highlight_from_slack_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SlackHighlightThreadRequest>,
) -> Result<Json<DeleteHighlightResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_slack_command(&state, &headers, payload.slack_user_id.trim()).await?;
    delete_highlight_by_id(&state, payload.thread_id.trim()).await
}

async fn authorize_slack_command(
    state: &AppState,
    headers: &HeaderMap,
    slack_user_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    require_slack_command_token(state, headers)?;
    require_admin_user_id(state, slack_user_id).await
}

fn require_slack_command_token(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(expected_token) = state.slack_command_token.as_deref() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "slack_command_auth_unavailable",
            }),
        ));
    };
    let provided_token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if provided_token != Some(expected_token) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid_slack_command_token",
            }),
        ));
    }
    Ok(())
}
