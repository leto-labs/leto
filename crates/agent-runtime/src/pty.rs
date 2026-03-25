use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::{Mutex as TokioMutex, broadcast, mpsc};
use tokio::task::JoinHandle;
use vt100::Parser;

use crate::{
    OpenPtyRequest, PtyCaptureMode, PtyCaptureRequest, PtyCaptureResult, PtyEvent, PtyEventKind,
    PtyExecRequest, PtyExecResult, PtyId, PtySessionState, PtyStatus, RuntimeError, RuntimeId,
};

const MAX_PTY_EVENTS: usize = 512;
const MAX_PTY_OUTPUT_BYTES: usize = 256 * 1024;

struct PtySharedState {
    session: PtySessionState,
    parser: Parser,
    raw_output: String,
    output_window_start: u64,
    events: VecDeque<PtyEvent>,
    next_event_sequence: u64,
}

impl PtySharedState {
    fn new(
        pty_id: PtyId,
        owner_runtime_id: RuntimeId,
        label: Option<String>,
        cwd: String,
        rows: u16,
        cols: u16,
    ) -> Self {
        Self {
            session: PtySessionState {
                pty_id,
                label,
                owner_runtime_id,
                cwd,
                rows,
                cols,
                status: PtyStatus::Starting,
                output_cursor: 0,
                exit_code: None,
            },
            parser: Parser::new(rows.max(1), cols.max(1), 0),
            raw_output: String::new(),
            output_window_start: 0,
            events: VecDeque::new(),
            next_event_sequence: 1,
        }
    }

    fn session_state(&self) -> PtySessionState {
        self.session.clone()
    }

    fn set_status(
        &mut self,
        status: PtyStatus,
        event_tx: &broadcast::Sender<PtyEvent>,
    ) -> Option<PtyEvent> {
        if self.session.status == status {
            return None;
        }
        self.session.status = status;
        Some(self.push_event(
            PtyEventKind::StatusChanged,
            serde_json::json!({
                "status": status,
            }),
            event_tx,
        ))
    }

    fn set_exit_code(&mut self, exit_code: Option<i32>) {
        self.session.exit_code = exit_code;
    }

    fn push_output(&mut self, text: &str, event_tx: &broadcast::Sender<PtyEvent>) -> PtyEvent {
        self.raw_output.push_str(text);
        self.session.output_cursor += text.len() as u64;

        while self.raw_output.len() > MAX_PTY_OUTPUT_BYTES {
            let mut drain_to = self.raw_output.len() - MAX_PTY_OUTPUT_BYTES;
            while drain_to < self.raw_output.len() && !self.raw_output.is_char_boundary(drain_to) {
                drain_to += 1;
            }
            self.raw_output.drain(..drain_to);
            self.output_window_start += drain_to as u64;
        }

        self.parser.process(text.as_bytes());
        self.push_event(
            PtyEventKind::Output,
            serde_json::json!({
                "text": text,
                "output_cursor": self.session.output_cursor,
            }),
            event_tx,
        )
    }

    fn resize(&mut self, rows: u16, cols: u16, event_tx: &broadcast::Sender<PtyEvent>) -> PtyEvent {
        self.session.rows = rows.max(1);
        self.session.cols = cols.max(1);
        self.parser
            .screen_mut()
            .set_size(self.session.rows, self.session.cols);
        self.push_event(
            PtyEventKind::Resized,
            serde_json::json!({
                "rows": self.session.rows,
                "cols": self.session.cols,
            }),
            event_tx,
        )
    }

    fn snapshot(&self, mode: PtyCaptureMode, since_cursor: Option<u64>) -> crate::PtySnapshot {
        let incremental_output = match mode {
            PtyCaptureMode::VisibleScreen => None,
            PtyCaptureMode::Incremental => {
                let cursor = since_cursor.unwrap_or(self.output_window_start);
                let start = cursor.max(self.output_window_start);
                let relative = start.saturating_sub(self.output_window_start) as usize;
                let slice = if relative >= self.raw_output.len() {
                    String::new()
                } else if self.raw_output.is_char_boundary(relative) {
                    self.raw_output[relative..].to_owned()
                } else {
                    self.raw_output
                        .char_indices()
                        .find_map(|(index, _)| {
                            (index >= relative).then(|| self.raw_output[index..].to_owned())
                        })
                        .unwrap_or_default()
                };
                Some(slice)
            }
        };

        crate::PtySnapshot {
            pty_id: self.session.pty_id,
            visible_screen: self.parser.screen().contents(),
            incremental_output,
            output_cursor: self.session.output_cursor,
        }
    }

    fn push_event(
        &mut self,
        kind: PtyEventKind,
        payload: serde_json::Value,
        event_tx: &broadcast::Sender<PtyEvent>,
    ) -> PtyEvent {
        let event = PtyEvent {
            pty_id: self.session.pty_id,
            sequence: self.next_event_sequence,
            kind,
            timestamp_ms: now_millis(),
            payload,
        };
        self.next_event_sequence += 1;
        self.events.push_back(event.clone());
        if self.events.len() > MAX_PTY_EVENTS {
            self.events.pop_front();
        }
        let _ = event_tx.send(event.clone());
        event
    }
}

/// Runtime-managed PTY session handle backed by a real pseudo-terminal.
pub(crate) struct PtyHandle {
    pty_id: PtyId,
    shared: Arc<StdMutex<PtySharedState>>,
    writer_tx: mpsc::Sender<Vec<u8>>,
    master: Arc<TokioMutex<Box<dyn MasterPty + Send>>>,
    killer: Arc<StdMutex<Option<Box<dyn portable_pty::ChildKiller + Send + Sync>>>>,
    event_tx: broadcast::Sender<PtyEvent>,
    exit_status: Arc<AtomicBool>,
    exit_code: Arc<StdMutex<Option<i32>>>,
    reader_handle: Arc<StdMutex<Option<JoinHandle<()>>>>,
    writer_handle: Arc<StdMutex<Option<JoinHandle<()>>>>,
    wait_handle: Arc<StdMutex<Option<JoinHandle<()>>>>,
}

impl PtyHandle {
    pub(crate) fn open(
        owner_runtime_id: RuntimeId,
        request: OpenPtyRequest,
    ) -> Result<Self, RuntimeError> {
        let pty_id = PtyId::new();
        let cwd = match request.cwd {
            Some(cwd) => PathBuf::from(cwd),
            None => std::env::current_dir()
                .map_err(|error| RuntimeError::Internal(format!("failed to get cwd: {error}")))?,
        };
        let cwd_string = cwd.to_string_lossy().into_owned();

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: request.rows.max(1),
                cols: request.cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| RuntimeError::Internal(format!("failed to open PTY: {error}")))?;

        let mut command = CommandBuilder::new("bash");
        command.arg("--noprofile");
        command.arg("--norc");
        command.arg("-i");
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");
        command.env("PS1", "");

        let mut child = pair.slave.spawn_command(command).map_err(|error| {
            RuntimeError::Internal(format!("failed to spawn PTY shell: {error}"))
        })?;
        let killer = child.clone_killer();

        let (event_tx, _) = broadcast::channel(512);
        let shared = Arc::new(StdMutex::new(PtySharedState::new(
            pty_id,
            owner_runtime_id,
            request.label,
            cwd_string,
            request.rows.max(1),
            request.cols.max(1),
        )));
        if let Ok(mut state) = shared.lock() {
            state.set_status(PtyStatus::Idle, &event_tx);
        }

        let mut reader = pair.master.try_clone_reader().map_err(|error| {
            RuntimeError::Internal(format!("failed to clone PTY reader: {error}"))
        })?;
        let writer = pair.master.take_writer().map_err(|error| {
            RuntimeError::Internal(format!("failed to take PTY writer: {error}"))
        })?;
        let master = Arc::new(TokioMutex::new(pair.master));

        let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(128);
        let exit_status = Arc::new(AtomicBool::new(false));
        let exit_code = Arc::new(StdMutex::new(None));

        let reader_shared = shared.clone();
        let reader_event_tx = event_tx.clone();
        let reader_handle = tokio::task::spawn_blocking(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        let text = String::from_utf8_lossy(&buffer[..read]).into_owned();
                        if let Ok(mut state) = reader_shared.lock() {
                            state.push_output(&text, &reader_event_tx);
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        let writer = Arc::new(TokioMutex::new(writer));
        let writer_handle = tokio::spawn({
            let writer = writer.clone();
            async move {
                while let Some(bytes) = writer_rx.recv().await {
                    let mut guard = writer.lock().await;
                    let _ = guard.write_all(&bytes);
                    let _ = guard.flush();
                }
            }
        });

        let wait_shared = shared.clone();
        let wait_event_tx = event_tx.clone();
        let wait_exit_status = exit_status.clone();
        let wait_exit_code = exit_code.clone();
        let wait_handle = tokio::task::spawn_blocking(move || {
            let code = match child.wait() {
                Ok(status) => status.exit_code() as i32,
                Err(_) => 1,
            };
            wait_exit_status.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = wait_exit_code.lock() {
                *guard = Some(code);
            }
            if let Ok(mut state) = wait_shared.lock() {
                state.set_exit_code(Some(code));
                state.set_status(PtyStatus::Closed, &wait_event_tx);
                state.push_event(
                    PtyEventKind::Closed,
                    serde_json::json!({ "exit_code": code }),
                    &wait_event_tx,
                );
            }
        });

        Ok(Self {
            pty_id,
            shared,
            writer_tx,
            master,
            killer: Arc::new(StdMutex::new(Some(killer))),
            event_tx,
            exit_status,
            exit_code,
            reader_handle: Arc::new(StdMutex::new(Some(reader_handle))),
            writer_handle: Arc::new(StdMutex::new(Some(writer_handle))),
            wait_handle: Arc::new(StdMutex::new(Some(wait_handle))),
        })
    }

    pub(crate) fn pty_id(&self) -> PtyId {
        self.pty_id
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<PtyEvent> {
        self.event_tx.subscribe()
    }

    pub(crate) fn snapshot_state(&self) -> Result<PtySessionState, RuntimeError> {
        self.shared
            .lock()
            .map(|state| state.session_state())
            .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))
    }

    pub(crate) async fn write_input(
        &self,
        input: String,
        wait_ms: Option<u64>,
        append_newline: bool,
        background: bool,
    ) -> Result<PtyExecResult, RuntimeError> {
        {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            state.set_status(PtyStatus::Running, &self.event_tx);
            state.push_event(
                PtyEventKind::ExecutionStarted,
                serde_json::json!({ "steps": [input.clone()] }),
                &self.event_tx,
            );
        }

        let mut bytes = input.as_bytes().to_vec();
        if append_newline {
            bytes.push(b'\n');
        }
        self.writer_tx
            .send(bytes)
            .await
            .map_err(|_| RuntimeError::Closed)?;

        let wait_ms = wait_ms.unwrap_or(50);
        if wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }

        let snapshot = {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            let status = if background {
                PtyStatus::Backgrounded
            } else if self.exit_status.load(Ordering::SeqCst) {
                PtyStatus::Closed
            } else {
                PtyStatus::Idle
            };
            state.set_status(status, &self.event_tx);
            let snapshot = state.snapshot(PtyCaptureMode::Incremental, None);
            state.push_event(
                PtyEventKind::ExecutionCompleted,
                serde_json::json!({
                    "steps": [input],
                    "backgrounded": background,
                    "output_cursor": snapshot.output_cursor,
                }),
                &self.event_tx,
            );
            snapshot
        };

        Ok(PtyExecResult {
            pty_id: self.pty_id,
            steps: vec![input],
            backgrounded: background,
            completed: !background,
            interrupted: false,
            shell_exited: self.exit_status.load(Ordering::SeqCst),
            exit_code: self.exit_code.lock().ok().and_then(|guard| *guard),
            snapshot,
        })
    }

    pub(crate) async fn execute_batch(
        &self,
        request: PtyExecRequest,
    ) -> Result<PtyExecResult, RuntimeError> {
        {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            state.set_status(PtyStatus::Running, &self.event_tx);
            state.push_event(
                PtyEventKind::ExecutionStarted,
                serde_json::json!({ "steps": request.steps }),
                &self.event_tx,
            );
        }

        for step in &request.steps {
            let mut bytes = step.as_bytes().to_vec();
            if !step.ends_with('\n') {
                bytes.push(b'\n');
            }
            self.writer_tx
                .send(bytes)
                .await
                .map_err(|_| RuntimeError::Closed)?;
        }

        let wait_ms = request.wait_ms.unwrap_or(75);
        if wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }

        let snapshot = {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            let status = if request.background {
                PtyStatus::Backgrounded
            } else if self.exit_status.load(Ordering::SeqCst) {
                PtyStatus::Closed
            } else {
                PtyStatus::Idle
            };
            state.set_status(status, &self.event_tx);
            let snapshot = state.snapshot(PtyCaptureMode::Incremental, None);
            state.push_event(
                PtyEventKind::ExecutionCompleted,
                serde_json::json!({
                    "steps": request.steps,
                    "backgrounded": request.background,
                    "output_cursor": snapshot.output_cursor,
                }),
                &self.event_tx,
            );
            snapshot
        };

        Ok(PtyExecResult {
            pty_id: self.pty_id,
            steps: request.steps,
            backgrounded: request.background,
            completed: !request.background,
            interrupted: false,
            shell_exited: self.exit_status.load(Ordering::SeqCst),
            exit_code: self.exit_code.lock().ok().and_then(|guard| *guard),
            snapshot,
        })
    }

    pub(crate) async fn capture(
        &self,
        request: PtyCaptureRequest,
    ) -> Result<PtyCaptureResult, RuntimeError> {
        let snapshot = self
            .shared
            .lock()
            .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?
            .snapshot(request.mode, request.since_cursor);
        Ok(PtyCaptureResult {
            pty_id: self.pty_id,
            snapshot,
        })
    }

    pub(crate) async fn resize(
        &self,
        rows: u16,
        cols: u16,
    ) -> Result<PtySessionState, RuntimeError> {
        {
            let master = self.master.lock().await;
            master
                .resize(PtySize {
                    rows: rows.max(1),
                    cols: cols.max(1),
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|error| {
                    RuntimeError::Internal(format!("failed to resize PTY: {error}"))
                })?;
        }
        let state = {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            state.resize(rows.max(1), cols.max(1), &self.event_tx);
            state.session_state()
        };
        Ok(state)
    }

    pub(crate) async fn interrupt(&self) -> Result<PtySessionState, RuntimeError> {
        self.writer_tx
            .send(vec![3u8])
            .await
            .map_err(|_| RuntimeError::Closed)?;
        let state = {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            state.push_event(
                PtyEventKind::Interrupted,
                serde_json::json!({}),
                &self.event_tx,
            );
            if !self.exit_status.load(Ordering::SeqCst) {
                state.set_status(PtyStatus::Idle, &self.event_tx);
            }
            state.session_state()
        };
        Ok(state)
    }

    pub(crate) fn background(&self) -> Result<PtySessionState, RuntimeError> {
        let state = {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            if !self.exit_status.load(Ordering::SeqCst) {
                state.set_status(PtyStatus::Backgrounded, &self.event_tx);
            }
            state.session_state()
        };
        Ok(state)
    }

    pub(crate) fn close(&self) -> Result<PtySessionState, RuntimeError> {
        if let Ok(mut killer) = self.killer.lock() {
            if let Some(mut killer) = killer.take() {
                let _ = killer.kill();
            }
        }
        let state = {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| RuntimeError::Internal("PTY state lock poisoned".into()))?;
            state.set_status(PtyStatus::Closed, &self.event_tx);
            state.push_event(PtyEventKind::Closed, serde_json::json!({}), &self.event_tx);
            state.session_state()
        };
        Ok(state)
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        let _ = self.close();
        if let Ok(mut handle) = self.reader_handle.lock() {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        }
        if let Ok(mut handle) = self.writer_handle.lock() {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        }
        if let Ok(mut handle) = self.wait_handle.lock() {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        }
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
