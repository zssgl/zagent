use agent_runtime::types::{RunCreateRequest, RunStatus, WorkflowRef};
use agent_sdk::client::Client;
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

    let run = client.wait_for_completion(&run_id, 10_000).await?;
    if !matches!(run.status, RunStatus::Succeeded) {
        println!("status: {:?}", run.status);
        println!("error: {:?}", run.error);
        return Ok(());
    }

    println!("run_id: {}", run_id);
    println!("todos: {}", run.output.unwrap_or_else(|| json!({})));
    Ok(())
}
