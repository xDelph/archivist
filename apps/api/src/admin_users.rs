use crate::{
    AppState,
    highlights::require_admin_user_id,
    user_privacy::ANONYMOUS_DISPLAY_NAME,
    user_store::{AdminUserRecord, UserStoreError},
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct AdminUsersQuery {
    query: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfirmRequest {
    confirm: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AdminUsersResponse {
    ok: bool,
    users: Vec<AdminUserResponse>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AdminUserResponse {
    slack_user_id: String,
    email: Option<String>,
    display_name: Option<String>,
    is_active: bool,
    is_anonymized: bool,
    roles: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct PrivacyActionResponse {
    ok: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn list_users(
    State(state): State<AppState>,
    Extension(claims): Extension<crate::auth::SessionClaims>,
    Query(query): Query<AdminUsersQuery>,
) -> Result<Json<AdminUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_admin(&state, &claims.slack_user_id).await?;
    let users = state
        .user_store
        .list_users(query.query.as_deref(), query.limit.unwrap_or(50))
        .await
        .map_err(user_store_failed)?;

    Ok(Json(AdminUsersResponse {
        ok: true,
        users: users.into_iter().map(AdminUserResponse::from).collect(),
    }))
}

pub(crate) async fn anonymize_user(
    State(state): State<AppState>,
    Extension(claims): Extension<crate::auth::SessionClaims>,
    Path(slack_user_id): Path<String>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<PrivacyActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_admin(&state, &claims.slack_user_id).await?;
    require_confirmation(body.confirm)?;
    set_anonymized_for_user(&state, &slack_user_id, true, Some(&claims.slack_user_id)).await?;
    Ok(Json(PrivacyActionResponse { ok: true }))
}

pub(crate) async fn de_anonymize_user(
    State(state): State<AppState>,
    Extension(claims): Extension<crate::auth::SessionClaims>,
    Path(slack_user_id): Path<String>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<PrivacyActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_admin(&state, &claims.slack_user_id).await?;
    require_confirmation(body.confirm)?;
    ensure_user_can_be_de_anonymized(&state, &slack_user_id).await?;
    set_anonymized_for_user(&state, &slack_user_id, false, Some(&claims.slack_user_id)).await?;
    Ok(Json(PrivacyActionResponse { ok: true }))
}

pub(crate) async fn deactivate_user(
    State(state): State<AppState>,
    Extension(claims): Extension<crate::auth::SessionClaims>,
    Path(slack_user_id): Path<String>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<PrivacyActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_admin(&state, &claims.slack_user_id).await?;
    require_confirmation(body.confirm)?;
    set_anonymized_for_user(&state, &slack_user_id, true, Some(&claims.slack_user_id)).await?;
    state
        .user_store
        .set_active(&slack_user_id, false)
        .await
        .map_err(user_store_failed)?;
    Ok(Json(PrivacyActionResponse { ok: true }))
}

pub(crate) async fn reactivate_user(
    State(state): State<AppState>,
    Extension(claims): Extension<crate::auth::SessionClaims>,
    Path(slack_user_id): Path<String>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<PrivacyActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_admin(&state, &claims.slack_user_id).await?;
    require_confirmation(body.confirm)?;
    state
        .user_store
        .set_active(&slack_user_id, true)
        .await
        .map_err(user_store_failed)?;
    set_anonymized_for_user(&state, &slack_user_id, false, Some(&claims.slack_user_id)).await?;
    Ok(Json(PrivacyActionResponse { ok: true }))
}

impl From<AdminUserRecord> for AdminUserResponse {
    fn from(value: AdminUserRecord) -> Self {
        Self {
            slack_user_id: value.slack_user_id,
            email: if value.is_anonymized {
                None
            } else {
                value.email
            },
            display_name: if value.is_anonymized {
                Some(ANONYMOUS_DISPLAY_NAME.to_owned())
            } else {
                value.display_name
            },
            is_active: value.is_active,
            is_anonymized: value.is_anonymized,
            roles: value.roles,
        }
    }
}

async fn set_anonymized_for_user(
    state: &AppState,
    slack_user_id: &str,
    anonymized: bool,
    initiated_by: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    state
        .user_store
        .set_anonymized(slack_user_id, anonymized, initiated_by)
        .await
        .map_err(user_store_failed)
}

async fn ensure_user_can_be_de_anonymized(
    state: &AppState,
    slack_user_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let user = state.user_store.find_user_raw(slack_user_id).await.ok_or((
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
    Ok(())
}

async fn ensure_admin(
    state: &AppState,
    slack_user_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    require_admin_user_id(state, slack_user_id)
        .await
        .map_err(|(status, _)| {
            (
                status,
                Json(ErrorResponse {
                    error: "admin_required",
                }),
            )
        })
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
#[path = "admin_users_tests.rs"]
mod tests;
