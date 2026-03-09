use super::{
    DEFAULT_QSTASH_BASE_URL, ProcessEventQueue, QStashQueue, QueueError, QueueMode,
    build_heartbeat_endpoint, build_process_event_endpoint, build_refresh_thread_summaries_endpoint,
};
use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[test]
fn process_event_endpoint_is_normalized() {
    assert_eq!(
        build_process_event_endpoint("http://127.0.0.1:4002/").expect("endpoint"),
        "http://127.0.0.1:4002/jobs/process_event"
    );
    assert_eq!(
        build_refresh_thread_summaries_endpoint("http://127.0.0.1:4002/").expect("endpoint"),
        "http://127.0.0.1:4002/jobs/refresh_thread_summaries"
    );
}

#[test]
fn heartbeat_endpoint_is_normalized() {
    assert_eq!(
        build_heartbeat_endpoint("http://127.0.0.1:4002/").expect("endpoint"),
        "http://127.0.0.1:4002/jobs/heartbeat"
    );
}

#[test]
fn empty_base_url_is_rejected() {
    assert!(matches!(
        build_process_event_endpoint("   "),
        Err(QueueError::EmptyWorkerBaseUrl)
    ));
}

#[test]
fn queue_defaults_to_direct_mode() {
    let queue = ProcessEventQueue::new("http://127.0.0.1:4002", None, None).expect("queue");

    assert_eq!(queue.mode(), QueueMode::Direct);
    assert_eq!(queue.endpoint(), "http://127.0.0.1:4002/jobs/process_event");
}

#[test]
fn queue_switches_to_qstash_mode_when_token_is_present() {
    let queue = ProcessEventQueue::new(
        "http://127.0.0.1:4002",
        Some("qstash.upstash.io"),
        Some("secret"),
    )
    .expect("queue");

    assert_eq!(queue.mode(), QueueMode::QStash);
    assert_eq!(queue.endpoint(), "http://127.0.0.1:4002/jobs/process_event");
}

#[tokio::test]
async fn qstash_publish_posts_to_publish_endpoint() {
    let state = RequestCapture::default();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("listener address");
    let app = Router::new()
        .route("/v2/publish/{*destination}", post(capture_request))
        .with_state(state.clone());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let queue = QStashQueue::new(
        "https://worker.example.com",
        Some(&format!("http://{address}")),
        "secret",
    )
    .expect("queue");
    let job = ProcessEventJob {
        event_id: "evt_1".to_owned(),
        team_id: "T123".to_owned(),
        event_time: 1,
        received_at: 2,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("hello".to_owned()),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            files: vec![],
        },
    };

    let receipt = queue
        .publish_process_event(&job)
        .await
        .expect("publish receipt");
    let captured = state.take().expect("captured request");

    handle.abort();

    assert_eq!(receipt.mode, QueueMode::QStash);
    assert_eq!(
        receipt.endpoint,
        "https://worker.example.com/jobs/process_event"
    );
    assert_eq!(
        captured.destination,
        "https://worker.example.com/jobs/process_event"
    );
    assert_eq!(captured.authorization.as_deref(), Some("Bearer secret"));
    assert_eq!(captured.content_type.as_deref(), Some("application/json"));
    assert_eq!(captured.body, serde_json::to_vec(&job).expect("job json"));
}

#[test]
fn empty_qstash_base_url_falls_back_to_default() {
    let queue =
        QStashQueue::new("https://worker.example.com", Some("   "), "secret").expect("queue");

    assert!(
        queue
            .publish_url
            .starts_with(&format!("{DEFAULT_QSTASH_BASE_URL}/v2/publish/"))
    );
}

#[derive(Debug, Clone, Default)]
struct RequestCapture {
    inner: Arc<Mutex<Vec<CapturedRequest>>>,
}

impl RequestCapture {
    fn push(&self, request: CapturedRequest) {
        self.inner.lock().expect("capture lock").push(request);
    }

    fn take(&self) -> Option<CapturedRequest> {
        self.inner.lock().expect("capture lock").pop()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedRequest {
    destination: String,
    authorization: Option<String>,
    content_type: Option<String>,
    body: Vec<u8>,
}

async fn capture_request(
    State(state): State<RequestCapture>,
    Path(destination): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) {
    state.push(CapturedRequest {
        destination,
        authorization: header_value(&headers, "authorization"),
        content_type: header_value(&headers, "content-type"),
        body: body.to_vec(),
    });
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
