//! Canonical process-oriented tool implementations.

pub mod shell;
#[cfg(feature = "native")]
mod truncation;

#[cfg(feature = "native")]
use std::sync::Arc;

#[cfg(feature = "native")]
use agent_tool::RegistryToolExecutor;

pub use shell::{ShellDriver, ShellRequest, ShellResponse, ShellTool};

#[cfg(feature = "native")]
pub use shell::native::NativeShellDriver;

/// Registers the native process tools into an existing registry.
#[cfg(feature = "native")]
pub fn register_native_tools(
    registry: &mut RegistryToolExecutor,
) -> Result<(), agent_tool::ToolRegistrationError> {
    registry.register(Arc::new(ShellTool::new(NativeShellDriver)))
}

/// Builds a registry containing only the native process tools.
#[cfg(feature = "native")]
pub fn native_tools() -> RegistryToolExecutor {
    let mut registry = RegistryToolExecutor::new();
    register_native_tools(&mut registry).expect("builtin process tool names should be unique");
    registry
}
