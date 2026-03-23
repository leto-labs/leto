#[cfg(feature = "acp")]
pub mod acp;
pub mod apply_patch;
pub mod echo;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob_search;
pub mod grep;
pub mod list_directory;
pub mod shell;
pub mod terminal_session;
mod truncation;

pub use apply_patch::{ApplyPatchDriver, ApplyPatchTool};
pub use echo::EchoTool;
pub use file_edit::{FileEditDriver, FileEditTool};
pub use file_read::{FileReadDriver, FileReadTool};
pub use file_write::{FileWriteDriver, FileWriteTool};
pub use glob_search::{GlobDriver, GlobTool};
pub use grep::{GrepDriver, GrepTool};
pub use list_directory::{ListDirectoryDriver, ListDirectoryTool};
pub use shell::{ShellDriver, ShellTool};
pub use terminal_session::{
    TerminalSessionAction, TerminalSessionDriver, TerminalSessionObservation,
    TerminalSessionRequest, TerminalSessionTool,
};

#[cfg(feature = "acp")]
pub use acp::{
    AcpClientHandle, AcpClientRequest, AcpClientToolContext, ActiveAcpClientContextGuard,
    active_acp_client_context, resolve_acp_client_path, set_active_acp_client_context,
};
#[cfg(feature = "native")]
pub use apply_patch::native::ApplyPatchDriverNative;
#[cfg(feature = "native")]
pub use file_edit::native::FileEditDriverNative;
#[cfg(feature = "acp")]
pub use file_read::acp::FileReadDriverAcp;
#[cfg(feature = "native")]
pub use file_read::native::FileReadDriverNative;
#[cfg(feature = "acp")]
pub use file_write::acp::FileWriteDriverAcp;
#[cfg(feature = "native")]
pub use file_write::native::FileWriteDriverNative;
#[cfg(feature = "native")]
pub use glob_search::native::GlobDriverNative;
#[cfg(feature = "native")]
pub use grep::native::{GrepDriverAuto, GrepDriverNative, GrepDriverRipgrep};
#[cfg(feature = "native")]
pub use list_directory::native::ListDirectoryDriverNative;
#[cfg(feature = "native")]
pub use shell::native::ShellDriverNative;
#[cfg(feature = "native")]
pub use terminal_session::native::TerminalSessionDriverNative;

#[cfg(feature = "native")]
pub fn native_tools() -> Vec<std::sync::Arc<dyn brain_types::Tool>> {
    use std::sync::Arc;
    vec![
        Arc::new(EchoTool),
        Arc::new(FileReadTool::new(FileReadDriverNative)),
        Arc::new(FileWriteTool::new(FileWriteDriverNative)),
        Arc::new(FileEditTool::new(FileEditDriverNative)),
        Arc::new(ApplyPatchTool::new(ApplyPatchDriverNative)),
        Arc::new(ShellTool::new(ShellDriverNative)),
        Arc::new(TerminalSessionTool::new(
            TerminalSessionDriverNative::default(),
        )),
        Arc::new(ListDirectoryTool::new(ListDirectoryDriverNative)),
        Arc::new(GlobTool::new(GlobDriverNative)),
        Arc::new(GrepTool::new(GrepDriverAuto::default())),
    ]
}
