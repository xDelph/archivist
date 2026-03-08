use domain::ProcessEventJob;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReceipt {
    pub endpoint: String,
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("worker base url cannot be empty")]
    EmptyWorkerBaseUrl,
    #[error("worker rejected the job with status {0}")]
    WorkerRejected(StatusCode),
    #[error("failed to publish process event job")]
    Publish(#[source] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct DirectQueue {
    client: reqwest::Client,
    endpoint: String,
}

impl DirectQueue {
    pub fn new(worker_base_url: &str) -> Result<Self, QueueError> {
        let endpoint = build_process_event_endpoint(worker_base_url)?;

        Ok(Self {
            client: reqwest::Client::new(),
            endpoint,
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
        })
    }
}

pub fn build_process_event_endpoint(worker_base_url: &str) -> Result<String, QueueError> {
    let trimmed = worker_base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(QueueError::EmptyWorkerBaseUrl);
    }

    Ok(format!("{trimmed}/jobs/process_event"))
}

#[cfg(test)]
mod tests {
    use super::{QueueError, build_process_event_endpoint};

    #[test]
    fn process_event_endpoint_is_normalized() {
        assert_eq!(
            build_process_event_endpoint("http://127.0.0.1:4002/").expect("endpoint"),
            "http://127.0.0.1:4002/jobs/process_event"
        );
    }

    #[test]
    fn empty_base_url_is_rejected() {
        assert!(matches!(
            build_process_event_endpoint("   "),
            Err(QueueError::EmptyWorkerBaseUrl)
        ));
    }
}
