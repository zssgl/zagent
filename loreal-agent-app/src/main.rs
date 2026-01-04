use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::InMemoryRuntime;
use std::path::Path;

use serde_json::Value;
use sqlx::MySqlPool;

use loreal_agent_app::tools::ToolManager;
use loreal_agent_app::workflows::{
    load_latest_active_spec_path, MeetingPrebriefDaily1_1Runner, WorkflowSpec,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let runtime = Arc::new(InMemoryRuntime::new());
    let workflow_spec_path = load_latest_active_spec_path().expect("discover active workflow spec");
    let workflow_spec = WorkflowSpec::load(&workflow_spec_path).expect("valid workflow spec");
    let input_schema = read_json_schema(&workflow_spec.input_schema_path());
    let output_schema = read_json_schema(&workflow_spec.output_schema_path());
    let mysql = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => match MySqlPool::connect(&url).await {
            Ok(pool) => Some(pool),
            Err(err) => {
                eprintln!("MySQL disabled: connect failed: {}", err);
                None
            }
        },
        _ => None,
    };
    let tools = std::sync::Arc::new(ToolManager::new(mysql));
    let workflow =
        MeetingPrebriefDaily1_1Runner::from_spec(&workflow_spec, tools).expect("load workflow");

    runtime
        .register_workflow_with_schemas(Arc::new(workflow), Some(input_schema), Some(output_schema))
        .await;

    let app = loreal_agent_app::server::router(runtime);
    let addr: SocketAddr = "127.0.0.1:9010".parse().expect("valid addr");
    println!("loreal agent app listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

fn read_json_schema(path: &Path) -> Value {
    let content = std::fs::read_to_string(path).expect("read schema");
    serde_json::from_str(&content).expect("valid schema json")
}
