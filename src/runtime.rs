use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde_json::{json, Value};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::types::{
    Artifact, ArtifactRef, ArtifactType, ErrorResponse, Event, EventType, Run, RunCreateRequest,
    RunStatus, SchemaBundle, Timing, Workflow, WorkflowRef, WorkflowSummary,
};

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("retryable: {0}")]
    Retryable(String),
    #[error("fatal: {0}")]
    Fatal(String),
}

#[derive(Debug, Clone)]
pub struct WorkflowOutput {
    pub output: Value,
    pub artifacts: Vec<Artifact>,
}

#[async_trait::async_trait]
pub trait WorkflowRunner: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> Option<&'static str> {
        None
    }
    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError>;
}

#[derive(Clone)]
pub struct InMemoryRuntime {
    workflows: Arc<RwLock<HashMap<String, Arc<dyn WorkflowRunner>>>>,
    runs: Arc<RwLock<HashMap<String, RunRecord>>>,
    artifacts: Arc<RwLock<HashMap<String, Artifact>>>,
}

struct RunRecord {
    run: Run,
    events: Vec<Event>,
    sender: broadcast::Sender<Event>,
}

impl InMemoryRuntime {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            runs: Arc::new(RwLock::new(HashMap::new())),
            artifacts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_workflow(&self, workflow: Arc<dyn WorkflowRunner>) {
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow.name().to_string(), workflow);
    }

    pub async fn list_workflows(&self) -> Vec<WorkflowSummary> {
        let workflows = self.workflows.read().await;
        workflows
            .values()
            .map(|workflow| WorkflowSummary {
                name: workflow.name().to_string(),
                version: workflow.version().map(|v| v.to_string()),
                description: None,
                tags: Vec::new(),
            })
            .collect()
    }

    pub async fn get_workflow(&self, name: &str) -> Option<Workflow> {
        let workflows = self.workflows.read().await;
        workflows.get(name).map(|workflow| Workflow {
            name: workflow.name().to_string(),
            version: workflow.version().map(|v| v.to_string()),
            description: None,
            tags: Vec::new(),
            input_schema_ref: None,
            output_schema_ref: None,
        })
    }

    pub async fn get_workflow_schemas(&self, name: &str) -> Option<SchemaBundle> {
        let workflows = self.workflows.read().await;
        workflows.get(name).map(|workflow| {
            let workflow_ref = WorkflowRef {
                name: workflow.name().to_string(),
                version: workflow.version().map(|v| v.to_string()),
            };
            SchemaBundle {
                workflow: workflow_ref,
                schema_hash: "stub".to_string(),
                schemas: HashMap::new(),
            }
        })
    }

    pub async fn create_run(&self, req: RunCreateRequest) -> Result<Run, ErrorResponse> {
        let workflow_name = req.workflow.name.clone();
        let workflows = self.workflows.read().await;
        let workflow = workflows.get(&workflow_name).cloned().ok_or_else(|| {
            ErrorResponse {
                code: "workflow_not_found".to_string(),
                message: format!("workflow {} not registered", workflow_name),
                retryable: false,
                details: None,
            }
        })?;
        drop(workflows);

        let run_id = format!("run_{}", Uuid::new_v4());
        let now = Utc::now();
        let timing = Timing {
            created_at: now,
            started_at: None,
            finished_at: None,
            wall_ms: None,
        };
        let run = Run {
            run_id: run_id.clone(),
            workflow: WorkflowRef {
                name: workflow.name().to_string(),
                version: workflow.version().map(|v| v.to_string()),
            },
            status: RunStatus::Queued,
            trace_id: None,
            tenant_id: None,
            timing,
            input: Some(req.input.clone()),
            context: req.context.clone(),
            output: None,
            error: None,
            artifacts: Vec::new(),
        };

        let (sender, _) = broadcast::channel(100);
        let record = RunRecord {
            run: run.clone(),
            events: Vec::new(),
            sender,
        };
        self.runs.write().await.insert(run_id.clone(), record);

        let runtime = self.clone();
        tokio::spawn(async move {
            runtime.execute_run(run_id, workflow, req.input).await;
        });

        Ok(run)
    }

    pub async fn get_run(&self, run_id: &str) -> Option<Run> {
        let runs = self.runs.read().await;
        runs.get(run_id).map(|record| record.run.clone())
    }

    pub async fn list_events(&self, run_id: &str) -> Option<Vec<Event>> {
        let runs = self.runs.read().await;
        runs.get(run_id).map(|record| record.events.clone())
    }

    pub async fn subscribe_events(&self, run_id: &str) -> Option<broadcast::Receiver<Event>> {
        let runs = self.runs.read().await;
        runs.get(run_id).map(|record| record.sender.subscribe())
    }

    pub async fn get_artifact(&self, artifact_id: &str) -> Option<Artifact> {
        let artifacts = self.artifacts.read().await;
        artifacts.get(artifact_id).cloned()
    }

    async fn execute_run(
        &self,
        run_id: String,
        workflow: Arc<dyn WorkflowRunner>,
        input: Value,
    ) {
        let started_at = Utc::now();
        self.update_run_status(&run_id, RunStatus::Running, Some(started_at), None, None)
            .await;
        self.emit_event(
            &run_id,
            EventType::RunStarted,
            None,
            json!({ "workflow": { "name": workflow.name(), "version": workflow.version() } }),
        )
        .await;
        self.emit_event(
            &run_id,
            EventType::StepStarted,
            Some("workflow.run".to_string()),
            json!({}),
        )
        .await;

        let result = workflow.run(input).await;
        match result {
            Ok(output) => {
                let mut artifact_refs = Vec::new();
                for artifact in output.artifacts {
                    let artifact_id = artifact.artifact_id.clone();
                    artifact_refs.push(ArtifactRef {
                        artifact_id: artifact_id.clone(),
                        r#type: artifact.r#type.clone(),
                        name: artifact.name.clone(),
                    });
                    self.artifacts
                        .write()
                        .await
                        .insert(artifact_id.clone(), artifact);
                    self.emit_event(
                        &run_id,
                        EventType::ArtifactCreated,
                        None,
                        json!({ "artifact_id": artifact_id }),
                    )
                    .await;
                }

                let finished_at = Utc::now();
                self.update_run_success(&run_id, output.output, artifact_refs, started_at, finished_at)
                    .await;
                self.emit_event(
                    &run_id,
                    EventType::StepCompleted,
                    Some("workflow.run".to_string()),
                    json!({ "ok": true }),
                )
                .await;
                self.emit_event(
                    &run_id,
                    EventType::RunCompleted,
                    None,
                    json!({ "status": "succeeded" }),
                )
                .await;
            }
            Err(err) => {
                let finished_at = Utc::now();
                let error = ErrorResponse {
                    code: "workflow_error".to_string(),
                    message: err.to_string(),
                    retryable: matches!(err, AgentError::Retryable(_)),
                    details: None,
                };
                self.update_run_failure(&run_id, error, started_at, finished_at)
                    .await;
                self.emit_event(
                    &run_id,
                    EventType::StepFailed,
                    Some("workflow.run".to_string()),
                    json!({ "ok": false }),
                )
                .await;
                self.emit_event(
                    &run_id,
                    EventType::RunFailed,
                    None,
                    json!({ "status": "failed" }),
                )
                .await;
            }
        }
    }

    async fn emit_event(&self, run_id: &str, event_type: EventType, step_id: Option<String>, payload: Value) {
        let event = Event {
            event_id: format!("evt_{}", Uuid::new_v4()),
            ts: Utc::now(),
            event_type,
            run_id: run_id.to_string(),
            step_id,
            tool_name: None,
            payload,
        };
        let mut runs = self.runs.write().await;
        if let Some(record) = runs.get_mut(run_id) {
            record.events.push(event.clone());
            let _ = record.sender.send(event);
        }
    }

    async fn update_run_status(
        &self,
        run_id: &str,
        status: RunStatus,
        started_at: Option<chrono::DateTime<Utc>>,
        finished_at: Option<chrono::DateTime<Utc>>,
        wall_ms: Option<i64>,
    ) {
        let mut runs = self.runs.write().await;
        if let Some(record) = runs.get_mut(run_id) {
            record.run.status = status;
            if let Some(started_at) = started_at {
                record.run.timing.started_at = Some(started_at);
            }
            if let Some(finished_at) = finished_at {
                record.run.timing.finished_at = Some(finished_at);
            }
            if let Some(wall_ms) = wall_ms {
                record.run.timing.wall_ms = Some(wall_ms);
            }
        }
    }

    async fn update_run_success(
        &self,
        run_id: &str,
        output: Value,
        artifacts: Vec<ArtifactRef>,
        started_at: chrono::DateTime<Utc>,
        finished_at: chrono::DateTime<Utc>,
    ) {
        let wall_ms = (finished_at - started_at).num_milliseconds();
        let mut runs = self.runs.write().await;
        if let Some(record) = runs.get_mut(run_id) {
            record.run.status = RunStatus::Succeeded;
            record.run.output = Some(output);
            record.run.artifacts = artifacts;
            record.run.timing.finished_at = Some(finished_at);
            record.run.timing.wall_ms = Some(wall_ms);
        }
    }

    async fn update_run_failure(
        &self,
        run_id: &str,
        error: ErrorResponse,
        started_at: chrono::DateTime<Utc>,
        finished_at: chrono::DateTime<Utc>,
    ) {
        let wall_ms = (finished_at - started_at).num_milliseconds();
        let mut runs = self.runs.write().await;
        if let Some(record) = runs.get_mut(run_id) {
            record.run.status = RunStatus::Failed;
            record.run.error = Some(error);
            record.run.timing.finished_at = Some(finished_at);
            record.run.timing.wall_ms = Some(wall_ms);
        }
    }
}

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
