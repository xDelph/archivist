use super::{parse_link_metadata, parse_metadata_url};
use crate::{
    ApiConfig,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    build_router,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::get,
};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[test]
fn parse_metadata_url_rejects_non_http_urls() {
    assert!(parse_metadata_url("https://example.com").is_some());
    assert!(parse_metadata_url("http://example.com").is_some());
    assert!(parse_metadata_url("mailto:test@example.com").is_none());
    assert!(parse_metadata_url("javascript:alert(1)").is_none());
}

#[test]
fn parse_link_metadata_reads_open_graph_fields() {
    let url = reqwest::Url::parse("https://example.com/post").expect("url");
    let metadata = parse_link_metadata(
        &url,
        r#"
        <html>
            <head>
                <title>Ignored title</title>
                <meta property="og:title" content="Launch plan" />
                <meta property="og:description" content="Ship checklist and owners." />
                <meta property="og:site_name" content="Example Docs" />
                <meta property="og:image" content="/assets/preview.png" />
            </head>
        </html>
        "#,
    );

    assert_eq!(metadata.title.as_deref(), Some("Launch plan"));
    assert_eq!(
        metadata.description.as_deref(),
        Some("Ship checklist and owners.")
    );
    assert_eq!(metadata.site_name.as_deref(), Some("Example Docs"));
    assert_eq!(
        metadata.image.as_deref(),
        Some("https://example.com/assets/preview.png")
    );
}

#[tokio::test]
async fn link_metadata_route_returns_metadata_for_authenticated_users() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token");
    let (mock_url, handle) = spawn_preview_server().await;

    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_token_url: None,
        session_secret: Some("session_secret".to_owned()),
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
        web_origin: crate::LOCAL_DEV_WEB_ORIGIN.to_owned(),
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri(format!(
                "/api/link-metadata?url={}",
                urlencoding::encode(&mock_url)
            ))
            .header("cookie", format!("archivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["title"], "Mock preview title");
    assert_eq!(payload["site_name"], "Mock Site");
    assert_eq!(payload["image"], format!("{mock_url}/images/card.png"));
}

async fn spawn_preview_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("local addr");
    let router = Router::new().route(
        "/",
        get(|| async {
            (
                [("content-type", "text/html; charset=utf-8")],
                r#"
                <html>
                    <head>
                        <meta property="og:title" content="Mock preview title" />
                        <meta property="og:description" content="Short summary." />
                        <meta property="og:site_name" content="Mock Site" />
                        <meta property="og:image" content="/images/card.png" />
                    </head>
                </html>
                "#,
            )
        }),
    );
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    });

    (format!("http://{address}"), handle)
}
