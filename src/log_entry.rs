//! Monotonically-sequenced log entry for chronological event ordering.
//!
//! Each subsystem (combat, dungeon, wilderness, time) stores its own
//! `Vec<LogEntry>`.  The `seq` field is a globally-monotonic counter so the
//! companion TUI can merge entries from all subsystems and sort them into
//! true chronological order.
//!
//! Serialisation is backward-compatible: old save files store plain strings
//! and are deserialised with `seq = 0`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A single log entry carrying a monotonic sequence number.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub message: String,
}

impl LogEntry {
    pub fn new(seq: u64, message: String) -> Self {
        Self { seq, message }
    }
}

// ── Ergonomic impls ─────────────────────────────────────────────────────
//
// Deref<Target=str> lets existing code call str methods on LogEntry
// references (`.contains()`, `.starts_with()`, etc.) without changes.

impl std::ops::Deref for LogEntry {
    type Target = str;
    fn deref(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl PartialEq<str> for LogEntry {
    fn eq(&self, other: &str) -> bool {
        self.message == other
    }
}

impl PartialEq<&str> for LogEntry {
    fn eq(&self, other: &&str) -> bool {
        self.message == *other
    }
}

impl PartialEq<String> for LogEntry {
    fn eq(&self, other: &String) -> bool {
        self.message == *other
    }
}

// ── Backward-compatible deserialisation ─────────────────────────────────
//
// Old saves: `"log": ["entry1", "entry2"]`  → seq = 0
// New saves: `"log": [{"seq": 1, "message": "entry1"}, ...]`

impl<'de> Deserialize<'de> for LogEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = LogEntry;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a string or {{\"seq\", \"message\"}} object")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<LogEntry, E> {
                Ok(LogEntry { seq: 0, message: v.to_string() })
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<LogEntry, E> {
                Ok(LogEntry { seq: 0, message: v })
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<LogEntry, A::Error> {
                let mut seq = None;
                let mut message = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "seq" => seq = Some(map.next_value()?),
                        "message" => message = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(LogEntry {
                    seq: seq.unwrap_or(0),
                    message: message
                        .ok_or_else(|| serde::de::Error::missing_field("message"))?,
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deref_contains() {
        let e = LogEntry::new(1, "Grond attacks Goblin".into());
        assert!(e.contains("Grond"));
        assert!(e.starts_with("Grond"));
    }

    #[test]
    fn display() {
        let e = LogEntry::new(42, "hello".into());
        assert_eq!(format!("{}", e), "hello");
    }

    #[test]
    fn deserialize_from_string() {
        let json = r#""Fighter attacks""#;
        let e: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.seq, 0);
        assert_eq!(e.message, "Fighter attacks");
    }

    #[test]
    fn deserialize_from_object() {
        let json = r#"{"seq": 5, "message": "Combat begins"}"#;
        let e: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.seq, 5);
        assert_eq!(e.message, "Combat begins");
    }

    #[test]
    fn serialize_roundtrip() {
        let e = LogEntry::new(7, "test".into());
        let json = serde_json::to_string(&e).unwrap();
        let e2: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(e2.seq, 7);
        assert_eq!(e2.message, "test");
    }

    #[test]
    fn deserialize_vec_of_strings() {
        let json = r#"["a", "b", "c"]"#;
        let entries: Vec<LogEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "a");
        assert_eq!(entries[0].seq, 0);
    }

    #[test]
    fn partial_eq_str() {
        let e = LogEntry::new(1, "hello".into());
        assert!(e == *"hello");
        assert!(e == "hello".to_string());
    }
}
