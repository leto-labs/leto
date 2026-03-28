//! ACP capabilities helper functions.

use agent_client_protocol as acp;

/// Configuration option IDs used by ACP.
pub const CONFIG_MODEL: &str = "model";
pub const CONFIG_THOUGHT_LEVEL: &str = "thought_level";
pub const CONFIG_LOOP: &str = "loop";

/// Builds the initialize response with agent capabilities.
pub fn initialize_response(protocol_version: acp::ProtocolVersion) -> acp::InitializeResponse {
    acp::InitializeResponse::new(protocol_version)
        .agent_info(acp::Implementation::new("agent-acp", env!("CARGO_PKG_VERSION")).title("Agent ACP"))
        .agent_capabilities(
            acp::AgentCapabilities::new()
                .load_session(true)
                .session_capabilities(
                    acp::SessionCapabilities::new().list(acp::SessionListCapabilities::new()),
                ),
        )
}
