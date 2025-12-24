use agent_runtime::types::{EventListResponse, Run, RunCreateRequest, RunCreateResponse};
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected status: {0}")]
    UnexpectedStatus(StatusCode),
}

#[derive(Clone)]
pub struct Client {
    base_url: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn create_run(
        &self,
        request: RunCreateRequest,
    ) -> Result<RunCreateResponse, ClientError> {
        let url = format!("{}/v1/runs", self.base_url.trim_end_matches('/'));
        let response = self.http.post(url).json(&request).send().await?;
        if response.status() != StatusCode::CREATED {
            return Err(ClientError::UnexpectedStatus(response.status()));
        }
        Ok(response.json::<RunCreateResponse>().await?)
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Run, ClientError> {
        let url = format!(
            "{}/v1/runs/{}",
            self.base_url.trim_end_matches('/'),
            run_id
        );
        let response = self.http.get(url).send().await?;
        if response.status() != StatusCode::OK {
            return Err(ClientError::UnexpectedStatus(response.status()));
        }
        Ok(response.json::<Run>().await?)
    }

    pub async fn list_events(&self, run_id: &str) -> Result<EventListResponse, ClientError> {
        let url = format!(
            "{}/v1/runs/{}/events",
            self.base_url.trim_end_matches('/'),
            run_id
        );
        let response = self.http.get(url).send().await?;
        if response.status() != StatusCode::OK {
            return Err(ClientError::UnexpectedStatus(response.status()));
        }
        Ok(response.json::<EventListResponse>().await?)
    }
}
