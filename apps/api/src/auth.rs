use crate::{ApiConfig, AppState};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const SLACK_AUTHORIZE_URL: &str = "https://slack.com/openid/connect/authorize";
const SLACK_TOKEN_URL: &str = "https://slack.com/api/openid.connect.token";
const SLACK_OIDC_SCOPE: &str = "openid profile email";
const SLACK_ISSUER: &str = "https://slack.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlackAuthConfig {
    pub(crate) client_id: Option<String>,
    pub(crate) client_secret: Option<String>,
    pub(crate) redirect_uri: Option<String>,
    pub(crate) workspace_id: Option<String>,
    pub(crate) token_url: Option<String>,
}

impl SlackAuthConfig {
    pub(crate) fn from_config(config: &ApiConfig) -> Self {
        Self {
            client_id: config.slack_client_id.clone(),
            client_secret: config.slack_client_secret.clone(),
            redirect_uri: config.slack_redirect_uri.clone(),
            workspace_id: config.slack_workspace_id.clone(),
            token_url: config.slack_token_url.clone(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlackCallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SlackIdentityResponse {
    ok: bool,
    slack_user_id: String,
    team_id: String,
    email: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackTokenExchangeResponse {
    ok: bool,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackIdentityClaims {
    iss: String,
    aud: String,
    exp: i64,
    #[serde(rename = "https://slack.com/user_id")]
    slack_user_id: String,
    #[serde(rename = "https://slack.com/team_id")]
    team_id: String,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

pub(crate) async fn slack_start(
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let authorize_url = build_authorize_url(&state.slack_auth).ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_auth_config",
        }),
    ))?;

    Ok(Redirect::temporary(&authorize_url).into_response())
}

pub(crate) async fn slack_callback(
    State(state): State<AppState>,
    Query(query): Query<SlackCallbackQuery>,
) -> Result<Json<SlackIdentityResponse>, (StatusCode, Json<ErrorResponse>)> {
    if query.error.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "slack_authorization_failed",
            }),
        ));
    }

    let code = query
        .code
        .as_deref()
        .filter(|code| !code.trim().is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_auth_code",
            }),
        ))?;
    let identity = exchange_code_for_identity(&state.slack_auth, code)
        .await
        .map_err(callback_error_response)?;

    Ok(Json(identity))
}

pub(crate) fn build_authorize_url(config: &SlackAuthConfig) -> Option<String> {
    let client_id = config.client_id.as_deref()?.trim();
    let redirect_uri = config.redirect_uri.as_deref()?.trim();
    if client_id.is_empty() || redirect_uri.is_empty() {
        return None;
    }

    let mut query = vec![
        ("response_type", "code".to_owned()),
        ("client_id", urlencoding::encode(client_id).into_owned()),
        ("scope", urlencoding::encode(SLACK_OIDC_SCOPE).into_owned()),
        (
            "redirect_uri",
            urlencoding::encode(redirect_uri).into_owned(),
        ),
    ];

    if let Some(workspace_id) = config
        .workspace_id
        .as_deref()
        .map(str::trim)
        .filter(|workspace_id| !workspace_id.is_empty())
    {
        query.push(("team", urlencoding::encode(workspace_id).into_owned()));
    }

    Some(format!(
        "{SLACK_AUTHORIZE_URL}?{}",
        query
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("&")
    ))
}

pub(crate) async fn exchange_code_for_identity(
    config: &SlackAuthConfig,
    code: &str,
) -> Result<SlackIdentityResponse, CallbackError> {
    let client_id = config
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CallbackError::MissingConfig)?;
    let client_secret = config
        .client_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CallbackError::MissingConfig)?;
    let redirect_uri = config
        .redirect_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CallbackError::MissingConfig)?;
    let response = reqwest::Client::new()
        .post(config.token_url.as_deref().unwrap_or(SLACK_TOKEN_URL))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|_| CallbackError::TokenExchangeFailed)?;
    if !response.status().is_success() {
        return Err(CallbackError::TokenExchangeFailed);
    }

    let exchange: SlackTokenExchangeResponse = response
        .json()
        .await
        .map_err(|_| CallbackError::TokenExchangeFailed)?;
    if !exchange.ok {
        return Err(CallbackError::TokenExchangeFailed);
    }

    validate_identity_token(
        config,
        exchange
            .id_token
            .as_deref()
            .ok_or(CallbackError::InvalidIdentityToken)?,
    )
}

pub(crate) fn validate_identity_token(
    config: &SlackAuthConfig,
    id_token: &str,
) -> Result<SlackIdentityResponse, CallbackError> {
    let client_id = config
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CallbackError::MissingConfig)?;
    let claims = parse_identity_claims(id_token)?;
    if claims.iss != SLACK_ISSUER
        || claims.aud != client_id
        || claims.exp <= current_unix_timestamp()
    {
        return Err(CallbackError::InvalidIdentityToken);
    }
    if let Some(workspace_id) = config
        .workspace_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && claims.team_id != workspace_id
    {
        return Err(CallbackError::WorkspaceMismatch);
    }

    Ok(SlackIdentityResponse {
        ok: true,
        slack_user_id: claims.slack_user_id,
        team_id: claims.team_id,
        email: claims.email,
        display_name: claims.name,
        avatar_url: claims.picture,
    })
}

fn parse_identity_claims(id_token: &str) -> Result<SlackIdentityClaims, CallbackError> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or(CallbackError::InvalidIdentityToken)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .map_err(|_| CallbackError::InvalidIdentityToken)?;
    serde_json::from_slice(&decoded).map_err(|_| CallbackError::InvalidIdentityToken)
}

fn callback_error_response(error: CallbackError) -> (StatusCode, Json<ErrorResponse>) {
    match error {
        CallbackError::MissingConfig => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "missing_slack_auth_config",
            }),
        ),
        CallbackError::TokenExchangeFailed => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "token_exchange_failed",
            }),
        ),
        CallbackError::InvalidIdentityToken => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid_identity_token",
            }),
        ),
        CallbackError::WorkspaceMismatch => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "workspace_mismatch",
            }),
        ),
    }
}

pub(crate) fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

#[derive(Debug)]
pub(crate) enum CallbackError {
    MissingConfig,
    TokenExchangeFailed,
    InvalidIdentityToken,
    WorkspaceMismatch,
}

#[cfg(test)]
mod tests;
