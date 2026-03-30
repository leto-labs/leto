//! ACP adapter surface backed by the shared `AgentCore` boundary.

pub mod adapter;
pub mod capabilities;
pub mod errors;
pub mod event_mapper;
pub mod history_replay;
pub mod ids;
pub mod stdio;

pub use adapter::AgentCoreAcpBackend;
pub use stdio::run_stdio;

#[cfg(test)]
mod tests;
