use std::net::SocketAddr;
use std::sync::Arc;

use agent_runtime::runtime::{EchoWorkflow, InMemoryRuntime};
use agent_runtime::server::router;

#[tokio::main]
async fn main() {
    let runtime = Arc::new(InMemoryRuntime::new());
    runtime.register_workflow(Arc::new(EchoWorkflow)).await;

    let app = router(runtime);
    let addr: SocketAddr = "127.0.0.1:3000".parse().expect("valid addr");
    println!("agent runtime listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
