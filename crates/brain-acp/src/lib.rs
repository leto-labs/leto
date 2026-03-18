pub mod mock;

pub use mock::{SEEDED_SESSION_ID, run_mock_stdio};

pub async fn run_stdio() -> Result<(), agent_client_protocol::Error> {
    run_mock_stdio().await
}
