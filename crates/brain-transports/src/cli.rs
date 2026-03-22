use std::io::Write;

use futures::future::BoxFuture;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;

use brain_types::*;

type StdinReader = tokio::io::BufReader<tokio::io::Stdin>;

pub struct CliTransport {
    stdin: Mutex<StdinReader>,
}

impl CliTransport {
    pub fn new() -> Self {
        Self {
            stdin: Mutex::new(tokio::io::BufReader::new(tokio::io::stdin())),
        }
    }
}

impl Default for CliTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for CliTransport {
    fn name(&self) -> &str {
        "cli"
    }

    fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>> {
        Box::pin(async {
            let mut reader = self.stdin.lock().await;
            loop {
                eprint!("> ");
                std::io::stderr().flush().ok();

                let mut line = String::new();
                let n = reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| BrainError::Internal(format!("stdin read: {e}")))?;

                if n == 0 {
                    return Ok(None);
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                return Ok(Some(InputEvent::Message(trimmed.to_owned())));
            }
        })
    }

    fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            match event {
                Event::Token { delta } => {
                    print!("{delta}");
                    std::io::stdout().flush().ok();
                }
                Event::ToolCallPending { name, .. } => {
                    println!("\n[tool pending: {name}]");
                }
                Event::ToolCallStart { name, .. } => {
                    println!("[tool: {name}]");
                }
                Event::ToolCallDone { result, .. } => {
                    println!("[result: {result}]");
                }
                Event::MessageDone { .. } => {}
                Event::TurnDone {
                    iterations,
                    prompt_tokens: _,
                    completion_tokens: _,
                    cache_read_tokens: _,
                    cache_write_tokens: _,
                    reasoning_tokens: _,
                    total_tokens,
                } => {
                    println!("\n[done: {iterations} iter, {total_tokens} tokens]");
                }
                Event::Error { message, .. } => {
                    eprintln!("\n[error: {message}]");
                }
                Event::SessionStart { session_id } => {
                    eprintln!("[session: {session_id}]");
                }
                Event::SessionResume { session_id } => {
                    eprintln!("[resumed: {session_id}]");
                }
                Event::Progress { message, .. } => {
                    eprintln!("[progress: {message}]");
                }
                _ => {}
            }
            Ok(())
        })
    }
}
