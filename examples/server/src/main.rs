use std::sync::Arc;

use brain_core::*;
use brain_server::{BrainServer, serve};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let provider: Arc<dyn Provider> = Arc::new(MockProvider::new().with_delay(50));
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);

    let brain = Brain::new(provider, store, agent_loop, vec![]);
    let server = Arc::new(BrainServer::new(brain));

    let addr = "127.0.0.1:8080";
    eprintln!("brain-server listening on http://{addr}");
    eprintln!();
    eprintln!("Try:");
    eprintln!("  curl -s http://{addr}/status | jq");
    eprintln!(
        "  curl -s -X POST http://{addr}/projects -H 'Content-Type: application/json' -d '{{\"name\":\"demo\"}}' | jq"
    );

    serve(server, addr).await.unwrap();
}
