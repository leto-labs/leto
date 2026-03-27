use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_core::{AgentCore, AgentCoreNative, CoreEvent};
use agent_core_remote::{AgentCoreRemote, AgentCoreRemoteConfig};
use agent_runtime::{Message, RuntimeEvent};
use agent_server::{AgentServer, build_router};
use agent_store::{
    CredentialEntry, CredentialError, CredentialHealth, Project, Session, SessionUpdate,
    StoredMessage,
};
use atif::{Agent, SchemaVersion, Step, StepSource, Trajectory};
use chrono::Utc;
use futures::StreamExt;
use provider::MockProvider;

fn make_server() -> Arc<AgentServer> {
    let core: Arc<dyn AgentCore> = Arc::new(futures::executor::block_on(async {
        AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap()
    }));
    Arc::new(AgentServer::new(core))
}

async fn start_server() -> String {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

async fn connect_remote(base: &str) -> AgentCoreRemote {
    AgentCoreRemote::connect(AgentCoreRemoteConfig::new(base.to_owned()))
        .await
        .unwrap()
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

fn sample_trajectory(session_id: agent_store::SessionId) -> Trajectory {
    Trajectory {
        schema_version: SchemaVersion::default(),
        session_id: session_id.to_string(),
        agent: Agent {
            name: "agent-server".into(),
            version: "0.1.0".into(),
            model_name: Some("mock-echo".into()),
            tool_definitions: None,
            extra: None,
        },
        steps: vec![Step {
            step_id: 1,
            timestamp: Some("2026-03-25T12:00:00Z".into()),
            source: StepSource::User,
            model_name: None,
            reasoning_effort: None,
            message: "hello remote".into(),
            reasoning_content: None,
            tool_calls: None,
            observation: None,
            metrics: None,
            is_copied_context: None,
            extra: None,
        }],
        notes: None,
        final_metrics: None,
        continued_trajectory_ref: None,
        extra: None,
    }
}

#[tokio::test]
async fn remote_core_connects_and_exposes_trait_object_metadata() {
    init_tracing();
    let base = start_server().await;
    let core: Arc<dyn AgentCore> = Arc::new(connect_remote(&base).await);

    assert_eq!(core.provider_names(), vec!["mock"]);
    assert_eq!(core.default_provider_name(), "mock");
    assert_eq!(core.default_loop_name(), "simple");
    assert!(core.loop_names().contains(&"simple".to_owned()));
    assert!(
        core.list_models()
            .iter()
            .any(|model| model.provider_name == "mock" && model.model.id == "mock-echo")
    );
}

#[tokio::test]
async fn remote_store_proxy_round_trips_full_surface() {
    init_tracing();
    let base = start_server().await;
    let remote = connect_remote(&base).await;
    let store = remote.store();

    let root = PathBuf::from("/tmp/agent-core-remote-store-surface");
    let project = Project::new(
        Some("remote".into()),
        Some(root.clone()),
        Default::default(),
    );
    let created_project = store.projects().create(project).await.unwrap();
    let found_project = store
        .projects()
        .find_by_root(Path::new(&root))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_project.id, created_project.id);

    let mut updated_project = created_project.clone();
    updated_project.name = Some("remote-updated".into());
    let updated_project = store
        .projects()
        .update(created_project.id, updated_project)
        .await
        .unwrap();
    assert_eq!(updated_project.name.as_deref(), Some("remote-updated"));
    assert_eq!(store.projects().list().await.unwrap().len(), 1);

    let session = Session::new(created_project.id);
    let created_session = store.sessions().create(session).await.unwrap();
    assert_eq!(store.sessions().list().await.unwrap().len(), 1);
    assert_eq!(
        store
            .sessions()
            .list_for_project(created_project.id)
            .await
            .unwrap()
            .len(),
        1
    );

    let patched_session = store
        .sessions()
        .patch(
            created_session.id,
            SessionUpdate {
                title: Some("patched".into()),
                provider: Some(Some("mock".into())),
                model: Some(Some("mock-echo".into())),
                loop_name: None,
                request: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(patched_session.title.as_deref(), Some("patched"));

    let mut updated_session = patched_session.clone();
    updated_session.title = Some("updated".into());
    let updated_session = store
        .sessions()
        .update(created_session.id, updated_session)
        .await
        .unwrap();
    assert_eq!(updated_session.title.as_deref(), Some("updated"));

    let messages = vec![StoredMessage::new(
        created_session.id,
        0,
        Message::user_text("hello remote store"),
    )];
    let replaced = store
        .messages()
        .replace_for_session(created_session.id, messages)
        .await
        .unwrap();
    assert_eq!(replaced.len(), 1);
    assert_eq!(
        store
            .messages()
            .list_for_session(created_session.id)
            .await
            .unwrap()
            .len(),
        1
    );

    let key = ("mock".to_owned(), "primary".to_owned());
    let credential = CredentialEntry::api_key("Primary", "secret");
    let created_credential = store
        .credentials()
        .create(key.clone(), credential)
        .await
        .unwrap();
    assert_eq!(created_credential.label, "Primary");
    assert_eq!(store.credentials().list().await.unwrap().len(), 1);
    assert_eq!(
        store
            .credentials()
            .list_for_provider("mock")
            .await
            .unwrap()
            .len(),
        1
    );

    let mut updated_credential = created_credential.clone();
    updated_credential.label = "Updated".into();
    let updated_credential = store
        .credentials()
        .update(key.clone(), updated_credential)
        .await
        .unwrap();
    assert_eq!(updated_credential.label, "Updated");

    let health = CredentialHealth {
        last_ok: None,
        last_error: Some(CredentialError {
            message: "boom".into(),
            code: None,
            recorded_at: Utc::now(),
        }),
        consecutive_errors: 1,
        updated_at: Utc::now(),
    };
    store
        .credentials()
        .update_health("mock", "primary", &health)
        .await
        .unwrap();
    let hydrated_credential = store.credentials().get(key.clone()).await.unwrap();
    assert_eq!(
        hydrated_credential
            .health
            .last_error
            .as_ref()
            .map(|error| error.message.as_str()),
        Some("boom")
    );

    let trajectory = sample_trajectory(created_session.id);
    let upserted = store
        .trajectories()
        .upsert(created_session.id, trajectory.clone())
        .await
        .unwrap();
    assert_eq!(upserted.session_id, trajectory.session_id);
    assert!(
        store
            .trajectories()
            .get_for_session(created_session.id)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(store.trajectories().list().await.unwrap().len(), 1);

    store
        .trajectories()
        .delete(created_session.id)
        .await
        .unwrap();
    assert!(
        store
            .trajectories()
            .get_for_session(created_session.id)
            .await
            .unwrap()
            .is_none()
    );

    store
        .messages()
        .delete_for_session(created_session.id)
        .await
        .unwrap();
    assert!(
        store
            .messages()
            .list_for_session(created_session.id)
            .await
            .unwrap()
            .is_empty()
    );

    store.credentials().delete(key).await.unwrap();
    assert!(store.credentials().list().await.unwrap().is_empty());
}

#[tokio::test]
async fn remote_core_streams_live_turns_against_agent_server() {
    init_tracing();
    let base = start_server().await;
    let core: Arc<dyn AgentCore> = Arc::new(connect_remote(&base).await);

    let project = core
        .resolve_or_create_project(PathBuf::from("/tmp/agent-core-remote-live-events"))
        .await
        .unwrap();
    let session = core.create_session(project.id).await.unwrap();
    let mut turn_events = core
        .turn(session.id, vec![Message::user_text("hello remote turn")])
        .await
        .unwrap();

    let mut saw_finished = false;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), turn_events.next())
        .await
        .unwrap()
    {
        if matches!(
            event,
            CoreEvent::Turn {
                event: RuntimeEvent::TurnFinished { .. },
                ..
            }
        ) {
            saw_finished = true;
            break;
        }
    }

    assert!(saw_finished);
}
