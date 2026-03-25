use std::sync::Arc;

use agent_core::AgentCore;
use agent_store::StoreError;

use crate::types::AgentServerStatus;

/// Hosted server state for the canonical and compatibility HTTP surfaces.
#[derive(Clone)]
pub struct AgentServer {
    core: Arc<dyn AgentCore>,
}

impl AgentServer {
    /// Creates a new server around the shared `AgentCore` boundary.
    pub fn new(core: Arc<dyn AgentCore>) -> Self {
        Self { core }
    }

    /// Returns the shared core boundary used by this server.
    pub fn core(&self) -> Arc<dyn AgentCore> {
        self.core.clone()
    }

    /// Returns lightweight server status information.
    pub async fn status(&self) -> Result<AgentServerStatus, StoreError> {
        let core = self.core();
        Ok(AgentServerStatus {
            provider_names: core.provider_names(),
            loop_names: core.loop_names(),
            default_provider_name: core.default_provider_name().to_owned(),
            default_loop_name: core.default_loop_name().to_owned(),
            project_count: core.store().projects().list().await?.len(),
            session_count: core.store().sessions().list().await?.len(),
        })
    }
}
