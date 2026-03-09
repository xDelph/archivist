use domain::ProcessEventJob;
use reqwest::StatusCode;
use thiserror::Error;

pub const DEFAULT_QSTASH_BASE_URL: &str = "https://qstash.upstash.io";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueMode {
    Direct,
    QStash,
}

impl QueueMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::QStash => "qstash",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReceipt {
    pub endpoint: String,
    pub mode: QueueMode,
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("worker base url cannot be empty")]
    EmptyWorkerBaseUrl,
    #[error("qstash token cannot be empty")]
    EmptyQStashToken,
    #[error("worker rejected the job with status {0}")]
    WorkerRejected(StatusCode),
    #[error("qstash rejected the job with status {0}")]
    QStashRejected(StatusCode),
    #[error("failed to publish process event job")]
    Publish(#[source] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct ProcessEventQueue {
    inner: QueueClient,
}

impl ProcessEventQueue {
    pub fn new(
        worker_base_url: &str,
        qstash_base_url: Option<&str>,
        qstash_token: Option<&str>,
    ) -> Result<Self, QueueError> {
        let inner = match qstash_token
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
            Some(token) => {
                QueueClient::QStash(QStashQueue::new(worker_base_url, qstash_base_url, token)?)
            }
            None => QueueClient::Direct(DirectQueue::new(worker_base_url)?),
        };

        Ok(Self { inner })
    }

    pub fn endpoint(&self) -> &str {
        match &self.inner {
            QueueClient::Direct(queue) => queue.endpoint(),
            QueueClient::QStash(queue) => queue.endpoint(),
        }
    }

    pub fn mode(&self) -> QueueMode {
        match &self.inner {
            QueueClient::Direct(_) => QueueMode::Direct,
            QueueClient::QStash(_) => QueueMode::QStash,
        }
    }

    pub async fn publish_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<PublishReceipt, QueueError> {
        match &self.inner {
            QueueClient::Direct(queue) => queue.publish_process_event(job).await,
            QueueClient::QStash(queue) => queue.publish_process_event(job).await,
        }
    }
}

#[derive(Debug, Clone)]
enum QueueClient {
    Direct(DirectQueue),
    QStash(QStashQueue),
}

#[derive(Debug, Clone)]
pub struct DirectQueue {
    client: reqwest::Client,
    endpoint: String,
}

impl DirectQueue {
    pub fn new(worker_base_url: &str) -> Result<Self, QueueError> {
        Ok(Self {
            client: reqwest::Client::new(),
            endpoint: build_process_event_endpoint(worker_base_url)?,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn publish_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<PublishReceipt, QueueError> {
        let response = self
            .client
            .post(&self.endpoint)
            .json(job)
            .send()
            .await
            .map_err(QueueError::Publish)?;

        if !response.status().is_success() {
            return Err(QueueError::WorkerRejected(response.status()));
        }

        Ok(PublishReceipt {
            endpoint: self.endpoint.clone(),
            mode: QueueMode::Direct,
        })
    }
}

#[derive(Debug, Clone)]
pub struct QStashQueue {
    client: reqwest::Client,
    endpoint: String,
    publish_url: String,
    token: String,
}

impl QStashQueue {
    pub fn new(
        worker_base_url: &str,
        qstash_base_url: Option<&str>,
        qstash_token: &str,
    ) -> Result<Self, QueueError> {
        let token = qstash_token.trim();
        if token.is_empty() {
            return Err(QueueError::EmptyQStashToken);
        }

        let endpoint = build_process_event_endpoint(worker_base_url)?;
        let publish_url = build_qstash_publish_url(qstash_base_url, &endpoint);

        Ok(Self {
            client: reqwest::Client::new(),
            endpoint,
            publish_url,
            token: token.to_owned(),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn publish_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<PublishReceipt, QueueError> {
        let response = self
            .client
            .post(&self.publish_url)
            .bearer_auth(&self.token)
            .json(job)
            .send()
            .await
            .map_err(QueueError::Publish)?;

        if !response.status().is_success() {
            return Err(QueueError::QStashRejected(response.status()));
        }

        Ok(PublishReceipt {
            endpoint: self.endpoint.clone(),
            mode: QueueMode::QStash,
        })
    }
}

pub fn build_process_event_endpoint(worker_base_url: &str) -> Result<String, QueueError> {
    build_worker_endpoint(worker_base_url, "/jobs/process_event")
}

pub fn build_heartbeat_endpoint(worker_base_url: &str) -> Result<String, QueueError> {
    build_worker_endpoint(worker_base_url, "/jobs/heartbeat")
}

pub fn build_refresh_thread_summaries_endpoint(
    worker_base_url: &str,
) -> Result<String, QueueError> {
    build_worker_endpoint(worker_base_url, "/jobs/refresh_thread_summaries")
}

fn build_worker_endpoint(worker_base_url: &str, path: &str) -> Result<String, QueueError> {
    let trimmed = worker_base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(QueueError::EmptyWorkerBaseUrl);
    }

    Ok(format!("{trimmed}{path}"))
}

fn build_qstash_publish_url(qstash_base_url: Option<&str>, endpoint: &str) -> String {
    let base_url = normalize_qstash_base_url(qstash_base_url);
    format!("{base_url}/v2/publish/{endpoint}")
}

fn normalize_qstash_base_url(qstash_base_url: Option<&str>) -> String {
    let raw = qstash_base_url.unwrap_or(DEFAULT_QSTASH_BASE_URL);
    let trimmed = raw
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .trim_end_matches('/');

    if trimmed.is_empty() {
        return DEFAULT_QSTASH_BASE_URL.to_owned();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_owned();
    }

    format!("https://{trimmed}")
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_QSTASH_BASE_URL, ProcessEventQueue, QStashQueue, QueueError, QueueMode,
        build_heartbeat_endpoint, build_process_event_endpoint,
        build_refresh_thread_summaries_endpoint,
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
            build_refresh_thread_summaries_endpoint("http://127.0.0.1:4002/")
                .expect("endpoint"),
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
}
