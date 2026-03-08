use axum::{Json, Router, routing::get};
use db::RepositoryMode;
use domain::WorkspaceMode;
use search::SearchBackend;
use serde::Serialize;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARCHIVIST_API_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARCHIVIST_API_PORT", DEFAULT_PORT),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    workspace_mode: &'static str,
    repository_mode: &'static str,
    search_backend: &'static str,
}

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
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
    use tower::util::ServiceExt;

    #[test]
    fn config_uses_defaults() {
        let config = ApiConfig::from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 4000);
        assert_eq!(config.bind_address(), "127.0.0.1:4000");
    }

    #[tokio::test]
    async fn health_route_reports_workspace_capabilities() {
        let response = build_router()
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
