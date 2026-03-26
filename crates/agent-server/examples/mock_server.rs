use std::sync::Arc;

use agent_core::{AgentCore, AgentCoreNative};
use agent_server::{AgentServer, serve};
use provider::MockProvider;

fn main() {
    let addr = std::env::var("AGENT_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:4096".to_owned());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .expect("failed to build tokio runtime");

    runtime.block_on(async move {
        let core: Arc<dyn AgentCore> = Arc::new(
            AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
                .with_provider("mock", Arc::new(MockProvider::new()))
                .build()
                .await
                .expect("failed to build mock agent core"),
        );
        let server = Arc::new(AgentServer::new(core));

        eprintln!("agent-server mock example listening on http://{addr}");
        eprintln!("OpenCode web defaults to http://localhost:4096 on localhost.");

        serve(server, &addr)
            .await
            .expect("failed to serve agent-server");
    });
}
