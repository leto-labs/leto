use std::io::IsTerminal;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .try_init();
}

fn main() -> Result<(), agent_client_protocol::Error> {
    init_tracing();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| agent_client_protocol::Error::internal_error())?
        .block_on(brain_acp::run_stdio())
}
