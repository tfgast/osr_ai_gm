use serde::Serialize;

use crate::rules::alignment::Alignment;
use crate::rules::class::Class;
use crate::state::wilderness::Terrain;

/// Typed success payload for `encounter` / `RollEncounter`.
#[derive(Debug, Clone, Serialize)]
pub struct RollEncounterResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub context: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain: Option<Terrain>,
    pub table_roll: u32,
    pub monster_name: String,
    pub number_notation: String,
    pub number_appearing: i32,
    pub party_surprise_roll: u32,
    pub monster_surprise_roll: u32,
    pub surprise: String,
    pub distance: u32,
}

/// Typed success payload for `surprise` / `RollSurprise`.
#[derive(Debug, Clone, Serialize)]
pub struct RollSurpriseResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub party_roll: u32,
    pub monster_roll: u32,
    pub result: String,
}

impl RollSurpriseResult {
    pub fn cli_message(&self) -> String {
        format!(
            "Party roll: {}  Monster roll: {}\n{}",
            self.party_roll, self.monster_roll, self.result
        )
    }

    pub fn api_message(&self) -> String {
        format!(
            "party roll: {} monster roll: {} — {}",
            self.party_roll, self.monster_roll, self.result
        )
    }
}

/// Typed success payload for `reaction` / `RollReaction`.
#[derive(Debug, Clone, Serialize)]
pub struct RollReactionResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub character: String,
    pub charisma: i32,
    pub cha_modifier: i32,
    pub raw_roll: i32,
    pub modified_roll: i32,
    pub reaction: String,
}

impl RollReactionResult {
    pub fn cli_message(&self) -> String {
        format!(
            "{} speaks (CHA {}, modifier {:+}).\nReaction roll: {} (2d6) {:+} = {}\n{}",
            self.character,
            self.charisma,
            self.cha_modifier,
            self.raw_roll,
            self.cha_modifier,
            self.modified_roll,
            self.reaction
        )
    }

    pub fn api_message(&self) -> String {
        format!(
            "{} speaks (CHA {}, modifier {:+}). reaction roll: {} {:+} = {} — {}",
            self.character,
            self.charisma,
            self.cha_modifier,
            self.raw_roll,
            self.cha_modifier,
            self.modified_roll,
            self.reaction
        )
    }
}

/// Typed success payload for `evade` / `Evade`.
#[derive(Debug, Clone, Serialize)]
pub struct EvadeResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub escaped: bool,
    pub party_size: u32,
    pub party_movement: u32,
    pub monster_count: u32,
    pub monster_movement: u32,
}

/// Structured NPC member details returned by `spawn_npc_party`.
#[derive(Debug, Clone, Serialize)]
pub struct SpawnNpcPartyMemberInfo {
    pub class: Class,
    pub level: u32,
    pub alignment: Alignment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Typed success payload for `spawn_npc_party`.
#[derive(Debug, Clone, Serialize)]
pub struct SpawnNpcPartyResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub party_type: String,
    pub member_count: usize,
    #[serde(rename = "members")]
    pub member_info: Vec<SpawnNpcPartyMemberInfo>,
    pub mounted: bool,
    pub notes: Vec<String>,
    pub status: String,
    pub distance: u32,
}
