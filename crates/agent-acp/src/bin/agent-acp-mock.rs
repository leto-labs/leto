use anyhow::Result;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    agent_acp::run_mock_stdio().await?;
    Ok(())
}
