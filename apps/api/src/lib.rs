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

use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, CONTENT_TYPE},
    },
    middleware,
    routing::get,
};
#[cfg(test)]
use db::JsonlEventStore;
use db::{EventStore, StoreError};
use domain::WorkspaceMode;
use search::SearchBackend;
use serde::Serialize;
#[cfg(test)]
use std::path::Path;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4000;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";
const DEFAULT_AUTH_STORE_PATH: &str = "logs/auth-identities.json";
const DEFAULT_SYNCED_USERS_PATH: &str = "logs/synced-users.json";
pub(crate) const LOCAL_DEV_WEB_ORIGIN: &str = "http://127.0.0.1:3001";
pub(crate) const LOCALHOST_WEB_ORIGIN: &str = "http://localhost:3001";
const DEFAULT_WEB_ORIGIN: &str = LOCAL_DEV_WEB_ORIGIN;
#[cfg(test)]
const DEFAULT_SAVED_ITEMS_PATH: &str = "logs/saved-items.json";
#[cfg(test)]
const DEFAULT_ANALYTICS_PATH: &str = "logs/analytics-events.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub event_log_path: String,
    pub slack_client_id: Option<String>,
    pub slack_client_secret: Option<String>,
    pub slack_redirect_uri: Option<String>,
    pub slack_token_url: Option<String>,
    pub session_secret: Option<String>,
    pub auth_store_path: String,
    pub synced_users_path: String,
    pub web_origin: String,
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
            slack_token_url: std::env::var("SLACK_OIDC_TOKEN_URL").ok(),
            session_secret: std::env::var("ARCHIVIST_SESSION_SECRET").ok(),
            auth_store_path: std::env::var("ARCHIVIST_AUTH_STORE_PATH")
                .unwrap_or_else(|_| DEFAULT_AUTH_STORE_PATH.to_owned()),
            synced_users_path: std::env::var("ARCHIVIST_SYNCED_USERS_PATH")
                .unwrap_or_else(|_| DEFAULT_SYNCED_USERS_PATH.to_owned()),
            web_origin: std::env::var("ARCHIVIST_WEB_ORIGIN")
                .unwrap_or_else(|_| DEFAULT_WEB_ORIGIN.to_owned()),
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
    pub(crate) web_origin: String,
    pub(crate) session_secret: Option<String>,
    pub(crate) auth_store: auth_store::AuthStore,
    pub(crate) user_store: user_store::UserStore,
    pub(crate) saved_store: saved_store::SavedItemStore,
    pub(crate) analytics_store: analytics_store::AnalyticsStore,
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
    let store = open_store(&config).await?;
    let auth_store = open_auth_store(&config, &store).await?;
    let user_store = open_user_store(&config, &store).await?;
    let saved_store = open_saved_store(&config, &store).await?;
    let analytics_store = open_analytics_store(&config, &store).await?;
    let state = AppState {
        store,
        slack_auth: auth::SlackAuthConfig::from_config(&config),
        web_origin: config.web_origin.clone(),
        session_secret: config.session_secret.clone(),
        auth_store,
        user_store,
        saved_store,
        analytics_store,
    };
    let web_origins = allowlisted_web_origins(&config.web_origin)?;
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([ACCEPT, CONTENT_TYPE])
        .allow_credentials(true)
        .allow_origin(AllowOrigin::list(web_origins));
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
        .layer(cors)
        .layer(TraceLayer::new_for_http()))
}

fn allowlisted_web_origins(web_origin: &str) -> Result<Vec<HeaderValue>, StoreError> {
    let mut origins = vec![web_origin.to_owned()];
    if web_origin == LOCAL_DEV_WEB_ORIGIN {
        origins.push(LOCALHOST_WEB_ORIGIN.to_owned());
    } else if web_origin == LOCALHOST_WEB_ORIGIN {
        origins.push(LOCAL_DEV_WEB_ORIGIN.to_owned());
    }

    origins
        .into_iter()
        .map(|origin| {
            HeaderValue::from_str(&origin)
                .map_err(|_| StoreError::InvalidRuntimeConfig("ARCHIVIST_WEB_ORIGIN"))
        })
        .collect()
}

#[cfg(test)]
async fn open_store(config: &ApiConfig) -> Result<EventStore, StoreError> {
    JsonlEventStore::open(&config.event_log_path)
        .await
        .map(Into::into)
}

#[cfg(not(test))]
async fn open_store(config: &ApiConfig) -> Result<EventStore, StoreError> {
    EventStore::open(&config.event_log_path).await
}

#[cfg(test)]
async fn open_auth_store(
    config: &ApiConfig,
    _store: &EventStore,
) -> Result<auth_store::AuthStore, StoreError> {
    auth_store::LocalAuthStore::open(&config.auth_store_path)
        .await
        .map(Into::into)
        .map_err(auth_store_error_to_store_error)
}

#[cfg(not(test))]
async fn open_auth_store(
    _config: &ApiConfig,
    store: &EventStore,
) -> Result<auth_store::AuthStore, StoreError> {
    let pool = store
        .postgres_pool()
        .ok_or(StoreError::MissingDatabaseUrl)?;
    Ok(auth_store::PostgresAuthStore::new(pool).into())
}

#[cfg(test)]
async fn open_user_store(
    config: &ApiConfig,
    _store: &EventStore,
) -> Result<user_store::UserStore, StoreError> {
    user_store::LocalUserStore::open(&config.synced_users_path)
        .await
        .map(Into::into)
        .map_err(user_store_error_to_store_error)
}

#[cfg(not(test))]
async fn open_user_store(
    _config: &ApiConfig,
    store: &EventStore,
) -> Result<user_store::UserStore, StoreError> {
    let pool = store
        .postgres_pool()
        .ok_or(StoreError::MissingDatabaseUrl)?;
    Ok(user_store::PostgresUserStore::new(pool).into())
}

#[cfg(test)]
async fn open_saved_store(
    config: &ApiConfig,
    _store: &EventStore,
) -> Result<saved_store::SavedItemStore, StoreError> {
    saved_store::LocalSavedItemStore::open(saved_items_path(config))
        .await
        .map(Into::into)
        .map_err(saved_store_error_to_store_error)
}

#[cfg(not(test))]
async fn open_saved_store(
    _config: &ApiConfig,
    store: &EventStore,
) -> Result<saved_store::SavedItemStore, StoreError> {
    let pool = store
        .postgres_pool()
        .ok_or(StoreError::MissingDatabaseUrl)?;
    Ok(saved_store::PostgresSavedItemStore::new(pool).into())
}

#[cfg(test)]
async fn open_analytics_store(
    config: &ApiConfig,
    _store: &EventStore,
) -> Result<analytics_store::AnalyticsStore, StoreError> {
    analytics_store::LocalAnalyticsStore::open(analytics_path(config))
        .await
        .map(Into::into)
        .map_err(analytics_store_error_to_store_error)
}

#[cfg(not(test))]
async fn open_analytics_store(
    _config: &ApiConfig,
    store: &EventStore,
) -> Result<analytics_store::AnalyticsStore, StoreError> {
    let pool = store
        .postgres_pool()
        .ok_or(StoreError::MissingDatabaseUrl)?;
    Ok(analytics_store::PostgresAnalyticsStore::new(pool).into())
}

#[cfg(test)]
fn auth_store_error_to_store_error(error: auth_store::AuthStoreError) -> StoreError {
    match error {
        auth_store::AuthStoreError::Read(error)
        | auth_store::AuthStoreError::CreateDirectory(error)
        | auth_store::AuthStoreError::Write(error) => StoreError::Read(error),
        auth_store::AuthStoreError::Sqlx(error) => StoreError::Sqlx(error),
        auth_store::AuthStoreError::Parse(error) => StoreError::Parse(error),
    }
}

#[cfg(test)]
fn user_store_error_to_store_error(error: user_store::UserStoreError) -> StoreError {
    match error {
        user_store::UserStoreError::Read(error) => StoreError::Read(error),
        user_store::UserStoreError::Parse(error) => StoreError::Parse(error),
        #[cfg(test)]
        user_store::UserStoreError::CreateDirectory(error)
        | user_store::UserStoreError::Write(error) => StoreError::Read(error),
    }
}

#[cfg(test)]
fn analytics_store_error_to_store_error(error: analytics_store::AnalyticsStoreError) -> StoreError {
    match error {
        analytics_store::AnalyticsStoreError::Read(error)
        | analytics_store::AnalyticsStoreError::CreateDirectory(error)
        | analytics_store::AnalyticsStoreError::Write(error) => StoreError::Read(error),
        analytics_store::AnalyticsStoreError::Sqlx(error) => StoreError::Sqlx(error),
        analytics_store::AnalyticsStoreError::Parse(error) => StoreError::Parse(error),
    }
}

#[cfg(test)]
fn saved_store_error_to_store_error(error: saved_store::SavedItemStoreError) -> StoreError {
    match error {
        saved_store::SavedItemStoreError::Read(error)
        | saved_store::SavedItemStoreError::CreateDirectory(error)
        | saved_store::SavedItemStoreError::Write(error) => StoreError::Read(error),
        saved_store::SavedItemStoreError::Sqlx(error) => StoreError::Sqlx(error),
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

#[cfg(test)]
fn saved_items_path(config: &ApiConfig) -> String {
    std::env::var("ARCHIVIST_SAVED_ITEMS_PATH").unwrap_or_else(|_| {
        Path::new(&config.auth_store_path)
            .with_file_name("saved-items.json")
            .display()
            .to_string()
            .if_empty(DEFAULT_SAVED_ITEMS_PATH)
    })
}

#[cfg(test)]
fn analytics_path(config: &ApiConfig) -> String {
    std::env::var("ARCHIVIST_ANALYTICS_PATH").unwrap_or_else(|_| {
        Path::new(&config.auth_store_path)
            .with_file_name("analytics-events.json")
            .display()
            .to_string()
            .if_empty(DEFAULT_ANALYTICS_PATH)
    })
}

#[cfg(test)]
trait DefaultIfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

#[cfg(test)]
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
