/// Context passed to a selection strategy when choosing a credential.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionContext {
    /// Optional stable session identifier used for sticky strategies.
    pub session_id: Option<String>,
    /// Credential ids that should be avoided during the current selection pass.
    pub failed_credential_ids: Vec<String>,
}
