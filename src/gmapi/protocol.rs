use serde::{Deserialize, Serialize};
use crate::rules::alignment::Alignment;
use crate::rules::attack::HitDice;
use crate::rules::class::Class;
use crate::state::dungeon::DoorState;
use crate::state::game::GameMode;
use crate::state::time::LightSourceKind;
use crate::state::wilderness::Terrain;

// =============================================================================
// GM Request — what the LLM sends to the game engine
// =============================================================================

/// A request from the GM (or player) to the game engine, sent as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GMRequest {
    /// Unique request ID for correlating responses.
    pub id: String,
    /// The command to execute.
    pub command: GMCommand,
}

/// All commands an AI GM can issue through the JSON protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum GMCommand {
    // -- State queries (read-only) --
    /// Query the current game state.
    QueryState,
    /// Get current game mode.
    QueryMode,
    /// Get party status.
    QueryParty,
    /// Get combat status (if in combat).
    QueryCombat,
    /// Get exploration status (if exploring).
    QueryExploration,
    /// Get wilderness status (if in wilderness).
    QueryWilderness,

    // -- Character management --
    /// Create a character.
    CreateCharacter {
        name: String,
        #[serde(deserialize_with = "deserialize_class")]
        class: Class,
        #[serde(default)]
        alignment: Alignment,
        /// Optional pre-rolled ability scores [STR, INT, WIS, DEX, CON, CHA].
        /// Each score must be 3-18. If omitted, abilities are rolled randomly.
        #[serde(default)]
        abilities: Option<[i32; 6]>,
    },

    // -- GM-only: encounter & combat --
    /// Spawn an encounter with monsters.
    SpawnEncounter {
        name: String,
        count: u32,
        hit_dice: HitDice,
        ac: i32,
        hp: i32,
        damage: String,
        morale: u32,
        distance: u32,
        /// XP per monster. If omitted, auto-looked up from monster database.
        #[serde(default)]
        xp_value: Option<u64>,
    },
    /// Roll initiative for the current combat round.
    RollInitiative,
    /// Perform a character attack.
    Attack {
        character: String,
        monster_idx: usize,
        #[serde(default = "default_weapon")]
        weapon: String,
    },
    /// Monster attacks a character.
    MonsterAttack {
        monster_idx: usize,
        character: String,
    },
    /// Check morale.
    CheckMorale,
    /// Turn undead.
    TurnUndead {
        character: String,
        monster_idx: usize,
    },
    /// Close distance to monsters.
    Close {
        character: String,
        #[serde(default)]
        feet: Option<u32>,
    },
    /// Retreat from combat — full speed, enemies get free attack at +2.
    Retreat {
        character: String,
    },
    /// Fighting withdrawal — half speed, no free attacks.
    FightingWithdrawal {
        character: String,
    },
    /// End the current combat.
    EndCombat,

    // -- GM-only: exploration --
    /// Enter dungeon exploration mode.
    EnterDungeon {
        level: u32,
        #[serde(default = "default_room_name")]
        room_name: String,
    },
    /// Advance one dungeon turn.
    AdvanceTurn,
    /// Add a room to the dungeon.
    AddRoom { id: u32, name: String },
    /// Add a door between rooms.
    AddDoor {
        id: u32,
        room_a: u32,
        room_b: u32,
        #[serde(default)]
        state: DoorState,
    },
    /// Move through a door.
    MoveRoom { door_id: u32 },
    /// Search the current room.
    Search {
        #[serde(default)]
        is_elf: bool,
    },
    /// Light a torch or lantern.
    Light {
        source: LightSourceKind,
        carrier: String,
    },
    /// Load a prewritten adventure module from a JSON file.
    LoadModule { path: String },

    // -- GM-only: wilderness --
    /// Enter wilderness travel mode.
    EnterWilderness {
        #[serde(default)]
        terrain: Terrain,
    },
    /// Add a hex to the wilderness map.
    AddHex {
        x: i32,
        y: i32,
        terrain: Terrain,
    },
    /// Travel to a hex.
    Travel { x: i32, y: i32 },
    /// Attempt to orient when lost (takes a full day).
    Orient,

    // -- GM-only: encounter resolution --
    /// Roll surprise.
    RollSurprise,
    /// Roll NPC reaction using a character's CHA.
    RollReaction { character: String },

    // -- GM-only: management --
    /// Award XP to a character (with prime requisite modifier and level-up check).
    AwardXp {
        character: String,
        xp: u64,
    },
    /// Award treasure XP (1gp = 1xp) and monster XP with level-up check.
    AwardTreasureXp {
        character: String,
        treasure_gp: u64,
        monster_xp: u64,
    },
    /// Check thief skill.
    ThiefSkillCheck {
        character: String,
        skill: String,
    },
    /// Perform a backstab attack.
    Backstab {
        character: String,
        monster_idx: usize,
        #[serde(default = "default_weapon")]
        weapon: String,
    },
    /// Query encumbrance for a character.
    QueryEncumbrance { character: String },
    /// Spawn monsters from the built-in monster database.
    SpawnMonster {
        name: String,
        count: u32,
        #[serde(default = "default_distance")]
        distance: u32,
    },
    /// Look up a spell definition.
    LookupSpell {
        name: String,
        #[serde(default)]
        list: String,
    },
    /// Hire a retainer.
    HireRetainer {
        employer: String,
        retainer_name: String,
        #[serde(deserialize_with = "deserialize_class")]
        retainer_class: Class,
        retainer_level: u32,
    },
    /// Check retainer loyalty.
    LoyaltyCheck {
        retainer_name: String,
        loyalty: u32,
    },
    /// Level up a character (if they have enough XP).
    LevelUp { character: String },
    /// Issue a GM ruling (free-text note recorded in the session log).
    Ruling { text: String },

    // -- System --
    /// Save game state.
    Save {
        #[serde(default = "default_save_path")]
        path: String,
    },
    /// Load game state.
    Load {
        #[serde(default = "default_save_path")]
        path: String,
    },
    /// Roll dice.
    Roll { notation: String },
    /// Quit the session.
    Quit,
}

fn default_weapon() -> String { "sword".to_string() }
fn default_room_name() -> String { "Entrance".to_string() }
fn default_save_path() -> String { "save.json".to_string() }
fn default_distance() -> u32 { 60 }

/// Deserialize a `Class` from a string, using `Class::parse` for flexible matching.
fn deserialize_class<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Class, D::Error> {
    let s = String::deserialize(deserializer)?;
    Class::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown class '{}'", s)))
}

// =============================================================================
// GM Response — what the game engine sends back
// =============================================================================

/// A response from the game engine, sent as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GMResponse {
    /// The request ID this response corresponds to.
    pub id: String,
    /// Whether the command succeeded.
    pub success: bool,
    /// Human-readable message (always present).
    pub message: String,
    /// Current game mode after the command executed.
    pub mode: GameMode,
    /// Structured data payload (command-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error details if success is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl GMResponse {
    pub fn ok(id: &str, message: impl Into<String>, mode: GameMode) -> Self {
        GMResponse {
            id: id.to_string(),
            success: true,
            message: message.into(),
            mode,
            data: None,
            error: None,
        }
    }

    pub fn ok_with_data(id: &str, message: impl Into<String>, mode: GameMode, data: serde_json::Value) -> Self {
        GMResponse {
            id: id.to_string(),
            success: true,
            message: message.into(),
            mode,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(id: &str, error: impl Into<String>, mode: GameMode) -> Self {
        let error_msg = error.into();
        GMResponse {
            id: id.to_string(),
            success: false,
            message: error_msg.clone(),
            mode,
            data: None,
            error: Some(error_msg),
        }
    }
}

// =============================================================================
// Parsing helpers
// =============================================================================

/// Parse a JSON line into a GMRequest.
pub fn parse_request(line: &str) -> Result<GMRequest, String> {
    serde_json::from_str(line).map_err(|e| format!("invalid JSON request: {}", e))
}

/// Serialize a GMResponse to a JSON line.
pub fn serialize_response(response: &GMResponse) -> String {
    // Compact JSON — one line, no pretty-printing.
    serde_json::to_string(response).unwrap_or_else(|e| {
        format!(r#"{{"id":"?","success":false,"message":"serialization error: {}","mode":"idle"}}"#, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_state() {
        let json = r#"{"id":"1","command":{"type":"QueryState"}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "1");
        assert!(matches!(req.command, GMCommand::QueryState));
    }

    #[test]
    fn parse_spawn_encounter() {
        let json = r#"{
            "id": "2",
            "command": {
                "type": "SpawnEncounter",
                "params": {
                    "name": "goblin",
                    "count": 3,
                    "hit_dice": "1",
                    "ac": 6,
                    "hp": 3,
                    "damage": "1d6",
                    "morale": 7,
                    "distance": 60
                }
            }
        }"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "2");
        match &req.command {
            GMCommand::SpawnEncounter { name, count, .. } => {
                assert_eq!(name, "goblin");
                assert_eq!(*count, 3);
            }
            _ => panic!("expected SpawnEncounter"),
        }
    }

    #[test]
    fn parse_create_character_defaults() {
        let json = r#"{"id":"3","command":{"type":"CreateCharacter","params":{"name":"Aldric","class":"Fighter"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::CreateCharacter { name, class, alignment, abilities } => {
                assert_eq!(name, "Aldric");
                assert_eq!(*class, Class::Fighter);
                assert_eq!(*alignment, Alignment::default()); // default
                assert!(abilities.is_none()); // default
            }
            _ => panic!("expected CreateCharacter"),
        }
    }

    #[test]
    fn parse_create_character_with_abilities() {
        let json = r#"{"id":"4","command":{"type":"CreateCharacter","params":{"name":"Hoyret","class":"Ranger","alignment":"Neutral","abilities":[12,13,11,12,5,6]}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::CreateCharacter { name, class, alignment, abilities } => {
                assert_eq!(name, "Hoyret");
                assert_eq!(*class, Class::Ranger);
                assert_eq!(*alignment, Alignment::Neutral);
                assert_eq!(*abilities, Some([12, 13, 11, 12, 5, 6]));
            }
            _ => panic!("expected CreateCharacter"),
        }
    }

    #[test]
    fn parse_attack() {
        let json = r#"{"id":"4","command":{"type":"Attack","params":{"character":"Aldric","monster_idx":0}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Attack { character, monster_idx, weapon } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*monster_idx, 0);
                assert_eq!(weapon, "sword"); // default
            }
            _ => panic!("expected Attack"),
        }
    }

    #[test]
    fn parse_award_xp() {
        let json = r#"{"id":"5","command":{"type":"AwardXp","params":{"character":"Aldric","xp":500}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::AwardXp { character, xp } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*xp, 500);
            }
            _ => panic!("expected AwardXp"),
        }
    }

    #[test]
    fn parse_ruling() {
        let json = r#"{"id":"6","command":{"type":"Ruling","params":{"text":"The portcullis is too heavy to lift without a lever."}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Ruling { text } => {
                assert!(text.contains("portcullis"));
            }
            _ => panic!("expected Ruling"),
        }
    }

    #[test]
    fn parse_roll() {
        let json = r#"{"id":"7","command":{"type":"Roll","params":{"notation":"2d6+3"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Roll { notation } => {
                assert_eq!(notation, "2d6+3");
            }
            _ => panic!("expected Roll"),
        }
    }

    #[test]
    fn parse_invalid_json() {
        let result = parse_request("not json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid JSON request"));
    }

    #[test]
    fn response_ok() {
        let r = GMResponse::ok("1", "all good", GameMode::Idle);
        assert!(r.success);
        assert_eq!(r.id, "1");
        assert!(r.error.is_none());
    }

    #[test]
    fn response_ok_with_data() {
        let data = serde_json::json!({"party_size": 3});
        let r = GMResponse::ok_with_data("2", "party info", GameMode::Exploration, data.clone());
        assert!(r.success);
        assert_eq!(r.data.unwrap(), data);
    }

    #[test]
    fn response_error() {
        let r = GMResponse::err("3", "something broke", GameMode::Combat);
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some("something broke"));
    }

    #[test]
    fn serialize_response_compact() {
        let r = GMResponse::ok("1", "ok", GameMode::Idle);
        let json = serialize_response(&r);
        assert!(!json.contains('\n'));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn parse_close() {
        let json = r#"{"id":"c1","command":{"type":"Close","params":{"character":"Aldric"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Close { character, feet } => {
                assert_eq!(character, "Aldric");
                assert!(feet.is_none());
            }
            _ => panic!("expected Close"),
        }
    }

    #[test]
    fn parse_close_with_feet() {
        let json = r#"{"id":"c2","command":{"type":"Close","params":{"character":"Aldric","feet":30}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Close { character, feet } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*feet, Some(30));
            }
            _ => panic!("expected Close"),
        }
    }

    #[test]
    fn request_roundtrip() {
        let req = GMRequest {
            id: "rt-1".to_string(),
            command: GMCommand::QueryState,
        };
        let json = serde_json::to_string(&req).unwrap();
        let req2: GMRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.id, req2.id);
        assert!(matches!(req2.command, GMCommand::QueryState));
    }

    #[test]
    fn response_roundtrip() {
        let resp = GMResponse::ok_with_data(
            "rt-2",
            "test",
            GameMode::Combat,
            serde_json::json!({"round": 3}),
        );
        let json = serde_json::to_string(&resp).unwrap();
        let resp2: GMResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.id, resp2.id);
        assert_eq!(resp.mode, resp2.mode);
        assert!(resp2.success);
    }

    #[test]
    fn all_query_commands_parse() {
        let commands = [
            r#"{"id":"q1","command":{"type":"QueryState"}}"#,
            r#"{"id":"q2","command":{"type":"QueryMode"}}"#,
            r#"{"id":"q3","command":{"type":"QueryParty"}}"#,
            r#"{"id":"q4","command":{"type":"QueryCombat"}}"#,
            r#"{"id":"q5","command":{"type":"QueryExploration"}}"#,
            r#"{"id":"q6","command":{"type":"QueryWilderness"}}"#,
        ];
        for json in &commands {
            assert!(parse_request(json).is_ok(), "failed to parse: {}", json);
        }
    }

    #[test]
    fn all_system_commands_parse() {
        let commands = [
            r#"{"id":"s1","command":{"type":"Save","params":{"path":"test.json"}}}"#,
            r#"{"id":"s2","command":{"type":"Load","params":{"path":"test.json"}}}"#,
            r#"{"id":"s3","command":{"type":"Quit"}}"#,
        ];
        for json in &commands {
            assert!(parse_request(json).is_ok(), "failed to parse: {}", json);
        }
    }

    #[test]
    fn parse_load_module() {
        let json = r#"{"id":"m1","command":{"type":"LoadModule","params":{"path":"data/modules/sample_crypt/module.json"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "m1");
        match &req.command {
            GMCommand::LoadModule { path } => {
                assert_eq!(path, "data/modules/sample_crypt/module.json");
            }
            _ => panic!("expected LoadModule"),
        }
    }
}
