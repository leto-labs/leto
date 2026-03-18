use std::{path::PathBuf, time::Duration};

use agent_client_protocol::{self as acp, Agent as _};
use serde_json::{json, value::to_raw_value};

use super::agent::SEEDED_SESSION_ID;
use super::test_support::{
    RecordingClient, completed_tool_output, initialized_connection, start_test_connection,
    streamed_agent_text, streamed_agent_thoughts,
};

#[tokio::test(flavor = "current_thread")]
async fn initialize_advertises_broad_mock_capabilities() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let client = RecordingClient::default();
            let connection = start_test_connection(client);

            let response = connection
                .initialize(
                    acp::InitializeRequest::new(acp::ProtocolVersion::V1).client_info(
                        acp::Implementation::new("test-client", "0.1.0").title("Test Client"),
                    ),
                )
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
            #[cfg(feature = "unstable_session_fork")]
            assert!(
                response
                    .agent_capabilities
                    .session_capabilities
                    .fork
                    .is_some()
            );
            #[cfg(feature = "unstable_session_resume")]
            assert!(
                response
                    .agent_capabilities
                    .session_capabilities
                    .resume
                    .is_some()
            );
            #[cfg(feature = "unstable_session_close")]
            assert!(
                response
                    .agent_capabilities
                    .session_capabilities
                    .close
                    .is_some()
            );
            assert_eq!(response.auth_methods.len(), 1);
            assert_eq!(
                response.agent_info.as_ref().map(|info| info.name.as_str()),
                Some("brain-acp-mock")
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn new_session_returns_modes_and_config_options() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/mock-project",
                )))
                .await
                .expect("new session should succeed");

            assert_eq!(
                session.modes.as_ref().map(|m| m.current_mode_id.0.as_ref()),
                Some("ask")
            );
            assert_eq!(session.config_options.as_ref().map(Vec::len), Some(2));
            #[cfg(feature = "unstable_session_model")]
            assert_eq!(
                session
                    .models
                    .as_ref()
                    .map(|m| m.current_model_id.0.as_ref()),
                Some("brain-mock-fast")
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn new_session_primes_available_commands_for_first_prompt() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/new-session-prime",
                )))
                .await
                .expect("new session should succeed");

            let notifications = client.take_notifications();
            let available_commands = notifications
                .iter()
                .find_map(|notification| match &notification.update {
                    acp::SessionUpdate::AvailableCommandsUpdate(update) => {
                        Some(update.available_commands.clone())
                    }
                    _ => None,
                })
                .expect("new session should emit available commands");

            assert!(
                available_commands
                    .iter()
                    .any(|command| command.name == "terminal")
            );
            assert!(
                available_commands
                    .iter()
                    .any(|command| command.name == "read-file")
            );
            assert!(
                available_commands
                    .iter()
                    .any(|command| command.name == "think")
            );
            assert!(
                available_commands
                    .iter()
                    .any(|command| command.name == "edit-file")
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn prompt_streams_mock_response_and_commands() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(
                    std::env::current_dir().expect("cwd"),
                ))
                .await
                .expect("new session should succeed");

            let response = connection
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec!["hello from test".into()],
                ))
                .await
                .expect("prompt should succeed");

            assert_eq!(response.stop_reason, acp::StopReason::EndTurn);

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);

            assert!(streamed.contains("Mock brain-acp response"));
            assert!(streamed.contains("hello from test"));
            assert!(notifications.iter().any(|notification| matches!(
                notification.update,
                acp::SessionUpdate::AvailableCommandsUpdate(_)
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_create_plan_emits_plan_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let client = RecordingClient::default();
            let (client, connection) = initialized_connection(client).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/create-plan",
                )))
                .await
                .expect("new session should succeed");

            let response = connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:create-plan".into()],
                ))
                .await
                .expect("prompt should succeed");

            assert_eq!(response.stop_reason, acp::StopReason::EndTurn);
            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            assert!(streamed.contains("Mock plan emitted."));
            assert!(
                notifications
                    .iter()
                    .filter(|notification| matches!(notification.update, acp::SessionUpdate::Plan(_)))
                    .count()
                    >= 2
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_summarize_session_reports_current_state() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/summarize-session",
                )))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:summarize-session".into()],
                ))
                .await
                .expect("prompt should succeed");

            let streamed = streamed_agent_text(&client.take_notifications());
            assert!(streamed.contains("Mock session summary:"));
            assert!(streamed.contains("cwd=\"/tmp/summarize-session\""));
            assert!(streamed.contains("mode=ask"));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_think_emits_reasoning_chunks_and_think_tool() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/think")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:think release checklist".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let thoughts = streamed_agent_thoughts(&notifications);
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(thoughts.contains("synthetic plan for: release checklist"));
            assert!(thoughts.contains("deterministic mock answer"));
            assert!(tool_output.contains("mock reasoning summary for release checklist"));
            assert!(streamed.contains("Mock reasoning probe completed for: release checklist."));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_search_emits_search_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/search")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:search TODO".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(tool_output.contains("mock search results for `TODO`"));
            assert!(streamed.contains("Mock search probe completed for `TODO`."));
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::ToolCall(tool_call)
                    if tool_call.kind == acp::ToolKind::Search
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_fetch_emits_fetch_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/fetch")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:fetch https://example.invalid/data".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(tool_output.contains("mock fetched content for https://example.invalid/data"));
            assert!(
                streamed.contains(
                    "Mock fetch probe completed for `https://example.invalid/data`."
                )
            );
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::ToolCall(tool_call)
                    if tool_call.kind == acp::ToolKind::Fetch
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_edit_file_emits_patch_style_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/edit-file")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:edit-file /tmp/mock-edit.rs".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(tool_output.contains("mock edit prepared for /tmp/mock-edit.rs"));
            assert!(streamed.contains("Mock edit probe completed for /tmp/mock-edit.rs."));
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::ToolCall(tool_call)
                    if tool_call.kind == acp::ToolKind::Edit
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_delete_file_emits_patch_style_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/delete-file")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:delete-file /tmp/mock-delete.rs".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(tool_output.contains("mock delete prepared for /tmp/mock-delete.rs"));
            assert!(streamed.contains("Mock delete probe completed for /tmp/mock-delete.rs."));
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::ToolCall(tool_call)
                    if tool_call.kind == acp::ToolKind::Delete
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_move_file_emits_move_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/move-file")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:move-file /tmp/old.rs /tmp/new.rs".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(tool_output.contains("mock move prepared from /tmp/old.rs to /tmp/new.rs"));
            assert!(streamed.contains("Mock move probe completed from /tmp/old.rs to /tmp/new.rs."));
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::ToolCall(tool_call)
                    if tool_call.kind == acp::ToolKind::Move
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_read_file_handles_unicode_content_without_hanging() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let client = RecordingClient::default()
                .with_file("/tmp/mock-unicode.txt", "hello with unicode: café → résumé");
            let (client, connection) = initialized_connection(client).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/unicode-read",
                )))
                .await
                .expect("new session should succeed");

            let response = connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:read-file /tmp/mock-unicode.txt".into()],
                ))
                .await
                .expect("prompt should succeed");

            assert_eq!(response.stop_reason, acp::StopReason::EndTurn);
            assert_eq!(client.reads(), vec![PathBuf::from("/tmp/mock-unicode.txt")]);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_read_file_without_path_uses_session_default() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let default_path = PathBuf::from("/tmp/read-default/Cargo.toml");
            let client = RecordingClient::default().with_file(&default_path, "mock cargo manifest");
            let (client, connection) = initialized_connection(client).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/read-default")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:read-file".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            assert!(streamed.contains("Mock file read probe completed for /tmp/read-default/Cargo.toml."));
            assert_eq!(client.reads(), vec![default_path]);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_read_file_uses_client_file_api_and_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let client = RecordingClient::default()
                .with_file("/tmp/mock-read.txt", "hello from the mock client file api");
            let (client, connection) = initialized_connection(client).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/read-file")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:read-file /tmp/mock-read.txt".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(streamed.contains("Mock file read probe completed for /tmp/mock-read.txt."));
            assert!(tool_output.contains("mock content for /tmp/mock-read.txt"));
            assert_eq!(client.reads(), vec![PathBuf::from("/tmp/mock-read.txt")]);
            assert!(notifications.iter().any(|notification| matches!(
                notification.update,
                acp::SessionUpdate::ToolCall(_)
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_write_file_is_stateless_and_streams_tool_updates() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/write-file-marker",
                )))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:write-file /tmp/mock-write-marker.txt hello-marker".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(
                streamed.contains("Mock file write probe completed for /tmp/mock-write-marker.txt.")
            );
            assert!(tool_output.contains("mock write completed for /tmp/mock-write-marker.txt"));
            assert!(client.writes().is_empty());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_write_file_without_args_uses_defaults() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/write-default")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:write-file".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(streamed.contains("Mock file write probe completed for /tmp/write-default/mock-output.txt."));
            assert!(tool_output.contains("mock write completed for /tmp/write-default/mock-output.txt"));
            assert!(client.writes().is_empty());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_request_permission_uses_client_permission_api() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let client = RecordingClient::default().with_permission_outcome(
                acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                    "allow-once",
                )),
            );
            let (client, connection) = initialized_connection(client).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/permission",
                )))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:request-permission".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(streamed.contains("Mock permission probe completed: allow-once."));
            assert!(tool_output.contains("mock permission outcome: allow-once"));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_terminal_uses_client_terminal_api() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/terminal")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:terminal echo hi".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(streamed.contains("Mock terminal probe completed for `echo hi` with exit=0."));
            assert!(tool_output.contains("mock terminal output: echo hi"));
            assert!(tool_output.contains("exit=0"));
            assert_eq!(
                client.created_terminals(),
                vec![("echo".to_owned(), vec!["hi".to_owned()])]
            );
            assert_eq!(client.released_terminals().len(), 1);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_terminal_without_args_uses_defaults() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/terminal-default")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:terminal".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(streamed.contains("Mock terminal probe completed for `echo hello-from-mock-terminal` with exit=0."));
            assert!(tool_output.contains("mock terminal output: echo hello-from-mock-terminal"));
            assert_eq!(
                client.created_terminals(),
                vec![("echo".to_owned(), vec!["hello-from-mock-terminal".to_owned()])]
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_edit_file_without_path_uses_defaults() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/edit-default")))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:edit-file".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(streamed.contains("Mock edit probe completed for /tmp/edit-default/mock-edit.rs."));
            assert!(tool_output.contains("mock edit prepared for /tmp/edit-default/mock-edit.rs"));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn mock_terminal_kill_uses_client_terminal_api() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/terminal-kill",
                )))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec!["mock:terminal-kill sleep 5".into()],
                ))
                .await
                .expect("prompt should succeed");

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            let tool_output = completed_tool_output(&notifications);
            assert!(
                streamed.contains("Mock terminal probe completed for `sleep 5` with exit=SIGKILL.")
            );
            assert!(tool_output.contains("mock terminal output: sleep 5"));
            assert!(tool_output.contains("exit=SIGKILL"));
            assert_eq!(client.released_terminals().len(), 1);
            assert_eq!(client.killed_terminals().len(), 1);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn load_session_replays_seed_history_and_state() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let response = connection
                .load_session(acp::LoadSessionRequest::new(
                    SEEDED_SESSION_ID,
                    std::env::current_dir().expect("cwd"),
                ))
                .await
                .expect("load session should succeed");

            assert_eq!(
                response.modes.as_ref().map(|m| m.current_mode_id.0.as_ref()),
                Some("ask")
            );
            assert_eq!(response.config_options.as_ref().map(Vec::len), Some(2));

            let notifications = client.take_notifications();
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::SessionInfoUpdate(info)
                    if info.title.as_opt_ref().flatten().map(|value| value.as_str()) == Some("Seeded Mock Session")
            )));
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
                    content: acp::ContentBlock::Text(text),
                    ..
                }) if text.text == "What can you do?"
            )));
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                    content: acp::ContentBlock::Text(text),
                    ..
                }) if text.text.contains("mock brain-acp agent")
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn load_session_restores_unknown_mock_session_for_external_clients() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;

            let restored_session_id = acp::SessionId::new("mock-session-42");
            let response = connection
                .load_session(acp::LoadSessionRequest::new(
                    restored_session_id.clone(),
                    PathBuf::from("/tmp/restored"),
                ))
                .await
                .expect("load_session should restore unknown mock session ids");

            assert_eq!(
                response.modes.as_ref().map(|m| m.current_mode_id.0.as_ref()),
                Some("ask")
            );

            let notifications = client.take_notifications();
            let streamed = streamed_agent_text(&notifications);
            assert!(streamed.contains("reconstructed"));
            assert!(notifications.iter().any(|notification| matches!(
                notification.update,
                acp::SessionUpdate::SessionInfoUpdate(_)
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn list_sessions_returns_mock_history_pages() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;

            for suffix in ["one", "two", "three"] {
                connection
                    .new_session(acp::NewSessionRequest::new(PathBuf::from(format!(
                        "/tmp/{suffix}"
                    ))))
                    .await
                    .expect("new session should succeed");
            }

            let first_page = connection
                .list_sessions(acp::ListSessionsRequest::new())
                .await
                .expect("list_sessions should succeed");

            assert_eq!(first_page.sessions.len(), 2);
            assert!(first_page.next_cursor.is_some());

            let second_page = connection
                .list_sessions(
                    acp::ListSessionsRequest::new()
                        .cursor(first_page.next_cursor.expect("next cursor")),
                )
                .await
                .expect("second page should succeed");

            assert!(!second_page.sessions.is_empty());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn set_session_mode_emits_current_mode_update() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;
            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/mode")))
                .await
                .expect("new session should succeed");

            connection
                .set_session_mode(acp::SetSessionModeRequest::new(
                    session.session_id.clone(),
                    "code",
                ))
                .await
                .expect("set_session_mode should succeed");

            let notifications = client.take_notifications();
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::CurrentModeUpdate(update)
                    if update.current_mode_id.0.as_ref() == "code"
            )));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn set_session_config_option_returns_updated_values() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;
            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/config")))
                .await
                .expect("new session should succeed");

            let response = connection
                .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                    session.session_id,
                    "reasoning_level",
                    "deep",
                ))
                .await
                .expect("set_session_config_option should succeed");

            let updated = response
                .config_options
                .into_iter()
                .find(|option| option.id.0.as_ref() == "reasoning_level")
                .expect("reasoning option should be present");

            match updated.kind {
                acp::SessionConfigKind::Select(select) => {
                    assert_eq!(select.current_value.0.as_ref(), "deep");
                }
                _ => panic!("expected select config option"),
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn ext_method_returns_mock_response() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;

            let params = to_raw_value(&json!({ "hello": "world" }))
                .expect("raw value conversion should succeed");
            let response = connection
                .ext_method(acp::ExtRequest::new("brain/ping", params.into()))
                .await
                .expect("ext method should succeed");

            let payload: serde_json::Value =
                serde_json::from_str(response.0.get()).expect("valid ext response JSON");
            assert_eq!(payload["ok"], serde_json::Value::Bool(true));
            assert_eq!(
                payload["echo"]["hello"],
                serde_json::Value::String("world".into())
            );
        })
        .await;
}

#[cfg(feature = "unstable_session_model")]
#[tokio::test(flavor = "current_thread")]
async fn set_session_model_updates_mock_state() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;
            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/model")))
                .await
                .expect("new session should succeed");

            connection
                .set_session_model(acp::SetSessionModelRequest::new(
                    session.session_id.clone(),
                    "brain-mock-deep",
                ))
                .await
                .expect("set_session_model should succeed");

            let resumed = connection
                .load_session(acp::LoadSessionRequest::new(
                    session.session_id,
                    PathBuf::from("/tmp/model"),
                ))
                .await
                .expect("load should succeed");

            assert_eq!(
                resumed
                    .models
                    .as_ref()
                    .map(|models| models.current_model_id.0.as_ref()),
                Some("brain-mock-deep")
            );
        })
        .await;
}

#[cfg(feature = "unstable_session_fork")]
#[tokio::test(flavor = "current_thread")]
async fn fork_session_clones_session_state() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;
            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from(
                    "/tmp/fork-source",
                )))
                .await
                .expect("new session should succeed");

            connection
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec!["fork me".into()],
                ))
                .await
                .expect("prompt should succeed");

            let fork = connection
                .fork_session(acp::ForkSessionRequest::new(
                    session.session_id,
                    PathBuf::from("/tmp/fork-target"),
                ))
                .await
                .expect("fork should succeed");

            assert!(fork.session_id.0.as_ref().starts_with("mock-session-"));
            assert_eq!(fork.config_options.as_ref().map(Vec::len), Some(2));
        })
        .await;
}

#[cfg(feature = "unstable_session_resume")]
#[tokio::test(flavor = "current_thread")]
async fn resume_session_returns_state_without_replay() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (client, connection) = initialized_connection(RecordingClient::default()).await;
            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/resume")))
                .await
                .expect("new session should succeed");
            let _ = client.take_notifications();

            let response = connection
                .resume_session(acp::ResumeSessionRequest::new(
                    session.session_id,
                    PathBuf::from("/tmp/resume"),
                ))
                .await
                .expect("resume should succeed");

            assert_eq!(response.config_options.as_ref().map(Vec::len), Some(2));
            assert!(client.take_notifications().is_empty());
        })
        .await;
}

#[cfg(feature = "unstable_session_close")]
#[tokio::test(flavor = "current_thread")]
async fn close_session_hides_session_from_listing() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;
            let session = connection
                .new_session(acp::NewSessionRequest::new(PathBuf::from("/tmp/close")))
                .await
                .expect("new session should succeed");

            connection
                .close_session(acp::CloseSessionRequest::new(session.session_id.clone()))
                .await
                .expect("close session should succeed");

            let listed = connection
                .list_sessions(acp::ListSessionsRequest::new())
                .await
                .expect("list_sessions should succeed");

            assert!(
                listed
                    .sessions
                    .iter()
                    .all(|info| info.session_id != session.session_id)
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancel_stops_active_prompt() {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (_, connection) = initialized_connection(RecordingClient::default()).await;

            let session = connection
                .new_session(acp::NewSessionRequest::new(
                    std::env::current_dir().expect("cwd"),
                ))
                .await
                .expect("new session should succeed");

            let prompt_connection = connection.clone();
            let session_id = session.session_id.clone();
            let prompt_task = tokio::task::spawn_local(async move {
                prompt_connection
                    .prompt(acp::PromptRequest::new(
                        session_id,
                        vec!["please stream long enough to cancel".into()],
                    ))
                    .await
            });

            tokio::time::sleep(Duration::from_millis(10)).await;
            connection
                .cancel(acp::CancelNotification::new(session.session_id))
                .await
                .expect("cancel should succeed");

            let response = prompt_task
                .await
                .expect("prompt task should complete")
                .expect("prompt result should be available");

            assert_eq!(response.stop_reason, acp::StopReason::Cancelled);
        })
        .await;
}
