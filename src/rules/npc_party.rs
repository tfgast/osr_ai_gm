//! NPC adventuring party generation tables.
//!
//! Provides random generation of NPC adventuring parties for encounters,
//! including class/level determination, alignment, and stronghold reactions.

use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::alignment::Alignment;

// ============================================================================
// JSON data structures
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NpcPartyData {
    class_level_table: Vec<ClassLevelEntry>,
    alignment_table: Vec<AlignmentEntry>,
    party_types: HashMap<String, PartyType>,
    strongholds: StrongholdData,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ClassLevelEntry {
    roll: u32,
    class: String,
    basic_level_dice: String,
    expert_level_dice: String,
    #[serde(default)]
    demihuman: bool,
    underworld_alternative: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct AlignmentEntry {
    min_roll: u32,
    max_roll: u32,
    alignment: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct PartyType {
    party_size_dice: Option<String>,
    level_tier: Option<String>,
    mounted_chance: Option<u32>,
    magic_item_chance_per_level: Option<u32>,
    leader: Option<LeaderDef>,
    companions: Option<Vec<CompanionDef>>,
    alternatives: Option<Vec<String>>,
    notes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct LeaderDef {
    class: String,
    level_dice: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct CompanionDef {
    class: String,
    count_dice: String,
    level_dice: String,
    role: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct StrongholdData {
    rulers: HashMap<String, RulerDef>,
    patrols: HashMap<String, PatrolDef>,
    ruler_reactions: Vec<ReactionEntry>,
    reaction_descriptions: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct RulerDef {
    level_dice: String,
    examples: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct PatrolDef {
    count_dice: String,
    #[serde(rename = "type")]
    troop_type: String,
    ac: AcValue,
    equipment: String,
    morale: u32,
}

#[derive(Debug, Deserialize, Clone)]
struct AcValue {
    descending: i32,
    ascending: i32,
}

#[derive(Debug, Deserialize, Clone)]
struct ReactionEntry {
    roll: u32,
    arcane: String,
    divine: String,
    martial: String,
}

// ============================================================================
// Public types
// ============================================================================

/// An NPC party member.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcMember {
    pub class: String,
    pub level: u32,
    pub alignment: Alignment,
    pub role: Option<String>,
}

/// A generated NPC party.
#[derive(Debug, Clone)]
pub struct NpcParty {
    pub party_type: String,
    pub members: Vec<NpcMember>,
    pub mounted: bool,
    pub notes: Vec<String>,
}

/// Stronghold ruler type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulerType {
    Arcane,
    Divine,
    Martial,
}

/// Stronghold patrol.
#[derive(Debug, Clone)]
pub struct Patrol {
    pub count: u32,
    pub troop_type: String,
    pub ac_descending: i32,
    pub ac_ascending: i32,
    pub equipment: String,
    pub morale: u32,
}

/// Ruler reaction to travelers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulerReaction {
    Chase,
    Ignore,
    Invite,
}

impl std::fmt::Display for RulerReaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulerReaction::Chase => write!(f, "Chase"),
            RulerReaction::Ignore => write!(f, "Ignore"),
            RulerReaction::Invite => write!(f, "Invite"),
        }
    }
}

// ============================================================================
// Data loading
// ============================================================================

static NPC_PARTY_DATA: OnceLock<NpcPartyData> = OnceLock::new();

fn load_data() -> &'static NpcPartyData {
    NPC_PARTY_DATA.get_or_init(|| {
        let json_str = include_str!("../../data/core/npc_parties.json");
        serde_json::from_str(json_str).expect("Failed to parse npc_parties.json")
    })
}

// ============================================================================
// Dice rolling helpers
// ============================================================================

fn roll_dice_expr<R: Rng>(rng: &mut R, expr: &str) -> u32 {
    // Parse expressions like "1d3", "1d6+4", "2d4+2"
    let expr = expr.trim();

    // Handle "XdY+Z" or "XdY"
    let (dice_part, modifier) = if let Some(pos) = expr.find('+') {
        let (d, m) = expr.split_at(pos);
        (d, m[1..].parse::<i32>().unwrap_or(0))
    } else if let Some(pos) = expr.find('-') {
        let (d, m) = expr.split_at(pos);
        (d, -m[1..].parse::<i32>().unwrap_or(0))
    } else {
        (expr, 0)
    };

    let parts: Vec<&str> = dice_part.split('d').collect();
    if parts.len() != 2 {
        return 1; // Fallback
    }

    let count: u32 = parts[0].parse().unwrap_or(1).min(100);
    let sides: u32 = parts[1].parse().unwrap_or(6).min(100);

    let mut total: i32 = 0;
    for _ in 0..count {
        total = total.saturating_add(rng.gen_range(1..=sides) as i32);
    }
    total = total.saturating_add(modifier);

    total.max(1) as u32
}

// ============================================================================
// Public API
// ============================================================================

/// Roll a random NPC class from the d20 table.
pub fn roll_class<R: Rng>(rng: &mut R) -> &'static str {
    let data = load_data();
    let roll: u32 = rng.gen_range(1..=20);

    for entry in &data.class_level_table {
        if entry.roll == roll {
            return &entry.class;
        }
    }

    "Fighter" // Fallback
}

/// Get the level dice expression for a class at a given tier.
pub fn level_dice_for_class(class: &str, tier: &str) -> Option<&'static str> {
    let data = load_data();

    for entry in &data.class_level_table {
        if entry.class == class {
            return match tier {
                "basic" => Some(&entry.basic_level_dice),
                "expert" => Some(&entry.expert_level_dice),
                _ => None,
            };
        }
    }
    None
}

/// Roll a random NPC class and level.
pub fn roll_class_and_level<R: Rng>(rng: &mut R, tier: &str) -> (String, u32) {
    let data = load_data();
    let roll: u32 = rng.gen_range(1..=20);

    for entry in &data.class_level_table {
        if entry.roll == roll {
            let dice = match tier {
                "basic" => &entry.basic_level_dice,
                _ => &entry.expert_level_dice,
            };
            let level = roll_dice_expr(rng, dice);
            return (entry.class.clone(), level);
        }
    }

    ("Fighter".to_string(), 1)
}

/// Roll a random alignment.
pub fn roll_alignment<R: Rng>(rng: &mut R) -> Alignment {
    let data = load_data();
    let roll: u32 = rng.gen_range(1..=6);

    for entry in &data.alignment_table {
        if roll >= entry.min_roll && roll <= entry.max_roll {
            return match entry.alignment.as_str() {
                "Lawful" => Alignment::Lawful,
                "Chaotic" => Alignment::Chaotic,
                _ => Alignment::Neutral,
            };
        }
    }

    Alignment::Neutral
}

/// Generate a basic adventuring party (1d4+4 members, levels 1-3).
pub fn generate_basic_party<R: Rng>(rng: &mut R) -> NpcParty {
    let party_size = roll_dice_expr(rng, "1d4+4");
    let party_alignment = roll_alignment(rng);

    let mut members = Vec::new();
    for _ in 0..party_size {
        let (class, level) = roll_class_and_level(rng, "basic");
        members.push(NpcMember {
            class,
            level,
            alignment: party_alignment,
            role: None,
        });
    }

    NpcParty {
        party_type: "Basic Adventurers".to_string(),
        members,
        mounted: false,
        notes: vec!["Treasure type U+V".to_string()],
    }
}

/// Generate an expert adventuring party (1d6+3 members, levels 5-10+).
pub fn generate_expert_party<R: Rng>(rng: &mut R) -> NpcParty {
    let party_size = roll_dice_expr(rng, "1d6+3");
    let party_alignment = roll_alignment(rng);
    let mounted = rng.gen_range(1..=100) <= 75;

    let mut members = Vec::new();
    for _ in 0..party_size {
        let (class, level) = roll_class_and_level(rng, "expert");
        members.push(NpcMember {
            class,
            level,
            alignment: party_alignment,
            role: None,
        });
    }

    NpcParty {
        party_type: "Expert Adventurers".to_string(),
        members,
        mounted,
        notes: vec![
            "Treasure type U+V".to_string(),
            format!("Mounted: {}", if mounted { "Yes" } else { "No" }),
            "Magic items: 5% per level per suitable sub-table".to_string(),
        ],
    }
}

/// Generate a high-level cleric party.
pub fn generate_high_level_cleric_party<R: Rng>(rng: &mut R) -> NpcParty {
    let party_alignment = roll_alignment(rng);
    let mounted = rng.gen_range(1..=100) <= 75;

    let mut members = Vec::new();

    // Leader
    let leader_level = roll_dice_expr(rng, "1d6+6");
    members.push(NpcMember {
        class: "Cleric".to_string(),
        level: leader_level,
        alignment: party_alignment,
        role: Some("Leader".to_string()),
    });

    // Accompanying clerics
    let cleric_count = roll_dice_expr(rng, "1d4");
    for _ in 0..cleric_count {
        let level = roll_dice_expr(rng, "1d4+1");
        members.push(NpcMember {
            class: "Cleric".to_string(),
            level,
            alignment: party_alignment,
            role: None,
        });
    }

    // Fighter escorts
    let fighter_count = roll_dice_expr(rng, "1d3");
    for _ in 0..fighter_count {
        let level = roll_dice_expr(rng, "1d6");
        members.push(NpcMember {
            class: "Fighter".to_string(),
            level,
            alignment: party_alignment,
            role: None,
        });
    }

    NpcParty {
        party_type: "High-Level Cleric".to_string(),
        members,
        mounted,
        notes: vec![
            "Alternative leaders: Bard, Druid, Paladin".to_string(),
            "Treasure type U+V".to_string(),
        ],
    }
}

/// Generate a high-level fighter party.
pub fn generate_high_level_fighter_party<R: Rng>(rng: &mut R) -> NpcParty {
    let party_alignment = roll_alignment(rng);
    let mounted = rng.gen_range(1..=100) <= 75;

    let mut members = Vec::new();

    // Leader
    let leader_level = roll_dice_expr(rng, "1d4+6");
    members.push(NpcMember {
        class: "Fighter".to_string(),
        level: leader_level,
        alignment: party_alignment,
        role: Some("Leader".to_string()),
    });

    // Retainers (any class)
    let retainer_count = roll_dice_expr(rng, "2d4");
    for _ in 0..retainer_count {
        let (class, _) = roll_class_and_level(rng, "basic");
        let level = roll_dice_expr(rng, "1d4+2");
        members.push(NpcMember {
            class,
            level,
            alignment: party_alignment,
            role: Some("Retainer".to_string()),
        });
    }

    NpcParty {
        party_type: "High-Level Fighter".to_string(),
        members,
        mounted,
        notes: vec![
            "Often on way to/from war".to_string(),
            "Alternative leaders: Barbarian, Knight, Paladin, Ranger".to_string(),
            "Treasure type U+V".to_string(),
        ],
    }
}

/// Generate a high-level magic-user party.
pub fn generate_high_level_magic_user_party<R: Rng>(rng: &mut R) -> NpcParty {
    let leader_alignment = roll_alignment(rng);
    let mounted = rng.gen_range(1..=100) <= 75;

    let mut members = Vec::new();

    // Leader
    let leader_level = roll_dice_expr(rng, "1d4+6");
    members.push(NpcMember {
        class: "Magic-User".to_string(),
        level: leader_level,
        alignment: leader_alignment,
        role: Some("Leader".to_string()),
    });

    // Apprentices (same alignment as leader)
    let apprentice_count = roll_dice_expr(rng, "1d4");
    for _ in 0..apprentice_count {
        let level = roll_dice_expr(rng, "1d3");
        members.push(NpcMember {
            class: "Magic-User".to_string(),
            level,
            alignment: leader_alignment,
            role: Some("Apprentice".to_string()),
        });
    }

    // Mercenaries (may differ in alignment)
    let merc_count = roll_dice_expr(rng, "1d4");
    for _ in 0..merc_count {
        let level = roll_dice_expr(rng, "1d4+1");
        let merc_alignment = roll_alignment(rng);
        members.push(NpcMember {
            class: "Fighter".to_string(),
            level,
            alignment: merc_alignment,
            role: Some("Mercenary".to_string()),
        });
    }

    NpcParty {
        party_type: "High-Level Magic-User".to_string(),
        members,
        mounted,
        notes: vec![
            "Often on quest for arcane lore".to_string(),
            "Alternative leader: Illusionist".to_string(),
            "Treasure type U+V".to_string(),
        ],
    }
}

/// Get stronghold ruler level for a given type.
pub fn roll_ruler_level<R: Rng>(rng: &mut R, ruler_type: RulerType) -> u32 {
    let dice = match ruler_type {
        RulerType::Arcane => "1d4+10",
        RulerType::Divine => "1d8+6",
        RulerType::Martial => "1d6+8",
    };
    roll_dice_expr(rng, dice)
}

/// Generate a stronghold patrol.
pub fn generate_patrol<R: Rng>(rng: &mut R, ruler_type: RulerType) -> Patrol {
    let data = load_data();
    let key = match ruler_type {
        RulerType::Arcane => "arcane",
        RulerType::Divine => "divine",
        RulerType::Martial => "martial",
    };

    if let Some(patrol_def) = data.strongholds.patrols.get(key) {
        let count = roll_dice_expr(rng, &patrol_def.count_dice);
        Patrol {
            count,
            troop_type: patrol_def.troop_type.clone(),
            ac_descending: patrol_def.ac.descending,
            ac_ascending: patrol_def.ac.ascending,
            equipment: patrol_def.equipment.clone(),
            morale: patrol_def.morale,
        }
    } else {
        // Fallback
        Patrol {
            count: 6,
            troop_type: "Guards".to_string(),
            ac_descending: 5,
            ac_ascending: 14,
            equipment: "Chainmail, swords".to_string(),
            morale: 8,
        }
    }
}

/// Roll stronghold ruler reaction to travelers.
pub fn roll_ruler_reaction<R: Rng>(rng: &mut R, ruler_type: RulerType) -> RulerReaction {
    let data = load_data();
    let roll: u32 = rng.gen_range(1..=6);

    for entry in &data.strongholds.ruler_reactions {
        if entry.roll == roll {
            let reaction_str = match ruler_type {
                RulerType::Arcane => &entry.arcane,
                RulerType::Divine => &entry.divine,
                RulerType::Martial => &entry.martial,
            };
            return match reaction_str.as_str() {
                "Chase" => RulerReaction::Chase,
                "Invite" => RulerReaction::Invite,
                _ => RulerReaction::Ignore,
            };
        }
    }

    RulerReaction::Ignore
}

/// Get description of a ruler reaction.
pub fn reaction_description(reaction: RulerReaction) -> &'static str {
    match reaction {
        RulerReaction::Chase => {
            "Patrol chases intruders or demands toll. May attack, drive away, or imprison if refused."
        }
        RulerReaction::Ignore => "Patrol leaves travelers to their business.",
        RulerReaction::Invite => {
            "Patrol brings invitation to stay at stronghold. Motive depends on ruler's personality."
        }
    }
}

/// Convert an NPC party member into a Monster for combat.
///
/// Derives combat stats from the member's class and level using class definitions
/// and the OSE attack tables.
pub fn npc_member_to_monster(member: &NpcMember) -> crate::model::Monster {
    use crate::model::Monster;
    use crate::rules::class::{Class, class_def, CombatAptitude};
    use crate::dice;

    let class = Class::parse(&member.class);
    let (hit_die, aptitude) = match class {
        Some(c) => {
            let def = class_def(c);
            (def.hit_die, def.combat_aptitude)
        }
        None => (6, CombatAptitude::SemiMartial), // sensible default for unknown classes
    };

    // Roll HP: level * hit_die (e.g. level 3 Fighter = 3d8)
    let hd_expr = format!("{}d{}", member.level, hit_die);
    let hp = dice::roll_str(&hd_expr)
        .map(|r| r.total.max(1))
        .unwrap_or((member.level as i32 * hit_die as i32 / 2).max(1));

    // AC by combat aptitude (descending): Martial=5 (chain), SemiMartial=7 (leather+shield), NonMartial=9
    let ac = match aptitude {
        CombatAptitude::Martial => 5,
        CombatAptitude::SemiMartial => 7,
        CombatAptitude::NonMartial => 9,
    };

    // Damage by combat aptitude
    let damage = match aptitude {
        CombatAptitude::Martial => "1d8",
        _ => "1d6",
    };

    let name = format!("NPC {} Lv{}", member.class, member.level);
    let mut m = Monster::new(&name, &hd_expr);
    m.hp = hp;
    m.max_hp = hp;
    m.ac = ac;
    m.damage = damage.to_string();
    m.morale = 9;
    m.xp_value = 0; // No XP for NPC adventurers
    m.attacks = vec!["attack".to_string()];
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn roll_class_returns_valid_class() {
        let mut rng = test_rng();
        let class = roll_class(&mut rng);
        assert!(!class.is_empty());
    }

    #[test]
    fn roll_alignment_returns_valid() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let alignment = roll_alignment(&mut rng);
            assert!(matches!(
                alignment,
                Alignment::Lawful | Alignment::Neutral | Alignment::Chaotic
            ));
        }
    }

    #[test]
    fn basic_party_has_correct_size() {
        let mut rng = test_rng();
        let party = generate_basic_party(&mut rng);
        assert!(party.members.len() >= 5); // 1d4+4 minimum is 5
        assert!(party.members.len() <= 8); // 1d4+4 maximum is 8
    }

    #[test]
    fn expert_party_has_correct_size() {
        let mut rng = test_rng();
        let party = generate_expert_party(&mut rng);
        assert!(party.members.len() >= 4); // 1d6+3 minimum is 4
        assert!(party.members.len() <= 9); // 1d6+3 maximum is 9
    }

    #[test]
    fn high_level_cleric_party_has_leader() {
        let mut rng = test_rng();
        let party = generate_high_level_cleric_party(&mut rng);

        let leader = party.members.iter().find(|m| m.role == Some("Leader".to_string()));
        assert!(leader.is_some());
        let leader = leader.unwrap();
        assert_eq!(leader.class, "Cleric");
        assert!(leader.level >= 7); // 1d6+6 minimum
    }

    #[test]
    fn high_level_fighter_party_has_leader() {
        let mut rng = test_rng();
        let party = generate_high_level_fighter_party(&mut rng);

        let leader = party.members.iter().find(|m| m.role == Some("Leader".to_string()));
        assert!(leader.is_some());
        let leader = leader.unwrap();
        assert_eq!(leader.class, "Fighter");
        assert!(leader.level >= 7); // 1d4+6 minimum
    }

    #[test]
    fn high_level_magic_user_party_has_apprentices() {
        let mut rng = test_rng();
        let party = generate_high_level_magic_user_party(&mut rng);

        let apprentices: Vec<_> = party
            .members
            .iter()
            .filter(|m| m.role == Some("Apprentice".to_string()))
            .collect();
        assert!(apprentices.len() >= 1);
        for a in apprentices {
            assert_eq!(a.class, "Magic-User");
        }
    }

    #[test]
    fn patrol_generation_works() {
        let mut rng = test_rng();

        let arcane_patrol = generate_patrol(&mut rng, RulerType::Arcane);
        assert_eq!(arcane_patrol.troop_type, "Heavy Footmen");
        assert!(arcane_patrol.count >= 2 && arcane_patrol.count <= 12);

        let divine_patrol = generate_patrol(&mut rng, RulerType::Divine);
        assert_eq!(divine_patrol.troop_type, "Medium Horsemen");

        let martial_patrol = generate_patrol(&mut rng, RulerType::Martial);
        assert_eq!(martial_patrol.troop_type, "Heavy Horsemen");
    }

    #[test]
    fn ruler_reaction_varies_by_type() {
        // Run many trials to verify different ruler types have different distributions
        let mut arcane_chases = 0;
        let mut martial_chases = 0;

        for seed in 0..1000 {
            let mut rng = StdRng::seed_from_u64(seed);
            if roll_ruler_reaction(&mut rng, RulerType::Arcane) == RulerReaction::Chase {
                arcane_chases += 1;
            }
            let mut rng = StdRng::seed_from_u64(seed);
            if roll_ruler_reaction(&mut rng, RulerType::Martial) == RulerReaction::Chase {
                martial_chases += 1;
            }
        }

        // Martial should chase more often than arcane
        assert!(
            martial_chases > arcane_chases,
            "Martial ({}) should chase more than Arcane ({})",
            martial_chases,
            arcane_chases
        );
    }

    #[test]
    fn dice_expr_parsing() {
        let mut rng = test_rng();

        // Test basic expressions
        for _ in 0..100 {
            let result = roll_dice_expr(&mut rng, "1d6");
            assert!(result >= 1 && result <= 6);
        }

        for _ in 0..100 {
            let result = roll_dice_expr(&mut rng, "1d6+4");
            assert!(result >= 5 && result <= 10);
        }

        for _ in 0..100 {
            let result = roll_dice_expr(&mut rng, "2d4");
            assert!(result >= 2 && result <= 8);
        }
    }

    #[test]
    fn all_classes_reachable() {
        let mut found_classes = std::collections::HashSet::new();

        for seed in 0..1000 {
            let mut rng = StdRng::seed_from_u64(seed);
            let class = roll_class(&mut rng);
            found_classes.insert(class.to_string());
        }

        // Should find at least these classes
        assert!(found_classes.contains("Fighter"));
        assert!(found_classes.contains("Cleric"));
        assert!(found_classes.contains("Magic-User"));
        assert!(found_classes.contains("Thief"));
    }

    #[test]
    fn npc_member_to_monster_fighter() {
        let member = NpcMember {
            class: "Fighter".to_string(),
            level: 5,
            alignment: Alignment::Neutral,
            role: None,
        };
        let m = npc_member_to_monster(&member);
        assert_eq!(m.name, "NPC Fighter Lv5");
        assert_eq!(m.ac, 5); // Martial = chain
        assert_eq!(m.damage, "1d8"); // Martial damage
        assert_eq!(m.morale, 9);
        assert_eq!(m.xp_value, 0);
        assert!(m.hp >= 5 && m.hp <= 40); // 5d8
    }

    #[test]
    fn npc_member_to_monster_magic_user() {
        let member = NpcMember {
            class: "Magic-User".to_string(),
            level: 3,
            alignment: Alignment::Chaotic,
            role: None,
        };
        let m = npc_member_to_monster(&member);
        assert_eq!(m.name, "NPC Magic-User Lv3");
        assert_eq!(m.ac, 9); // NonMartial = unarmored
        assert_eq!(m.damage, "1d6");
        assert!(m.hp >= 3 && m.hp <= 12); // 3d4
    }
}
