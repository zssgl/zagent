use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::InMemoryRuntime;
use agent_runtime::server::router;
use std::path::Path;

use serde_json::Value;

mod workflows;
use workflows::{load_latest_active_spec_path, MeetingPrebriefDaily1_1Runner, WorkflowSpec};

#[tokio::main]
async fn main() {
    let runtime = Arc::new(InMemoryRuntime::new());
    let workflow_spec_path = load_latest_active_spec_path().expect("discover active workflow spec");
    let workflow_spec = WorkflowSpec::load(&workflow_spec_path).expect("valid workflow spec");
    let input_schema = read_json_schema(&workflow_spec.input_schema_path());
    let output_schema = read_json_schema(&workflow_spec.output_schema_path());
    let workflow = MeetingPrebriefDaily1_1Runner::from_spec(&workflow_spec).expect("load workflow");

    runtime
        .register_workflow_with_schemas(Arc::new(workflow), Some(input_schema), Some(output_schema))
        .await;

    let app = router(runtime);
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
