//! Lightweight ephemeral key-value store with per-entry TTL.
//!
//! Stores observations, decision context, and anomaly baselines.
//! Cleared entirely on self-destruct.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// In-memory key-value store with automatic TTL expiry.
///
/// Each entry carries its own TTL; reads of expired entries return `None`
/// and the entry is lazily retained until the next [`expire_stale`] sweep.
pub struct EphemeralState {
    entries: HashMap<String, Entry>,
    default_ttl: Duration,
}

struct Entry {
    value: String,
    inserted_at: Instant,
    ttl: Duration,
}

impl Entry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() >= self.ttl
    }
}

impl EphemeralState {
    /// Create a new store with the given default TTL for entries that don't
    /// specify one explicitly.
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
        }
    }

    /// Retrieve a value if it exists and has not expired.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).and_then(|e| {
            if e.is_expired() {
                None
            } else {
                Some(e.value.as_str())
            }
        })
    }

    /// Insert or overwrite a value using the default TTL.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.set_with_ttl(key, value, self.default_ttl);
    }

    /// Insert or overwrite a value with a custom TTL.
    pub fn set_with_ttl(&mut self, key: &str, value: impl Into<String>, ttl: Duration) {
        self.entries.insert(
            key.to_owned(),
            Entry {
                value: value.into(),
                inserted_at: Instant::now(),
                ttl,
            },
        );
    }

    /// Remove an entry, returning its value if it existed (even if expired).
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key).map(|e| e.value)
    }

    /// Check whether a non-expired entry exists for `key`.
    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Remove all expired entries. Returns the number removed.
    pub fn expire_stale(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| !e.is_expired());
        before - self.entries.len()
    }

    /// Wipe all state unconditionally (for self-destruct).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of entries stored (including any that may have expired but
    /// not yet been swept).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return all keys whose entries have not expired.
    pub fn keys(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, e)| !e.is_expired())
            .map(|(k, _)| k.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        state.set("key1", "value1");
        assert_eq!(state.get("key1"), Some("value1"));
    }

    #[test]
    fn test_get_missing_key() {
        let state = EphemeralState::new(Duration::from_secs(60));
        assert_eq!(state.get("nonexistent"), None);
    }

    #[test]
    fn test_overwrite() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        state.set("key1", "v1");
        state.set("key1", "v2");
        assert_eq!(state.get("key1"), Some("v2"));
    }

    #[test]
    fn test_remove() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        state.set("key1", "value1");
        let removed = state.remove("key1");
        assert_eq!(removed, Some("value1".to_string()));
        assert_eq!(state.get("key1"), None);
    }

    #[test]
    fn test_remove_missing() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        assert_eq!(state.remove("nope"), None);
    }

    #[test]
    fn test_contains() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        assert!(!state.contains("key1"));
        state.set("key1", "value1");
        assert!(state.contains("key1"));
    }

    #[test]
    fn test_ttl_expiry() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        // Insert with zero TTL so it is already expired.
        state.set_with_ttl("ephemeral", "gone", Duration::from_millis(0));
        // Give a tiny bit of time for the expiry to be observable.
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(state.get("ephemeral"), None);
        assert!(!state.contains("ephemeral"));
    }

    #[test]
    fn test_expire_stale() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        state.set_with_ttl("short", "x", Duration::from_millis(0));
        state.set("long", "y");
        std::thread::sleep(Duration::from_millis(1));
        let removed = state.expire_stale();
        assert_eq!(removed, 1);
        assert_eq!(state.len(), 1);
        assert_eq!(state.get("long"), Some("y"));
    }

    #[test]
    fn test_clear() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        state.set("a", "1");
        state.set("b", "2");
        state.clear();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn test_keys_excludes_expired() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        state.set_with_ttl("expired", "x", Duration::from_millis(0));
        state.set("alive", "y");
        std::thread::sleep(Duration::from_millis(1));
        let keys = state.keys();
        assert_eq!(keys, vec!["alive"]);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut state = EphemeralState::new(Duration::from_secs(60));
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        state.set("k", "v");
        assert!(!state.is_empty());
        assert_eq!(state.len(), 1);
    }
}
