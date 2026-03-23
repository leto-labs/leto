use agent_client_protocol as acp;
use brain_core::{Message, Role, Session};

use super::capabilities::session_info_update;

pub fn replay_updates(session: &Session, messages: &[Message]) -> Vec<acp::SessionUpdate> {
    let mut updates = vec![acp::SessionUpdate::SessionInfoUpdate(session_info_update(
        session,
    ))];

    updates.extend(messages.iter().filter_map(message_update));
    updates
}

fn message_update(message: &Message) -> Option<acp::SessionUpdate> {
    let content = match message.role {
        Role::System => format!("[system] {}", message.content),
        Role::User | Role::Assistant => message.content.to_string(),
        Role::Tool => return None,
    };

    let chunk = acp::ContentChunk::new(content.into());
    Some(match message.role {
        Role::User => acp::SessionUpdate::UserMessageChunk(chunk),
        Role::System | Role::Assistant => acp::SessionUpdate::AgentMessageChunk(chunk),
        Role::Tool => unreachable!(),
    })
}
