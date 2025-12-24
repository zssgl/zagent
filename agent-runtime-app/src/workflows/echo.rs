use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use agent_runtime::types::{Artifact, ArtifactType};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct EchoWorkflow;

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
