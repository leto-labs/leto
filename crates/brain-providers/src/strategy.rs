use std::sync::atomic::{AtomicUsize, Ordering};

use brain_types::{CredentialEntry, SelectionContext, SelectionStrategy};

/// Round-robin credential selection with session stickiness.
///
/// New sessions get the next credential via round-robin. Sessions stay on
/// their credential unless it becomes unhealthy. On error: skip to next
/// healthy credential. If all unhealthy: pick the one with the oldest error.
pub struct StickyRoundRobin {
    index: AtomicUsize,
}

impl StickyRoundRobin {
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
            .filter(|e| !context.failed_credential_ids.contains(&e.id))
            .collect();

        if available.is_empty() {
            return entries
                .iter()
                .min_by_key(|e| e.health.last_error.as_ref().map(|err| err.at));
        }

        let idx = self.index.fetch_add(1, Ordering::Relaxed);
        Some(available[idx % available.len()])
    }
}

/// Ordered-priority credential selection.
///
/// Always prefers the first healthy credential in the list. Advances to
/// the next only when earlier ones have failed (i.e. their ids are in
/// `failed_credential_ids`).
pub struct Fallback;

impl Fallback {
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
            .find(|e| !context.failed_credential_ids.contains(&e.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_types::CredentialEntry;

    fn make_entries(n: usize) -> Vec<CredentialEntry> {
        (0..n)
            .map(|i| CredentialEntry::api_key(format!("key-{i}"), format!("sk-{i}")))
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
        let ctx = ctx(&[]);

        let ids: Vec<&str> = (0..6)
            .map(|_| strategy.select(&entries, &ctx).unwrap().id.as_str())
            .collect();

        assert_eq!(ids[0], "key-0");
        assert_eq!(ids[1], "key-1");
        assert_eq!(ids[2], "key-2");
        assert_eq!(ids[3], "key-0");
    }

    #[test]
    fn sticky_round_robin_skips_failed() {
        let strategy = StickyRoundRobin::new();
        let entries = make_entries(3);
        let ctx = ctx(&["key-0"]);

        let id = strategy.select(&entries, &ctx).unwrap().id.as_str();
        assert_ne!(id, "key-0");
    }

    #[test]
    fn sticky_round_robin_all_failed_returns_least_errored() {
        let strategy = StickyRoundRobin::new();
        let mut entries = make_entries(2);
        entries[0].health.record_error("fail", Some("429".into()));
        entries[1].health.record_error("fail", Some("500".into()));

        let ctx = ctx(&["key-0", "key-1"]);
        let result = strategy.select(&entries, &ctx);
        assert!(result.is_some());
    }

    #[test]
    fn sticky_round_robin_empty() {
        let strategy = StickyRoundRobin::new();
        assert!(strategy.select(&[], &ctx(&[])).is_none());
    }

    #[test]
    fn fallback_prefers_first() {
        let strategy = Fallback::new();
        let entries = make_entries(3);
        let ctx = ctx(&[]);

        let id = strategy.select(&entries, &ctx).unwrap().id.as_str();
        assert_eq!(id, "key-0");

        let id2 = strategy.select(&entries, &ctx).unwrap().id.as_str();
        assert_eq!(id2, "key-0");
    }

    #[test]
    fn fallback_advances_on_failure() {
        let strategy = Fallback::new();
        let entries = make_entries(3);

        let id = strategy
            .select(&entries, &ctx(&["key-0"]))
            .unwrap()
            .id
            .as_str();
        assert_eq!(id, "key-1");

        let id = strategy
            .select(&entries, &ctx(&["key-0", "key-1"]))
            .unwrap()
            .id
            .as_str();
        assert_eq!(id, "key-2");
    }

    #[test]
    fn fallback_all_failed() {
        let strategy = Fallback::new();
        let entries = make_entries(2);
        assert!(
            strategy
                .select(&entries, &ctx(&["key-0", "key-1"]))
                .is_none()
        );
    }

    #[test]
    fn fallback_empty() {
        let strategy = Fallback::new();
        assert!(strategy.select(&[], &ctx(&[])).is_none());
    }
}
