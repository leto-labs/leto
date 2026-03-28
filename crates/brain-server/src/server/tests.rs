use std::sync::Arc;

use super::*;
use brain_loops::SimpleLoop;
use brain_providers::MockProvider;
use brain_stores::InMemoryStore;

fn make_brain() -> Brain {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
    let store = Arc::new(InMemoryStore::new());
    let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
    Brain::new(provider, store, agent_loop, vec![])
}

fn make_server() -> BrainServer {
    BrainServer::new(make_brain())
}

async fn make_project(server: &BrainServer) -> Project {
    server
        .create_project(Project::with_defaults("test"))
        .await
        .unwrap()
}

#[tokio::test]
async fn project_crud() {
    let server = make_server();

    let p = server
        .create_project(Project::with_defaults("myproject"))
        .await
        .unwrap();
    assert_eq!(p.name.as_deref(), Some("myproject"));

    let got = server.get_project(p.id).await.unwrap();
    assert_eq!(got.id, p.id);

    let list = server.list_projects().await.unwrap();
    assert_eq!(list.len(), 1);

    server.delete_project(p.id).await.unwrap();
    let list = server.list_projects().await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn create_and_list_sessions() {
    let server = make_server();
    let project = make_project(&server).await;

    let s1 = server.create_session(project.id).await.unwrap();
    let s2 = server.create_session(project.id).await.unwrap();
    assert_ne!(s1.id, s2.id);

    let list = server.list_sessions(project.id).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn sessions_scoped_to_project() {
    let server = make_server();
    let p1 = make_project(&server).await;
    let p2 = server
        .create_project(Project::with_defaults("other"))
        .await
        .unwrap();

    server.create_session(p1.id).await.unwrap();
    server.create_session(p1.id).await.unwrap();
    server.create_session(p2.id).await.unwrap();

    assert_eq!(server.list_sessions(p1.id).await.unwrap().len(), 2);
    assert_eq!(server.list_sessions(p2.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn get_session() {
    let server = make_server();
    let project = make_project(&server).await;
    let s = server.create_session(project.id).await.unwrap();

    let got = server.get_session(s.id).await.unwrap();
    assert_eq!(got.id, s.id);
    assert_eq!(got.project_id, project.id);
}

#[tokio::test]
async fn delete_session() {
    let server = make_server();
    let project = make_project(&server).await;
    let s = server.create_session(project.id).await.unwrap();

    server.delete_session(s.id).await.unwrap();
    let list = server.list_sessions(project.id).await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn send_message_produces_events() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    let mut rx = server.subscribe();
    server.send_message(session.id, "hello").await.unwrap();

    let mut saw_token = false;

    loop {
        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timed out waiting for event")
            .expect("channel closed");

        assert_eq!(ev.session_id, session.id);
        match &ev.event {
            Event::Token { .. } => saw_token = true,
            Event::TurnDone { .. } => break,
            _ => {}
        }
    }

    assert!(saw_token, "should have received token events");
}

#[tokio::test]
async fn reject_concurrent_turn_same_session() {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::new().with_delay(200));
    let store = Arc::new(InMemoryStore::new());
    let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
    let brain = Brain::new(provider, store, agent_loop, vec![]);
    let server = BrainServer::new(brain);

    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    server.send_message(session.id, "first").await.unwrap();

    let result = server.send_message(session.id, "second").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), BrainErrorCode::TurnActive);
}

#[tokio::test]
async fn concurrent_turns_different_sessions() {
    let server = make_server();
    let project = make_project(&server).await;
    let s1 = server.create_session(project.id).await.unwrap();
    let s2 = server.create_session(project.id).await.unwrap();

    server.send_message(s1.id, "hello").await.unwrap();
    server.send_message(s2.id, "world").await.unwrap();

    let mut rx = server.subscribe();
    let mut s1_done = false;
    let mut s2_done = false;

    for _ in 0..50 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
            Ok(Ok(ev)) => {
                if matches!(&ev.event, Event::TurnDone { .. }) {
                    if ev.session_id == s1.id {
                        s1_done = true;
                    }
                    if ev.session_id == s2.id {
                        s2_done = true;
                    }
                }
                if s1_done && s2_done {
                    break;
                }
            }
            _ => break,
        }
    }

    assert!(s1_done, "session 1 should complete");
    assert!(s2_done, "session 2 should complete");
}

#[tokio::test]
async fn cancel_active_turn() {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::new().with_delay(500));
    let store = Arc::new(InMemoryStore::new());
    let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
    let brain = Brain::new(provider, store, agent_loop, vec![]);
    let server = BrainServer::new(brain);

    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();
    let mut rx = server.subscribe();

    server.send_message(session.id, "hello").await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    server.cancel_turn(session.id).await.unwrap();

    let mut saw_cancelled = false;
    for _ in 0..20 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
            Ok(Ok(ev)) => {
                if let Event::Error {
                    code: BrainErrorCode::Cancelled,
                    ..
                } = &ev.event
                {
                    saw_cancelled = true;
                    break;
                }
            }
            _ => break,
        }
    }

    assert!(saw_cancelled, "should receive cancelled event");
}

#[tokio::test]
async fn cancel_no_active_turn_is_ok() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();
    server.cancel_turn(session.id).await.unwrap();
}

#[tokio::test]
async fn status_reports_tools_and_sessions() {
    let server = make_server();
    let project = make_project(&server).await;
    server.create_session(project.id).await.unwrap();
    server.create_session(project.id).await.unwrap();

    let st = server.status().await.unwrap();
    assert_eq!(st.active_sessions, 2);
    assert!(st.active_turns.is_empty());
    assert!(st.tools.is_empty());
}

#[tokio::test]
async fn client_returns_working_api() {
    let server = make_server();
    let client = server.client();

    let project = client
        .create_project(Project::with_defaults("test"))
        .await
        .unwrap();
    let session = client.create_session(project.id).await.unwrap();
    let list = client.list_sessions(project.id).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, session.id);
}

#[tokio::test]
async fn active_turn_cleared_after_completion() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    server.send_message(session.id, "hello").await.unwrap();

    let mut rx = server.subscribe();
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
            Ok(Ok(ev)) if matches!(&ev.event, Event::TurnDone { .. }) => break,
            Ok(Ok(_)) => continue,
            _ => panic!("timed out waiting for TurnDone"),
        }
    }

    server.send_message(session.id, "again").await.unwrap();
}

#[tokio::test]
async fn update_session_title() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();
    assert!(session.title.is_none());

    server
        .update_session(session.id, SessionUpdate::title("renamed"))
        .await
        .unwrap();

    let got = server.get_session(session.id).await.unwrap();
    assert_eq!(got.title.as_deref(), Some("renamed"));
}

#[tokio::test]
async fn list_messages_empty_session() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    let msgs = server.list_messages(session.id).await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn list_messages_after_turn() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    server.send_message(session.id, "hello").await.unwrap();

    let mut rx = server.subscribe();
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
            Ok(Ok(ev)) if matches!(&ev.event, Event::TurnDone { .. }) => break,
            Ok(Ok(_)) => continue,
            _ => panic!("timed out waiting for TurnDone"),
        }
    }

    let msgs = server.list_messages(session.id).await.unwrap();
    assert!(!msgs.is_empty(), "should have messages after a turn");
    assert_eq!(msgs[0].role, brain_types::Role::User);
    assert_eq!(msgs[0].content, brain_types::MessageContent::text("hello"));
}

#[tokio::test]
async fn send_message_stream_returns_events() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    let mut stream = server.send_message_stream(session.id, "hello").await.unwrap();

    let mut saw_token = false;
    let mut saw_done = false;

    while let Some(event) = futures::StreamExt::next(&mut stream).await {
        match &event {
            Event::Token { .. } => saw_token = true,
            Event::TurnDone { .. } => {
                saw_done = true;
                break;
            }
            _ => {}
        }
    }

    assert!(saw_token, "should have received token events");
    assert!(saw_done, "should have received TurnDone");
}

#[tokio::test]
async fn send_message_stream_also_publishes_to_bus() {
    let server = make_server();
    let project = make_project(&server).await;
    let session = server.create_session(project.id).await.unwrap();

    let mut bus_rx = server.subscribe();
    let mut stream = server.send_message_stream(session.id, "hello").await.unwrap();

    while futures::StreamExt::next(&mut stream).await.is_some() {}

    let mut bus_saw_done = false;
    for _ in 0..50 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), bus_rx.recv()).await {
            Ok(Ok(ev)) if matches!(&ev.event, Event::TurnDone { .. }) => {
                bus_saw_done = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }

    assert!(bus_saw_done, "bus should have received TurnDone");
}

#[tokio::test]
async fn list_providers_returns_mock() {
    let server = make_server();
    let providers = server.list_providers().await.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "mock");
    assert!(!providers[0].models.is_empty());
}

#[tokio::test]
async fn credential_crud() {
    let server = make_server();

    let list = server.list_credentials().await.unwrap();
    assert!(list.is_empty());

    let entry = brain_types::CredentialEntry::api_key("key-1", "sk-test");
    server.save_credential("openai", entry).await.unwrap();

    let loaded = server.get_credentials("openai").await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "key-1");
    match &loaded[0].credential {
        brain_types::ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-test"),
        _ => panic!("expected ApiKey"),
    }

    let list = server.list_credentials().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, "openai");

    server.delete_credential("openai", "key-1").await.unwrap();
    let loaded = server.get_credentials("openai").await.unwrap();
    assert!(loaded.is_empty());
}

#[tokio::test]
async fn status_includes_provider_info() {
    let server = make_server();
    let st = server.status().await.unwrap();
    assert_eq!(st.providers.len(), 1);
    assert_eq!(st.providers[0].name, "mock");
}
