use std::sync::Arc;

use agent_core::AgentCore;
use agent_store::StoreError;
use tokio_util::sync::CancellationToken;

use crate::compat::opencode::state::CompatState;
use crate::types::{AgentInfoRecord, AgentServerStatus};

const BUILTIN_AGENT_NAMES: &[&str] = &[
    "plan",
    "build",
    "general",
    "explore",
    "title",
    "summary",
    "compaction",
];

/// Hosted server state for the canonical and compatibility HTTP surfaces.
#[derive(Clone)]
pub struct AgentServer {
    core: Arc<dyn AgentCore>,
    compat: Arc<CompatState>,
    shutdown: CancellationToken,
}

impl AgentServer {
    /// Creates a new server around the shared `AgentCore` boundary.
    pub fn new(core: Arc<dyn AgentCore>) -> Self {
        Self {
            core,
            compat: Arc::new(CompatState::new()),
            shutdown: CancellationToken::new(),
        }
    }

    /// Returns the shared core boundary used by this server.
    pub fn core(&self) -> Arc<dyn AgentCore> {
        self.core.clone()
    }

    pub(crate) fn compat(&self) -> Arc<CompatState> {
        self.compat.clone()
    }

    pub(crate) fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    pub(crate) fn begin_shutdown(&self) {
        self.shutdown.cancel();
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

    pub(crate) fn agent_info(&self) -> Vec<AgentInfoRecord> {
        BUILTIN_AGENT_NAMES
            .iter()
            .map(|name| AgentInfoRecord {
                name: (*name).to_owned(),
                description: Some((*name).to_owned()),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agent_core::{AgentCore, AgentCoreNative};
    use agent_store::{Project, Session};
    use provider::{MockProvider, Provider};

    use super::*;

    async fn make_server_with_provider(provider: Arc<dyn Provider>) -> AgentServer {
        let core: Arc<dyn AgentCore> = Arc::new(
            AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
                .with_provider("mock", provider)
                .build()
                .await
                .unwrap(),
        );
        AgentServer::new(core)
    }

    #[tokio::test]
    async fn status_reports_registered_providers_loops_and_store_counts() {
        let server = make_server_with_provider(Arc::new(MockProvider::new())).await;

        let project = server
            .core()
            .store()
            .projects()
            .create(Project::new(
                Some("status-test".to_owned()),
                None,
                Default::default(),
            ))
            .await
            .unwrap();
        server
            .core()
            .store()
            .sessions()
            .create(Session::new(project.id))
            .await
            .unwrap();

        let status = server.status().await.unwrap();

        assert_eq!(status.provider_names, vec!["mock".to_owned()]);
        assert_eq!(status.default_provider_name, "mock");
        assert_eq!(status.default_loop_name, "simple");
        assert!(status.loop_names.contains(&status.default_loop_name));
        assert_eq!(status.project_count, 1);
        assert_eq!(status.session_count, 1);
    }
}
