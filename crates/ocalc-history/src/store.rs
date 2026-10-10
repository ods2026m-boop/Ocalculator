//! In-memory history store with search + JSON persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::HistoryError;

/// History file format version (Phase 3, v1).
///
/// Stored as `{"format_version":1,…}` alongside the payload. Loaders reject
/// unknown future versions instead of silently misreading them.
pub const HISTORY_FORMAT_VERSION: u32 = 1;

/// One recorded calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Sequential id (1-based within this store).
    pub id: u64,
    /// Original expression text.
    pub expression: String,
    /// Rendered result text.
    pub result: String,
    /// When the calculation happened.
    pub timestamp: DateTime<Utc>,
    /// Optional evaluation duration in microseconds.
    pub duration_us: Option<u64>,
}

impl HistoryEntry {
    /// Build an entry stamped with `Utc::now()`.
    #[must_use]
    pub fn new(id: u64, expression: &str, result: &str) -> Self {
        Self {
            id,
            expression: expression.to_owned(),
            result: result.to_owned(),
            timestamp: Utc::now(),
            duration_us: None,
        }
    }

    /// Attach a duration.
    #[must_use]
    pub fn with_duration(mut self, us: u64) -> Self {
        self.duration_us = Some(us);
        self
    }
}

/// Bounded in-memory store (ringEviction by insertion order).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryStore {
    entries: Vec<HistoryEntry>,
    next_id: u64,
    #[serde(default = "default_capacity")]
    capacity: usize,
    /// File-format version (always [`HISTORY_FORMAT_VERSION`]).
    #[serde(default = "default_format_version")]
    format_version: u32,
}

fn default_capacity() -> usize {
    10_000
}

fn default_format_version() -> u32 {
    HISTORY_FORMAT_VERSION
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryStore {
    /// Empty store with default capacity (10 000 entries).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    /// Empty store with explicit capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
            capacity,
            format_version: HISTORY_FORMAT_VERSION,
        }
    }

    /// Format version of this payload.
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Record a calculation, returning the stored entry.
    pub fn push(&mut self, expression: &str, result: &str) -> HistoryEntry {
        self.push_with_duration(expression, result, None)
    }

    /// Record with an optional evaluation duration.
    pub fn push_with_duration(
        &mut self,
        expression: &str,
        result: &str,
        duration_us: Option<u64>,
    ) -> HistoryEntry {
        let mut entry = HistoryEntry::new(self.next_id, expression, result);
        entry.duration_us = duration_us;
        self.next_id += 1;
        self.entries.push(entry.clone());
        // Ring eviction without O(n) `remove(0)` per push: drain in bulk.
        if self.entries.len() > self.capacity {
            let excess = self.entries.len() - self.capacity;
            self.entries.drain(..excess);
        }
        entry
    }

    /// All entries, oldest first.
    #[must_use]
    pub fn all(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all entries (keeps `next_id` monotonic).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Most recent `n` entries, newest first.
    #[must_use]
    pub fn last(&self, n: usize) -> Vec<HistoryEntry> {
        self.entries.iter().rev().take(n).cloned().collect()
    }

    /// Case-insensitive substring search over expression + result.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<HistoryEntry> {
        let q = query.to_ascii_lowercase();
        // Pre-lowercase the query once (not per entryfield twice).
        self.entries
            .iter()
            .filter(|e| {
                e.expression.to_ascii_lowercase().contains(&q)
                    || e.result.to_ascii_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    /// Serialise to versioned pretty JSON (`{"format_version":1,…}`).
    pub fn to_json(&self) -> Result<String, HistoryError> {
        #[derive(Serialize)]
        struct Envelope<'a> {
            format_version: u32,
            capacity: usize,
            next_id: u64,
            entries: &'a [HistoryEntry],
        }
        serde_json::to_string_pretty(&Envelope {
            format_version: HISTORY_FORMAT_VERSION,
            capacity: self.capacity,
            next_id: self.next_id,
            entries: &self.entries,
        })
        .map_err(|e| HistoryError::Storage(e.to_string()))
    }

    /// Deserialise versioned JSON (accepts both v1 envelopes and legacy bare
    /// stores without `format_version` for backward compatibility).
    pub fn from_json(s: &str) -> Result<Self, HistoryError> {
        let v: serde_json::Value =
            serde_json::from_str(s).map_err(|e| HistoryError::Storage(e.to_string()))?;
        if let Some(ver) = v.get("format_version").and_then(|n| n.as_u64()) {
            if ver != u64::from(HISTORY_FORMAT_VERSION) {
                return Err(HistoryError::Storage(format!(
                    "unsupported history format version {ver}, expected {}",
                    HISTORY_FORMAT_VERSION
                )));
            }
            let capacity = v.get("capacity").and_then(|n| n.as_u64()).unwrap_or(10_000) as usize;
            let next_id = v.get("next_id").and_then(|n| n.as_u64()).unwrap_or(1);
            let entries: Vec<HistoryEntry> = v
                .get("entries")
                .map(|e| serde_json::from_value(e.clone()))
                .transpose()
                .map_err(|e: serde_json::Error| HistoryError::Storage(e.to_string()))?
                .unwrap_or_default();
            let mut store = Self {
                entries,
                next_id,
                capacity,
                format_version: HISTORY_FORMAT_VERSION,
            };
            // Enforce capacity on load (a hand-edited file may overfill).
            if store.entries.len() > store.capacity {
                let excess = store.entries.len() - store.capacity;
                store.entries.drain(..excess);
            }
            // Repair monotonic ids when the file was hand-edited.
            let max_id = store.entries.iter().map(|e| e.id).max().unwrap_or(0);
            if store.next_id <= max_id {
                store.next_id = max_id + 1;
            }
            return Ok(store);
        }
        // Legacy bare store: {"entries":…,"next_id":…,"capacity":…}.
        let mut store: Self =
            serde_json::from_value(v).map_err(|e| HistoryError::Storage(e.to_string()))?;
        store.format_version = HISTORY_FORMAT_VERSION;
        Ok(store)
    }

    /// Persist atomically: write to a temp sibling then rename, so crashes
    /// never leave a half-written history file.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), HistoryError> {
        let json = self.to_json()?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(HistoryError::from)?;
            }
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes()).map_err(HistoryError::from)?;
        std::fs::rename(&tmp, path).map_err(HistoryError::from)?;
        Ok(())
    }

    /// Load from a file (version-checked).
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, HistoryError> {
        let s = std::fs::read_to_string(path).map_err(HistoryError::from)?;
        Self::from_json(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_search_last() {
        let mut h = HistoryStore::with_capacity(10);
        h.push("1+1", "2");
        h.push("sin(0)", "0");
        assert_eq!(h.len(), 2);
        assert_eq!(h.search("sin").len(), 1);
        assert_eq!(h.search("2").len(), 1);
        assert_eq!(h.last(1)[0].expression, "sin(0)");
    }

    #[test]
    fn capacity_evicts_oldest() {
        let mut h = HistoryStore::with_capacity(2);
        h.push("1", "1");
        h.push("2", "2");
        h.push("3", "3");
        assert_eq!(h.len(), 2);
        assert_eq!(h.all()[0].expression, "2");
    }

    #[test]
    fn json_roundtrip() {
        let mut h = HistoryStore::new();
        h.push("2*3", "6");
        let j = h.to_json().unwrap();
        assert!(j.contains("format_version"));
        let back = HistoryStore::from_json(&j).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back.all()[0].result, "6");
        assert_eq!(back.format_version(), HISTORY_FORMAT_VERSION);
    }

    #[test]
    fn legacy_bare_json_still_loads() {
        let legacy = r#"{"entries":[],"next_id":1,"capacity":100}"#;
        let back = HistoryStore::from_json(legacy).unwrap();
        assert_eq!(back.format_version(), HISTORY_FORMAT_VERSION);
    }

    #[test]
    fn rejects_future_version() {
        let future = r#"{"format_version":99,"entries":[],"next_id":1,"capacity":10}"#;
        assert!(HistoryStore::from_json(future).is_err());
    }

    #[test]
    fn clear_keeps_ids_monotonic() {
        let mut h = HistoryStore::new();
        h.push("1", "1");
        h.clear();
        assert!(h.is_empty());
        let e = h.push("2", "2");
        assert_eq!(e.id, 2);
    }
}
