mod auth;
mod auth_store;
mod catch_up;
mod channels;
mod threads;
mod user_store;

use axum::{Json, Router, middleware, routing::get};
use db::{JsonlEventStore, RepositoryMode, StoreError};
use domain::WorkspaceMode;
use search::SearchBackend;
use serde::Serialize;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4000;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";
const DEFAULT_AUTH_STORE_PATH: &str = "logs/auth-identities.json";
const DEFAULT_SYNCED_USERS_PATH: &str = "logs/synced-users.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub event_log_path: String,
    pub slack_client_id: Option<String>,
    pub slack_client_secret: Option<String>,
    pub slack_redirect_uri: Option<String>,
    pub slack_workspace_id: Option<String>,
    pub slack_token_url: Option<String>,
    pub session_secret: Option<String>,
    pub auth_store_path: String,
    pub synced_users_path: String,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARCHIVIST_API_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARCHIVIST_API_PORT", DEFAULT_PORT),
            event_log_path: std::env::var("ARCHIVIST_EVENT_LOG_PATH")
                .unwrap_or_else(|_| DEFAULT_EVENT_LOG_PATH.to_owned()),
            slack_client_id: std::env::var("SLACK_CLIENT_ID").ok(),
            slack_client_secret: std::env::var("SLACK_CLIENT_SECRET").ok(),
            slack_redirect_uri: std::env::var("SLACK_REDIRECT_URI").ok(),
            slack_workspace_id: std::env::var("SLACK_WORKSPACE_ID").ok(),
            slack_token_url: std::env::var("SLACK_OIDC_TOKEN_URL").ok(),
            session_secret: std::env::var("ARCHIVIST_SESSION_SECRET").ok(),
            auth_store_path: std::env::var("ARCHIVIST_AUTH_STORE_PATH")
                .unwrap_or_else(|_| DEFAULT_AUTH_STORE_PATH.to_owned()),
            synced_users_path: std::env::var("ARCHIVIST_SYNCED_USERS_PATH")
                .unwrap_or_else(|_| DEFAULT_SYNCED_USERS_PATH.to_owned()),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) store: JsonlEventStore,
    pub(crate) slack_auth: auth::SlackAuthConfig,
    pub(crate) session_secret: Option<String>,
    pub(crate) auth_store: auth_store::LocalAuthStore,
    pub(crate) user_store: user_store::LocalUserStore,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    workspace_mode: &'static str,
    repository_mode: &'static str,
    search_backend: &'static str,
}

pub async fn build_router(config: ApiConfig) -> Result<Router, StoreError> {
    let store = JsonlEventStore::open(&config.event_log_path).await?;
    let auth_store = auth_store::LocalAuthStore::open(&config.auth_store_path)
        .await
        .map_err(auth_store_error_to_store_error)?;
    let user_store = user_store::LocalUserStore::open(&config.synced_users_path)
        .await
        .map_err(user_store_error_to_store_error)?;
    let state = AppState {
        store,
        slack_auth: auth::SlackAuthConfig::from_config(&config),
        session_secret: config.session_secret.clone(),
        auth_store,
        user_store,
    };
    let protected_api = Router::new()
        .route("/api/catch-up", get(catch_up::catch_up))
        .route("/api/channels", get(channels::channels))
        .route("/api/threads/{id}", get(threads::thread_detail))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_session,
        ));

    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/auth/slack/start", get(auth::slack_start))
        .route("/api/auth/slack/callback", get(auth::slack_callback))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", axum::routing::post(auth::logout))
        .merge(protected_api)
        .with_state(state)
        .layer(TraceLayer::new_for_http()))
}

fn auth_store_error_to_store_error(error: auth_store::AuthStoreError) -> StoreError {
    match error {
        auth_store::AuthStoreError::Read(error)
        | auth_store::AuthStoreError::CreateDirectory(error)
        | auth_store::AuthStoreError::Write(error) => StoreError::Read(error),
        auth_store::AuthStoreError::Parse(error) => StoreError::Parse(error),
    }
}

fn user_store_error_to_store_error(error: user_store::UserStoreError) -> StoreError {
    match error {
        user_store::UserStoreError::Read(error) => StoreError::Read(error),
        user_store::UserStoreError::Parse(error) => StoreError::Parse(error),
        #[cfg(test)]
        user_store::UserStoreError::CreateDirectory(error)
        | user_store::UserStoreError::Write(error) => StoreError::Read(error),
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "api",
        version: env!("CARGO_PKG_VERSION"),
        workspace_mode: WorkspaceMode::SingleWorkspace.as_str(),
        repository_mode: RepositoryMode::LocalJsonlMock.as_str(),
        search_backend: SearchBackend::PostgresTsvectorPlaceholder.as_str(),
    })
}

fn read_port(key: &str, fallback: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::{ApiConfig, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[test]
    fn config_uses_defaults() {
        let config = ApiConfig::from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 4000);
        assert_eq!(config.event_log_path, "logs/process-events.jsonl");
        assert_eq!(config.slack_client_id, None);
        assert_eq!(config.slack_client_secret, None);
        assert_eq!(config.slack_redirect_uri, None);
        assert_eq!(config.slack_workspace_id, None);
        assert_eq!(config.slack_token_url, None);
        assert_eq!(config.session_secret, None);
        assert_eq!(config.auth_store_path, "logs/auth-identities.json");
        assert_eq!(config.synced_users_path, "logs/synced-users.json");
        assert_eq!(config.bind_address(), "127.0.0.1:4000");
    }

    #[tokio::test]
    async fn health_route_reports_workspace_capabilities() {
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
            slack_token_url: None,
            session_secret: None,
            auth_store_path: tempdir
                .path()
                .join("auth-identities.json")
                .display()
                .to_string(),
            synced_users_path: tempdir
                .path()
                .join("synced-users.json")
                .display()
                .to_string(),
        })
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(payload["service"], "api");
        assert_eq!(payload["workspace_mode"], "single_workspace");
        assert_eq!(payload["repository_mode"], "local_jsonl_mock");
    }
}
