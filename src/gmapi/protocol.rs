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

/// Parameters for spawning a custom encounter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterParams {
    pub name: String,
    pub count: u32,
    pub hit_dice: HitDice,
    pub ac: i32,
    pub hp: i32,
    pub damage: String,
    pub morale: u32,
    pub distance: u32,
    /// XP per monster. If omitted, auto-looked up from monster database.
    #[serde(default)]
    pub xp_value: Option<u64>,
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
    SpawnEncounter(EncounterParams),
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
    /// Query the combat log for the current encounter.
    QueryCombatLog,
    /// Declare spell casting for a round (spell disrupted if caster takes damage).
    DeclareSpell {
        character: String,
        spell: String,
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
    /// Open a door (force if stuck/closed) and move through it.
    OpenDoor { door_id: u32 },
    /// Force open a stuck or closed door using a party member.
    ForceDoor { door_id: u32, character: String },
    /// Listen at a door (1-in-6, demihumans 2-in-6). Takes one turn.
    Listen {
        #[serde(default)]
        is_demihuman: bool,
    },
    /// Rest for one turn (resets activity counter).
    Rest,

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
    /// Forage for food in the current hex (takes a full day).
    Forage,
    /// Hunt for game in the current hex (takes a full day).
    Hunt,
    /// Roll a full encounter from tables (type + number + surprise + distance).
    RollEncounter,
    /// Attempt to evade an encounter.
    Evade {
        monster_count: u32,
        monster_movement: u32,
    },

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
    /// Spawn a random NPC adventuring party.
    SpawnNpcParty {
        party_type: String,
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
    /// List all session notes.
    ListNotes,
    /// Delete a note by 1-based index.
    DeleteNote { index: usize },
    /// List all retainers.
    ListRetainers,
    /// Dismiss a retainer by name.
    DismissRetainer { name: String },

    // -- GM-only: fiat commands (direct state manipulation) --
    /// Heal a character (capped at max HP).
    Heal { character: String, amount: i32 },
    /// Damage a character (can kill).
    Damage { character: String, amount: i32 },
    /// Set a character's HP directly.
    SetHp { character: String, hp: i32 },
    /// Mark a monster as helpless (sleeping, paralyzed, held, etc.).
    SetHelpless { monster_idx: usize, #[serde(default = "default_helpless")] helpless: bool },
    /// Auto-kill a helpless monster.
    Kill { character: String, monster_idx: usize },
    /// Set party rations to a fixed amount.
    SetRations { amount: u32 },
    /// Add rations to the party supply.
    AddRations { amount: u32 },

    // -- Inventory management --
    /// Buy equipment from tables.
    Buy {
        character: String,
        item_name: String,
    },
    /// Drop an item from inventory.
    Drop {
        character: String,
        item_name: String,
    },
    /// Equip or unequip an item (toggles equipped state, recalculates AC).
    Equip {
        character: String,
        item_name: String,
    },
    /// Loot treasure from current room (or free-form if no dungeon).
    Loot {
        character: String,
        item_name: String,
        #[serde(default)]
        value_gp: Option<u32>,
    },

    // -- Inventory queries --
    /// List all available equipment for purchase (weapons, armour, gear, ammunition).
    ListEquipment {
        /// Optional category filter: "weapons", "armour", "gear", or "ammunition".
        #[serde(default)]
        category: Option<String>,
    },

    // -- Lookup & reference --
    /// Look up a magic item by name.
    LookupItem { name: String },
    /// Search magic items by keyword.
    SearchItems { query: String },
    /// Look up a treasure type table by letter (A-V).
    LookupTreasureType { letter: String },
    /// Roll on a treasure type table to generate treasure.
    RollTreasure { letter: String },
    /// List all character classes with requirements.
    ListClasses,
    /// Show eligible classes for given ability scores.
    EligibleClasses { abilities: [i32; 6] },

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

fn default_helpless() -> bool { true }
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

/// Maximum request size in bytes (64KB).
const MAX_REQUEST_SIZE: usize = 65_536;

/// Parse a JSON line into a GMRequest, with size and field validation.
pub fn parse_request(line: &str) -> Result<GMRequest, String> {
    if line.len() > MAX_REQUEST_SIZE {
        return Err(format!("request too large: {} bytes (max {})", line.len(), MAX_REQUEST_SIZE));
    }
    let req: GMRequest = serde_json::from_str(line).map_err(|e| format!("invalid JSON request: {}", e))?;
    req.validate()?;
    Ok(req)
}

fn check_len(field: &str, value: &str, max: usize) -> Result<(), String> {
    if value.len() > max {
        Err(format!("{} too long ({} chars, max {})", field, value.len(), max))
    } else {
        Ok(())
    }
}

fn check_count(field: &str, value: u32, max: u32) -> Result<(), String> {
    if value > max {
        Err(format!("{} too large ({}, max {})", field, value, max))
    } else {
        Ok(())
    }
}

impl GMRequest {
    fn validate(&self) -> Result<(), String> {
        check_len("id", &self.id, 64)?;
        self.command.validate()
    }
}

impl GMCommand {
    fn validate(&self) -> Result<(), String> {
        match self {
            GMCommand::CreateCharacter { name, .. } => check_len("name", name, 128),
            GMCommand::SpawnEncounter(p) => {
                check_len("name", &p.name, 128)?;
                check_len("damage", &p.damage, 128)?;
                check_count("count", p.count, 100)
            }
            GMCommand::Attack { character, weapon, .. } => {
                check_len("character", character, 128)?;
                check_len("weapon", weapon, 128)
            }
            GMCommand::MonsterAttack { character, .. } => check_len("character", character, 128),
            GMCommand::TurnUndead { character, .. } => check_len("character", character, 128),
            GMCommand::Close { character, .. } => check_len("character", character, 128),
            GMCommand::Retreat { character } => check_len("character", character, 128),
            GMCommand::FightingWithdrawal { character } => check_len("character", character, 128),
            GMCommand::DeclareSpell { character, spell } => {
                check_len("character", character, 128)?;
                check_len("spell", spell, 128)
            }
            GMCommand::EnterDungeon { room_name, .. } => check_len("room_name", room_name, 128),
            GMCommand::AddRoom { name, .. } => check_len("name", name, 128),
            GMCommand::Light { carrier, .. } => check_len("carrier", carrier, 128),
            GMCommand::LoadModule { path } => check_len("path", path, 512),
            GMCommand::ForceDoor { character, .. } => check_len("character", character, 128),
            GMCommand::AwardXp { character, .. } => check_len("character", character, 128),
            GMCommand::AwardTreasureXp { character, .. } => check_len("character", character, 128),
            GMCommand::ThiefSkillCheck { character, skill } => {
                check_len("character", character, 128)?;
                check_len("skill", skill, 128)
            }
            GMCommand::Backstab { character, weapon, .. } => {
                check_len("character", character, 128)?;
                check_len("weapon", weapon, 128)
            }
            GMCommand::QueryEncumbrance { character } => check_len("character", character, 128),
            GMCommand::SpawnMonster { name, count, .. } => {
                check_len("name", name, 128)?;
                check_count("count", *count, 100)
            }
            GMCommand::SpawnNpcParty { party_type, .. } => check_len("party_type", party_type, 128),
            GMCommand::LookupSpell { name, list } => {
                check_len("name", name, 128)?;
                check_len("list", list, 128)
            }
            GMCommand::HireRetainer { employer, retainer_name, .. } => {
                check_len("employer", employer, 128)?;
                check_len("retainer_name", retainer_name, 128)
            }
            GMCommand::LoyaltyCheck { retainer_name, .. } => check_len("retainer_name", retainer_name, 128),
            GMCommand::LevelUp { character } => check_len("character", character, 128),
            GMCommand::Ruling { text } => check_len("text", text, 4096),
            GMCommand::DismissRetainer { name } => check_len("name", name, 128),
            GMCommand::Heal { character, .. } => check_len("character", character, 128),
            GMCommand::Damage { character, .. } => check_len("character", character, 128),
            GMCommand::SetHp { character, .. } => check_len("character", character, 128),
            GMCommand::Kill { character, .. } => check_len("character", character, 128),
            GMCommand::Buy { character, item_name } => {
                check_len("character", character, 128)?;
                check_len("item_name", item_name, 128)
            }
            GMCommand::Drop { character, item_name } => {
                check_len("character", character, 128)?;
                check_len("item_name", item_name, 128)
            }
            GMCommand::Equip { character, item_name } => {
                check_len("character", character, 128)?;
                check_len("item_name", item_name, 128)
            }
            GMCommand::Loot { character, item_name, .. } => {
                check_len("character", character, 128)?;
                check_len("item_name", item_name, 128)
            }
            GMCommand::ListEquipment { category } => {
                if let Some(c) = category {
                    check_len("category", c, 128)
                } else {
                    Ok(())
                }
            }
            GMCommand::LookupItem { name } => check_len("name", name, 128),
            GMCommand::SearchItems { query } => check_len("query", query, 128),
            GMCommand::LookupTreasureType { letter } => check_len("letter", letter, 16),
            GMCommand::RollTreasure { letter } => check_len("letter", letter, 16),
            GMCommand::RollReaction { character } => check_len("character", character, 128),
            GMCommand::Save { path } => check_len("path", path, 512),
            GMCommand::Load { path } => check_len("path", path, 512),
            GMCommand::Roll { notation } => check_len("notation", notation, 128),
            _ => Ok(()),
        }
    }
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
            GMCommand::SpawnEncounter(params) => {
                assert_eq!(params.name, "goblin");
                assert_eq!(params.count, 3);
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
    fn parse_list_notes() {
        let json = r#"{"id":"8","command":{"type":"ListNotes"}}"#;
        let req = parse_request(json).unwrap();
        assert!(matches!(req.command, GMCommand::ListNotes));
    }

    #[test]
    fn parse_delete_note() {
        let json = r#"{"id":"9","command":{"type":"DeleteNote","params":{"index":2}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::DeleteNote { index } => assert_eq!(*index, 2),
            _ => panic!("expected DeleteNote"),
        }
    }

    #[test]
    fn parse_list_retainers() {
        let json = r#"{"id":"10","command":{"type":"ListRetainers"}}"#;
        let req = parse_request(json).unwrap();
        assert!(matches!(req.command, GMCommand::ListRetainers));
    }

    #[test]
    fn parse_dismiss_retainer() {
        let json = r#"{"id":"11","command":{"type":"DismissRetainer","params":{"name":"Gurd"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::DismissRetainer { name } => assert_eq!(name, "Gurd"),
            _ => panic!("expected DismissRetainer"),
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
    fn parse_heal() {
        let json = r#"{"id":"h1","command":{"type":"Heal","params":{"character":"Aldric","amount":5}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Heal { character, amount } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*amount, 5);
            }
            _ => panic!("expected Heal"),
        }
    }

    #[test]
    fn parse_damage() {
        let json = r#"{"id":"d1","command":{"type":"Damage","params":{"character":"Aldric","amount":3}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Damage { character, amount } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*amount, 3);
            }
            _ => panic!("expected Damage"),
        }
    }

    #[test]
    fn parse_set_hp() {
        let json = r#"{"id":"s1","command":{"type":"SetHp","params":{"character":"Aldric","hp":5}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::SetHp { character, hp } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*hp, 5);
            }
            _ => panic!("expected SetHp"),
        }
    }

    #[test]
    fn parse_set_helpless() {
        let json = r#"{"id":"sh1","command":{"type":"SetHelpless","params":{"monster_idx":0,"helpless":true}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::SetHelpless { monster_idx, helpless } => {
                assert_eq!(*monster_idx, 0);
                assert!(*helpless);
            }
            _ => panic!("expected SetHelpless"),
        }
    }

    #[test]
    fn parse_set_helpless_default() {
        let json = r#"{"id":"sh2","command":{"type":"SetHelpless","params":{"monster_idx":1}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::SetHelpless { monster_idx, helpless } => {
                assert_eq!(*monster_idx, 1);
                assert!(*helpless); // default is true
            }
            _ => panic!("expected SetHelpless"),
        }
    }

    #[test]
    fn parse_kill() {
        let json = r#"{"id":"k1","command":{"type":"Kill","params":{"character":"Aldric","monster_idx":0}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Kill { character, monster_idx } => {
                assert_eq!(character, "Aldric");
                assert_eq!(*monster_idx, 0);
            }
            _ => panic!("expected Kill"),
        }
    }

    #[test]
    fn parse_set_rations() {
        let json = r#"{"id":"sr1","command":{"type":"SetRations","params":{"amount":20}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::SetRations { amount } => {
                assert_eq!(*amount, 20);
            }
            _ => panic!("expected SetRations"),
        }
    }

    #[test]
    fn parse_add_rations() {
        let json = r#"{"id":"ar1","command":{"type":"AddRations","params":{"amount":10}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::AddRations { amount } => {
                assert_eq!(*amount, 10);
            }
            _ => panic!("expected AddRations"),
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
    fn parse_buy() {
        let json = r#"{"id":"b1","command":{"type":"Buy","params":{"character":"Aldric","item_name":"Sword"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Buy { character, item_name } => {
                assert_eq!(character, "Aldric");
                assert_eq!(item_name, "Sword");
            }
            _ => panic!("expected Buy"),
        }
    }

    #[test]
    fn parse_drop() {
        let json = r#"{"id":"d1","command":{"type":"Drop","params":{"character":"Aldric","item_name":"Sword"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Drop { character, item_name } => {
                assert_eq!(character, "Aldric");
                assert_eq!(item_name, "Sword");
            }
            _ => panic!("expected Drop"),
        }
    }

    #[test]
    fn parse_equip() {
        let json = r#"{"id":"e1","command":{"type":"Equip","params":{"character":"Aldric","item_name":"Leather"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Equip { character, item_name } => {
                assert_eq!(character, "Aldric");
                assert_eq!(item_name, "Leather");
            }
            _ => panic!("expected Equip"),
        }
    }

    #[test]
    fn parse_loot() {
        let json = r#"{"id":"l1","command":{"type":"Loot","params":{"character":"Aldric","item_name":"Ruby gem","value_gp":500}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Loot { character, item_name, value_gp } => {
                assert_eq!(character, "Aldric");
                assert_eq!(item_name, "Ruby gem");
                assert_eq!(*value_gp, Some(500));
            }
            _ => panic!("expected Loot"),
        }
    }

    #[test]
    fn parse_loot_no_value() {
        let json = r#"{"id":"l2","command":{"type":"Loot","params":{"character":"Aldric","item_name":"Old key"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Loot { value_gp, .. } => {
                assert!(value_gp.is_none());
            }
            _ => panic!("expected Loot"),
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
    fn parse_query_combat_log() {
        let json = r#"{"id":"cl1","command":{"type":"QueryCombatLog"}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "cl1");
        assert!(matches!(req.command, GMCommand::QueryCombatLog));
    }

    #[test]
    fn parse_declare_spell() {
        let json = r#"{"id":"ds1","command":{"type":"DeclareSpell","params":{"character":"Mira","spell":"Sleep"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "ds1");
        match &req.command {
            GMCommand::DeclareSpell { character, spell } => {
                assert_eq!(character, "Mira");
                assert_eq!(spell, "Sleep");
            }
            _ => panic!("expected DeclareSpell"),
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

    #[test]
    fn parse_open_door() {
        let json = r#"{"id":"o1","command":{"type":"OpenDoor","params":{"door_id":3}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "o1");
        match &req.command {
            GMCommand::OpenDoor { door_id } => assert_eq!(*door_id, 3),
            _ => panic!("expected OpenDoor"),
        }
    }

    #[test]
    fn parse_force_door() {
        let json = r#"{"id":"f1","command":{"type":"ForceDoor","params":{"door_id":0,"character":"Aldric"}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::ForceDoor { door_id, character } => {
                assert_eq!(*door_id, 0);
                assert_eq!(character, "Aldric");
            }
            _ => panic!("expected ForceDoor"),
        }
    }

    #[test]
    fn parse_listen() {
        let json = r#"{"id":"l1","command":{"type":"Listen","params":{"is_demihuman":true}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Listen { is_demihuman } => assert!(*is_demihuman),
            _ => panic!("expected Listen"),
        }
    }

    #[test]
    fn parse_listen_default() {
        let json = r#"{"id":"l2","command":{"type":"Listen","params":{"is_demihuman":false}}}"#;
        let req = parse_request(json).unwrap();
        match &req.command {
            GMCommand::Listen { is_demihuman } => assert!(!*is_demihuman),
            _ => panic!("expected Listen"),
        }
    }

    #[test]
    fn parse_rest() {
        let json = r#"{"id":"r1","command":{"type":"Rest"}}"#;
        let req = parse_request(json).unwrap();
        assert!(matches!(req.command, GMCommand::Rest));
    }

    #[test]
    fn parse_lookup_item() {
        let json = r#"{"id":"li1","command":{"type":"LookupItem","params":{"name":"Bag of Holding"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "li1");
        match &req.command {
            GMCommand::LookupItem { name } => assert_eq!(name, "Bag of Holding"),
            _ => panic!("expected LookupItem"),
        }
    }

    #[test]
    fn parse_search_items() {
        let json = r#"{"id":"si1","command":{"type":"SearchItems","params":{"query":"healing"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "si1");
        match &req.command {
            GMCommand::SearchItems { query } => assert_eq!(query, "healing"),
            _ => panic!("expected SearchItems"),
        }
    }

    #[test]
    fn parse_lookup_treasure_type() {
        let json = r#"{"id":"lt1","command":{"type":"LookupTreasureType","params":{"letter":"A"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "lt1");
        match &req.command {
            GMCommand::LookupTreasureType { letter } => assert_eq!(letter, "A"),
            _ => panic!("expected LookupTreasureType"),
        }
    }

    #[test]
    fn parse_roll_treasure() {
        let json = r#"{"id":"rt1","command":{"type":"RollTreasure","params":{"letter":"P"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "rt1");
        match &req.command {
            GMCommand::RollTreasure { letter } => assert_eq!(letter, "P"),
            _ => panic!("expected RollTreasure"),
        }
    }

    #[test]
    fn parse_list_equipment() {
        let json = r#"{"id":"le1","command":{"type":"ListEquipment","params":{}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "le1");
        match &req.command {
            GMCommand::ListEquipment { category } => assert!(category.is_none()),
            _ => panic!("expected ListEquipment"),
        }
    }

    #[test]
    fn parse_list_equipment_with_category() {
        let json = r#"{"id":"le2","command":{"type":"ListEquipment","params":{"category":"weapons"}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "le2");
        match &req.command {
            GMCommand::ListEquipment { category } => assert_eq!(category.as_deref(), Some("weapons")),
            _ => panic!("expected ListEquipment"),
        }
    }

    #[test]
    fn parse_list_classes() {
        let json = r#"{"id":"lc1","command":{"type":"ListClasses"}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "lc1");
        assert!(matches!(req.command, GMCommand::ListClasses));
    }

    #[test]
    fn parse_eligible_classes() {
        let json = r#"{"id":"ec1","command":{"type":"EligibleClasses","params":{"abilities":[16,10,10,12,14,12]}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "ec1");
        match &req.command {
            GMCommand::EligibleClasses { abilities } => {
                assert_eq!(*abilities, [16, 10, 10, 12, 14, 12]);
            }
            _ => panic!("expected EligibleClasses"),
        }
    }

    #[test]
    fn parse_spawn_npc_party() {
        let json = r#"{"id":"n1","command":{"type":"SpawnNpcParty","params":{"party_type":"basic","distance":60}}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, "n1");
        match &req.command {
            GMCommand::SpawnNpcParty { party_type, distance } => {
                assert_eq!(party_type, "basic");
                assert_eq!(*distance, 60);
            }
            _ => panic!("expected SpawnNpcParty"),
        }
    }

    #[test]
    fn reject_oversized_request() {
        let big = "x".repeat(MAX_REQUEST_SIZE + 1);
        assert!(parse_request(&big).is_err());
    }

    #[test]
    fn reject_long_request_id() {
        let long_id = "a".repeat(65);
        let json = format!(r#"{{"id":"{}","command":{{"type":"QueryState"}}}}"#, long_id);
        let result = parse_request(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("id too long"));
    }

    #[test]
    fn reject_long_character_name() {
        let long_name = "a".repeat(129);
        let json = format!(
            r#"{{"id":"v1","command":{{"type":"Attack","params":{{"character":"{}","monster_idx":0}}}}}}"#,
            long_name
        );
        let result = parse_request(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn reject_excessive_monster_count() {
        let json = r#"{"id":"v2","command":{"type":"SpawnMonster","params":{"name":"goblin","count":101}}}"#;
        let result = parse_request(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn accept_valid_monster_count() {
        let json = r#"{"id":"v3","command":{"type":"SpawnMonster","params":{"name":"goblin","count":100}}}"#;
        assert!(parse_request(json).is_ok());
    }
}
