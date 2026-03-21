use std::cell::RefCell;
use std::path::{Path, PathBuf};

use agent_client_protocol as acp;
use brain_types::BrainError;
use tokio::sync::{mpsc, oneshot};

thread_local! {
    static ACTIVE_CONTEXT: RefCell<Option<AcpClientToolContext>> = const { RefCell::new(None) };
}

#[derive(Clone)]
pub struct AcpClientHandle {
    request_tx: mpsc::UnboundedSender<AcpClientRequest>,
}

impl AcpClientHandle {
    pub fn new(request_tx: mpsc::UnboundedSender<AcpClientRequest>) -> Self {
        Self { request_tx }
    }

    pub fn send(&self, request: AcpClientRequest) -> Result<(), BrainError> {
        self.request_tx
            .send(request)
            .map_err(|_| BrainError::ToolFailed {
                tool: "acp".into(),
                reason: "ACP client request bridge is unavailable".into(),
            })
    }
}

#[derive(Clone)]
pub struct AcpClientToolContext {
    pub handle: AcpClientHandle,
    pub session_id: acp::SessionId,
    pub cwd: PathBuf,
}

impl AcpClientToolContext {
    pub fn new(handle: AcpClientHandle, session_id: acp::SessionId, cwd: PathBuf) -> Self {
        Self {
            handle,
            session_id,
            cwd,
        }
    }
}

pub struct ActiveAcpClientContextGuard;

impl Drop for ActiveAcpClientContextGuard {
    fn drop(&mut self) {
        ACTIVE_CONTEXT.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}

pub fn set_active_acp_client_context(context: AcpClientToolContext) -> ActiveAcpClientContextGuard {
    ACTIVE_CONTEXT.with(|slot| {
        *slot.borrow_mut() = Some(context);
    });
    ActiveAcpClientContextGuard
}

pub fn active_acp_client_context(tool: &'static str) -> Result<AcpClientToolContext, BrainError> {
    ACTIVE_CONTEXT.with(|slot| {
        slot.borrow().clone().ok_or_else(|| BrainError::ToolFailed {
            tool: tool.into(),
            reason: "ACP client context is not available".into(),
        })
    })
}

pub fn resolve_acp_client_path(cwd: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    }
}

pub enum AcpClientRequest {
    WriteTextFile {
        session_id: acp::SessionId,
        path: PathBuf,
        content: String,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    ReadTextFile {
        session_id: acp::SessionId,
        path: PathBuf,
        response_tx: oneshot::Sender<Result<String, String>>,
    },
}
