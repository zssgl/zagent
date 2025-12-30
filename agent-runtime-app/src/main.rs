use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::InMemoryRuntime;
use agent_runtime::server::router;
use serde_json::json;

mod llm;
mod workflows;
use workflows::{ConversationWorkflow, EchoWorkflow, MeetingTodoWorkflow};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
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
                    "summary": { "type": "string" }
                },
                "required": ["summary"]
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
    runtime
        .register_workflow_with_schemas(
            Arc::new(ConversationWorkflow),
            Some(json!({
                "type": "object",
                "properties": {
                    "conversation_id": { "type": "string" },
                    "messages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "role": { "type": "string" },
                                "content": { "type": "string" }
                            },
                            "required": ["role", "content"]
                        }
                    }
                },
                "required": ["messages"]
            })),
            Some(json!({
                "type": "object",
                "properties": {
                    "conversation_id": { "type": "string" },
                    "reply": { "type": "string" },
                    "messages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "role": { "type": "string" },
                                "content": { "type": "string" }
                            },
                            "required": ["role", "content"]
                        }
                    }
                },
                "required": ["conversation_id", "reply", "messages"]
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
