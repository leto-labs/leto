use ulid::Ulid;

use crate::CredentialEntry;

/// Context passed to a selection strategy when choosing a credential.
pub struct SelectionContext {
    pub session_id: Option<Ulid>,
    pub failed_credential_ids: Vec<String>,
}

/// Pluggable strategy for selecting a credential from a pool of entries.
///
/// Implementations receive only enabled entries. They return `None` if no
/// suitable credential exists (e.g. all have been tried).
pub trait SelectionStrategy: Send + Sync {
    fn select<'a>(
        &self,
        entries: &'a [CredentialEntry],
        context: &SelectionContext,
    ) -> Option<&'a CredentialEntry>;
}
