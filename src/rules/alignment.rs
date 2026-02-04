use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

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
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Alignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lawful" | "l" => Ok(Alignment::Lawful),
            "neutral" | "n" => Ok(Alignment::Neutral),
            "chaotic" | "c" => Ok(Alignment::Chaotic),
            _ => Err(format!("invalid alignment '{}': must be Lawful, Neutral, or Chaotic", s)),
        }
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
}
