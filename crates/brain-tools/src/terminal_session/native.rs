use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use futures::future::BoxFuture;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::Duration;

use brain_types::BrainError;

use super::{
    TerminalSessionAction, TerminalSessionDriver, TerminalSessionObservation,
    TerminalSessionRequest,
};
use crate::truncation::truncate_middle_with_notice;

const DEFAULT_ROWS: u16 = 40;
const DEFAULT_COLS: u16 = 160;
const TMUX_HISTORY_LIMIT: u64 = 10_000_000;
const TMUX_SEND_KEYS_MAX_COMMAND_LENGTH: usize = 16_000;

#[derive(Default)]
pub struct TerminalSessionDriverNative {
    sessions: Arc<Mutex<HashMap<String, Arc<TerminalSessionHandle>>>>,
    tmux_ready: Arc<Mutex<bool>>,
}

impl TerminalSessionDriver for TerminalSessionDriverNative {
    fn execute(
        &self,
        request: TerminalSessionRequest,
    ) -> BoxFuture<'_, Result<TerminalSessionObservation, BrainError>> {
        Box::pin(async move {
            match request.action {
                TerminalSessionAction::SendKeys => {
                    let session = self.get_or_create_session(&request).await?;
                    session
                        .send_keys(
                            request.keystrokes.as_deref().unwrap_or_default(),
                            request.keys.unwrap_or_default(),
                            request.min_wait_seconds,
                            request.max_bytes,
                        )
                        .await
                }
                TerminalSessionAction::Capture => {
                    let session = self.get_or_create_session(&request).await?;
                    session.capture(request.max_bytes).await
                }
                TerminalSessionAction::Close => self.close_session(&request.session_id).await,
            }
        })
    }
}

impl TerminalSessionDriverNative {
    async fn get_or_create_session(
        &self,
        request: &TerminalSessionRequest,
    ) -> Result<Arc<TerminalSessionHandle>, BrainError> {
        if let Some(session) = self.sessions.lock().await.get(&request.session_id).cloned() {
            if session.is_alive().await? {
                return Ok(session);
            }
            self.sessions.lock().await.remove(&request.session_id);
        }

        self.ensure_tmux_available().await?;

        let cwd = match request.working_directory.as_deref() {
            Some(path) => PathBuf::from(path),
            None => match std::env::current_dir() {
                Ok(path) => path,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "failed to resolve current directory for terminal session; falling back to temp dir"
                    );
                    std::env::temp_dir()
                }
            },
        };

        let session = Arc::new(
            TerminalSessionHandle::spawn(
                &request.session_id,
                &cwd,
                request.rows.max(DEFAULT_ROWS),
                request.cols.max(DEFAULT_COLS),
            )
            .await?,
        );
        self.sessions
            .lock()
            .await
            .insert(request.session_id.clone(), session.clone());

        Ok(session)
    }

    async fn close_session(
        &self,
        session_id: &str,
    ) -> Result<TerminalSessionObservation, BrainError> {
        let session = self.sessions.lock().await.remove(session_id);
        let Some(session) = session else {
            return Ok(TerminalSessionObservation {
                timed_out: false,
                terminal_state: String::new(),
                observation: "Terminal session already closed.".into(),
                shell_exited: true,
                exit_code: None,
            });
        };

        session.close().await?;

        Ok(TerminalSessionObservation {
            timed_out: false,
            terminal_state: String::new(),
            observation: "Terminal session closed.".into(),
            shell_exited: true,
            exit_code: None,
        })
    }

    async fn ensure_tmux_available(&self) -> Result<(), BrainError> {
        let mut ready = self.tmux_ready.lock().await;
        if *ready && command_exists("tmux").await? {
            return Ok(());
        }
        if command_exists("tmux").await? {
            *ready = true;
            return Ok(());
        }

        for command in tmux_install_commands() {
            let _ = run_shell_command(&command, None).await?;
            if command_exists("tmux").await? {
                *ready = true;
                return Ok(());
            }
        }

        let _ = run_shell_command(TMUX_BUILD_FROM_SOURCE_COMMAND, None).await?;
        if command_exists("tmux").await? {
            *ready = true;
            return Ok(());
        }

        Err(BrainError::ToolFailed {
            tool: "terminal_session".into(),
            reason: "tmux is required for Terminus-2 terminal sessions and could not be installed"
                .into(),
        })
    }
}

struct TerminalSessionHandle {
    session_name: String,
    previous_buffer: Mutex<Option<String>>,
}

impl TerminalSessionHandle {
    async fn spawn(session_id: &str, cwd: &Path, rows: u16, cols: u16) -> Result<Self, BrainError> {
        let session_name = tmux_session_name(session_id);
        let cwd = cwd.to_string_lossy().into_owned();
        let tmux_new_session = format!(
            "tmux new-session -x {cols} -y {rows} -d -s {} -c {} 'bash --login'",
            shell_quote(&session_name),
            shell_quote(&cwd),
        );

        let start_command = if command_exists("script").await? {
            format!(
                "export TERM=xterm-256color && export SHELL=/bin/bash && script -qc {} /dev/null",
                shell_quote(&tmux_new_session)
            )
        } else {
            format!("export TERM=xterm-256color && export SHELL=/bin/bash && {tmux_new_session}")
        };

        let start_result = run_shell_command(&start_command, None).await?;
        if !start_result.success() {
            return Err(BrainError::ToolFailed {
                tool: "terminal_session".into(),
                reason: format!(
                    "failed to start tmux session '{}': {}",
                    session_name,
                    start_result.stderr.trim()
                ),
            });
        }

        let history_result = run_shell_command(
            &format!("tmux set-option -g history-limit {TMUX_HISTORY_LIMIT}"),
            None,
        )
        .await?;
        if !history_result.success() {
            tracing::warn!(
                stderr = history_result.stderr.trim(),
                "failed to increase tmux history limit"
            );
        }

        Ok(Self {
            session_name,
            previous_buffer: Mutex::new(None),
        })
    }

    async fn send_keys(
        &self,
        keystrokes: &str,
        keys: Vec<String>,
        min_wait_seconds: f64,
        max_bytes: usize,
    ) -> Result<TerminalSessionObservation, BrainError> {
        if !self.is_alive().await? {
            return Err(BrainError::ToolFailed {
                tool: "terminal_session".into(),
                reason: "tmux session is no longer alive".into(),
            });
        }

        let keys = if keys.is_empty() {
            if keystrokes.is_empty() {
                Vec::new()
            } else {
                vec![keystrokes.to_owned()]
            }
        } else {
            keys
        };

        let started = Instant::now();
        for command in tmux_send_keys_commands(&self.session_name, &keys) {
            let result = run_shell_command(&command, None).await?;
            if !result.success() {
                return Err(BrainError::ToolFailed {
                    tool: "terminal_session".into(),
                    reason: format!(
                        "failed to send tmux keys to '{}': {}",
                        self.session_name,
                        result.stderr.trim()
                    ),
                });
            }
        }

        let elapsed = started.elapsed();
        let min_wait = Duration::from_secs_f64(min_wait_seconds.max(0.0));
        if elapsed < min_wait {
            tokio::time::sleep(min_wait - elapsed).await;
        }

        self.capture(max_bytes).await
    }

    async fn capture(&self, max_bytes: usize) -> Result<TerminalSessionObservation, BrainError> {
        let current_buffer = self.capture_pane(true).await?;
        let visible_screen = self.capture_pane(false).await?;

        let mut previous_buffer = self.previous_buffer.lock().await;
        let observation = match previous_buffer.as_deref() {
            None => {
                *previous_buffer = Some(current_buffer.clone());
                format!("Current Terminal Screen:\n{visible_screen}")
            }
            Some(previous) => {
                let new_content = find_new_content(previous, &current_buffer);
                *previous_buffer = Some(current_buffer.clone());
                match new_content {
                    Some(new_content) if !new_content.trim().is_empty() => {
                        format!("New Terminal Output:\n{new_content}")
                    }
                    _ => format!("Current Terminal Screen:\n{visible_screen}"),
                }
            }
        };
        drop(previous_buffer);

        Ok(TerminalSessionObservation {
            timed_out: false,
            terminal_state: truncate_screen(&visible_screen, max_bytes),
            observation: truncate_screen(&observation, max_bytes),
            shell_exited: !self.is_alive().await?,
            exit_code: None,
        })
    }

    async fn capture_pane(&self, capture_entire: bool) -> Result<String, BrainError> {
        let mut command = String::from("tmux capture-pane -p");
        if capture_entire {
            command.push_str(" -S -");
        }
        command.push_str(" -t ");
        command.push_str(&shell_quote(&self.session_name));

        let result = run_shell_command(&command, None).await?;
        if !result.success() {
            return Err(BrainError::ToolFailed {
                tool: "terminal_session".into(),
                reason: format!(
                    "failed to capture tmux pane '{}': {}",
                    self.session_name,
                    result.stderr.trim()
                ),
            });
        }

        Ok(normalize_capture(&result.stdout))
    }

    async fn is_alive(&self) -> Result<bool, BrainError> {
        let result = run_shell_command(
            &format!("tmux has-session -t {}", shell_quote(&self.session_name)),
            None,
        )
        .await?;
        Ok(result.success())
    }

    async fn close(&self) -> Result<(), BrainError> {
        let result = run_shell_command(
            &format!("tmux kill-session -t {}", shell_quote(&self.session_name)),
            None,
        )
        .await?;
        if result.success() || result.stderr.contains("can't find session") {
            Ok(())
        } else {
            Err(BrainError::ToolFailed {
                tool: "terminal_session".into(),
                reason: format!(
                    "failed to close tmux session '{}': {}",
                    self.session_name,
                    result.stderr.trim()
                ),
            })
        }
    }
}

struct ShellOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

impl ShellOutput {
    fn success(&self) -> bool {
        self.status == 0
    }
}

async fn run_shell_command(command: &str, cwd: Option<&Path>) -> Result<ShellOutput, BrainError> {
    let mut process = Command::new("bash");
    process.arg("-lc").arg(command);
    process.stdout(std::process::Stdio::piped());
    process.stderr(std::process::Stdio::piped());
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }

    let output = process.output().await.map_err(terminal_tool_error)?;
    Ok(ShellOutput {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

async fn command_exists(command: &str) -> Result<bool, BrainError> {
    let result = run_shell_command(&format!("command -v {command} >/dev/null 2>&1"), None).await?;
    Ok(result.success())
}

fn tmux_install_commands() -> Vec<String> {
    vec![
        "DEBIAN_FRONTEND=noninteractive apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y tmux".into(),
        "dnf install -y tmux".into(),
        "yum install -y tmux".into(),
        "apk add --no-cache tmux".into(),
        "pacman -S --noconfirm tmux".into(),
        "brew install tmux".into(),
        "ASSUME_ALWAYS_YES=yes pkg install -y tmux".into(),
        "zypper install -y -n tmux".into(),
    ]
}

const TMUX_BUILD_FROM_SOURCE_COMMAND: &str = concat!(
    "cd /tmp && ",
    "DEBIAN_FRONTEND=noninteractive apt-get update && ",
    "DEBIAN_FRONTEND=noninteractive apt-get install -y build-essential libevent-dev libncurses5-dev curl && ",
    "curl -L https://github.com/tmux/tmux/releases/download/3.4/tmux-3.4.tar.gz -o tmux.tar.gz && ",
    "tar -xzf tmux.tar.gz && ",
    "cd tmux-3.4 && ",
    "./configure --prefix=/usr/local && ",
    "make && ",
    "make install"
);

fn tmux_session_name(session_id: &str) -> String {
    let sanitized = session_id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect::<String>();
    let suffix = sanitized.trim_matches('-');
    let suffix = if suffix.is_empty() { "session" } else { suffix };
    let suffix = suffix.chars().take(48).collect::<String>();
    format!("brain-{suffix}")
}

fn tmux_send_keys_commands(session_name: &str, keys: &[String]) -> Vec<String> {
    let prefix = format!("tmux send-keys -t {}", shell_quote(session_name));
    if keys.is_empty() {
        return Vec::new();
    }

    let escaped_keys = keys.iter().map(|key| shell_quote(key)).collect::<Vec<_>>();
    let single = format!("{prefix} {}", escaped_keys.join(" "));
    if single.len() <= TMUX_SEND_KEYS_MAX_COMMAND_LENGTH {
        return vec![single];
    }

    let mut commands = Vec::new();
    let mut current = Vec::new();
    let mut current_len = prefix.len();

    let flush = |commands: &mut Vec<String>, current: &mut Vec<String>, current_len: &mut usize| {
        if current.is_empty() {
            return;
        }
        commands.push(format!("{prefix} {}", current.join(" ")));
        current.clear();
        *current_len = prefix.len();
    };

    for key in keys {
        let escaped = shell_quote(key);
        let addition = 1 + escaped.len();
        if current_len + addition <= TMUX_SEND_KEYS_MAX_COMMAND_LENGTH {
            current.push(escaped);
            current_len += addition;
        } else if prefix.len() + addition <= TMUX_SEND_KEYS_MAX_COMMAND_LENGTH {
            flush(&mut commands, &mut current, &mut current_len);
            current.push(escaped);
            current_len = prefix.len() + addition;
        } else {
            flush(&mut commands, &mut current, &mut current_len);
            for chunk in
                split_key_for_tmux(key, TMUX_SEND_KEYS_MAX_COMMAND_LENGTH - prefix.len() - 1)
            {
                let addition = 1 + chunk.len();
                if current_len + addition > TMUX_SEND_KEYS_MAX_COMMAND_LENGTH {
                    flush(&mut commands, &mut current, &mut current_len);
                }
                current.push(chunk);
                current_len += addition;
            }
        }
    }

    flush(&mut commands, &mut current, &mut current_len);
    commands
}

fn split_key_for_tmux(key: &str, max_escaped_len: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let chars = key.char_indices().collect::<Vec<_>>();

    while start < chars.len() {
        let mut low = start + 1;
        let mut high = chars.len();
        let mut best = start + 1;

        while low <= high {
            let mid = (low + high) / 2;
            let end_byte = chars.get(mid).map(|(idx, _)| *idx).unwrap_or(key.len());
            let escaped = shell_quote(&key[chars[start].0..end_byte]);
            if escaped.len() <= max_escaped_len {
                best = mid;
                low = mid + 1;
            } else if mid == 0 {
                break;
            } else {
                high = mid - 1;
            }
        }

        let end_byte = chars.get(best).map(|(idx, _)| *idx).unwrap_or(key.len());
        chunks.push(shell_quote(&key[chars[start].0..end_byte]));
        start = best;
    }

    chunks
}

fn find_new_content(previous_buffer: &str, current_buffer: &str) -> Option<String> {
    let previous_buffer = previous_buffer.trim();
    if current_buffer.contains(previous_buffer) {
        let mut index = current_buffer.find(previous_buffer)?;
        if let Some(newline_index) = previous_buffer.rfind('\n') {
            index = newline_index;
        }
        return Some(current_buffer[index..].to_owned());
    }
    None
}

fn normalize_capture(output: &str) -> String {
    output.trim_end_matches('\n').to_owned()
}

fn truncate_screen(text: &str, max_bytes: usize) -> String {
    truncate_middle_with_notice(text, max_bytes, "terminal observation")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn terminal_tool_error(error: impl std::fmt::Display) -> BrainError {
    BrainError::ToolFailed {
        tool: "terminal_session".into(),
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn tmux_available() -> bool {
        command_exists("tmux").await.unwrap_or(false)
    }

    #[tokio::test]
    async fn session_preserves_state_across_calls() {
        if !tmux_available().await {
            return;
        }

        let dir = tempdir().unwrap();
        let driver = TerminalSessionDriverNative::default();
        let session_id = "test-session".to_owned();

        driver
            .execute(TerminalSessionRequest {
                session_id: session_id.clone(),
                action: TerminalSessionAction::SendKeys,
                keystrokes: Some("printf 'hello\\n' > note.txt\n".into()),
                keys: None,
                min_wait_seconds: 0.1,
                working_directory: Some(dir.path().display().to_string()),
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();

        let observation = driver
            .execute(TerminalSessionRequest {
                session_id: session_id.clone(),
                action: TerminalSessionAction::SendKeys,
                keystrokes: Some("cat note.txt\n".into()),
                keys: None,
                min_wait_seconds: 0.1,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();

        assert!(observation.observation.contains("hello"));

        let _ = driver
            .execute(TerminalSessionRequest {
                session_id,
                action: TerminalSessionAction::Close,
                keystrokes: None,
                keys: None,
                min_wait_seconds: 0.0,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn capture_returns_current_screen_when_no_new_delta_found() {
        if !tmux_available().await {
            return;
        }

        let driver = TerminalSessionDriverNative::default();
        let session_id = "capture-session".to_owned();

        let first = driver
            .execute(TerminalSessionRequest {
                session_id: session_id.clone(),
                action: TerminalSessionAction::Capture,
                keystrokes: None,
                keys: None,
                min_wait_seconds: 0.0,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();

        assert!(first.observation.contains("Current Terminal Screen"));

        let _ = driver
            .execute(TerminalSessionRequest {
                session_id,
                action: TerminalSessionAction::Close,
                keystrokes: None,
                keys: None,
                min_wait_seconds: 0.0,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn logical_ctrl_c_interrupts_running_command() {
        if !tmux_available().await {
            return;
        }

        let driver = TerminalSessionDriverNative::default();
        let session_id = "ctrl-c-session".to_owned();

        let _ = driver
            .execute(TerminalSessionRequest {
                session_id: session_id.clone(),
                action: TerminalSessionAction::SendKeys,
                keystrokes: Some("sleep 30\n".into()),
                keys: None,
                min_wait_seconds: 0.1,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();

        let _ = driver
            .execute(TerminalSessionRequest {
                session_id: session_id.clone(),
                action: TerminalSessionAction::SendKeys,
                keystrokes: None,
                keys: Some(vec!["C-c".into()]),
                min_wait_seconds: 0.1,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();

        let observation = driver
            .execute(TerminalSessionRequest {
                session_id: session_id.clone(),
                action: TerminalSessionAction::SendKeys,
                keystrokes: Some("printf 'done\\n'\n".into()),
                keys: None,
                min_wait_seconds: 0.1,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();

        assert!(observation.observation.contains("done"));

        let _ = driver
            .execute(TerminalSessionRequest {
                session_id,
                action: TerminalSessionAction::Close,
                keystrokes: None,
                keys: None,
                min_wait_seconds: 0.0,
                working_directory: None,
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                max_bytes: 10_000,
            })
            .await
            .unwrap();
    }

    #[test]
    fn compute_observation_uses_full_buffer_delta() {
        let previous = "line1\nline2";
        let full_buffer = "line1\nline2\nline3\nline4";

        let observation = find_new_content(previous, full_buffer).unwrap();

        assert!(observation.contains("line2"));
        assert!(observation.contains("line3"));
        assert!(observation.contains("line4"));
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
    }
}
