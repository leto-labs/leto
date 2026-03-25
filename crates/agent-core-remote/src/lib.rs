//! Remote HTTP implementation of the shared `AgentCore` boundary.

pub mod protocol;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "client")]
pub use client::{AgentCoreRemote, AgentCoreRemoteConfig, RemoteError};
pub use protocol::*;
