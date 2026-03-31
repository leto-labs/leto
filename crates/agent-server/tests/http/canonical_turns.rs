#[tokio::test]
async fn canonical_turn_endpoint_streams_ndjson() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .post(format!("{base}/v1/sessions/{}/turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "hello canonical server"}]
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap(),
        "application/x-ndjson"
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("turn_finished"));
}

#[tokio::test]
async fn canonical_batch_turns_endpoint_runs_turns_sequentially() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .post(format!("{base}/v1/sessions/{}/batch-turns", session.id))
        .json(&serde_json::json!({
            "turns": [
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "first batch turn"}]
                        }
                    ]
                },
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "second batch turn"}]
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap(),
        "application/x-ndjson"
    );

    let events = parse_ndjson_events(&response.text().await.unwrap());
    let finished = events
        .iter()
        .filter(|event| {
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
        .count();
    assert_eq!(finished, 2);
}

#[tokio::test]
async fn canonical_batch_turns_endpoint_emits_turn_lifecycle_events_in_order() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .post(format!("{base}/v1/sessions/{}/batch-turns", session.id))
        .json(&serde_json::json!({
            "turns": [
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "first ordered batch turn"}]
                        }
                    ]
                },
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "second ordered batch turn"}]
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let lifecycle = parse_ndjson_events(&response.text().await.unwrap())
        .into_iter()
        .filter_map(|event| match event {
            CoreEvent::Turn {
                session_id,
                event:
                    RuntimeEvent::TurnStarted {
                        session_id: started_session_id,
                        turn_index,
                    },
            } if session_id == session.id && started_session_id == session.id => {
                Some(("started", turn_index))
            }
            CoreEvent::Turn {
                session_id,
                event:
                    RuntimeEvent::TurnFinished {
                        session_id: finished_session_id,
                        turn_index,
                        ..
                    },
            } if session_id == session.id && finished_session_id == session.id => {
                Some(("finished", turn_index))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        lifecycle,
        vec![
            ("started", 1_u64),
            ("finished", 1_u64),
            ("started", 1_u64),
            ("finished", 1_u64),
        ]
    );
}

#[tokio::test]
async fn canonical_tool_calls_route_appends_tool_call_history() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let appended: Vec<StoredMessage> = client
        .post(format!("{base}/v1/sessions/{}/tool-calls", session.id))
        .json(&serde_json::json!({
            "calls": [
                {
                    "id": "call_1",
                    "name": "echo",
                    "input": {"text": "hello tool"},
                    "output": {"text": "tool output"},
                    "is_error": false
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(appended.len(), 2);
    assert_eq!(appended[0].session_id, session.id);
    assert_eq!(appended[0].ordinal, 0);
    assert_eq!(appended[0].message.role, MessageRole::Assistant);
    assert!(matches!(
        appended[0].message.content.as_slice(),
        [ContentBlock::ToolCall { id, name, input, .. }]
            if id == "call_1" && name == "echo" && input == &serde_json::json!({"text": "hello tool"})
    ));
    assert_eq!(appended[1].session_id, session.id);
    assert_eq!(appended[1].ordinal, 1);
    assert_eq!(appended[1].message.role, MessageRole::User);
    assert!(matches!(
        appended[1].message.content.as_slice(),
        [ContentBlock::ToolResult {
            call_id,
            output,
            is_error: Some(false),
        }] if call_id == "call_1" && output == &serde_json::json!({"text": "tool output"})
    ));

    let history: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(history, appended);
}

#[tokio::test]
async fn canonical_session_workflow_round_trips_turns_tool_calls_and_history() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let batch_response = client
        .post(format!("{base}/v1/sessions/{}/batch-turns", session.id))
        .json(&serde_json::json!({
            "turns": [
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "workflow step one"}]
                        }
                    ]
                },
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "workflow step two"}]
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let batch_events = parse_ndjson_events(&batch_response.text().await.unwrap());
    let finished = batch_events
        .iter()
        .filter(|event| {
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
        .count();
    assert_eq!(finished, 2);

    let tool_messages: Vec<StoredMessage> = client
        .post(format!("{base}/v1/sessions/{}/tool-calls", session.id))
        .json(&serde_json::json!({
            "calls": [
                {
                    "id": "workflow_call",
                    "name": "echo",
                    "input": {"text": "workflow tool input"},
                    "output": {"text": "workflow tool output"},
                    "is_error": false
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(tool_messages.len(), 2);

    let history: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(history.len(), 6);
    assert_eq!(history[0].message.role, MessageRole::User);
    assert_eq!(history[0].message.plain_text_lossy(), "workflow step one");
    assert_eq!(history[1].message.role, MessageRole::Assistant);
    assert!(
        history[1]
            .message
            .plain_text_lossy()
            .contains("workflow step one")
    );
    assert_eq!(history[2].message.role, MessageRole::User);
    assert_eq!(history[2].message.plain_text_lossy(), "workflow step two");
    assert_eq!(history[3].message.role, MessageRole::Assistant);
    assert!(
        history[3]
            .message
            .plain_text_lossy()
            .contains("workflow step two")
    );
    assert_eq!(history[4].message.role, MessageRole::Assistant);
    assert!(matches!(
        history[4].message.content.as_slice(),
        [ContentBlock::ToolCall { id, name, input, .. }]
            if id == "workflow_call"
                && name == "echo"
                && input == &serde_json::json!({"text": "workflow tool input"})
    ));
    assert_eq!(history[5].message.role, MessageRole::User);
    assert!(matches!(
        history[5].message.content.as_slice(),
        [ContentBlock::ToolResult {
            call_id,
            output,
            is_error: Some(false),
        }] if call_id == "workflow_call"
            && output == &serde_json::json!({"text": "workflow tool output"})
    ));
}

#[tokio::test]
async fn canonical_event_endpoint_streams_runtime_events() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .get(format!("{base}/v1/events"))
        .send()
        .await
        .unwrap();
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

    let turn_response = client
        .post(format!("{base}/v1/sessions/{}/turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "hello runtime bus"}]
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(turn_response.status(), reqwest::StatusCode::OK);

    let events = read_sse_events_until(response, |events| {
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
