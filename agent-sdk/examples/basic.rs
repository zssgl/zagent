use agent_runtime::types::{RunCreateRequest, WorkflowRef};
use agent_sdk::client::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("AGENT_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let client = Client::new(base_url);
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
