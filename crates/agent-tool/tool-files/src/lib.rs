//! Canonical file and workspace-oriented tool implementations.

pub mod apply_patch;
#[cfg(feature = "component")]
pub mod component;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob_search;
pub mod grep;
pub mod list_directory;
#[cfg(feature = "native")]
mod truncation;

#[cfg(feature = "native")]
use std::sync::Arc;

#[cfg(feature = "native")]
use agent_tool::{RegistryToolExecutor, ToolRegistrationError};

pub use apply_patch::{ApplyPatchDriver, ApplyPatchRequest, ApplyPatchResponse, ApplyPatchTool};
#[cfg(feature = "component")]
pub use component::{ComponentError, ComponentHost, ComponentToolPlugin};
pub use file_edit::{FileEditDriver, FileEditRequest, FileEditResponse, FileEditTool};
pub use file_read::{FileReadDriver, FileReadRequest, FileReadResponse, FileReadTool};
pub use file_write::{FileWriteDriver, FileWriteRequest, FileWriteResponse, FileWriteTool};
pub use glob_search::{GlobSearchDriver, GlobSearchRequest, GlobSearchResponse, GlobSearchTool};
pub use grep::{GrepDriver, GrepRequest, GrepResponse, GrepTool};
pub use list_directory::{
    ListDirectoryDriver, ListDirectoryRequest, ListDirectoryResponse, ListDirectoryTool,
};

#[cfg(feature = "native")]
pub use apply_patch::native::NativeApplyPatchDriver;
#[cfg(feature = "native")]
pub use file_edit::native::NativeFileEditDriver;
#[cfg(feature = "native")]
pub use file_read::native::NativeFileReadDriver;
#[cfg(feature = "native")]
pub use file_write::native::NativeFileWriteDriver;
#[cfg(feature = "native")]
pub use glob_search::native::NativeGlobSearchDriver;
#[cfg(feature = "native")]
pub use grep::native::{AutoGrepDriver, NativeGrepDriver, RipgrepDriver};
#[cfg(feature = "native")]
pub use list_directory::native::NativeListDirectoryDriver;

/// Registers the native file/workspace tools into an existing registry.
#[cfg(feature = "native")]
pub fn register_native_tools(
    registry: &mut RegistryToolExecutor,
) -> Result<(), ToolRegistrationError> {
    registry
        .register(Arc::new(FileReadTool::new(NativeFileReadDriver)))
        .expect("builtin file tool name should be unique");
    registry
        .register(Arc::new(FileWriteTool::new(NativeFileWriteDriver)))
        .expect("builtin file tool name should be unique");
    registry
        .register(Arc::new(FileEditTool::new(NativeFileEditDriver)))
        .expect("builtin file tool name should be unique");
    registry
        .register(Arc::new(ApplyPatchTool::new(NativeApplyPatchDriver)))
        .expect("builtin file tool name should be unique");
    registry
        .register(Arc::new(ListDirectoryTool::new(NativeListDirectoryDriver)))
        .expect("builtin file tool name should be unique");
    registry
        .register(Arc::new(GlobSearchTool::new(NativeGlobSearchDriver)))
        .expect("builtin file tool name should be unique");
    registry
        .register(Arc::new(GrepTool::new(AutoGrepDriver)))
        .expect("builtin file tool name should be unique");
    Ok(())
}

/// Builds a registry containing only the native file/workspace tools.
#[cfg(feature = "native")]
pub fn native_tools() -> RegistryToolExecutor {
    let mut registry = RegistryToolExecutor::new();
    register_native_tools(&mut registry).expect("builtin file tool names should be unique");
    registry
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "native")]
    use super::native_tools;

    #[test]
    #[cfg(feature = "native")]
    fn native_tools_register_expected_names() {
        let executor = native_tools();
        assert_eq!(
            executor.tool_names(),
            vec![
                "apply_patch",
                "file_edit",
                "file_read",
                "file_write",
                "glob_search",
                "grep",
                "list_directory",
            ]
        );
    }
}
