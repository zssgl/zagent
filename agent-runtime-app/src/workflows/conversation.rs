use std::collections::HashMap;

use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use agent_runtime::types::{Artifact, ArtifactType};
use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm::{LlmClient, LlmConfig, LlmMessage};

static CONVERSATIONS: Lazy<RwLock<HashMap<String, Vec<LlmMessage>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Deserialize)]
struct ConversationInput {
    conversation_id: Option<String>,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

pub struct ConversationWorkflow;

#[async_trait::async_trait]
impl WorkflowRunner for ConversationWorkflow {
    fn name(&self) -> &'static str {
        "conversation"
    }

    fn version(&self) -> Option<&'static str> {
        Some("0.1.0")
    }

    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError> {
        let parsed: ConversationInput =
            serde_json::from_value(input).map_err(|err| AgentError::Fatal(err.to_string()))?;

        let conversation_id = parsed
            .conversation_id
            .unwrap_or_else(|| format!("conv_{}", Uuid::new_v4()));

        let mut history = {
            let store = CONVERSATIONS.read().await;
            store.get(&conversation_id).cloned().unwrap_or_default()
        };

        let mut new_messages: Vec<LlmMessage> = parsed
            .messages
            .into_iter()
            .map(|msg| LlmMessage {
                role: msg.role,
                content: msg.content,
            })
            .collect();
        history.append(&mut new_messages);

        let config = LlmConfig::from_env().ok_or_else(|| {
            AgentError::Fatal("LLM is not enabled; set LLM_ENABLED=1".to_string())
        })?;
        let client = LlmClient::new(config);
        let reply = client
            .chat(&history)
            .await
            .map_err(|err| AgentError::Retryable(format!("llm error: {}", err)))?;

        history.push(LlmMessage {
            role: "assistant".to_string(),
            content: reply.clone(),
        });

        {
            let mut store = CONVERSATIONS.write().await;
            store.insert(conversation_id.clone(), history.clone());
        }

        let output = json!({
            "conversation_id": conversation_id,
            "reply": reply,
            "messages": history,
        });
        let artifact = Artifact {
            artifact_id: format!("art_{}", Uuid::new_v4()),
            r#type: ArtifactType::Record,
            name: Some("conversation".to_string()),
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
