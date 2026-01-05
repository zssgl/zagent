use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::InMemoryRuntime;
use std::path::Path;

use serde_json::Value;
use sqlx::MySqlPool;
use tracing::{info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use loreal_agent_app::tools::ToolManager;
use loreal_agent_app::workflows::{
    load_latest_active_spec_path, MeetingPrebriefDaily1_1Runner, WorkflowSpec,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let _guards = init_tracing();
    let runtime = Arc::new(InMemoryRuntime::new());
    let workflow_spec_path = load_latest_active_spec_path().expect("discover active workflow spec");
    let workflow_spec = WorkflowSpec::load(&workflow_spec_path).expect("valid workflow spec");
    let input_schema = read_json_schema(&workflow_spec.input_schema_path());
    let output_schema = read_json_schema(&workflow_spec.output_schema_path());
    let mysql = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => match MySqlPool::connect(&url).await {
            Ok(pool) => Some(pool),
            Err(err) => {
                warn!(error = %err, "mysql disabled: connect failed");
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
    info!(%addr, "loreal agent app listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

fn read_json_schema(path: &Path) -> Value {
    let content = std::fs::read_to_string(path).expect("read schema");
    serde_json::from_str(&content).expect("valid schema json")
}

fn init_tracing() -> (WorkerGuard, WorkerGuard) {
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(log_dir, "loreal-agent-app.log");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

    let file_layer = fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(file_writer);
    let stdout_layer = fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(stdout_writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    (file_guard, stdout_guard)
}
