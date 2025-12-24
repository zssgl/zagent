use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use agent_runtime::types::{Artifact, ArtifactType};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::llm::{LlmClient, LlmConfig};

pub struct MeetingTodoWorkflow;

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
        let output = if let Some(config) = LlmConfig::from_env() {
            match LlmClient::new(config).extract_todos(minutes).await {
                Ok(value) => value,
                Err(err) => {
                    return Err(AgentError::Retryable(format!("llm error: {}", err)));
                }
            }
        } else {
            let todos = extract_todos(minutes);
            json!({ "todos": todos })
        };
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
