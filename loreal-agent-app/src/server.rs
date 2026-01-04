use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use axum::response::sse::Event as SseEvent;
use futures::StreamExt;
use serde_json::Value;
use tokio_stream::wrappers::BroadcastStream;

use agent_runtime::runtime::InMemoryRuntime;
use agent_runtime::types::{
    Artifact, Event, EventListResponse, ErrorResponse, Run, RunCreateRequest, RunCreateResponse,
    SchemaBundle, Workflow, WorkflowListResponse,
};

#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<InMemoryRuntime>,
}

pub fn router(runtime: Arc<InMemoryRuntime>) -> Router {
    let state = AppState { runtime };
    Router::new()
        .route("/v1/runs", post(create_run))
        .route("/v1/runs/:run_id", get(get_run))
        .route("/v1/runs/:run_id/events", get(get_events))
        .route("/v1/artifacts/:artifact_id", get(get_artifact))
        .route("/v1/workflows", get(list_workflows))
        .route("/v1/workflows/:name", get(get_workflow))
        .route("/v1/workflows/:name/schemas", get(get_workflow_schemas))
        .with_state(state)
}

async fn create_run(
    State(state): State<AppState>,
    Json(mut req): Json<RunCreateRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Workflow runners only receive `input`, not `context`; forward it explicitly for tool usage.
    if let Some(context) = &req.context {
        if let Value::Object(map) = &mut req.input {
            map.insert("__context".to_string(), context.clone());
        }
    }

    let run = state
        .runtime
        .create_run(req)
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, Json(err)))?;
    Ok((StatusCode::CREATED, Json(RunCreateResponse { run })))
}

async fn get_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<Run>, (StatusCode, Json<ErrorResponse>)> {
    match state.runtime.get_run(&run_id).await {
        Some(run) => Ok(Json(run)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "not_found".to_string(),
                message: "run not found".to_string(),
                retryable: false,
                details: None,
            }),
        )),
    }
}

async fn get_events(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("text/event-stream") {
        let receiver = state.runtime.subscribe_events(&run_id).await.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    code: "not_found".to_string(),
                    message: "run not found".to_string(),
                    retryable: false,
                    details: None,
                }),
            )
        })?;
        let stream = BroadcastStream::new(receiver).filter_map(|result| async move {
            match result {
                Ok(event) => Some(Ok::<SseEvent, std::convert::Infallible>(to_sse_event(event))),
                Err(_) => None,
            }
        });
        Ok(Sse::new(stream).into_response())
    } else {
        let events = state.runtime.list_events(&run_id).await.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    code: "not_found".to_string(),
                    message: "run not found".to_string(),
                    retryable: false,
                    details: None,
                }),
            )
        })?;
        Ok(Json(EventListResponse {
            data: events,
            next_cursor: None,
        })
        .into_response())
    }
}

fn to_sse_event(event: Event) -> SseEvent {
    let event_name = serde_json::to_string(&event.event_type)
        .ok()
        .and_then(|s| s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).map(|s| s.to_string()))
        .unwrap_or_else(|| "event".to_string());
    let payload = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
    SseEvent::default().event(event_name).data(payload)
}

async fn get_artifact(
    State(state): State<AppState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<Artifact>, (StatusCode, Json<ErrorResponse>)> {
    match state.runtime.get_artifact(&artifact_id).await {
        Some(artifact) => Ok(Json(artifact)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "not_found".to_string(),
                message: "artifact not found".to_string(),
                retryable: false,
                details: None,
            }),
        )),
    }
}

async fn list_workflows(State(state): State<AppState>) -> Json<WorkflowListResponse> {
    let workflows = state.runtime.list_workflows().await;
    Json(WorkflowListResponse {
        data: workflows,
        next_cursor: None,
    })
}

async fn get_workflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Workflow>, (StatusCode, Json<ErrorResponse>)> {
    match state.runtime.get_workflow(&name).await {
        Some(workflow) => Ok(Json(workflow)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "not_found".to_string(),
                message: "workflow not found".to_string(),
                retryable: false,
                details: None,
            }),
        )),
    }
}

async fn get_workflow_schemas(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<SchemaBundle>, (StatusCode, Json<ErrorResponse>)> {
    match state.runtime.get_workflow_schemas(&name).await {
        Some(bundle) => Ok(Json(bundle)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "not_found".to_string(),
                message: "workflow not found".to_string(),
                retryable: false,
                details: None,
            }),
        )),
    }
}
