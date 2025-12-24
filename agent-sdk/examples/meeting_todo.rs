use agent_runtime::types::{RunCreateRequest, RunStatus, WorkflowRef};
use agent_sdk::client::Client;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("AGENT_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let client = Client::new(base_url).with_bearer_auth("dev-token");

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

    let mut output = None;
    for _ in 0..60 {  // Increased to 60 iterations (3 seconds total)
        let run = client.get_run(&run_id).await?;
        if matches!(run.status, RunStatus::Succeeded) {
            output = run.output;
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    println!("run_id: {}", run_id);
    println!("todos: {}", output.unwrap_or_else(|| json!({})));
    Ok(())
}
