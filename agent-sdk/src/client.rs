use agent_runtime::types::{ErrorResponse, EventListResponse, Run, RunCreateRequest, RunCreateResponse};
use futures_util::StreamExt;
use reqwest::StatusCode;
use tokio::time::{timeout, Duration};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected status: {0}")]
    UnexpectedStatus(StatusCode),
    #[error("api error: {0:?}")]
    Api(ErrorResponse),
    #[error("timed out waiting for completion")]
    Timeout,
    #[error("event stream ended before completion")]
    StreamEnded,
}

#[derive(Clone)]
pub struct Client {
    base_url: String,
    http: reqwest::Client,
    default_headers: reqwest::header::HeaderMap,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
            default_headers: reqwest::header::HeaderMap::new(),
        }
    }

    pub fn with_http(base_url: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            base_url: base_url.into(),
            http,
            default_headers: reqwest::header::HeaderMap::new(),
        }
    }

    pub fn with_bearer_auth(mut self, token: impl AsRef<str>) -> Self {
        let value = format!("Bearer {}", token.as_ref());
        if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value) {
            self.default_headers
                .insert(reqwest::header::AUTHORIZATION, header_value);
        }
        self
    }

    pub fn with_header(mut self, name: reqwest::header::HeaderName, value: reqwest::header::HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    pub async fn create_run(
        &self,
        request: RunCreateRequest,
    ) -> Result<RunCreateResponse, ClientError> {
        let url = format!("{}/v1/runs", self.base_url.trim_end_matches('/'));
        self.create_run_inner(None, request).await
    }

    pub async fn create_run_with_idempotency(
        &self,
        idempotency_key: impl AsRef<str>,
        request: RunCreateRequest,
    ) -> Result<RunCreateResponse, ClientError> {
        self.create_run_inner(Some(idempotency_key.as_ref()), request)
            .await
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Run, ClientError> {
        let url = format!(
            "{}/v1/runs/{}",
            self.base_url.trim_end_matches('/'),
            run_id
        );
        let response = self
            .http
            .get(url)
            .headers(self.default_headers.clone())
            .send()
            .await?;
        self.handle_response(response, StatusCode::OK).await
    }

    pub async fn list_events(&self, run_id: &str) -> Result<EventListResponse, ClientError> {
        let url = format!(
            "{}/v1/runs/{}/events",
            self.base_url.trim_end_matches('/'),
            run_id
        );
        let response = self
            .http
            .get(url)
            .headers(self.default_headers.clone())
            .send()
            .await?;
        self.handle_response(response, StatusCode::OK).await
    }

    pub async fn wait_for_completion(
        &self,
        run_id: &str,
        timeout_ms: u64,
    ) -> Result<Run, ClientError> {
        let url = format!(
            "{}/v1/runs/{}/events",
            self.base_url.trim_end_matches('/'),
            run_id
        );
        let response = self
            .http
            .get(url)
            .headers(self.default_headers.clone())
            .header("accept", "text/event-stream")
            .send()
            .await?;
        if response.status() != StatusCode::OK {
            if let Ok(api_error) = response.json::<ApiErrorEnvelope>().await {
                return Err(ClientError::Api(api_error.error));
            }
            return Err(ClientError::UnexpectedStatus(response.status()));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let wait = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);
                // SSE 事件以空行分隔，这里按 "\n\n" 切分完整事件块。
                while let Some(pos) = buffer.find("\n\n") {
                    let event_block = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();
                    if let Some(event_type) = sse_event_type(&event_block) {
                        if matches!(event_type, "run.completed" | "run.failed") {
                            return Ok(());
                        }
                    }
                }
            }
            Err(ClientError::StreamEnded)
        };

        match timeout(Duration::from_millis(timeout_ms), wait).await {
            Ok(Ok(())) => self.get_run(run_id).await,
            Ok(Err(err)) => Err(err),
            Err(_) => Err(ClientError::Timeout),
        }
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
        expected: StatusCode,
    ) -> Result<T, ClientError> {
        let status = response.status();
        if status == expected {
            return Ok(response.json::<T>().await?);
        }
        if let Ok(api_error) = response.json::<ApiErrorEnvelope>().await {
            return Err(ClientError::Api(api_error.error));
        }
        Err(ClientError::UnexpectedStatus(status))
    }

    async fn create_run_inner(
        &self,
        idempotency_key: Option<&str>,
        request: RunCreateRequest,
    ) -> Result<RunCreateResponse, ClientError> {
        let url = format!("{}/v1/runs", self.base_url.trim_end_matches('/'));
        let mut req = self
            .http
            .post(url)
            .headers(self.default_headers.clone())
            .json(&request);
        if let Some(key) = idempotency_key {
            req = req.header("Idempotency-Key", key);
        }
        let response = req.send().await?;
        self.handle_response(response, StatusCode::CREATED).await
    }
}

fn sse_event_type(event_block: &str) -> Option<&str> {
    for line in event_block.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            return Some(rest.trim());
        }
    }
    None
}

#[derive(Debug, serde::Deserialize)]
struct ApiErrorEnvelope {
    error: ErrorResponse,
}
