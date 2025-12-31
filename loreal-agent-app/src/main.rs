use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::InMemoryRuntime;
use agent_runtime::server::router;
use serde_json::Value;

mod workflows;
use workflows::MeetingPrebriefDailyWorkflow;

const INPUT_SCHEMA: &str =
    include_str!("../workflows/meeting_prebrief_daily/v1.0.0/input.schema.json");
const OUTPUT_SCHEMA: &str =
    include_str!("../workflows/meeting_prebrief_daily/v1.0.0/output.schema.json");

#[tokio::main]
async fn main() {
    let runtime = Arc::new(InMemoryRuntime::new());
    let input_schema: Value =
        serde_json::from_str(INPUT_SCHEMA).expect("valid meeting_prebrief_daily input schema");
    let output_schema: Value =
        serde_json::from_str(OUTPUT_SCHEMA).expect("valid meeting_prebrief_daily output schema");

    runtime
        .register_workflow_with_schemas(
            Arc::new(MeetingPrebriefDailyWorkflow),
            Some(input_schema),
            Some(output_schema),
        )
        .await;

    let app = router(runtime);
    let addr: SocketAddr = "127.0.0.1:9010".parse().expect("valid addr");
    println!("loreal agent app listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
