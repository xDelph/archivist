mod session;

pub(crate) use self::session::SessionClaims;
#[cfg(test)]
pub(crate) use self::session::build_session_token;

use self::session::{
    SESSION_COOKIE_NAME, SessionError, session_claims, session_cookie, session_cookie_header,
    validate_session_token,
};
use crate::{ApiConfig, AppState};
use axum::{
    Json,
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    middleware::Next,
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
    pub(crate) team_id: Option<String>,
    pub(crate) token_url: Option<String>,
}

impl SlackAuthConfig {
    pub(crate) fn from_config(config: &ApiConfig) -> Self {
        Self {
            client_id: config.slack_client_id.clone(),
            client_secret: config.slack_client_secret.clone(),
            redirect_uri: config.slack_redirect_uri.clone(),
            team_id: config.slack_team_id.clone(),
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
    pub(crate) ok: bool,
    pub(crate) slack_user_id: String,
    pub(crate) email: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MeResponse {
    ok: bool,
    user: SessionUserResponse,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct LogoutResponse {
    ok: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SessionUserResponse {
    slack_user_id: String,
    email: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    roles: Vec<String>,
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
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

fn require_session_secret<'a>(
    state: &'a AppState,
    context: &'static str,
    request_path: Option<&str>,
) -> Result<&'a str, (StatusCode, Json<ErrorResponse>)> {
    state.session_secret.as_deref().ok_or_else(|| {
        if let Some(request_path) = request_path {
            tracing::error!(
                context,
                request_path,
                web_origin = %state.web_origin,
                env_var = "ARKIVIST_SESSION_SECRET",
                "session configuration missing in API runtime state"
            );
        } else {
            tracing::error!(
                context,
                web_origin = %state.web_origin,
                env_var = "ARKIVIST_SESSION_SECRET",
                "session configuration missing in API runtime state"
            );
        }

        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "missing_session_config",
            }),
        )
    })
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
    headers: HeaderMap,
    Query(query): Query<SlackCallbackQuery>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
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
    let synced_user = state
        .user_store
        .find_user(&identity.slack_user_id)
        .await
        .ok_or_else(|| callback_error_response(CallbackError::UserNotSynced))?;
    if !synced_user.is_active {
        return Err(callback_error_response(CallbackError::UserInactive));
    }
    state
        .auth_store
        .upsert_identity(&identity)
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                slack_user_id = %identity.slack_user_id,
                "failed to upsert auth identity"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "identity_store_failed",
                }),
            )
        })?;
    let session_secret =
        require_session_secret(&state, "auth.slack_callback", Some("/auth/slack/callback"))?;
    let session_cookie = session_cookie_header(
        session_secret,
        &session_claims(
            identity.slack_user_id.clone(),
            identity.email.clone(),
            identity.display_name.clone(),
            identity.avatar_url.clone(),
        ),
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "session_cookie_failed",
            }),
        )
    })?;
    let mut response = if prefers_html_response(&headers) {
        Redirect::to(&state.web_origin).into_response()
    } else {
        Json(identity).into_response()
    };
    response.headers_mut().insert(
        SET_COOKIE,
        session_cookie.parse().expect("valid session cookie"),
    );

    Ok(response)
}

pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session_token = session_cookie(&headers).ok_or((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "missing_session",
        }),
    ))?;
    let session_secret = require_session_secret(&state, "auth.me", Some("/auth/me"))?;
    let claims =
        validate_session_token(session_secret, session_token).map_err(me_error_response)?;
    let roles = state
        .user_role_store
        .list_roles(&claims.slack_user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "user_roles_unavailable",
                }),
            )
        })?;

    Ok(Json(MeResponse {
        ok: true,
        user: SessionUserResponse {
            slack_user_id: claims.slack_user_id,
            email: claims.email,
            display_name: claims.display_name,
            avatar_url: claims.avatar_url,
            roles,
        },
    }))
}

pub(crate) async fn require_session(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let session_token = session_cookie(request.headers()).ok_or((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "missing_session",
        }),
    ))?;
    let request_path = request.uri().path().to_owned();
    let session_secret =
        require_session_secret(&state, "auth.require_session", Some(request_path.as_str()))?;
    let claims =
        validate_session_token(session_secret, session_token).map_err(me_error_response)?;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

pub(crate) async fn logout() -> Response {
    let mut response = Json(LogoutResponse { ok: true }).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        format!("{SESSION_COOKIE_NAME}=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax")
            .parse()
            .expect("valid session clearing cookie"),
    );
    response
}

pub(crate) fn build_authorize_url(config: &SlackAuthConfig) -> Option<String> {
    let client_id = config.client_id.as_deref()?.trim();
    let redirect_uri = config.redirect_uri.as_deref()?.trim();
    if client_id.is_empty() || redirect_uri.is_empty() {
        return None;
    }

    let query = vec![
        ("response_type", "code".to_owned()),
        ("client_id", urlencoding::encode(client_id).into_owned()),
        ("scope", urlencoding::encode(SLACK_OIDC_SCOPE).into_owned()),
        (
            "redirect_uri",
            urlencoding::encode(redirect_uri).into_owned(),
        ),
    ];
    let mut query = query;
    if let Some(team_id) = config
        .team_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query.push(("team", urlencoding::encode(team_id).into_owned()));
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
    Ok(SlackIdentityResponse {
        ok: true,
        slack_user_id: claims.slack_user_id,
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
        CallbackError::UserNotSynced => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "user_not_synced",
            }),
        ),
        CallbackError::UserInactive => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "user_inactive",
            }),
        ),
    }
}

fn prefers_html_response(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("text/html"))
}

pub(crate) fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

fn me_error_response(error: SessionError) -> (StatusCode, Json<ErrorResponse>) {
    let error = match error {
        SessionError::ExpiredSession => "expired_session",
        SessionError::InvalidSession => "invalid_session",
    };

    (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error }))
}

#[derive(Debug)]
pub(crate) enum CallbackError {
    MissingConfig,
    TokenExchangeFailed,
    InvalidIdentityToken,
    UserNotSynced,
    UserInactive,
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
