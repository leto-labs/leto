use std::path::PathBuf;
use std::pin::Pin;

use futures::{Stream, future::BoxFuture};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use crate::registry::{RegistryLoop, RegistryProvider, RegistryTool};
use crate::{
    AgentConfig, BrainError, Event, EventStream, InferenceConfig, Project, ProjectId,
    ProviderModelInfo, Session, Store,
};

pub type RuntimeBusStream = Pin<Box<dyn Stream<Item = RuntimeBusEvent> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeBusEvent {
    Turn {
        session_id: Ulid,
        event: Event,
    },
    ProjectCreated {
        project: Project,
    },
    ProjectUpdated {
        project: Project,
    },
    ProjectDeleted {
        project_id: ProjectId,
    },
    SessionCreated {
        session: Session,
    },
    SessionUpdated {
        session: Session,
    },
    SessionDeleted {
        session_id: Ulid,
        project_id: ProjectId,
    },
}

pub trait BrainRuntime: Send + Sync {
    fn store(&self) -> &dyn Store;

    fn providers(&self) -> &dyn RegistryProvider;

    fn tools(&self) -> &dyn RegistryTool;

    fn loops(&self) -> &dyn RegistryLoop;

    fn resolve_or_create_project(
        &self,
        root: PathBuf,
    ) -> BoxFuture<'_, Result<Project, BrainError>>;

    fn subscribe(&self) -> RuntimeBusStream;

    fn turn(&self, session_id: Ulid, input: &str, cancel: CancellationToken) -> EventStream;

    fn cancel_turn(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>>;

    fn effective_inference_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<InferenceConfig, BrainError>>;

    fn effective_agent_config_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<AgentConfig, BrainError>>;

    fn current_loop_name_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Option<String>, BrainError>> {
        Box::pin(async move {
            let session = self.store().sessions().get(session_id).await?;
            if let Some(loop_name) = session.loop_name {
                return Ok(Some(loop_name));
            }

            let project = self.store().projects().get(session.project_id).await?;
            Ok(project.config.agent.loop_name)
        })
    }

    fn list_models(&self) -> Result<Vec<ProviderModelInfo>, BrainError> {
        let mut models = self
            .providers()
            .list()?
            .into_iter()
            .flat_map(|(provider_name, provider)| {
                provider
                    .info()
                    .models
                    .into_iter()
                    .map(move |model| ProviderModelInfo {
                        provider: provider_name.clone(),
                        model,
                    })
            })
            .collect::<Vec<_>>();
        models.sort_by(|left, right| {
            right
                .model
                .last_updated
                .or(right.model.release_date)
                .cmp(&left.model.last_updated.or(left.model.release_date))
                .then_with(|| right.model.release_date.cmp(&left.model.release_date))
                .then_with(|| left.provider.cmp(&right.provider))
                .then_with(|| left.model.id.cmp(right.model.id))
        });
        Ok(models)
    }

    fn current_model_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<ProviderModelInfo, BrainError>> {
        Box::pin(async move {
            let effective = self.effective_inference_for_session(session_id).await?;
            let current_model_id = self
                .current_model_id_for_session(session_id)
                .await?
                .ok_or_else(|| BrainError::Internal("no current model for session".into()))?;
            let models = self.list_models()?;
            let matches = models
                .into_iter()
                .filter(|model| {
                    model.model.id == current_model_id
                        && effective
                            .provider
                            .as_deref()
                            .is_none_or(|provider| model.provider == provider)
                })
                .collect::<Vec<_>>();

            match matches.as_slice() {
                [] => Err(BrainError::Internal(format!(
                    "current model is not available: {current_model_id}"
                ))),
                [model] => Ok(model.clone()),
                _ => Err(BrainError::Internal(format!(
                    "current model is ambiguous: {current_model_id}"
                ))),
            }
        })
    }

    fn current_model_id_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Option<String>, BrainError>>;

    fn set_session_model(
        &self,
        session_id: Ulid,
        model_id: &str,
    ) -> BoxFuture<'_, Result<Session, BrainError>> {
        let model_id = model_id.to_owned();
        Box::pin(async move {
            let matches = self
                .providers()
                .list()?
                .into_iter()
                .filter_map(|(provider_name, provider)| {
                    provider
                        .info()
                        .models
                        .iter()
                        .any(|model| model.id == model_id)
                        .then_some(provider_name)
                })
                .collect::<Vec<_>>();

            let provider_name = match matches.as_slice() {
                [] | [_, _, ..] => {
                    return Err(BrainError::Internal(format!(
                        "unknown or ambiguous model: {model_id}"
                    )));
                }
                [provider_name] => provider_name.clone(),
            };

            let mut session = self.store().sessions().get(session_id).await?;
            let mut inference = session.inference.unwrap_or_default();
            let selected_model = self
                .providers()
                .get(&provider_name)
                .ok_or_else(|| {
                    BrainError::Internal(format!("provider not registered: {provider_name}"))
                })?
                .info()
                .models
                .into_iter()
                .find(|model| model.id == model_id)
                .ok_or_else(|| {
                    BrainError::Internal(format!(
                        "model {model_id} is not available for provider {provider_name}"
                    ))
                })?;

            inference.provider = Some(provider_name);
            inference.model = Some(model_id);
            let keep_reasoning = match (inference.reasoning.as_deref(), selected_model.reasoning) {
                (None, _) => true,
                (Some(value), Some(levels)) => levels.contains(&value),
                (Some(_), None) => false,
            };
            if !keep_reasoning {
                inference.reasoning = None;
            }
            session.inference = Some(inference);

            let key = session.id;
            self.store().sessions().update(key, session).await
        })
    }

    fn set_session_thought_level(
        &self,
        session_id: Ulid,
        thought_level: &str,
    ) -> BoxFuture<'_, Result<Session, BrainError>> {
        let thought_level = thought_level.to_owned();
        Box::pin(async move {
            let current_model = self.current_model_for_session(session_id).await?;
            let supported = current_model.model.reasoning.ok_or_else(|| {
                BrainError::Internal(format!(
                    "model {} does not support thought level configuration",
                    current_model.model.id
                ))
            })?;
            if !supported.contains(&thought_level.as_str()) {
                return Err(BrainError::Internal(format!(
                    "invalid thought level {thought_level} for model {}",
                    current_model.model.id
                )));
            }

            let mut session = self.store().sessions().get(session_id).await?;
            let mut inference = session.inference.unwrap_or_default();
            inference.reasoning = Some(thought_level);
            session.inference = Some(inference);

            self.store().sessions().update(session.id, session).await
        })
    }

    fn set_session_loop(
        &self,
        session_id: Ulid,
        loop_name: &str,
    ) -> BoxFuture<'_, Result<Session, BrainError>> {
        let loop_name = loop_name.to_owned();
        Box::pin(async move {
            if self.loops().get(&loop_name).is_none() {
                return Err(BrainError::Internal(format!(
                    "loop not registered: {loop_name}"
                )));
            }

            let mut session = self.store().sessions().get(session_id).await?;
            session.loop_name = Some(loop_name);
            self.store().sessions().update(session.id, session).await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::Utc;
    use futures::{StreamExt, future::ready};
    use std::sync::Arc;

    use super::*;
    use crate::registry::{
        Registry, RegistryLoopHashMap, RegistryProviderHashMap, RegistryToolHashMap,
    };
    use crate::{
        AgentLoop, CredentialEntry, CredentialHealth, CredentialStoreKey, CrudStore, Event,
        Message, MessageStoreKey, ModelInfo, ProjectConfig, ProjectId, ProjectStore, Provider,
        ProviderInfo, Session, Tool, ToolDef,
    };

    struct DummyProjectStore;

    impl CrudStore for DummyProjectStore {
        type Key = ProjectId;
        type Record = Project;
        type Event = crate::ProjectStoreEvent;

        fn create(
            &self,
            _key: ProjectId,
            project: Project,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(project)))
        }

        fn get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(Project {
                id,
                name: Some("dummy".into()),
                root: None,
                config: ProjectConfig::default(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: ProjectId,
            project: Project,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(project)))
        }

        fn delete(&self, _id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> crate::ProjectStoreEventStream {
            Box::pin(futures::stream::iter(vec![
                crate::ProjectStoreEvent::Created {
                    project: Project::new(Some("dummy".into()), None, ProjectConfig::default()),
                },
            ]))
        }
    }

    impl ProjectStore for DummyProjectStore {
        fn find_by_root(&self, _root: &Path) -> BoxFuture<'_, Result<Option<Project>, BrainError>> {
            Box::pin(ready(Ok(None)))
        }
    }

    struct DummySessionStore;

    impl CrudStore for DummySessionStore {
        type Key = Ulid;
        type Record = Session;
        type Event = crate::SessionStoreEvent;

        fn create(
            &self,
            _key: Ulid,
            session: Session,
        ) -> BoxFuture<'_, Result<Session, BrainError>> {
            Box::pin(ready(Ok(session)))
        }

        fn get(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
            Box::pin(ready(Ok(Session {
                id,
                project_id: Ulid::nil(),
                title: None,
                inference: None,
                loop_name: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
            Box::pin(ready(Ok(vec![Session::new(Ulid::nil())])))
        }

        fn update(
            &self,
            _key: Ulid,
            session: Session,
        ) -> BoxFuture<'_, Result<Session, BrainError>> {
            Box::pin(ready(Ok(session)))
        }

        fn delete(&self, _id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> crate::SessionStoreEventStream {
            Box::pin(futures::stream::iter(vec![
                crate::SessionStoreEvent::Created {
                    session: Session::new(Ulid::nil()),
                },
            ]))
        }
    }

    impl crate::SessionStore for DummySessionStore {
        fn list_for_project(
            &self,
            _project_id: ProjectId,
        ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
            Box::pin(ready(Ok(vec![Session::new(Ulid::nil())])))
        }
    }

    struct DummyMessageStore;

    impl CrudStore for DummyMessageStore {
        type Key = MessageStoreKey;
        type Record = Message;
        type Event = crate::MessageStoreEvent;

        fn create(
            &self,
            _key: MessageStoreKey,
            record: Message,
        ) -> BoxFuture<'_, Result<Message, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn get(&self, key: MessageStoreKey) -> BoxFuture<'_, Result<Message, BrainError>> {
            Box::pin(ready(Ok(Message {
                id: key.1,
                role: crate::Role::User,
                content: "dummy".into(),
                tool_calls: Vec::new(),
                tool_call_id: None,
                created_at: Utc::now(),
            })))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: MessageStoreKey,
            record: Message,
        ) -> BoxFuture<'_, Result<Message, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, _key: MessageStoreKey) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> crate::MessageStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    impl crate::MessageStore for DummyMessageStore {
        fn list_for_session(
            &self,
            _session_id: Ulid,
        ) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }
    }

    struct DummyCredentialStore;

    impl CrudStore for DummyCredentialStore {
        type Key = CredentialStoreKey;
        type Record = CredentialEntry;
        type Event = crate::CredentialStoreEvent;

        fn create(
            &self,
            _key: CredentialStoreKey,
            record: CredentialEntry,
        ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn get(
            &self,
            key: CredentialStoreKey,
        ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
            Box::pin(ready(Ok(CredentialEntry::api_key(key.1, "test"))))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: CredentialStoreKey,
            record: CredentialEntry,
        ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, _key: CredentialStoreKey) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> crate::CredentialStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    impl crate::CredentialStore for DummyCredentialStore {
        fn list_for_provider(
            &self,
            _provider_name: &str,
        ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update_health(
            &self,
            _provider_name: &str,
            _credential_id: &str,
            _health: &CredentialHealth,
        ) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }
    }

    struct DummyStore;

    impl Store for DummyStore {
        fn projects(&self) -> &dyn ProjectStore {
            static STORE: DummyProjectStore = DummyProjectStore;
            &STORE
        }

        fn sessions(&self) -> &dyn crate::SessionStore {
            static STORE: DummySessionStore = DummySessionStore;
            &STORE
        }

        fn messages(&self) -> &dyn crate::MessageStore {
            static STORE: DummyMessageStore = DummyMessageStore;
            &STORE
        }

        fn credentials(&self) -> &dyn crate::CredentialStore {
            static STORE: DummyCredentialStore = DummyCredentialStore;
            &STORE
        }

        fn subscribe(&self) -> crate::StoreEventStream {
            Box::pin(futures::stream::iter(vec![
                crate::StoreEvent::ProjectCreated {
                    project: Project::new(Some("dummy".into()), None, ProjectConfig::default()),
                },
            ]))
        }
    }

    struct DummyProvider;

    impl Provider for DummyProvider {
        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<crate::ChatStream, BrainError>> {
            Box::pin(ready(Ok(
                Box::pin(futures::stream::empty()) as crate::ChatStream
            )))
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "dummy-provider".into(),
                default_model: Some("dummy-model".into()),
                models: vec![ModelInfo {
                    id: "dummy-model",
                    name: "Dummy Model",
                    family: None,
                    reasoning: Some(&["low", "medium", "high"]),
                    tool_call: true,
                    attachment: false,
                    structured_output: None,
                    temperature: None,
                    knowledge: None,
                    release_date: Some("2026-01-01"),
                    last_updated: Some("2026-02-01"),
                    open_weights: None,
                    input_modalities: &["text"],
                    output_modalities: &["text"],
                    cost: None,
                    limit: None,
                    status: None,
                }],
            }
        }
    }

    struct OlderDummyProvider;

    impl Provider for OlderDummyProvider {
        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<crate::ChatStream, BrainError>> {
            Box::pin(ready(Ok(
                Box::pin(futures::stream::empty()) as crate::ChatStream
            )))
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "older-provider".into(),
                default_model: Some("older-model".into()),
                models: vec![ModelInfo {
                    id: "older-model",
                    name: "Older Model",
                    family: None,
                    reasoning: None,
                    tool_call: true,
                    attachment: false,
                    structured_output: None,
                    temperature: None,
                    knowledge: None,
                    release_date: Some("2025-01-01"),
                    last_updated: Some("2025-01-15"),
                    open_weights: None,
                    input_modalities: &["text"],
                    output_modalities: &["text"],
                    cost: None,
                    limit: None,
                    status: None,
                }],
            }
        }
    }

    struct DummyTool;

    impl Tool for DummyTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "dummy-tool".into(),
                description: "dummy".into(),
                parameters: serde_json::json!({}),
            }
        }

        fn execute(&self, _args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
            Box::pin(ready(Ok("ok".into())))
        }
    }

    struct DummyLoop;

    impl AgentLoop for DummyLoop {
        fn run(
            &self,
            _provider: Arc<dyn Provider>,
            _tools: Vec<Arc<dyn Tool>>,
            _messages: Vec<Message>,
            _config: AgentConfig,
            _cancel: CancellationToken,
            _session_id: Option<Ulid>,
        ) -> EventStream {
            Box::pin(futures::stream::iter(vec![
                Event::MessageDone {
                    message: Message::assistant("loop"),
                },
                Event::TurnDone {
                    iterations: 1,
                    total_tokens: 0,
                },
            ]))
        }
    }

    struct DummyRuntime;

    impl BrainRuntime for DummyRuntime {
        fn store(&self) -> &dyn Store {
            static STORE: DummyStore = DummyStore;
            &STORE
        }

        fn providers(&self) -> &dyn RegistryProvider {
            static REGISTRY: std::sync::LazyLock<RegistryProviderHashMap> =
                std::sync::LazyLock::new(|| {
                    let registry = RegistryProviderHashMap::default();
                    registry
                        .set("dummy-provider".into(), Arc::new(DummyProvider))
                        .unwrap();
                    registry
                        .set("older-provider".into(), Arc::new(OlderDummyProvider))
                        .unwrap();
                    registry
                });
            &*REGISTRY
        }

        fn tools(&self) -> &dyn RegistryTool {
            static REGISTRY: std::sync::LazyLock<RegistryToolHashMap> =
                std::sync::LazyLock::new(|| {
                    let registry = RegistryToolHashMap::default();
                    registry
                        .set("dummy-tool".into(), Arc::new(DummyTool))
                        .unwrap();
                    registry
                });
            &*REGISTRY
        }

        fn loops(&self) -> &dyn RegistryLoop {
            static REGISTRY: std::sync::LazyLock<RegistryLoopHashMap> =
                std::sync::LazyLock::new(|| {
                    let registry = RegistryLoopHashMap::default();
                    registry
                        .set("dummy-loop".into(), Arc::new(DummyLoop))
                        .unwrap();
                    registry
                });
            &*REGISTRY
        }

        fn resolve_or_create_project(
            &self,
            _root: PathBuf,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(Project::new(
                Some("dummy".into()),
                None,
                ProjectConfig::default(),
            ))))
        }

        fn turn(&self, _session_id: Ulid, _input: &str, _cancel: CancellationToken) -> EventStream {
            Box::pin(futures::stream::iter(vec![Event::TurnDone {
                iterations: 0,
                total_tokens: 0,
            }]))
        }

        fn subscribe(&self) -> RuntimeBusStream {
            Box::pin(futures::stream::iter(vec![
                RuntimeBusEvent::ProjectCreated {
                    project: Project::new(Some("dummy".into()), None, ProjectConfig::default()),
                },
            ]))
        }

        fn cancel_turn(&self, _session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn effective_inference_for_session(
            &self,
            _session_id: Ulid,
        ) -> BoxFuture<'_, Result<InferenceConfig, BrainError>> {
            Box::pin(ready(Ok(InferenceConfig::default())))
        }

        fn effective_agent_config_for_session(
            &self,
            _session_id: Ulid,
        ) -> BoxFuture<'_, Result<AgentConfig, BrainError>> {
            Box::pin(ready(Ok(AgentConfig::default())))
        }

        fn current_model_id_for_session(
            &self,
            _session_id: Ulid,
        ) -> BoxFuture<'_, Result<Option<String>, BrainError>> {
            Box::pin(ready(Ok(Some("dummy-model".into()))))
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let runtime = DummyRuntime;
        let dyn_runtime: &dyn BrainRuntime = &runtime;

        let _ = dyn_runtime;
    }

    #[test]
    fn runtime_bus_subscription_is_available() {
        let runtime = DummyRuntime;
        let mut bus = runtime.subscribe();

        let event = futures::executor::block_on(bus.next()).unwrap();
        assert!(matches!(event, RuntimeBusEvent::ProjectCreated { .. }));
    }

    #[test]
    fn registry_list_exposes_named_runtime_items() {
        let runtime = DummyRuntime;

        let providers = runtime.providers().list().unwrap();
        let tools = runtime.tools().list().unwrap();
        let loops = runtime.loops().list().unwrap();

        assert_eq!(providers.len(), 2);
        assert!(providers.iter().any(|(name, provider)| {
            name == "dummy-provider" && provider.info().name == "dummy-provider"
        }));
        assert!(
            providers
                .iter()
                .any(|(name, provider)| name == "older-provider"
                    && provider.info().name == "older-provider")
        );
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "dummy-tool");
        assert_eq!(tools[0].1.definition().name, "dummy-tool");
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].0, "dummy-loop");
    }

    #[test]
    fn store_accessor_exposes_session_and_message_apis() {
        let runtime = DummyRuntime;
        let session = Session::new(Ulid::nil());
        let session =
            futures::executor::block_on(runtime.store().sessions().create(session.id, session))
                .unwrap();
        let listed =
            futures::executor::block_on(runtime.store().sessions().list_for_project(Ulid::nil()))
                .unwrap();
        assert_eq!(listed.len(), 1);

        let fetched =
            futures::executor::block_on(runtime.store().sessions().get(session.id)).unwrap();
        assert_eq!(fetched.id, session.id);

        let messages =
            futures::executor::block_on(runtime.store().messages().list_for_session(session.id))
                .unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn trait_level_model_helpers_are_callable() {
        let runtime = DummyRuntime;
        let session = Session::new(Ulid::nil());
        let session =
            futures::executor::block_on(runtime.store().sessions().create(session.id, session))
                .unwrap();

        let updated =
            futures::executor::block_on(runtime.set_session_model(session.id, "dummy-model"))
                .unwrap();
        assert_eq!(updated.id, session.id);

        let models = runtime.list_models().unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].provider, "dummy-provider");
        assert_eq!(models[0].model.id, "dummy-model");
        assert_eq!(models[1].provider, "older-provider");
        assert_eq!(models[1].model.id, "older-model");

        let current =
            futures::executor::block_on(runtime.current_model_id_for_session(session.id)).unwrap();
        assert_eq!(current.as_deref(), Some("dummy-model"));

        let current_model =
            futures::executor::block_on(runtime.current_model_for_session(session.id)).unwrap();
        assert_eq!(current_model.provider, "dummy-provider");
        assert_eq!(current_model.model.id, "dummy-model");

        let current_loop =
            futures::executor::block_on(runtime.current_loop_name_for_session(session.id)).unwrap();
        assert_eq!(current_loop.as_deref(), None);

        let updated =
            futures::executor::block_on(runtime.set_session_thought_level(session.id, "high"))
                .unwrap();
        assert_eq!(
            updated
                .inference
                .as_ref()
                .and_then(|config| config.reasoning.as_deref()),
            Some("high")
        );

        let updated =
            futures::executor::block_on(runtime.set_session_loop(session.id, "dummy-loop"))
                .unwrap();
        assert_eq!(updated.loop_name.as_deref(), Some("dummy-loop"));
    }

    #[test]
    fn trait_level_loop_helper_rejects_unknown_loop() {
        let runtime = DummyRuntime;
        let session = Session::new(Ulid::nil());
        let session =
            futures::executor::block_on(runtime.store().sessions().create(session.id, session))
                .unwrap();

        let error = futures::executor::block_on(runtime.set_session_loop(session.id, "missing"))
            .expect_err("unknown loop should fail");
        assert!(error.to_string().contains("loop not registered: missing"));
    }

    #[test]
    fn registry_set_returns_previous_entry() {
        let registry = RegistryToolHashMap::default();

        assert!(
            registry
                .set("tool".into(), Arc::new(DummyTool))
                .unwrap()
                .is_none()
        );
        assert!(
            registry
                .set("tool".into(), Arc::new(DummyTool))
                .unwrap()
                .is_some()
        );
        assert!(registry.get("tool").is_some());
    }
}
