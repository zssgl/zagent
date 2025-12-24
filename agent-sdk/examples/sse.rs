use agent_runtime::types::{RunCreateRequest, WorkflowRef};
use agent_sdk::client::Client;
use futures_util::StreamExt;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("AGENT_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let client = Client::new(base_url.clone());
    let request = RunCreateRequest {
        workflow: WorkflowRef {
            name: "echo".to_string(),
            version: Some("0.1.0".to_string()),
        },
        input: json!({ "hello": "sse" }),
        context: None,
        metadata: None,
        labels: None,
    };

    let created = client.create_run(request).await?;
    let run_id = created.run.run_id;

    let url = format!("{}/v1/runs/{}/events", base_url.trim_end_matches('/'), run_id);
    let http = reqwest::Client::new();
    let response = http
        .get(url)
        .header("accept", "text/event-stream")
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    println!("streaming events (Ctrl+C to stop)...");
    while let Some(item) = stream.next().await {
        let chunk = item?;
        let text = String::from_utf8_lossy(&chunk);
        print!("{}", text);
    }
    Ok(())
}
