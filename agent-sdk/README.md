# Agent SDK (Rust)

Minimal Rust SDK for the Agent Runtime API. This SDK focuses on run lifecycle and events.

## Install (local workspace)

```toml
[dependencies]
agent_sdk = { path = "../agent-sdk" }
```

## Quick start

```rust
use agent_runtime::types::{RunCreateRequest, WorkflowRef};
use agent_sdk::client::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("http://127.0.0.1:9000")
        .with_bearer_auth("dev-token");
    let request = RunCreateRequest {
        workflow: WorkflowRef {
            name: "echo".to_string(),
            version: Some("0.1.0".to_string()),
        },
        input: json!({ "hello": "sdk" }),
        context: None,
        metadata: None,
        labels: None,
    };

    let created = client.create_run(request).await?;
    println!("run_id: {}", created.run.run_id);

    let run = client.get_run(&created.run.run_id).await?;
    println!("status: {:?}", run.status);

    let events = client.list_events(&created.run.run_id).await?;
    println!("events: {}", events.data.len());
    Ok(())
}
```

## SSE streaming example

```bash
cargo run -p agent_sdk --example sse
```

## Meeting minutes to todo example

```bash
cargo run -p agent_sdk --example meeting_todo
```

You can override the base URL:

```bash
AGENT_BASE_URL=http://127.0.0.1:9000 cargo run -p agent_sdk --example sse
```

## API

- `Client::new(base_url)`
- `Client::with_http(base_url, reqwest::Client)`
- `Client::with_bearer_auth(token)`
- `Client::with_header(name, value)`
- `Client::create_run(RunCreateRequest)`
- `Client::create_run_with_idempotency(idempotency_key, RunCreateRequest)`
- `Client::get_run(run_id)`
- `Client::list_events(run_id)`
- `Client::wait_for_completion(run_id, timeout_ms)`

## Notes

- The SDK returns `ClientError::Api` when the server provides a structured error response.
- Retries are not implemented yet (planned).
