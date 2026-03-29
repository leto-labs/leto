use futures::future::BoxFuture;
use ulid::Ulid;

use brain_core::OpenAiConfigPreset;
use brain_types::*;

use crate::types::ServerStatus;

use super::BrainServer;

impl BrainServer {
    pub(super) fn list_providers_api(
        &self,
    ) -> BoxFuture<'_, Result<Vec<ProviderInfo>, BrainError>> {
        Box::pin(async move { Ok(vec![self.inner.brain.provider.info()]) })
    }

    pub(super) fn list_credentials_api(
        &self,
    ) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>> {
        Box::pin(async move {
            let mut credentials = Vec::new();
            for preset in OpenAiConfigPreset::ALL {
                let provider_name = preset.name.to_owned();
                for entry in self
                    .inner
                    .brain
                    .store
                    .credentials()
                    .list_for_provider(preset.name)
                    .await?
                {
                    credentials.push((provider_name.clone(), entry));
                }
            }
            Ok(credentials)
        })
    }

    pub(super) fn get_credentials_api(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move {
            self.inner
                .brain
                .store
                .credentials()
                .list_for_provider(&name)
                .await
        })
    }

    pub(super) fn save_credential_api(
        &self,
        provider_name: &str,
        entry: CredentialEntry,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move {
            let key = (name, entry.id.clone());
            match self.inner.brain.store.credentials().get(key.clone()).await {
                Ok(_) => {
                    self.inner
                        .brain
                        .store
                        .credentials()
                        .update(key, entry)
                        .await?;
                }
                Err(_) => {
                    self.inner
                        .brain
                        .store
                        .credentials()
                        .create(key, entry)
                        .await?;
                }
            }
            Ok(())
        })
    }

    pub(super) fn delete_credential_api(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        let cid = credential_id.to_owned();
        Box::pin(async move {
            self.inner
                .brain
                .store
                .credentials()
                .delete((name, cid))
                .await
        })
    }

    pub(super) fn status_api(&self) -> BoxFuture<'_, Result<ServerStatus, BrainError>> {
        let inner = &self.inner;
        Box::pin(async move {
            let tools: Vec<String> = inner
                .brain
                .tools
                .iter()
                .map(|t| t.definition().name)
                .collect();
            let active = inner.active_turns.read().await;
            let active_turn_ids: Vec<Ulid> = active.keys().copied().collect();
            let projects = inner.brain.store.projects().list().await?;

            let mut total_sessions = 0usize;
            for project in &projects {
                let sessions = inner
                    .brain
                    .store
                    .sessions()
                    .list_for_project(project.id)
                    .await?;
                total_sessions += sessions.len();
            }

            Ok(ServerStatus {
                providers: vec![inner.brain.provider.info()],
                tools,
                active_sessions: total_sessions,
                active_turns: active_turn_ids,
            })
        })
    }
}
