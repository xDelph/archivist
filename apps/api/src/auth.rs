use crate::{ApiConfig, AppState};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Serialize;

const SLACK_AUTHORIZE_URL: &str = "https://slack.com/openid/connect/authorize";
const SLACK_OIDC_SCOPE: &str = "openid profile email";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlackAuthConfig {
    client_id: Option<String>,
    redirect_uri: Option<String>,
    workspace_id: Option<String>,
}

impl SlackAuthConfig {
    pub(crate) fn from_config(config: &ApiConfig) -> Self {
        Self {
            client_id: config.slack_client_id.clone(),
            redirect_uri: config.slack_redirect_uri.clone(),
            workspace_id: config.slack_workspace_id.clone(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
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

fn build_authorize_url(config: &SlackAuthConfig) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::{SlackAuthConfig, build_authorize_url};
    use crate::{ApiConfig, build_router};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[test]
    fn authorize_url_omits_team_when_workspace_is_not_configured() {
        let url = build_authorize_url(&SlackAuthConfig {
            client_id: Some("client_123".to_owned()),
            redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
            workspace_id: None,
        })
        .expect("authorize url");

        assert!(url.starts_with("https://slack.com/openid/connect/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=client_123"));
        assert!(url.contains("scope=openid%20profile%20email"));
        assert!(
            url.contains(
                "redirect_uri=https%3A%2F%2Farchivist.dev%2Fapi%2Fauth%2Fslack%2Fcallback"
            )
        );
        assert!(!url.contains("&team="));
    }

    #[test]
    fn authorize_url_requires_client_id_and_redirect_uri() {
        assert_eq!(
            build_authorize_url(&SlackAuthConfig {
                client_id: None,
                redirect_uri: Some("https://archivist.dev/callback".to_owned()),
                workspace_id: None,
            }),
            None
        );
        assert_eq!(
            build_authorize_url(&SlackAuthConfig {
                client_id: Some("client_123".to_owned()),
                redirect_uri: Some("   ".to_owned()),
                workspace_id: None,
            }),
            None
        );
    }

    #[tokio::test]
    async fn slack_start_redirects_to_slack_oidc() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
            slack_client_id: Some("client_123".to_owned()),
            slack_client_secret: Some("secret".to_owned()),
            slack_redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
            slack_workspace_id: Some("T123".to_owned()),
        })
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/start")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .expect("location header");

        assert!(location.contains("response_type=code"));
        assert!(location.contains("client_id=client_123"));
        assert!(location.contains("scope=openid%20profile%20email"));
        assert!(location.contains("team=T123"));
    }

    #[tokio::test]
    async fn slack_start_rejects_missing_config() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_workspace_id: None,
        })
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/start")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
