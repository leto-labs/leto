pub mod echo;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob_search;
pub mod grep;
pub mod shell;

pub use echo::EchoTool;
pub use file_edit::{FileEditDriver, FileEditTool};
pub use file_read::{FileReadDriver, FileReadTool};
pub use file_write::{FileWriteDriver, FileWriteTool};
pub use glob_search::{GlobDriver, GlobTool};
pub use grep::{GrepDriver, GrepTool};
pub use shell::{ShellDriver, ShellTool};

#[cfg(feature = "native")]
pub use file_edit::native::FileEditDriverNative;
#[cfg(feature = "native")]
pub use file_read::native::FileReadDriverNative;
#[cfg(feature = "native")]
pub use file_write::native::FileWriteDriverNative;
#[cfg(feature = "native")]
pub use glob_search::native::GlobDriverNative;
#[cfg(feature = "native")]
pub use grep::native::GrepDriverNative;
#[cfg(feature = "native")]
pub use shell::native::ShellDriverNative;

#[cfg(feature = "native")]
pub fn native_tools() -> Vec<std::sync::Arc<dyn brain_types::Tool>> {
    use std::sync::Arc;
    vec![
        Arc::new(EchoTool),
        Arc::new(FileReadTool::new(FileReadDriverNative)),
        Arc::new(FileWriteTool::new(FileWriteDriverNative)),
        Arc::new(FileEditTool::new(FileEditDriverNative)),
        Arc::new(ShellTool::new(ShellDriverNative)),
        Arc::new(GlobTool::new(GlobDriverNative)),
        Arc::new(GrepTool::new(GrepDriverNative)),
    ]
}
