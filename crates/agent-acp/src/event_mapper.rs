//! Event mapper for converting AgentCore events to ACP updates.

use std::collections::HashMap;

use agent_client_protocol as acp;
use agent_core::CoreEvent;
use agent_runtime::{
    BlockDelta, Message, MessageRole, RuntimeEvent, ToolCall, ToolExecutionResult,
};
use provider::ContentBlock;

pub enum MappedEvent {
    Updates(Vec<acp::SessionUpdate>),
    Cancelled,
    Failed(String),
    TurnComplete(acp::StopReason),
}

pub struct EventMapper {
    tool_calls: HashMap<String, ToolPresentation>,
    streamed_assistant_text: bool,
}

impl EventMapper {
    pub fn new() -> Self {
        Self {
            tool_calls: HashMap::new(),
            streamed_assistant_text: false,
        }
    }

    pub fn map(&mut self, event: CoreEvent) -> MappedEvent {
        match event {
            CoreEvent::Store { event: _ } => MappedEvent::Updates(vec![]),
            CoreEvent::Turn {
                session_id: _,
                event,
            } => self.map_runtime_event(event),
            CoreEvent::TurnCancelled { .. } => MappedEvent::Cancelled,
        }
    }

    fn map_runtime_event(&mut self, event: RuntimeEvent) -> MappedEvent {
        match event {
            RuntimeEvent::OutputBlockDelta { delta, .. } => self.map_block_delta(delta),
            RuntimeEvent::MessageCommitted { message } => self.map_message_committed(message),
            RuntimeEvent::ToolCallPending { call } => {
                let presentation = tool_presentation(&call);
                self.tool_calls
                    .insert(call.id.clone(), presentation.clone());
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCall(
                    acp::ToolCall::new(call.id, presentation.title)
                        .kind(presentation.kind)
                        .status(acp::ToolCallStatus::Pending)
                        .raw_input(presentation.raw_input),
                )])
            }
            RuntimeEvent::ToolCallStarted { call } => {
                let presentation = tool_presentation(&call);
                self.tool_calls
                    .insert(call.id.clone(), presentation.clone());
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCall(
                    acp::ToolCall::new(call.id, presentation.title)
                        .kind(presentation.kind)
                        .status(acp::ToolCallStatus::InProgress)
                        .raw_input(presentation.raw_input),
                )])
            }
            RuntimeEvent::ToolCallFinished { call, result } => {
                let presentation = self
                    .tool_calls
                    .get(&call.id)
                    .cloned()
                    .unwrap_or_else(|| tool_presentation(&call));
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCallUpdate(
                    acp::ToolCallUpdate::new(
                        call.id,
                        acp::ToolCallUpdateFields::new()
                            .title(presentation.title)
                            .kind(presentation.kind)
                            .status(acp::ToolCallStatus::Completed)
                            .content(vec![acp::ToolCallContent::from(acp::ContentBlock::Text(
                                acp::TextContent::new(render_tool_result(&result)),
                            ))])
                            .raw_output(result.output.clone()),
                    ),
                )])
            }
            RuntimeEvent::TurnFinished { .. } => {
                MappedEvent::TurnComplete(acp::StopReason::EndTurn)
            }
            RuntimeEvent::Error { message, .. } => MappedEvent::Failed(message),
            RuntimeEvent::InputQueued { .. }
            | RuntimeEvent::ControlQueued { .. }
            | RuntimeEvent::AgentQueued
            | RuntimeEvent::TurnStarted { .. }
            | RuntimeEvent::PhaseChanged { .. }
            | RuntimeEvent::BoundaryReached { .. }
            | RuntimeEvent::SteeringQueued { .. }
            | RuntimeEvent::SteeringApplied { .. }
            | RuntimeEvent::ApprovalRequested { .. }
            | RuntimeEvent::ApprovalResolved { .. }
            | RuntimeEvent::Interrupted { .. }
            | RuntimeEvent::ChildSpawnRequested { .. }
            | RuntimeEvent::ChildSpawned { .. }
            | RuntimeEvent::ChildStatusChanged { .. }
            | RuntimeEvent::ChildCompleted { .. }
            | RuntimeEvent::ChildFailed { .. }
            | RuntimeEvent::AgentInputQueued { .. }
            | RuntimeEvent::AgentInputDelivered { .. }
            | RuntimeEvent::AgentMessageQueued { .. }
            | RuntimeEvent::AgentMessageDelivered { .. }
            | RuntimeEvent::AgentInterrupted { .. }
            | RuntimeEvent::AgentWaitTimedOut { .. }
            | RuntimeEvent::ChildReportReceived { .. }
            | RuntimeEvent::ChildReportInjected { .. }
            | RuntimeEvent::PtyOpened { .. }
            | RuntimeEvent::PtyUpdated { .. }
            | RuntimeEvent::PtyEventEmitted { .. }
            | RuntimeEvent::PtySubscribed { .. }
            | RuntimeEvent::PtyUnsubscribed { .. }
            | RuntimeEvent::PtyEventDelivered { .. }
            | RuntimeEvent::PtyEventInjected { .. }
            | RuntimeEvent::PtyCaptured { .. }
            | RuntimeEvent::EnvelopeQueued { .. }
            | RuntimeEvent::EnvelopeDelivered { .. }
            | RuntimeEvent::EnvelopeReceived { .. }
            | RuntimeEvent::OutputBlockStart { .. }
            | RuntimeEvent::OutputBlockStop { .. }
            | RuntimeEvent::Usage { .. }
            | RuntimeEvent::SubcallFinished { .. }
            | RuntimeEvent::TranscriptRewritten { .. }
            | RuntimeEvent::TranscriptMessagesAppended { .. }
            | RuntimeEvent::Retry { .. }
            | RuntimeEvent::Compaction { .. }
            | RuntimeEvent::DoomLoopWarning { .. }
            | RuntimeEvent::AtifTrajectoryStarted { .. }
            | RuntimeEvent::AtifStepCompleted { .. }
            | RuntimeEvent::AtifFinalMetrics { .. }
            | RuntimeEvent::AtifTrajectoryCompleted { .. } => MappedEvent::Updates(vec![]),
        }
    }

    fn map_block_delta(&mut self, delta: BlockDelta) -> MappedEvent {
        match delta {
            BlockDelta::Text { text } | BlockDelta::Refusal { text } => {
                self.streamed_assistant_text = true;
                MappedEvent::Updates(vec![acp::SessionUpdate::AgentMessageChunk(
                    acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(text))),
                )])
            }
            BlockDelta::Reasoning { text } => {
                MappedEvent::Updates(vec![acp::SessionUpdate::AgentThoughtChunk(
                    acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(text))),
                )])
            }
            BlockDelta::Json { .. } | BlockDelta::Signature { .. } | BlockDelta::Unknown { .. } => {
                MappedEvent::Updates(vec![])
            }
        }
    }

    fn map_message_committed(&mut self, message: Message) -> MappedEvent {
        match message.role {
            MessageRole::Assistant => {
                let updates = message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } | ContentBlock::Refusal { text }
                            if !self.streamed_assistant_text =>
                        {
                            Some(acp::SessionUpdate::AgentMessageChunk(
                                acp::ContentChunk::new(acp::ContentBlock::Text(
                                    acp::TextContent::new(text.clone()),
                                )),
                            ))
                        }
                        ContentBlock::Reasoning { text } => Some(
                            acp::SessionUpdate::AgentThoughtChunk(acp::ContentChunk::new(
                                acp::ContentBlock::Text(acp::TextContent::new(text.clone())),
                            )),
                        ),
                        ContentBlock::ToolCall { id, name, input } => {
                            let call = ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                input: input.clone(),
                            };
                            let presentation = tool_presentation(&call);
                            self.tool_calls.insert(id.clone(), presentation.clone());
                            Some(acp::SessionUpdate::ToolCall(
                                acp::ToolCall::new(id.clone(), presentation.title)
                                    .kind(presentation.kind)
                                    .status(acp::ToolCallStatus::Pending)
                                    .raw_input(presentation.raw_input),
                            ))
                        }
                        ContentBlock::ImageUrl { .. } | ContentBlock::ToolResult { .. } => None,
                        ContentBlock::Text { .. } => None,
                        ContentBlock::Refusal { .. } => None,
                    })
                    .collect();
                MappedEvent::Updates(updates)
            }
            MessageRole::User => {
                let updates = message
                    .content
                    .iter()
                    .filter_map(content_block_to_user_update)
                    .collect();
                MappedEvent::Updates(updates)
            }
            MessageRole::System | MessageRole::Developer => MappedEvent::Updates(vec![]),
        }
    }
}

#[derive(Clone)]
struct ToolPresentation {
    title: String,
    kind: acp::ToolKind,
    raw_input: serde_json::Value,
}

fn tool_presentation(call: &ToolCall) -> ToolPresentation {
    ToolPresentation {
        title: default_tool_title(&call.name).to_owned(),
        kind: tool_kind(&call.name),
        raw_input: call.input.clone(),
    }
}

fn tool_kind(name: &str) -> acp::ToolKind {
    match name {
        "file_read" => acp::ToolKind::Read,
        "file_edit" | "file_write" => acp::ToolKind::Edit,
        "glob_search" | "grep" => acp::ToolKind::Search,
        "list_directory" => acp::ToolKind::Other,
        _ => acp::ToolKind::Execute,
    }
}

fn default_tool_title(name: &str) -> &'static str {
    match name {
        "file_read" => "Read File",
        "file_edit" => "Edit File",
        "file_write" => "Write File",
        "list_directory" => "List Directory",
        "glob_search" => "Glob Search",
        "grep" => "Grep",
        "shell" => "Shell",
        "echo" => "Echo",
        _ => "Tool",
    }
}

fn render_tool_result(result: &ToolExecutionResult) -> String {
    let body =
        serde_json::to_string_pretty(&result.output).unwrap_or_else(|_| result.output.to_string());
    if result.is_error {
        format!("[error]\n{body}")
    } else {
        body
    }
}

fn content_block_to_user_update(block: &ContentBlock) -> Option<acp::SessionUpdate> {
    let text = match block {
        ContentBlock::Text { text }
        | ContentBlock::Reasoning { text }
        | ContentBlock::Refusal { text } => text.clone(),
        ContentBlock::ImageUrl { url } => format!("[image:{url}]"),
        ContentBlock::ToolCall { name, .. } => format!("[tool_call:{name}]"),
        ContentBlock::ToolResult {
            call_id, output, ..
        } => {
            format!("[tool_result:{call_id}] {}", output)
        }
    };
    Some(acp::SessionUpdate::UserMessageChunk(
        acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(text))),
    ))
}
