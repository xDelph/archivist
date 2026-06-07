use crate::{AppState, auth::SessionClaims, user_store::UserStoreError};
use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct ConfirmRequest {
    confirm: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PrivacyActionResponse {
    ok: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn anonymize_self(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<PrivacyActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_confirmation(body.confirm)?;
    state
        .user_store
        .set_anonymized(&claims.slack_user_id, true, Some(&claims.slack_user_id))
        .await
        .map_err(user_store_failed)?;
    Ok(Json(PrivacyActionResponse { ok: true }))
}

pub(crate) async fn de_anonymize_self(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<PrivacyActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_confirmation(body.confirm)?;
    let user = state
        .user_store
        .find_user_raw(&claims.slack_user_id)
        .await
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "user_not_found",
            }),
        ))?;
    if !user.is_active {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "user_deactivated",
            }),
        ));
    }
    state
        .user_store
        .set_anonymized(&claims.slack_user_id, false, Some(&claims.slack_user_id))
        .await
        .map_err(user_store_failed)?;
    Ok(Json(PrivacyActionResponse { ok: true }))
}

fn require_confirmation(confirm: bool) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if confirm {
        return Ok(());
    }
    Err((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "confirmation_required",
        }),
    ))
}

fn user_store_failed(error: UserStoreError) -> (StatusCode, Json<ErrorResponse>) {
    match error {
        UserStoreError::NotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "user_not_found",
            }),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "user_store_failed",
            }),
        ),
    }
}

#[cfg(test)]
#[path = "auth_privacy_tests.rs"]
mod tests;
