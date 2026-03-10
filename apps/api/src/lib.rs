mod analytics;
mod analytics_store;
mod auth;
mod auth_store;
mod catch_up;
mod channels;
mod saved;
mod saved_store;
mod search_api;
mod thread_list;
mod threads;
mod user_store;

use axum::{Json, Router, extract::State, middleware, routing::get};
use db::{EventStore, StoreError};
use domain::WorkspaceMode;
use search::SearchBackend;
use serde::Serialize;
use std::path::Path;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4000;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";
const DEFAULT_AUTH_STORE_PATH: &str = "logs/auth-identities.json";
const DEFAULT_SYNCED_USERS_PATH: &str = "logs/synced-users.json";
const DEFAULT_SAVED_ITEMS_PATH: &str = "logs/saved-items.json";
const DEFAULT_ANALYTICS_PATH: &str = "logs/analytics-events.json";

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
    pub(crate) store: EventStore,
    pub(crate) slack_auth: auth::SlackAuthConfig,
    pub(crate) session_secret: Option<String>,
    pub(crate) auth_store: auth_store::LocalAuthStore,
    pub(crate) user_store: user_store::LocalUserStore,
    pub(crate) saved_store: saved_store::LocalSavedItemStore,
    pub(crate) analytics_store: analytics_store::LocalAnalyticsStore,
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
    let store = EventStore::open(&config.event_log_path).await?;
    let auth_store = auth_store::LocalAuthStore::open(&config.auth_store_path)
        .await
        .map_err(auth_store_error_to_store_error)?;
    let user_store = user_store::LocalUserStore::open(&config.synced_users_path)
        .await
        .map_err(user_store_error_to_store_error)?;
    let saved_store = saved_store::LocalSavedItemStore::open(saved_items_path(&config))
        .await
        .map_err(saved_store_error_to_store_error)?;
    let analytics_store = analytics_store::LocalAnalyticsStore::open(analytics_path(&config))
        .await
        .map_err(analytics_store_error_to_store_error)?;
    let state = AppState {
        store,
        slack_auth: auth::SlackAuthConfig::from_config(&config),
        session_secret: config.session_secret.clone(),
        auth_store,
        user_store,
        saved_store,
        analytics_store,
    };
    let protected_api = Router::new()
        .route("/api/catch-up", get(catch_up::catch_up))
        .route("/api/channels", get(channels::channels))
        .route(
            "/api/saved",
            get(saved::list_saved_items).post(saved::save_item),
        )
        .route(
            "/api/saved/{id}",
            axum::routing::delete(saved::delete_saved_item),
        )
        .route("/api/search", get(search_api::search))
        .route("/api/threads", get(thread_list::thread_list))
        .route("/api/threads/{id}", get(threads::thread_detail))
        .route(
            "/api/analytics/events",
            axum::routing::post(analytics::record_event),
        )
        .route("/api/analytics/metrics", get(analytics::metrics))
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

fn analytics_store_error_to_store_error(error: analytics_store::AnalyticsStoreError) -> StoreError {
    match error {
        analytics_store::AnalyticsStoreError::Read(error)
        | analytics_store::AnalyticsStoreError::CreateDirectory(error)
        | analytics_store::AnalyticsStoreError::Write(error) => StoreError::Read(error),
        analytics_store::AnalyticsStoreError::Parse(error) => StoreError::Parse(error),
    }
}

fn saved_store_error_to_store_error(error: saved_store::SavedItemStoreError) -> StoreError {
    match error {
        saved_store::SavedItemStoreError::Read(error)
        | saved_store::SavedItemStoreError::CreateDirectory(error)
        | saved_store::SavedItemStoreError::Write(error) => StoreError::Read(error),
        saved_store::SavedItemStoreError::Parse(error) => StoreError::Parse(error),
    }
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "api",
        version: env!("CARGO_PKG_VERSION"),
        workspace_mode: WorkspaceMode::SingleWorkspace.as_str(),
        repository_mode: state.store.mode().as_str(),
        search_backend: SearchBackend::PostgresTsvectorPlaceholder.as_str(),
    })
}

fn read_port(key: &str, fallback: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(fallback)
}

fn saved_items_path(config: &ApiConfig) -> String {
    std::env::var("ARCHIVIST_SAVED_ITEMS_PATH").unwrap_or_else(|_| {
        Path::new(&config.auth_store_path)
            .with_file_name("saved-items.json")
            .display()
            .to_string()
            .if_empty(DEFAULT_SAVED_ITEMS_PATH)
    })
}

fn analytics_path(config: &ApiConfig) -> String {
    std::env::var("ARCHIVIST_ANALYTICS_PATH").unwrap_or_else(|_| {
        Path::new(&config.auth_store_path)
            .with_file_name("analytics-events.json")
            .display()
            .to_string()
            .if_empty(DEFAULT_ANALYTICS_PATH)
    })
}

trait DefaultIfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl DefaultIfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_owned()
        } else {
            self
        }
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
