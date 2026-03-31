//! History replay helpers for ACP session loading.

use agent_client_protocol as acp;
use agent_store::{Session, StoredMessage};
use provider::{ContentBlock, MessageRole};

/// Generates ACP session updates for replaying session history.
pub fn replay_updates(session: &Session, messages: &[StoredMessage]) -> Vec<acp::SessionUpdate> {
    let mut updates = vec![];

    // Emit session info
    updates.push(acp::SessionUpdate::SessionInfoUpdate(
        acp::SessionInfoUpdate::new().updated_at(session.updated_at.to_rfc3339()),
    ));

    // Emit message history
    for msg in messages {
        updates.extend(message_updates(msg));
    }

    updates
}

fn message_updates(message: &StoredMessage) -> Vec<acp::SessionUpdate> {
    let prefix = match message.message.role {
        MessageRole::System => Some("[system] "),
        MessageRole::Developer => Some("[developer] "),
        MessageRole::User | MessageRole::Assistant => None,
    };

    message
        .message
        .content
        .iter()
        .filter_map(|block| {
            let text = render_block(block)?;
            let text = match prefix {
                Some(prefix) => format!("{prefix}{text}"),
                None => text,
            };
            let chunk =
                acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(text)));
            Some(match message.message.role {
                MessageRole::User => acp::SessionUpdate::UserMessageChunk(chunk),
                MessageRole::Assistant | MessageRole::System | MessageRole::Developer => {
                    acp::SessionUpdate::AgentMessageChunk(chunk)
                }
            })
        })
        .collect()
}

fn render_block(block: &ContentBlock) -> Option<String> {
    Some(match block {
        ContentBlock::Text { text }
        | ContentBlock::Reasoning { text }
        | ContentBlock::Refusal { text } => text.clone(),
        ContentBlock::ImageUrl { url } => format!("[image:{url}]"),
        ContentBlock::ToolCall {
            id,
            call_id,
            name,
            input,
        } => {
            format!(
                "[tool_call:{name}:{}] {input}",
                call_id.clone().unwrap_or_else(|| id.clone())
            )
        }
        ContentBlock::ToolResult {
            call_id,
            output,
            is_error,
        } => match is_error {
            Some(true) => format!("[tool_error:{call_id}] {output}"),
            _ => format!("[tool_result:{call_id}] {output}"),
        },
    })
}
