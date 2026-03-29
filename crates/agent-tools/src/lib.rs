//! Typed v2 tool implementations and the default native tool executor.

mod apply_patch;
mod echo;
mod file_edit;
mod file_read;
mod file_write;
mod glob_search;
mod grep;
mod list_directory;
mod registry;
mod shell;
mod truncation;

use std::sync::Arc;

pub use apply_patch::NativeApplyPatchDriver;
pub use apply_patch::{ApplyPatchDriver, ApplyPatchRequest, ApplyPatchResponse, ApplyPatchTool};
pub use echo::{EchoRequest, EchoResponse, EchoTool};
pub use file_edit::NativeFileEditDriver;
pub use file_edit::{FileEditDriver, FileEditRequest, FileEditResponse, FileEditTool};
pub use file_read::NativeFileReadDriver;
pub use file_read::{FileReadDriver, FileReadRequest, FileReadResponse, FileReadTool};
pub use file_write::NativeFileWriteDriver;
pub use file_write::{FileWriteDriver, FileWriteRequest, FileWriteResponse, FileWriteTool};
pub use glob_search::NativeGlobSearchDriver;
pub use glob_search::{GlobSearchDriver, GlobSearchRequest, GlobSearchResponse, GlobSearchTool};
pub use grep::{AutoGrepDriver, NativeGrepDriver, RipgrepDriver};
pub use grep::{GrepDriver, GrepRequest, GrepResponse, GrepTool};
pub use list_directory::NativeListDirectoryDriver;
pub use list_directory::{
    ListDirectoryDriver, ListDirectoryRequest, ListDirectoryResponse, ListDirectoryTool,
};
pub use registry::{RegistryToolExecutor, TypedTool};
pub use shell::NativeShellDriver;
pub use shell::{ShellDriver, ShellRequest, ShellResponse, ShellTool};

/// Builds the default native tool executor for local runs.
pub fn native_tools() -> RegistryToolExecutor {
    let mut registry = RegistryToolExecutor::new();
    registry.register(Arc::new(EchoTool));
    registry.register(Arc::new(FileReadTool::new(NativeFileReadDriver)));
    registry.register(Arc::new(FileWriteTool::new(NativeFileWriteDriver)));
    registry.register(Arc::new(FileEditTool::new(NativeFileEditDriver)));
    registry.register(Arc::new(ApplyPatchTool::new(NativeApplyPatchDriver)));
    registry.register(Arc::new(ShellTool::new(NativeShellDriver)));
    registry.register(Arc::new(ListDirectoryTool::new(NativeListDirectoryDriver)));
    registry.register(Arc::new(GlobSearchTool::new(NativeGlobSearchDriver)));
    registry.register(Arc::new(GrepTool::new(AutoGrepDriver)));
    registry
}

#[cfg(test)]
mod tests {
    use super::native_tools;

    #[test]
    fn native_tools_register_expected_names() {
        let executor = native_tools();
        assert_eq!(
            executor.tool_names(),
            vec![
                "apply_patch",
                "echo",
                "file_edit",
                "file_read",
                "file_write",
                "glob_search",
                "grep",
                "list_directory",
                "shell",
            ]
        );
    }
}
