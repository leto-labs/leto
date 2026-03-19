use std::collections::{HashMap, HashSet};

use agent_client_protocol as acp;
use brain_core::{BrainErrorCode, Event, Message, Role};

pub struct EventMapper {
    tool_names: HashMap<String, String>,
    streamed_assistant_text: bool,
    seen_done: bool,
}

pub enum MappedEvent {
    Updates(Vec<acp::SessionUpdate>),
    Cancelled,
    Failed(String),
    TurnComplete(acp::StopReason),
}

impl EventMapper {
    pub fn new() -> Self {
        Self {
            tool_names: HashMap::new(),
            streamed_assistant_text: false,
            seen_done: false,
        }
    }

    pub fn map(&mut self, event: Event) -> MappedEvent {
        match event {
            Event::Token { delta } => {
                self.streamed_assistant_text = true;
                MappedEvent::Updates(vec![acp::SessionUpdate::AgentMessageChunk(
                    acp::ContentChunk::new(delta.into()),
                )])
            }
            Event::MessageDone { message } => self.map_message_done(message),
            Event::ToolCallStart {
                id,
                name,
                arguments,
            } => {
                self.tool_names.insert(id.clone(), name.clone());
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCall(
                    acp::ToolCall::new(id.clone(), format!("Run {name}"))
                        .kind(tool_kind(&name))
                        .status(acp::ToolCallStatus::InProgress)
                        .raw_input(arguments),
                )])
            }
            Event::ToolCallDelta { .. }
            | Event::ToolCallPending { .. }
            | Event::ToolCallApproved { .. }
            | Event::ToolCallRejected { .. }
            | Event::Progress { .. }
            | Event::SessionStart { .. }
            | Event::SessionResume { .. }
            | Event::Retry { .. }
            | Event::Compaction { .. }
            | Event::DoomLoopWarning { .. } => MappedEvent::Updates(Vec::new()),
            Event::ToolCallDone { id, result, .. } => {
                let name = self
                    .tool_names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "tool".into());
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCallUpdate(
                    acp::ToolCallUpdate::new(
                        id.clone(),
                        acp::ToolCallUpdateFields::new()
                            .title(format!("Run {name}"))
                            .kind(tool_kind(&name))
                            .status(acp::ToolCallStatus::Completed)
                            .content(vec![result.into()]),
                    ),
                )])
            }
            Event::TurnDone { .. } => {
                self.seen_done = true;
                MappedEvent::TurnComplete(acp::StopReason::EndTurn)
            }
            Event::Error { code, message, .. } => {
                if code == BrainErrorCode::Cancelled {
                    MappedEvent::Cancelled
                } else {
                    MappedEvent::Failed(message)
                }
            }
            _ => MappedEvent::Updates(Vec::new()),
        }
    }

    fn map_message_done(&mut self, message: Message) -> MappedEvent {
        if message.role != Role::Assistant {
            return MappedEvent::Updates(Vec::new());
        }

        let mut updates = Vec::new();
        if !self.streamed_assistant_text && !message.content.is_empty() {
            updates.push(acp::SessionUpdate::AgentMessageChunk(
                acp::ContentChunk::new(message.content.into()),
            ));
        }

        let mut emitted_tool_ids = HashSet::new();
        for tool_call in message.tool_calls {
            if !emitted_tool_ids.insert(tool_call.id.clone()) {
                continue;
            }
            self.tool_names
                .insert(tool_call.id.clone(), tool_call.name.clone());
            updates.push(acp::SessionUpdate::ToolCall(
                acp::ToolCall::new(tool_call.id.clone(), format!("Run {}", tool_call.name))
                    .kind(tool_kind(&tool_call.name))
                    .status(acp::ToolCallStatus::Pending)
                    .raw_input(tool_call.arguments),
            ));
        }

        MappedEvent::Updates(updates)
    }
}

fn tool_kind(name: &str) -> acp::ToolKind {
    match name {
        "file_read" => acp::ToolKind::Read,
        "file_edit" => acp::ToolKind::Edit,
        "glob_search" | "grep" => acp::ToolKind::Search,
        "shell" | "echo" | "file_write" => acp::ToolKind::Execute,
        _ => acp::ToolKind::Execute,
    }
}
