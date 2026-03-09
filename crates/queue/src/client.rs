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
#[path = "client_tests.rs"]
mod tests;
