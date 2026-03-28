use agent_client_protocol as acp;
use agent_core::CoreEvent;
use agent_runtime::{BlockDelta, Message, RuntimeEvent, ToolCall, ToolExecutionResult};
use agent_store::{Session, StoredMessage};
use provider::{ContentBlock, MessageRole};

use crate::event_mapper::{EventMapper, MappedEvent};
use crate::history_replay::replay_updates;

#[test]
fn output_text_delta_maps_to_agent_message_chunk() {
    let mut mapper = EventMapper::new();

    let mapped = mapper.map(CoreEvent::Turn {
        session_id: Session::new(ulid::Ulid::new()).id,
        event: RuntimeEvent::OutputBlockDelta {
            id: "block-1".into(),
            delta: BlockDelta::Text {
                text: "hello".into(),
            },
        },
    });

    let MappedEvent::Updates(updates) = mapped else {
        panic!("expected updates");
    };
    assert!(matches!(
        updates.as_slice(),
        [acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
        })] if text.text == "hello"
    ));
}

#[test]
fn tool_events_map_to_pending_and_completed_updates() {
    let mut mapper = EventMapper::new();
    let session_id = Session::new(ulid::Ulid::new()).id;
    let call = ToolCall {
        id: "call-1".into(),
        name: "shell".into(),
        input: serde_json::json!({"cmd": "pwd"}),
    };

    let pending = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::ToolCallStarted { call: call.clone() },
    });
    let completed = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::ToolCallFinished {
            call,
            result: ToolExecutionResult::success(serde_json::json!({"stdout": "/tmp"})),
        },
    });

    let MappedEvent::Updates(pending_updates) = pending else {
        panic!("expected pending updates");
    };
    assert!(matches!(
        pending_updates.as_slice(),
        [acp::SessionUpdate::ToolCall(tool_call)]
            if tool_call.id.0 == "call-1"
                && matches!(tool_call.status, Some(acp::ToolCallStatus::InProgress))
    ));

    let MappedEvent::Updates(completed_updates) = completed else {
        panic!("expected completed updates");
    };
    assert!(matches!(
        completed_updates.as_slice(),
        [acp::SessionUpdate::ToolCallUpdate(update)]
            if update.tool_call_id.0 == "call-1"
    ));
}

#[test]
fn replay_updates_converts_stored_message_blocks_to_chunks() {
    let project_id = ulid::Ulid::new();
    let session = Session::new(project_id);
    let messages = vec![
        StoredMessage::new(
            session.id,
            0,
            Message::new(
                MessageRole::User,
                vec![
                    ContentBlock::text("hello"),
                    ContentBlock::image_url("https://example.com/cat.png"),
                ],
            ),
        ),
        StoredMessage::new(
            session.id,
            1,
            Message::new(
                MessageRole::Assistant,
                vec![ContentBlock::Reasoning {
                    text: "thinking".into(),
                }],
            ),
        ),
    ];

    let updates = replay_updates(&session, &messages);

    assert!(matches!(
        updates.get(1),
        Some(acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
        })) if text.text == "hello"
    ));
    assert!(matches!(
        updates.get(2),
        Some(acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
        })) if text.text == "[image:https://example.com/cat.png]"
    ));
    assert!(matches!(
        updates.get(3),
        Some(acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
        })) if text.text == "thinking"
    ));
}
