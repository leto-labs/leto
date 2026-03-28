use std::sync::Arc;
use std::time::Duration;

use agent_core::{AgentCore, AgentCoreNative, CoreEvent};
use futures::StreamExt;
use agent_runtime::RuntimeEvent;
use agent_server::{AgentServer, build_router};
use agent_store::{Project, Session, StoredMessage};
use provider::{FinishReason, MessageRole, MockProvider};

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

async fn create_project(client: &reqwest::Client, base: &str) -> Project {
    client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({"name": "agent-server-turn-test"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn create_session(client: &reqwest::Client, base: &str, project: &Project) -> Session {
    client
        .post(format!("{base}/v1/projects/{}/sessions", project.id))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn post_turn(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
    text: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/v1/sessions/{}/turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": text}]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
}

async fn post_stream_turn(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
    text: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/v1/sessions/{}/stream-turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": text}]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
}

async fn get_session_events(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
) -> reqwest::Response {
    client
        .get(format!("{base}/v1/sessions/{}/events", session.id))
        .send()
        .await
        .unwrap()
}

fn parse_ndjson_events(body: &str) -> Vec<CoreEvent> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn parse_sse_events(body: &str) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    let mut data_lines = Vec::new();

    for line in body.lines() {
        if line.is_empty() {
            if !data_lines.is_empty() {
                events.push(serde_json::from_str(&data_lines.join("\n")).unwrap());
                data_lines.clear();
            }
            continue;
        }

        if line.starts_with(':') {
            continue;
        }

        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start().to_owned());
        }
    }

    if !data_lines.is_empty() {
        events.push(serde_json::from_str(&data_lines.join("\n")).unwrap());
    }

    events
}

async fn read_sse_events_until<F>(response: reqwest::Response, predicate: F) -> Vec<CoreEvent>
where
    F: Fn(&[CoreEvent]) -> bool,
{
    let mut stream = response.bytes_stream();
    let mut pending = String::new();
    let mut events = Vec::new();

    loop {
        let chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        pending.push_str(std::str::from_utf8(&chunk).unwrap());

        while let Some(boundary) = pending.find("\n\n") {
            let frame = pending[..boundary].to_owned();
            pending.drain(..boundary + 2);

            let mut data_lines = Vec::new();
            for line in frame.lines() {
                if line.starts_with(':') {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data:") {
                    data_lines.push(data.trim_start().to_owned());
                }
            }

            if !data_lines.is_empty() {
                events.push(serde_json::from_str(&data_lines.join("\n")).unwrap());
            }
        }

        if predicate(&events) {
            return events;
        }
    }
}

#[tokio::test]
async fn post_turns_completes_turn_and_persists_assistant_message() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_turn(&client, &base, &session, "hello canonical server").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap(),
        "application/x-ndjson"
    );

    let events = parse_ndjson_events(&response.text().await.unwrap());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Stop),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(
        messages[0].message.plain_text_lossy(),
        "hello canonical server"
    );
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert!(
        messages[1]
            .message
            .plain_text_lossy()
            .contains("hello canonical server")
    );
}

#[tokio::test]
async fn post_turns_allows_follow_up_turn_after_completion() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let first = post_turn(&client, &base, &session, "first turn").await;
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let first_events = parse_ndjson_events(&first.text().await.unwrap());
    assert!(matches!(
        first_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let second = post_turn(&client, &base, &session, "second turn").await;
    assert_eq!(second.status(), reqwest::StatusCode::OK);
    let second_events = parse_ndjson_events(&second.text().await.unwrap());
    assert!(matches!(
        second_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[0].message.plain_text_lossy(), "first turn");
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert_eq!(messages[2].message.role, MessageRole::User);
    assert_eq!(messages[2].message.plain_text_lossy(), "second turn");
    assert_eq!(messages[3].message.role, MessageRole::Assistant);
    assert!(
        messages[3]
            .message
            .plain_text_lossy()
            .contains("second turn")
    );
}

#[tokio::test]
async fn post_stream_turns_streams_sse_until_turn_finished() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_stream_turn(&client, &base, &session, "hello sse turn").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let events = parse_sse_events(&response.text().await.unwrap());
    assert!(!events.is_empty());
    assert!(matches!(
        events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[0].message.plain_text_lossy(), "hello sse turn");
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert!(
        messages[1]
            .message
            .plain_text_lossy()
            .contains("hello sse turn")
    );
}

#[tokio::test]
async fn session_events_stream_sse_until_turn_finished() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let events_response = get_session_events(&client, &base, &session).await;
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    assert!(
        events_response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let turn_response = post_turn(&client, &base, &session, "hello session events").await;
    assert_eq!(turn_response.status(), reqwest::StatusCode::OK);

    let events = read_sse_events_until(events_response, |events| {
        events.iter().any(|event| {
            matches!(
                event,
                CoreEvent::Turn {
                    session_id,
                    event: RuntimeEvent::TurnFinished {
                        session_id: finished_session_id,
                        finish_reason: Some(FinishReason::Stop),
                        ..
                    },
                } if *session_id == session.id && *finished_session_id == session.id
            )
        })
    })
    .await;

    assert!(!events.is_empty());
    assert!(events.iter().all(|event| {
        matches!(
            event,
            CoreEvent::Turn { session_id, .. } | CoreEvent::TurnCancelled { session_id }
                if *session_id == session.id
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Stop),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));
}
