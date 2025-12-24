use std::sync::Arc;

use agent_runtime::runtime::{AgentError, InMemoryRuntime, WorkflowOutput, WorkflowRunner};
use agent_runtime::server::router;
use agent_runtime::types::{EventListResponse, RunCreateResponse, WorkflowListResponse};
use axum::body::Body;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;
use tokio::time::{sleep, timeout, Duration};
use uuid::Uuid;

struct TestWorkflow;

#[async_trait::async_trait]
impl WorkflowRunner for TestWorkflow {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn version(&self) -> Option<&'static str> {
        Some("0.1.0")
    }

    async fn run(&self, input: serde_json::Value) -> Result<WorkflowOutput, AgentError> {
        let artifact = agent_runtime::types::Artifact {
            artifact_id: format!("art_{}", Uuid::new_v4()),
            r#type: agent_runtime::types::ArtifactType::Record,
            name: Some("echo".to_string()),
            created_at: chrono::Utc::now(),
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

async fn read_body_bytes(body: Body) -> Vec<u8> {
    let mut data = Vec::new();
    let mut body = body;
    while let Some(frame) = body.frame().await {
        let frame = frame.expect("body frame");
        if let Some(chunk) = frame.data_ref() {
            data.extend_from_slice(chunk);
        }
    }
    data
}

async fn read_first_body_frame(body: Body) -> Vec<u8> {
    let mut body = body;
    let frame = timeout(Duration::from_millis(200), body.frame())
        .await
        .expect("frame timeout")
        .expect("frame stream")
        .expect("frame ok");
    frame
        .data_ref()
        .map(|data| data.to_vec())
        .unwrap_or_default()
}

#[tokio::test]
async fn create_and_get_run() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime.register_workflow(Arc::new(TestWorkflow)).await;
    let app = router(runtime);

    let payload = json!({
        "workflow": { "name": "echo", "version": "0.1.0" },
        "input": { "hello": "world" }
    });

    let response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("create run response");

    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    let body_bytes = read_body_bytes(response.into_body()).await;
    let created: RunCreateResponse =
        serde_json::from_slice(&body_bytes).expect("parse create response");
    let run_id = created.run.run_id;

    let response = app
        .oneshot(
            axum::http::Request::get(format!("/v1/runs/{}", run_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get run response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn list_events_json_fallback() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime.register_workflow(Arc::new(TestWorkflow)).await;
    let app = router(runtime);

    let payload = json!({
        "workflow": { "name": "echo", "version": "0.1.0" },
        "input": { "hello": "events" }
    });

    let response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("create run response");

    let body_bytes = read_body_bytes(response.into_body()).await;
    let created: RunCreateResponse =
        serde_json::from_slice(&body_bytes).expect("parse create response");
    let run_id = created.run.run_id;

    let mut events = EventListResponse {
        data: Vec::new(),
        next_cursor: None,
    };
    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::get(format!("/v1/runs/{}/events", run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("events response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body_bytes = read_body_bytes(response.into_body()).await;
        events = serde_json::from_slice(&body_bytes).expect("parse events response");
        if !events.data.is_empty() {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert!(!events.data.is_empty());
}

#[tokio::test]
async fn stream_events_sse() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime.register_workflow(Arc::new(TestWorkflow)).await;
    let app = router(runtime);

    let payload = json!({
        "workflow": { "name": "echo", "version": "0.1.0" },
        "input": { "hello": "sse" }
    });

    let response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("create run response");

    let body_bytes = read_body_bytes(response.into_body()).await;
    let created: RunCreateResponse =
        serde_json::from_slice(&body_bytes).expect("parse create response");
    let run_id = created.run.run_id;

    let response = app
        .oneshot(
            axum::http::Request::get(format!("/v1/runs/{}/events", run_id))
                .header("accept", "text/event-stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("sse response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let chunk = read_first_body_frame(response.into_body()).await;
    let chunk_str = String::from_utf8_lossy(&chunk);
    assert!(chunk_str.contains("event:"));
    assert!(chunk_str.contains("data:"));
}

#[tokio::test]
async fn create_run_unknown_workflow_returns_400() {
    let runtime = Arc::new(InMemoryRuntime::new());
    let app = router(runtime);

    let payload = json!({
        "workflow": { "name": "missing", "version": "0.1.0" },
        "input": { "hello": "world" }
    });

    let response = app
        .oneshot(
            axum::http::Request::post("/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("create run response");

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_run_not_found_returns_404() {
    let runtime = Arc::new(InMemoryRuntime::new());
    let app = router(runtime);

    let response = app
        .oneshot(
            axum::http::Request::get("/v1/runs/run_missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get run response");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_artifact_not_found_returns_404() {
    let runtime = Arc::new(InMemoryRuntime::new());
    let app = router(runtime);

    let response = app
        .oneshot(
            axum::http::Request::get("/v1/artifacts/art_missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get artifact response");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_workflows_returns_registered() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime.register_workflow(Arc::new(TestWorkflow)).await;
    let app = router(runtime);

    let response = app
        .oneshot(
            axum::http::Request::get("/v1/workflows")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list workflows response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body_bytes = read_body_bytes(response.into_body()).await;
    let list: WorkflowListResponse =
        serde_json::from_slice(&body_bytes).expect("parse workflow list");
    assert!(!list.data.is_empty());
}

#[tokio::test]
async fn get_workflow_returns_registered() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime.register_workflow(Arc::new(TestWorkflow)).await;
    let app = router(runtime);

    let response = app
        .oneshot(
            axum::http::Request::get("/v1/workflows/echo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get workflow response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn get_workflow_schemas_returns_schemas() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime
        .register_workflow_with_schemas(
            Arc::new(TestWorkflow),
            Some(json!({ "type": "object" })),
            Some(json!({ "type": "object" })),
        )
        .await;
    let app = router(runtime);

    let response = app
        .oneshot(
            axum::http::Request::get("/v1/workflows/echo/schemas")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get schema response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body_bytes = read_body_bytes(response.into_body()).await;
    let payload: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse schema response");
    assert!(payload.get("schemas").is_some());
}
