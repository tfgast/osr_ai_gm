use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Top-level game system manifest parsed from game.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct GameManifest {
    pub game: GameInfo,
    pub paths: ManifestPaths,
    pub rules: RulesConfig,
    pub data: HashMap<String, String>,
    pub mechanics: MechanicsConfig,
    #[serde(default)]
    pub features: FeaturesConfig,
}

/// Core game system metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct GameInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    pub system_id: String,
}

/// Relative directory paths within the game system folder.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestPaths {
    pub rules_dir: String,
    pub data_dir: String,
}

/// DSL rule file configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RulesConfig {
    pub files: Vec<String>,
}

/// Supported mechanic groups.
#[derive(Debug, Clone, Deserialize)]
pub struct MechanicsConfig {
    pub supported: Vec<String>,
}

/// Optional feature flags for the game system.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FeaturesConfig {
    #[serde(default)]
    pub requires_dsl: bool,
}

impl GameManifest {
    /// Load a game manifest from the given game system directory.
    /// Expects `<game_dir>/game.toml` to exist.
    pub fn load(game_dir: &Path) -> Result<Self, String> {
        let manifest_path = game_dir.join("game.toml");
        let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
            format!(
                "Failed to read game manifest '{}': {}",
                manifest_path.display(),
                e
            )
        })?;
        Self::parse(&content)
    }

    /// Parse a manifest from TOML string content.
    pub fn parse(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse game manifest: {}", e))
    }

    /// Resolve the absolute rules directory path relative to a game system directory.
    pub fn rules_dir(&self, game_dir: &Path) -> PathBuf {
        game_dir.join(&self.paths.rules_dir)
    }

    /// Resolve the absolute data directory path relative to a game system directory.
    pub fn data_dir(&self, game_dir: &Path) -> PathBuf {
        game_dir.join(&self.paths.data_dir)
    }

    /// Resolve a data file path by key (e.g. "spells" -> "<game_dir>/data/spells.json").
    pub fn data_file(&self, game_dir: &Path, key: &str) -> Option<PathBuf> {
        self.data
            .get(key)
            .map(|filename| self.data_dir(game_dir).join(filename))
    }

    /// Resolve all rule file paths relative to a game system directory.
    pub fn rules_files(&self, game_dir: &Path) -> Vec<PathBuf> {
        let rules = self.rules_dir(game_dir);
        self.rules.files.iter().map(|f| rules.join(f)).collect()
    }

    /// Check if a mechanic group name is supported by this game system.
    pub fn supports_mechanic(&self, mechanic: &str) -> bool {
        self.mechanics.supported.iter().any(|m| m == mechanic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const SAMPLE_MANIFEST: &str = r#"
[game]
name = "Old-School Essentials"
version = "1.0.0"
description = "Classic fantasy adventure game"
author = "Necrotic Gnome"
system_id = "ose"

[paths]
rules_dir = "rules"
data_dir = "data"

[rules]
files = ["ose_combat.ttrpg", "ose_saves.ttrpg"]

[data]
spells = "spells.json"
monsters = "monsters.json"

[mechanics]
supported = ["combat", "saves", "ability"]

[features]
requires_dsl = true
"#;

    #[test]
    fn parse_manifest() {
        let manifest = GameManifest::parse(SAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.game.name, "Old-School Essentials");
        assert_eq!(manifest.game.version, "1.0.0");
        assert_eq!(manifest.game.system_id, "ose");
        assert_eq!(manifest.game.author, "Necrotic Gnome");
        assert_eq!(manifest.paths.rules_dir, "rules");
        assert_eq!(manifest.paths.data_dir, "data");
        assert_eq!(manifest.rules.files.len(), 2);
        assert_eq!(manifest.data.len(), 2);
        assert_eq!(manifest.mechanics.supported.len(), 3);
        assert!(manifest.features.requires_dsl);
    }

    #[test]
    fn path_resolution() {
        let manifest = GameManifest::parse(SAMPLE_MANIFEST).unwrap();
        let game_dir = Path::new("data/games/ose");

        assert_eq!(manifest.rules_dir(game_dir), Path::new("data/games/ose/rules"));
        assert_eq!(manifest.data_dir(game_dir), Path::new("data/games/ose/data"));
        assert_eq!(
            manifest.data_file(game_dir, "spells"),
            Some(PathBuf::from("data/games/ose/data/spells.json"))
        );
        assert_eq!(manifest.data_file(game_dir, "nonexistent"), None);
    }

    #[test]
    fn rules_files_resolution() {
        let manifest = GameManifest::parse(SAMPLE_MANIFEST).unwrap();
        let game_dir = Path::new("data/games/ose");
        let files = manifest.rules_files(game_dir);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], Path::new("data/games/ose/rules/ose_combat.ttrpg"));
        assert_eq!(files[1], Path::new("data/games/ose/rules/ose_saves.ttrpg"));
    }

    #[test]
    fn supports_mechanic() {
        let manifest = GameManifest::parse(SAMPLE_MANIFEST).unwrap();
        assert!(manifest.supports_mechanic("combat"));
        assert!(manifest.supports_mechanic("saves"));
        assert!(!manifest.supports_mechanic("thief"));
    }

    #[test]
    fn load_ose_manifest() {
        let manifest = GameManifest::load(Path::new("data/games/ose")).unwrap();
        assert_eq!(manifest.game.system_id, "ose");
        assert_eq!(manifest.game.name, "Old-School Essentials");
        assert_eq!(manifest.rules.files.len(), 7);
        assert_eq!(manifest.data.len(), 9);
        assert_eq!(manifest.mechanics.supported.len(), 9);
    }

    #[test]
    fn features_default() {
        let minimal = r#"
[game]
name = "Test"
version = "0.1.0"
system_id = "test"

[paths]
rules_dir = "rules"
data_dir = "data"

[rules]
files = []

[data]

[mechanics]
supported = []
"#;
        let manifest = GameManifest::parse(minimal).unwrap();
        assert!(!manifest.features.requires_dsl);
        assert_eq!(manifest.game.description, "");
        assert_eq!(manifest.game.author, "");
    }
}
