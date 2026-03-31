use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    agent_acp::run_embedded_stdio().await
}
