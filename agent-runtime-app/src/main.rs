use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::{
    AgentError, InMemoryRuntime, WorkflowOutput, WorkflowRunner,
};
use agent_runtime::server::router;
use agent_runtime::types::{Artifact, ArtifactType};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime
        .register_workflow_with_schemas(
            Arc::new(EchoWorkflow),
            Some(json!({
                "type": "object",
                "properties": {
                    "hello": { "type": "string" }
                },
                "required": ["hello"]
            })),
            Some(json!({
                "type": "object",
                "properties": {
                    "echo": { "type": "object" }
                },
                "required": ["echo"]
            })),
        )
        .await;
    runtime
        .register_workflow_with_schemas(
            Arc::new(MeetingTodoWorkflow),
            Some(json!({
                "type": "object",
                "properties": {
                    "minutes": { "type": "string" }
                },
                "required": ["minutes"]
            })),
            Some(json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "action": { "type": "string" },
                                "owner": { "type": "string" },
                                "due": { "type": "string" }
                            },
                            "required": ["action"]
                        }
                    }
                },
                "required": ["todos"]
            })),
        )
        .await;

    let app = router(runtime);
    let addr: SocketAddr = "127.0.0.1:9000".parse().expect("valid addr");
    println!("agent runtime listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

struct EchoWorkflow;

#[async_trait::async_trait]
impl WorkflowRunner for EchoWorkflow {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn version(&self) -> Option<&'static str> {
        Some("0.1.0")
    }

    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError> {
        let artifact = Artifact {
            artifact_id: format!("art_{}", Uuid::new_v4()),
            r#type: ArtifactType::Record,
            name: Some("echo".to_string()),
            created_at: Utc::now(),
            mime_type: Some("application/json".to_string()),
            data: Some(json!({ "echo": input })),
            file: None,
        };
        Ok(WorkflowOutput {
            output: json!({ "echo": input }),
            artifacts: vec![artifact],
        })
    }
}

struct MeetingTodoWorkflow;

#[async_trait::async_trait]
impl WorkflowRunner for MeetingTodoWorkflow {
    fn name(&self) -> &'static str {
        "meeting-todo"
    }

    fn version(&self) -> Option<&'static str> {
        Some("0.1.0")
    }

    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError> {
        let minutes = input
            .get("minutes")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let todos = extract_todos(minutes);
        let output = json!({ "todos": todos });
        let artifact = Artifact {
            artifact_id: format!("art_{}", Uuid::new_v4()),
            r#type: ArtifactType::Record,
            name: Some("meeting-todo".to_string()),
            created_at: Utc::now(),
            mime_type: Some("application/json".to_string()),
            data: Some(output.clone()),
            file: None,
        };
        Ok(WorkflowOutput {
            output,
            artifacts: vec![artifact],
        })
    }
}

fn extract_todos(minutes: &str) -> Vec<Value> {
    let mut todos = Vec::new();
    for line in minutes.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut text = trimmed;
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("todo:") {
            text = trimmed[5..].trim();
        } else if lower.starts_with("action:") {
            text = trimmed[7..].trim();
        } else if lower.starts_with("action item:") {
            text = trimmed[12..].trim();
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            text = &trimmed[2..];
        } else {
            continue;
        }
        let action = text.trim().trim_end_matches('.');
        if !action.is_empty() {
            todos.push(json!({ "action": action }));
        }
    }
    todos
}
