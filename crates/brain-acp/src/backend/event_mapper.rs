use std::collections::{HashMap, HashSet};

use agent_client_protocol as acp;
use brain_core::{BrainErrorCode, Event, Message, Role};

pub struct EventMapper {
    tool_calls: HashMap<String, ToolPresentation>,
    streamed_assistant_text: bool,
    seen_done: bool,
}

#[derive(Clone)]
struct ToolPresentation {
    title: String,
    kind: acp::ToolKind,
    raw_input: serde_json::Value,
}

#[derive(Debug)]
pub enum MappedEvent {
    Updates(Vec<acp::SessionUpdate>),
    Cancelled,
    Failed(String),
    TurnComplete(acp::StopReason),
}

impl EventMapper {
    pub fn new() -> Self {
        Self {
            tool_calls: HashMap::new(),
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
                let presentation = tool_presentation(&name, arguments);
                self.tool_calls.insert(id.clone(), presentation.clone());
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCall(
                    acp::ToolCall::new(id.clone(), presentation.title.clone())
                        .kind(presentation.kind)
                        .status(acp::ToolCallStatus::InProgress)
                        .raw_input(presentation.raw_input),
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
                let presentation =
                    self.tool_calls
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| ToolPresentation {
                            title: "Tool".into(),
                            kind: acp::ToolKind::Execute,
                            raw_input: serde_json::json!({}),
                        });
                MappedEvent::Updates(vec![acp::SessionUpdate::ToolCallUpdate(
                    acp::ToolCallUpdate::new(
                        id.clone(),
                        acp::ToolCallUpdateFields::new()
                            .title(presentation.title)
                            .kind(presentation.kind)
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
                } else if code == BrainErrorCode::MaxIterations {
                    self.seen_done = true;
                    MappedEvent::TurnComplete(acp::StopReason::EndTurn)
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
            let presentation = tool_presentation(&tool_call.name, tool_call.arguments);
            self.tool_calls
                .insert(tool_call.id.clone(), presentation.clone());
            updates.push(acp::SessionUpdate::ToolCall(
                acp::ToolCall::new(tool_call.id.clone(), presentation.title)
                    .kind(presentation.kind)
                    .status(acp::ToolCallStatus::Pending)
                    .raw_input(presentation.raw_input),
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
        "file_write" => acp::ToolKind::Edit,
        // Nori has a title-based fallback for list operations, so avoid
        // forcing this into Read/Execute and let the title drive rendering.
        "list_directory" => acp::ToolKind::Other,
        "shell" | "echo" => acp::ToolKind::Execute,
        _ => acp::ToolKind::Execute,
    }
}

fn tool_presentation(name: &str, arguments: serde_json::Value) -> ToolPresentation {
    if name == "apply_patch" {
        if let Some(summary) = summarize_apply_patch(&arguments) {
            return summary;
        }
    }

    ToolPresentation {
        title: default_tool_title(name).into(),
        kind: tool_kind(name),
        raw_input: arguments,
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
        "apply_patch" => "Apply Patch",
        _ => "Tool",
    }
}

fn summarize_apply_patch(arguments: &serde_json::Value) -> Option<ToolPresentation> {
    let patch = arguments.get("patch")?.as_str()?;
    let normalized = patch.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();

    let begin = lines
        .iter()
        .position(|line| line.trim() == "*** Begin Patch")?;
    let end = lines
        .iter()
        .rposition(|line| line.trim() == "*** End Patch")?;
    if begin >= end {
        return None;
    }

    let mut ops = Vec::new();
    let mut index = begin + 1;
    while index < end {
        let line = lines[index].trim_end();
        if line.is_empty() {
            index += 1;
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Add File:") {
            let path = path.trim();
            if path.is_empty() {
                return None;
            }
            index += 1;
            let mut contents = Vec::new();
            while index < end && !is_patch_header(lines[index]) {
                let add_line = lines[index];
                let added = add_line.strip_prefix('+')?;
                contents.push(added.to_owned());
                index += 1;
            }
            ops.push(PatchSummaryOp::Add {
                path: path.to_owned(),
                content: contents.join("\n"),
            });
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Delete File:") {
            let path = path.trim();
            if path.is_empty() {
                return None;
            }
            index += 1;
            ops.push(PatchSummaryOp::Delete {
                path: path.to_owned(),
            });
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Update File:") {
            let path = path.trim();
            if path.is_empty() {
                return None;
            }
            index += 1;

            if index < end && lines[index].starts_with("*** Move to:") {
                return None;
            }

            let mut old_lines = Vec::new();
            let mut new_lines = Vec::new();
            let mut hunk_count = 0usize;

            while index < end && !is_patch_header(lines[index]) {
                if lines[index].trim().is_empty() {
                    index += 1;
                    continue;
                }
                if !lines[index].starts_with("@@") {
                    return None;
                }
                hunk_count += 1;
                index += 1;

                while index < end
                    && !lines[index].starts_with("@@")
                    && !is_patch_header(lines[index])
                {
                    let hunk_line = lines[index];
                    if hunk_line == "*** End of File" {
                        index += 1;
                        break;
                    }
                    if let Some(line) = hunk_line.strip_prefix(' ') {
                        old_lines.push(line.to_owned());
                        new_lines.push(line.to_owned());
                    } else if let Some(line) = hunk_line.strip_prefix('-') {
                        old_lines.push(line.to_owned());
                    } else if let Some(line) = hunk_line.strip_prefix('+') {
                        new_lines.push(line.to_owned());
                    } else {
                        return None;
                    }
                    index += 1;
                }
            }

            if hunk_count != 1 {
                return None;
            }

            ops.push(PatchSummaryOp::Update {
                path: path.to_owned(),
                old_string: old_lines.join("\n"),
                new_string: new_lines.join("\n"),
            });
            continue;
        }

        return None;
    }

    if ops.len() != 1 {
        let count = ops.len();
        return Some(ToolPresentation {
            title: format!("Apply Patch ({count} files)"),
            kind: acp::ToolKind::Execute,
            raw_input: arguments.clone(),
        });
    }

    match ops.into_iter().next()? {
        PatchSummaryOp::Add { path, content } => Some(ToolPresentation {
            title: "Write File".into(),
            kind: acp::ToolKind::Edit,
            raw_input: serde_json::json!({
                "path": path,
                "content": content,
            }),
        }),
        PatchSummaryOp::Delete { path } => Some(ToolPresentation {
            title: "Delete File".into(),
            kind: acp::ToolKind::Delete,
            raw_input: serde_json::json!({
                "path": path,
            }),
        }),
        PatchSummaryOp::Update {
            path,
            old_string,
            new_string,
        } => Some(ToolPresentation {
            title: "Edit File".into(),
            kind: acp::ToolKind::Edit,
            raw_input: serde_json::json!({
                "path": path,
                "old_string": old_string,
                "new_string": new_string,
            }),
        }),
    }
}

fn is_patch_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("*** Add File:")
        || trimmed.starts_with("*** Delete File:")
        || trimmed.starts_with("*** Update File:")
        || trimmed.starts_with("*** End Patch")
}

enum PatchSummaryOp {
    Add {
        path: String,
        content: String,
    },
    Delete {
        path: String,
    },
    Update {
        path: String,
        old_string: String,
        new_string: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_iterations_error_ends_turn_instead_of_failing_prompt() {
        let mut mapper = EventMapper::new();
        let mapped = mapper.map(Event::Error {
            code: BrainErrorCode::MaxIterations,
            message: "max iterations reached: 20".into(),
            recoverable: false,
        });

        match mapped {
            MappedEvent::TurnComplete(acp::StopReason::EndTurn) => {}
            other => panic!("unexpected mapping: {other:?}"),
        }
    }

    #[test]
    fn cancelled_error_maps_to_cancelled_stop_reason() {
        let mut mapper = EventMapper::new();
        let mapped = mapper.map(Event::Error {
            code: BrainErrorCode::Cancelled,
            message: "cancelled".into(),
            recoverable: false,
        });

        match mapped {
            MappedEvent::Cancelled => {}
            other => panic!("unexpected mapping: {other:?}"),
        }
    }

    #[test]
    fn list_directory_uses_human_title_and_other_kind() {
        let presentation = tool_presentation("list_directory", serde_json::json!({"path": "."}));
        assert_eq!(presentation.title, "List Directory");
        assert_eq!(presentation.kind, acp::ToolKind::Other);
        assert_eq!(presentation.raw_input["path"], ".");
    }

    #[test]
    fn file_write_maps_to_edit() {
        let presentation = tool_presentation(
            "file_write",
            serde_json::json!({"path": "tmp/note.txt", "content": "hello"}),
        );
        assert_eq!(presentation.title, "Write File");
        assert_eq!(presentation.kind, acp::ToolKind::Edit);
        assert_eq!(presentation.raw_input["path"], "tmp/note.txt");
    }

    #[test]
    fn apply_patch_single_add_maps_to_write_file() {
        let presentation = tool_presentation(
            "apply_patch",
            serde_json::json!({
                "patch": "*** Begin Patch\n*** Add File: tmp/new.txt\n+hello\n*** End Patch"
            }),
        );
        assert_eq!(presentation.title, "Write File");
        assert_eq!(presentation.kind, acp::ToolKind::Edit);
        assert_eq!(presentation.raw_input["path"], "tmp/new.txt");
        assert_eq!(presentation.raw_input["content"], "hello");
    }

    #[test]
    fn apply_patch_single_update_maps_to_edit_file() {
        let presentation = tool_presentation(
            "apply_patch",
            serde_json::json!({
                "patch": "*** Begin Patch\n*** Update File: tmp/note.txt\n@@\n-old\n+new\n*** End Patch"
            }),
        );
        assert_eq!(presentation.title, "Edit File");
        assert_eq!(presentation.kind, acp::ToolKind::Edit);
        assert_eq!(presentation.raw_input["path"], "tmp/note.txt");
        assert_eq!(presentation.raw_input["old_string"], "old");
        assert_eq!(presentation.raw_input["new_string"], "new");
    }

    #[test]
    fn apply_patch_multi_file_stays_generic() {
        let presentation = tool_presentation(
            "apply_patch",
            serde_json::json!({
                "patch": "*** Begin Patch\n*** Add File: a.txt\n+one\n*** Add File: b.txt\n+two\n*** End Patch"
            }),
        );
        assert_eq!(presentation.title, "Apply Patch (2 files)");
        assert_eq!(presentation.kind, acp::ToolKind::Execute);
        assert!(presentation.raw_input.get("patch").is_some());
    }
}
