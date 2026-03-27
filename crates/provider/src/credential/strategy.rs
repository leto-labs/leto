use std::sync::atomic::{AtomicUsize, Ordering};

use crate::credential::{CredentialEntry, SelectionContext};

/// Strategy used by the shared credential pool to choose one credential.
pub trait SelectionStrategy: Send + Sync {
    /// Selects one credential from the provided entries.
    fn select<'a>(
        &self,
        entries: &'a [CredentialEntry],
        context: &SelectionContext,
    ) -> Option<&'a CredentialEntry>;
}

/// Round-robin credential selection with session stickiness.
pub struct StickyRoundRobin {
    index: AtomicUsize,
}

impl StickyRoundRobin {
    /// Creates a sticky round-robin strategy.
    pub fn new() -> Self {
        Self {
            index: AtomicUsize::new(0),
        }
    }
}

impl Default for StickyRoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStrategy for StickyRoundRobin {
    fn select<'a>(
        &self,
        entries: &'a [CredentialEntry],
        context: &SelectionContext,
    ) -> Option<&'a CredentialEntry> {
        if entries.is_empty() {
            return None;
        }

        let available: Vec<&CredentialEntry> = entries
            .iter()
            .filter(|entry| {
                entry.health.is_healthy() && !context.failed_credential_ids.contains(&entry.id)
            })
            .collect();

        if !available.is_empty() {
            let idx = self.index.fetch_add(1, Ordering::Relaxed);
            return Some(available[idx % available.len()]);
        }

        entries
            .iter()
            .filter(|entry| !context.failed_credential_ids.contains(&entry.id))
            .min_by_key(|entry| {
                entry
                    .health
                    .last_error
                    .as_ref()
                    .map(|error| error.recorded_at_ms)
            })
            .or_else(|| {
                entries.iter().min_by_key(|entry| {
                    entry
                        .health
                        .last_error
                        .as_ref()
                        .map(|error| error.recorded_at_ms)
                })
            })
    }
}

/// Ordered-priority credential selection.
pub struct Fallback;

impl Fallback {
    /// Creates a fallback strategy.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Fallback {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStrategy for Fallback {
    fn select<'a>(
        &self,
        entries: &'a [CredentialEntry],
        context: &SelectionContext,
    ) -> Option<&'a CredentialEntry> {
        entries
            .iter()
            .find(|entry| {
                entry.health.is_healthy() && !context.failed_credential_ids.contains(&entry.id)
            })
            .or_else(|| {
                entries
                    .iter()
                    .find(|entry| !context.failed_credential_ids.contains(&entry.id))
            })
            .or_else(|| entries.first())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::{CredentialFailure, CredentialMaterial};

    fn make_entries(n: usize) -> Vec<CredentialEntry> {
        (0..n)
            .map(|i| CredentialEntry::new(format!("key-{i}"), CredentialMaterial::new()))
            .collect()
    }

    fn ctx(failed: &[&str]) -> SelectionContext {
        SelectionContext {
            session_id: None,
            failed_credential_ids: failed.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn sticky_round_robin_distributes() {
        let strategy = StickyRoundRobin::new();
        let entries = make_entries(3);

        let ids: Vec<&str> = (0..6)
            .map(|_| strategy.select(&entries, &ctx(&[])).unwrap().id.as_str())
            .collect();

        assert_eq!(ids[0], "key-0");
        assert_eq!(ids[1], "key-1");
        assert_eq!(ids[2], "key-2");
        assert_eq!(ids[3], "key-0");
    }

    #[test]
    fn sticky_round_robin_skips_failed_and_unhealthy() {
        let strategy = StickyRoundRobin::new();
        let mut entries = make_entries(3);
        entries[1]
            .health
            .record_error(CredentialFailure::new("rate limited"));

        let id = strategy
            .select(&entries, &ctx(&["key-0"]))
            .unwrap()
            .id
            .as_str();
        assert_eq!(id, "key-2");
    }

    #[test]
    fn fallback_prefers_first_healthy() {
        let strategy = Fallback::new();
        let entries = make_entries(3);
        let id = strategy.select(&entries, &ctx(&[])).unwrap().id.as_str();
        assert_eq!(id, "key-0");
    }

    #[test]
    fn fallback_advances_when_first_is_unhealthy() {
        let strategy = Fallback::new();
        let mut entries = make_entries(2);
        entries[0]
            .health
            .record_error(CredentialFailure::new("boom"));

        let id = strategy.select(&entries, &ctx(&[])).unwrap().id.as_str();
        assert_eq!(id, "key-1");
    }
}
