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
        // 读取会议纪要文本，空值则走空字符串
        let summary_text = input
            .get("summary")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        // 如果启用了 LLM，则优先使用模型抽取，否则走本地规则解析
        let output = if let Some(config) = LlmConfig::from_env() {
            match LlmClient::new(config).extract_todos(summary_text).await {
                Ok(value) => value,
                Err(err) => {
                    return Err(AgentError::retryable(format!("llm error: {}", err)));
                }
            }
        } else {
            let todos = extract_todos(summary_text);
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
        // 产出结构化 todos 结果
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
        // 只识别带有 TODO/Action 的行（含常见标记和列表前缀）
        let lower = trimmed.to_ascii_lowercase();
        let text = if lower.starts_with("todo:") {
            trimmed[5..].trim()
        } else if lower.starts_with("action:") {
            trimmed[7..].trim()
        } else if lower.starts_with("action item:") {
            trimmed[12..].trim()
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            &trimmed[2..]
        } else {
            continue;
        };
        let action = text.trim().trim_end_matches('.');
        if !action.is_empty() {
            // 只输出 action 字段，owner/due 暂不做规则解析
            todos.push(json!({ "action": action }));
        }
    }
    todos
}
