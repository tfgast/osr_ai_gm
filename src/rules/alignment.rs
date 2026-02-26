use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use serde::{Deserialize, Deserializer, Serialize};

/// The three alignments in B/X D&D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Alignment {
    #[serde(alias = "lawful", alias = "L", alias = "l")]
    Lawful,
    #[default]
    #[serde(alias = "neutral", alias = "N", alias = "n")]
    Neutral,
    #[serde(alias = "chaotic", alias = "C", alias = "c")]
    Chaotic,
}

impl Alignment {
    pub fn name(self) -> &'static str {
        match self {
            Alignment::Lawful => "Lawful",
            Alignment::Neutral => "Neutral",
            Alignment::Chaotic => "Chaotic",
        }
    }

    /// Parse alignment name (case-insensitive, accepts common variants).
    pub fn parse(s: &str) -> Option<Alignment> {
        match s.to_lowercase().as_str() {
            "lawful" | "l" => Some(Alignment::Lawful),
            "neutral" | "n" => Some(Alignment::Neutral),
            "chaotic" | "c" => Some(Alignment::Chaotic),
            _ => None,
        }
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Alignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Alignment::parse(s).ok_or_else(|| {
            format!("invalid alignment '{}': must be Lawful, Neutral, or Chaotic", s)
        })
    }
}

// ── AlignmentId: string-based alignment identifier ──────────

/// String-based alignment identifier for data-driven alignment lookups.
/// Wraps `Arc<str>` for O(1) clone. Canonical form is the display name
/// (e.g., "Lawful", "Neutral", "Chaotic").
/// Serializes transparently as a plain string for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct AlignmentId(Arc<str>);

impl<'de> Deserialize<'de> for AlignmentId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(AlignmentId::new(&s))
    }
}

impl AlignmentId {
    /// Create an AlignmentId, normalizing to canonical form.
    /// Known alignments are normalized via `Alignment::parse()`.
    /// Unknown names (homebrew alignments) are stored as-is.
    pub fn new(s: &str) -> Self {
        if let Some(alignment) = Alignment::parse(s) {
            AlignmentId(Arc::from(alignment.name()))
        } else {
            AlignmentId(Arc::from(s))
        }
    }

    /// Create an AlignmentId from an `Alignment` enum variant.
    pub fn from_enum(alignment: Alignment) -> Self {
        AlignmentId(Arc::from(alignment.name()))
    }

    /// Try to resolve this AlignmentId back to an `Alignment` enum variant.
    /// Returns `None` for homebrew alignments not in the core 3.
    pub fn to_enum(&self) -> Option<Alignment> {
        Alignment::parse(&self.0)
    }

    /// The canonical string form of this alignment identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Display name for this alignment (same as as_str).
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl Default for AlignmentId {
    fn default() -> Self {
        AlignmentId::from_enum(Alignment::default())
    }
}

impl fmt::Display for AlignmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Alignment> for AlignmentId {
    fn from(alignment: Alignment) -> Self {
        AlignmentId::from_enum(alignment)
    }
}

impl From<&str> for AlignmentId {
    fn from(s: &str) -> Self {
        AlignmentId::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_canonical() {
        assert_eq!("Lawful".parse::<Alignment>().unwrap(), Alignment::Lawful);
        assert_eq!("Neutral".parse::<Alignment>().unwrap(), Alignment::Neutral);
        assert_eq!("Chaotic".parse::<Alignment>().unwrap(), Alignment::Chaotic);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!("lawful".parse::<Alignment>().unwrap(), Alignment::Lawful);
        assert_eq!("NEUTRAL".parse::<Alignment>().unwrap(), Alignment::Neutral);
    }

    #[test]
    fn parse_shortcuts() {
        assert_eq!("L".parse::<Alignment>().unwrap(), Alignment::Lawful);
        assert_eq!("n".parse::<Alignment>().unwrap(), Alignment::Neutral);
        assert_eq!("C".parse::<Alignment>().unwrap(), Alignment::Chaotic);
    }

    #[test]
    fn parse_invalid() {
        assert!("evil".parse::<Alignment>().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let a = Alignment::Lawful;
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "\"Lawful\"");
        let parsed: Alignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, a);
    }

    #[test]
    fn serde_alias() {
        let a: Alignment = serde_json::from_str("\"lawful\"").unwrap();
        assert_eq!(a, Alignment::Lawful);
        let b: Alignment = serde_json::from_str("\"L\"").unwrap();
        assert_eq!(b, Alignment::Lawful);
    }

    // ── AlignmentId tests ──

    #[test]
    fn alignment_id_from_enum() {
        let id = AlignmentId::from_enum(Alignment::Lawful);
        assert_eq!(id.as_str(), "Lawful");
        assert_eq!(id.to_enum(), Some(Alignment::Lawful));
    }

    #[test]
    fn alignment_id_new_normalizes() {
        let id = AlignmentId::new("lawful");
        assert_eq!(id.as_str(), "Lawful");
        assert_eq!(id.to_enum(), Some(Alignment::Lawful));
    }

    #[test]
    fn alignment_id_unknown_stored_as_is() {
        let id = AlignmentId::new("Good");
        assert_eq!(id.as_str(), "Good");
        assert_eq!(id.to_enum(), None);
    }

    #[test]
    fn alignment_id_default() {
        let id = AlignmentId::default();
        assert_eq!(id.as_str(), "Neutral");
        assert_eq!(id.to_enum(), Some(Alignment::Neutral));
    }

    #[test]
    fn alignment_id_serde_roundtrip() {
        let id = AlignmentId::from_enum(Alignment::Chaotic);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"Chaotic\"");
        let parsed: AlignmentId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn alignment_id_from_trait() {
        let id: AlignmentId = Alignment::Lawful.into();
        assert_eq!(id.as_str(), "Lawful");
        let id2: AlignmentId = "chaotic".into();
        assert_eq!(id2.as_str(), "Chaotic");
    }

    #[test]
    fn alignment_id_backward_compat() {
        // Old saves stored alignment as "Lawful", "Neutral", "Chaotic"
        let id: AlignmentId = serde_json::from_str("\"Lawful\"").unwrap();
        assert_eq!(id.to_enum(), Some(Alignment::Lawful));
    }
}
