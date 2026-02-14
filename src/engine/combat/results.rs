use serde::Serialize;

use crate::rules::attack::HitDice;
use crate::rules::turn::TurnResult;
use crate::state::game::GameMode;

use super::attack::{AttackResult as CoreAttackResult, CoupDeGraceResult as CoreCoupDeGraceResult};
use super::morale::MoraleResult as CoreMoraleResult;
use super::movement::RetreatResult as CoreRetreatResult;
use super::turn_undead::TurnUndeadResult as CoreTurnUndeadResult;

/// Typed success payload for `spawn_monster` (by bestiary name).
#[derive(Debug, Clone, Serialize)]
pub struct SpawnMonsterResult {
    #[serde(skip_serializing)]
    pub message: String,
    #[serde(rename = "monster")]
    pub monster_name: String,
    pub count: u32,
    pub hit_dice: HitDice,
    pub ac: i32,
    pub damage: String,
    pub morale: u32,
    pub distance: u32,
    pub xp_per_monster: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub special: String,
    pub status: String,
}

/// Typed success payload for `spawn_encounter`.
#[derive(Debug, Clone, Serialize)]
pub struct SpawnEncounterResult {
    pub message: String,
    pub encounter_name: String,
    pub count: u32,
    pub hit_dice: HitDice,
    pub ac: i32,
    pub hp: i32,
    pub damage: String,
    pub morale: u32,
    pub distance: u32,
    pub xp_per_monster: u64,
    pub status: String,
}

/// Which side won initiative for the round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InitiativeWinner {
    Party,
    Monsters,
    Simultaneous,
}

impl InitiativeWinner {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Party => "party",
            Self::Monsters => "monsters",
            Self::Simultaneous => "simultaneous",
        }
    }
}

/// Typed success payload for `initiative`.
#[derive(Debug, Clone, Serialize)]
pub struct InitiativeResult {
    pub message: String,
    pub round: u32,
    pub party_initiative: i32,
    pub monster_initiative: i32,
    pub winner: InitiativeWinner,
}

/// Typed success payload for `attack`.
#[derive(Debug, Clone, Serialize)]
pub struct AttackResult {
    pub message: String,
    pub attacker: String,
    pub target: String,
    pub roll: Option<u32>,
    pub modifiers: Option<i32>,
    pub target_number: Option<i32>,
    pub hit: bool,
    pub damage: Option<i32>,
    pub damage_rolls: Vec<u32>,
    pub target_hp_after: Option<i32>,
    pub target_killed: bool,
    pub auto_kill: bool,
    pub target_was_helpless: bool,
}

impl From<CoreAttackResult> for AttackResult {
    fn from(value: CoreAttackResult) -> Self {
        Self {
            message: value.to_string(),
            attacker: value.attacker,
            target: value.target,
            roll: Some(value.roll),
            modifiers: Some(value.modifiers),
            target_number: Some(value.target_number),
            hit: value.hit,
            damage: Some(value.damage),
            damage_rolls: value.damage_rolls,
            target_hp_after: Some(value.target_hp_after),
            target_killed: value.target_killed,
            auto_kill: false,
            target_was_helpless: false,
        }
    }
}

impl From<CoreCoupDeGraceResult> for AttackResult {
    fn from(value: CoreCoupDeGraceResult) -> Self {
        Self {
            message: value.to_string(),
            attacker: value.attacker,
            target: value.target,
            roll: None,
            modifiers: None,
            target_number: None,
            hit: true,
            damage: None,
            damage_rolls: Vec::new(),
            target_hp_after: Some(0),
            target_killed: true,
            auto_kill: true,
            target_was_helpless: value.target_was_helpless,
        }
    }
}

/// Typed success payload for `monster_attack`.
#[derive(Debug, Clone, Serialize)]
pub struct MonsterAttackResult {
    pub message: String,
    pub attack: AttackResult,
}

impl From<CoreAttackResult> for MonsterAttackResult {
    fn from(value: CoreAttackResult) -> Self {
        let attack = AttackResult::from(value);
        Self {
            message: attack.message.clone(),
            attack,
        }
    }
}

/// Typed success payload for `morale`.
#[derive(Debug, Clone, Serialize)]
pub struct MoraleResult {
    pub message: String,
    pub roll: i32,
    pub morale_score: u32,
    pub passed: bool,
}

impl From<CoreMoraleResult> for MoraleResult {
    fn from(value: CoreMoraleResult) -> Self {
        Self {
            message: value.to_string(),
            roll: value.roll,
            morale_score: value.morale_score,
            passed: value.passed,
        }
    }
}

/// Typed success payload for `turn_undead`.
#[derive(Debug, Clone, Serialize)]
pub struct TurnUndeadResult {
    pub message: String,
    pub cleric_name: String,
    pub cleric_level: u32,
    pub undead_type: String,
    pub undead_rank: u32,
    pub table_result: TurnResult,
    pub roll: Option<i32>,
    pub success: bool,
    pub hd_affected: u32,
    pub destroyed: bool,
}

impl From<CoreTurnUndeadResult> for TurnUndeadResult {
    fn from(value: CoreTurnUndeadResult) -> Self {
        Self {
            message: value.to_string(),
            cleric_name: value.cleric_name,
            cleric_level: value.cleric_level,
            undead_type: value.undead_type,
            undead_rank: value.undead_rank,
            table_result: value.table_result,
            roll: value.roll,
            success: value.success,
            hd_affected: value.hd_affected,
            destroyed: value.destroyed,
        }
    }
}

/// Typed success payload for `close`.
#[derive(Debug, Clone, Serialize)]
pub struct CloseResult {
    pub message: String,
    pub character: String,
    pub distance_closed: u32,
    pub new_distance: u32,
}

/// Typed success payload for `retreat`.
#[derive(Debug, Clone, Serialize)]
pub struct RetreatResult {
    pub message: String,
    pub retreater: String,
    pub distance_moved: u32,
    pub new_distance: u32,
    pub free_attacks: Vec<AttackResult>,
}

impl From<CoreRetreatResult> for RetreatResult {
    fn from(value: CoreRetreatResult) -> Self {
        Self {
            message: value.to_string(),
            retreater: value.retreater,
            distance_moved: value.distance_moved,
            new_distance: value.new_distance,
            free_attacks: value
                .free_attacks
                .into_iter()
                .map(AttackResult::from)
                .collect(),
        }
    }
}

/// Typed success payload for `fighting_withdrawal`.
#[derive(Debug, Clone, Serialize)]
pub struct FightingWithdrawalResult {
    pub message: String,
    pub withdrawer: String,
    pub distance_moved: u32,
    pub new_distance: u32,
}

/// Typed success payload for `query_combat_log`.
#[derive(Debug, Clone, Serialize)]
pub struct CombatLogResult {
    pub message: String,
    pub log: Vec<String>,
}

/// Typed success payload for `backstab`.
#[derive(Debug, Clone, Serialize)]
pub struct BackstabResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub hit: bool,
    pub attack_roll: i32,
    pub target_number: i32,
    pub attack_bonus: i32,
    pub multiplier: u32,
    pub damage: Option<i32>,
    pub monster_alive: Option<bool>,
}

/// Typed success payload for `declare_spell`.
#[derive(Debug, Clone, Serialize)]
pub struct DeclareSpellResult {
    pub message: String,
    pub character: String,
    pub spell: String,
}

/// End-combat loyalty outcome per surviving retainer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetainerLoyaltyOutcome {
    Loyal,
    Wavering,
    Disloyal,
}

/// Loyalty check result for one retainer at the end of combat.
#[derive(Debug, Clone, Serialize)]
pub struct RetainerLoyaltyCheckResult {
    pub name: String,
    pub loyalty: u32,
    pub outcome: RetainerLoyaltyOutcome,
}

/// Typed success payload for `combat_status`.
#[derive(Debug, Clone, Serialize)]
pub struct CombatStatusResult {
    pub message: String,
    pub status: String,
}

/// Typed success payload for `set_helpless`.
#[derive(Debug, Clone, Serialize)]
pub struct SetHelplessResult {
    pub message: String,
    pub monster_idx: usize,
    pub helpless: bool,
}

/// Typed success payload for `end_combat`.
#[derive(Debug, Clone, Serialize)]
pub struct EndCombatResult {
    pub message: String,
    pub rounds: u32,
    pub monsters_defeated: usize,
    pub total_monsters: usize,
    pub total_xp: u64,
    pub party_casualties: usize,
    pub mode_after: GameMode,
    pub retainer_xp_each: Option<u64>,
    pub retainer_xp_recipients: Vec<String>,
    pub retainer_loyalty_checks: Vec<RetainerLoyaltyCheckResult>,
}
