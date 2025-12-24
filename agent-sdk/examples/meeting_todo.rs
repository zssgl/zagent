use agent_runtime::types::{RunCreateRequest, RunStatus, WorkflowRef};
use agent_sdk::client::Client;
use futures_util::StreamExt;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("AGENT_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let client = Client::new(base_url.clone()).with_bearer_auth("dev-token");

    let minutes = r#"
Meeting recap
- Action: Prepare Q2 budget draft by Friday
- Action item: Review vendor quotes and share summary
TODO: Schedule follow-up with product team
"#;

    let request = RunCreateRequest {
        workflow: WorkflowRef {
            name: "meeting-todo".to_string(),
            version: Some("0.1.0".to_string()),
        },
        input: json!({ "minutes": minutes }),
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
        .bearer_auth("dev-token")
        .send()
        .await?;
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut completed = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();
            let mut event_type = None;
            for line in event_block.lines() {
                if let Some(rest) = line.strip_prefix("event:") {
                    event_type = Some(rest.trim().to_string());
                }
            }
            if matches!(event_type.as_deref(), Some("run.completed") | Some("run.failed")) {
                completed = true;
                break;
            }
        }
        if completed {
            break;
        }
    }

    let run = client.get_run(&run_id).await?;
    if !matches!(run.status, RunStatus::Succeeded) {
        println!("status: {:?}", run.status);
        println!("error: {:?}", run.error);
        return Ok(());
    }

    println!("run_id: {}", run_id);
    println!("todos: {}", run.output.unwrap_or_else(|| json!({})));
    Ok(())
}
