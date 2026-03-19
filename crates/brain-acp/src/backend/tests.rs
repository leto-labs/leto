use std::{path::PathBuf, time::Duration};

use agent_client_protocol::{self as acp, Agent as _};

use super::test_support::{
    RecordingClient, build_test_app, initialized_connection, streamed_agent_text,
};

#[tokio::test(flavor = "current_thread")]
async fn initialize_advertises_real_backend_capabilities() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let app = build_test_app(0);
            let client = RecordingClient::default();
            let connection = super::test_support::start_test_connection(client, app);

            let response = connection
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::V1))
                .await
                .expect("initialize should succeed");

            assert_eq!(response.protocol_version, acp::ProtocolVersion::V1);
            assert!(response.agent_capabilities.load_session);
            assert!(
                response
                    .agent_capabilities
                    .session_capabilities
                    .list
                    .is_some()
            );
            assert_eq!(
                response.agent_info.as_ref().map(|info| info.name.as_str()),
                Some("brain-acp")
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn new_session_list_and_load_use_real_store() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let app = build_test_app(0);
            let (client, connection) =
                initialized_connection(RecordingClient::default(), app).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/acp-real")))
                .await
                .expect("new session should succeed");
            #[cfg(feature = "unstable_session_model")]
            assert_eq!(
                session
                    .models
                    .as_ref()
                    .map(|models| models.current_model_id.0.as_ref()),
                Some("mock-echo")
            );

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec!["hello from real backend".into()],
                ))
                .await
                .expect("prompt should succeed");
            let _ = client.take_notifications();

            let listed = connection
                .list_sessions(acp::ListSessionsRequest::new().cwd(PathBuf::from("/tmp/acp-real")))
                .await
                .expect("list should succeed");
            assert_eq!(listed.sessions.len(), 1);
            assert_eq!(listed.sessions[0].session_id, session.session_id);

            connection
                .load_session(acp::LoadSessionRequest::new(
                    session.session_id,
                    PathBuf::from("/tmp/acp-real"),
                ))
                .await
                .expect("load should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            assert!(streamed.contains("hello from real backend"));
        })
        .await;
}

#[cfg(feature = "unstable_session_model")]
#[tokio::test(flavor = "current_thread")]
async fn set_session_model_persists_real_session_inference() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let app = build_test_app(0);
            let (_, connection) =
                initialized_connection(RecordingClient::default(), app.clone()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/acp-model")))
                .await
                .expect("new session should succeed");

            connection
                .set_session_model(acp::SetSessionModelRequest::new(
                    session.session_id.clone(),
                    acp::ModelId::new("mock-echo"),
                ))
                .await
                .expect("set_session_model should succeed");

            let session_id = session
                .session_id
                .0
                .parse()
                .expect("session id should parse");
            let stored = app
                .brain
                .store
                .session_get(session_id)
                .await
                .expect("session should exist");

            assert_eq!(
                stored
                    .inference
                    .as_ref()
                    .and_then(|cfg| cfg.provider.as_deref()),
                Some("mock")
            );
            assert_eq!(
                stored
                    .inference
                    .as_ref()
                    .and_then(|cfg| cfg.model.as_deref()),
                Some("mock-echo")
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn prompt_streams_real_brain_turn_output() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let app = build_test_app(0);
            let (client, connection) =
                initialized_connection(RecordingClient::default(), app).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/acp-prompt",
                )))
                .await
                .expect("new session should succeed");

            let response = connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["real backend prompt path".into()],
                ))
                .await
                .expect("prompt should succeed");

            assert_eq!(response.stop_reason, acp::StopReason::EndTurn);
            let streamed = streamed_agent_text(&client.take_notifications());
            assert!(streamed.contains("real backend prompt path"));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancel_returns_cancelled_stop_reason() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let app = build_test_app(30);
            let (_, connection) = initialized_connection(RecordingClient::default(), app).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/acp-cancel",
                )))
                .await
                .expect("new session should succeed");

            let session_id = session.session_id.clone();
            let prompt_connection = connection.clone();
            let prompt = tokio::task::spawn_local(async move {
                prompt_connection
                    .prompt(acp::PromptRequest::new(
                        session_id.clone(),
                        vec!["slow streaming prompt for cancellation".into()],
                    ))
                    .await
                    .expect("prompt should return")
            });

            tokio::time::sleep(Duration::from_millis(20)).await;
            connection
                .cancel(acp::CancelNotification::new(session.session_id))
                .await
                .expect("cancel should succeed");

            let response = prompt.await.expect("task should join");
            assert_eq!(response.stop_reason, acp::StopReason::Cancelled);
        })
        .await;
}
