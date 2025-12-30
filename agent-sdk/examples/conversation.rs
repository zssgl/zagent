use agent_runtime::types::{RunCreateRequest, RunStatus, WorkflowRef};
use agent_sdk::client::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("AGENT_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let client = Client::new(base_url).with_bearer_auth("dev-token");

    // 第一轮对话：不传 conversation_id，服务端自动创建
    let first = RunCreateRequest {
        workflow: WorkflowRef {
            name: "conversation".to_string(),
            version: Some("0.1.0".to_string()),
        },
        input: json!({
            "messages": [
                { "role": "user", "content": "你好，帮我做个自我介绍。" }
            ]
        }),
        context: None,
        metadata: None,
        labels: None,
    };

    let created = client.create_run(first).await?;
    // 等待完成并获取输出
    let run = client.wait_for_completion(&created.run.run_id, 15_000).await?;
    if !matches!(run.status, RunStatus::Succeeded) {
        println!("status: {:?}", run.status);
        println!("error: {:?}", run.error);
        return Ok(());
    }
    // 从输出里拿到 conversation_id，用于下一轮
    let output = run.output.clone().unwrap_or_else(|| json!({}));
    let conv_id = output
        .get("conversation_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    println!("conversation_id: {}", conv_id);
    println!("reply1: {}", output);

    // 第二轮对话：带上 conversation_id 继续对话
    let second = RunCreateRequest {
        workflow: WorkflowRef {
            name: "conversation".to_string(),
            version: Some("0.1.0".to_string()),
        },
        input: json!({
            "conversation_id": conv_id,
            "messages": [
                { "role": "user", "content": "再简短一点，50字以内。" }
            ]
        }),
        context: None,
        metadata: None,
        labels: None,
    };

    let created = client.create_run(second).await?;
    let run = client.wait_for_completion(&created.run.run_id, 15_000).await?;
    if !matches!(run.status, RunStatus::Succeeded) {
        println!("status: {:?}", run.status);
        println!("error: {:?}", run.error);
        return Ok(());
    }
    println!("reply2: {}", run.output.unwrap_or_else(|| json!({})));
    Ok(())
}
