//! Character generation engine for OSE.
//! Implements the full chargen pipeline: roll abilities, validate class,
//! apply racial modifiers, roll HP, roll gold, calculate derived stats.

use rand::Rng;
use crate::dice;
use crate::model::{AbilityScores, Character};
use crate::rules::ability;
use crate::rules::alignment::AlignmentId;
#[cfg(test)]
use crate::rules::alignment::Alignment;
use crate::rules::class::{self, ClassId, CombatAptitude, class_def};
use crate::rules::save;
use crate::rules::equipment;

// ── DSL evaluation helpers (gated) ─────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_eval {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::value::Value;

    use crate::backend::{NullState, SimpleDiceHandler};
    use crate::rules::class::CombatAptitude;

    /// Roll a single ability score via DSL mechanic (3d6).
    pub(super) fn roll_ability() -> Option<i32> {
        let runtime = crate::backend::dsl()?;
        let mut handler = SimpleDiceHandler::new();
        let result = runtime
            .evaluate_mechanic(&NullState, &mut handler, "roll_ability", vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as i32),
            _ => None,
        }
    }

    /// Roll starting HP via DSL mechanic.
    pub(super) fn roll_starting_hp(hit_die: u32, con_mod: i32) -> Option<i32> {
        let runtime = crate::backend::dsl()?;
        let mut handler = SimpleDiceHandler::new();
        let result = runtime
            .evaluate_mechanic(
                &NullState,
                &mut handler,
                "roll_starting_hp",
                vec![Value::Int(hit_die as i64), Value::Int(con_mod as i64)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as i32),
            _ => None,
        }
    }

    /// Look up THAC0 via DSL derive (wraps character_thac0 table).
    pub(super) fn character_thac0(aptitude: CombatAptitude, level: u32) -> Option<u32> {
        let runtime = crate::backend::dsl()?;
        let mut handler = SimpleDiceHandler::new();

        let aptitude_str = match aptitude {
            CombatAptitude::Martial => "martial",
            CombatAptitude::SemiMartial => "semi_martial",
            CombatAptitude::NonMartial => "non_martial",
        };
        let aptitude_val = Value::EnumVariant {
            enum_name: "CombatAptitude".into(),
            variant: Name::from(aptitude_str),
            fields: BTreeMap::new(),
        };

        let result = runtime
            .evaluate_derive(
                &NullState,
                &mut handler,
                "get_character_thac0",
                vec![aptitude_val, Value::Int(level as i64)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }
}

#[cfg(feature = "dsl-backend")]
use crate::backend::{is_dsl, MechanicGroup};

/// Returns true when the Chargen mechanic group is using the DSL backend.
#[cfg(feature = "dsl-backend")]
#[inline]
fn use_dsl() -> bool {
    is_dsl(MechanicGroup::Chargen)
}

/// Base movement rate (unencumbered).
pub const BASE_MOVEMENT: u32 = 120;

/// Roll 3d6 in order for six abilities: STR, INT, WIS, DEX, CON, CHA.
pub fn roll_abilities() -> [i32; 6] {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        let mut abilities = [0i32; 6];
        let mut all_ok = true;
        for slot in &mut abilities {
            if let Some(v) = dsl_eval::roll_ability() {
                *slot = v;
            } else {
                all_ok = false;
                break;
            }
        }
        if all_ok {
            return abilities;
        }
    }
    roll_abilities_with(&mut rand::thread_rng())
}

pub fn roll_abilities_with<R: Rng>(rng: &mut R) -> [i32; 6] {
    let expr = dice::parse("3d6").expect("hardcoded dice expression '3d6'");
    let mut abilities = [0i32; 6];
    for slot in &mut abilities {
        *slot = dice::roll_with(&expr, rng).total;
    }
    abilities
}

/// Roll HP for level 1: 1d(hit_die) + CON modifier, minimum 1.
pub fn roll_hp(hit_die: u32, con_mod: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::roll_starting_hp(hit_die, con_mod) {
            return v;
        }
    }
    roll_hp_with(hit_die, con_mod, &mut rand::thread_rng())
}

pub fn roll_hp_with<R: Rng>(hit_die: u32, con_mod: i32, rng: &mut R) -> i32 {
    let expr = dice::DiceExpr::Standard { count: 1, sides: hit_die, modifier: 0 };
    let roll = dice::roll_with(&expr, rng).total;
    (roll + con_mod).max(1)
}

/// Roll starting gold. Notation is "3d6x10" (3d6 multiplied by 10).
pub fn roll_gold(notation: &str) -> u32 {
    roll_gold_with(notation, &mut rand::thread_rng())
}

pub fn roll_gold_with<R: Rng>(notation: &str, rng: &mut R) -> u32 {
    if let Some((dice_part, mult_str)) = notation.rsplit_once('x') {
        let multiplier: u32 = mult_str.parse().unwrap_or(1);
        let expr = dice::parse(dice_part).expect("valid dice notation in starting_gold");
        let total = dice::roll_with(&expr, rng).total.max(0) as u32;
        total.saturating_mul(multiplier)
    } else {
        let expr = dice::parse(notation).expect("valid dice notation in starting_gold");
        dice::roll_with(&expr, rng).total.max(0) as u32
    }
}

/// THAC0 by combat aptitude and level.
/// Per OSE attack matrix tables.
pub fn thac0(aptitude: CombatAptitude, level: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::character_thac0(aptitude, level) {
            return v;
        }
    }
    match aptitude {
        CombatAptitude::Martial => match level {
            1..=3 => 19,
            4..=6 => 17,
            7..=9 => 14,
            10..=12 => 12,
            _ => 10,
        },
        CombatAptitude::SemiMartial => match level {
            1..=4 => 19,
            5..=8 => 17,
            9..=12 => 14,
            _ => 12,
        },
        CombatAptitude::NonMartial => match level {
            1..=5 => 19,
            6..=10 => 17,
            _ => 14,
        },
    }
}

/// Create a complete level-1 character.
/// Abilities should already have racial modifiers applied.
///
/// Uses DSL-gated paths for all rule lookups (class_def, saving_throws,
/// thac0) and dice rolls (roll_hp, roll_gold). This is the production
/// entry point — see `create_character_with` for a deterministic test variant.
pub fn create_character(
    name: &str,
    class: impl Into<ClassId>,
    abilities: [i32; 6],
    alignment: impl Into<AlignmentId>,
) -> Character {
    let class_id: ClassId = class.into();
    let alignment_id: AlignmentId = alignment.into();
    let def = class_def(&class_id);

    let con_mod = ability::con_hp_mod(abilities[class::CON]);
    let hp = roll_hp(def.hit_die, con_mod);
    let gold = roll_gold(def.starting_gold);

    let dex_mod = ability::dex_ac_mod(abilities[class::DEX]);
    let ac = equipment::calculate_ac(9, false, dex_mod);

    let saves = save::saving_throws(&def.save_category, 1);
    let thac0_val = thac0(def.combat_aptitude, 1);

    Character {
        name: name.to_string(),
        class: class_id,
        level: 1,
        abilities: AbilityScores::from_array(&abilities),
        hp,
        max_hp: hp,
        ac,
        xp: 0,
        inventory: Vec::new(),
        spells: Vec::new(),
        alignment: alignment_id,
        gold_gp: gold,
        saving_throws: Some(saves),
        thac0: thac0_val,
        movement_rate: BASE_MOVEMENT,
        spell_slots_used: [0; 6],
        prepared_spells: Vec::new(),
        spell_points_used: 0,
        effects: Vec::new(),
    }
}

/// Create a complete level-1 character with a specific RNG (for testing).
pub fn create_character_with<R: Rng>(
    name: &str,
    class: impl Into<ClassId>,
    abilities: [i32; 6],
    alignment: impl Into<AlignmentId>,
    rng: &mut R,
) -> Character {
    let class_id: ClassId = class.into();
    let alignment_id: AlignmentId = alignment.into();
    let def = class_def(&class_id);

    let con_mod = ability::con_hp_mod(abilities[class::CON]);
    let hp = roll_hp_with(def.hit_die, con_mod, rng);
    let gold = roll_gold_with(def.starting_gold, rng);

    let dex_mod = ability::dex_ac_mod(abilities[class::DEX]);
    let ac = equipment::calculate_ac(9, false, dex_mod); // unarmoured at creation

    let saves = save::saving_throws(&def.save_category, 1);
    let thac0_val = thac0(def.combat_aptitude, 1);

    Character {
        name: name.to_string(),
        class: class_id,
        level: 1,
        abilities: AbilityScores::from_array(&abilities),
        hp,
        max_hp: hp,
        ac,
        xp: 0,
        inventory: Vec::new(),
        spells: Vec::new(),
        alignment: alignment_id,
        gold_gp: gold,
        saving_throws: Some(saves),
        thac0: thac0_val,
        movement_rate: BASE_MOVEMENT,
        spell_slots_used: [0; 6],
        prepared_spells: Vec::new(),
        spell_points_used: 0,
        effects: Vec::new(),
    }
}

/// Format a character sheet for display.
pub fn character_sheet(c: &Character) -> String {
    let mut out = String::new();
    out.push_str(&format!("=== {} ===\n", c.name));
    out.push_str(&format!(
        "Class: {}  Level: {}  Alignment: {}\n",
        c.class.name(), c.level, c.alignment
    ));
    out.push_str(&format!(
        "HP: {}/{}  AC: {}  THAC0: {}\n",
        c.hp, c.max_hp, c.ac, c.thac0
    ));
    out.push_str(&format!("Movement: {}'\n", c.movement_rate));

    out.push_str("\nAbilities:\n");
    out.push_str(&format!(
        "  STR: {:2}  INT: {:2}  WIS: {:2}\n",
        c.abilities.strength, c.abilities.intelligence, c.abilities.wisdom
    ));
    out.push_str(&format!(
        "  DEX: {:2}  CON: {:2}  CHA: {:2}\n",
        c.abilities.dexterity, c.abilities.constitution, c.abilities.charisma
    ));

    if let Some(ref saves) = c.saving_throws {
        out.push_str("\nSaving Throws:\n");
        out.push_str(&format!(
            "  Death: {}  Wands: {}  Paralysis: {}  Breath: {}  Spells: {}\n",
            saves.death(), saves.wands(), saves.paralysis(), saves.breath(), saves.spells()
        ));
    }

    out.push_str(&format!("\nGold: {} gp\n", c.gold_gp));
    out.push_str(&format!("XP: {}\n", c.xp));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::save::SavingThrows;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn roll_abilities_bounds() {
        let abilities = roll_abilities();
        for &a in &abilities {
            assert!(a >= 3 && a <= 18, "ability {} out of range", a);
        }
    }

    #[test]
    fn roll_abilities_deterministic() {
        let a1 = roll_abilities_with(&mut test_rng());
        let a2 = roll_abilities_with(&mut test_rng());
        assert_eq!(a1, a2);
    }

    #[test]
    fn roll_hp_minimum_1() {
        let hp = roll_hp_with(4, -10, &mut test_rng());
        assert_eq!(hp, 1);
    }

    #[test]
    fn roll_hp_bounds() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let hp = roll_hp_with(8, 0, &mut rng);
            assert!(hp >= 1 && hp <= 8);
        }
    }

    #[test]
    fn roll_gold_3d6x10() {
        let mut rng = test_rng();
        let gold = roll_gold_with("3d6x10", &mut rng);
        assert!(gold >= 30 && gold <= 180, "gold {} out of range", gold);
    }

    #[test]
    fn thac0_martial_level_1() {
        assert_eq!(thac0(CombatAptitude::Martial, 1), 19);
    }

    #[test]
    fn thac0_martial_progression() {
        assert_eq!(thac0(CombatAptitude::Martial, 3), 19);
        assert_eq!(thac0(CombatAptitude::Martial, 4), 17);
        assert_eq!(thac0(CombatAptitude::Martial, 7), 14);
        assert_eq!(thac0(CombatAptitude::Martial, 10), 12);
        assert_eq!(thac0(CombatAptitude::Martial, 13), 10);
    }

    #[test]
    fn thac0_semi_martial_progression() {
        assert_eq!(thac0(CombatAptitude::SemiMartial, 4), 19);
        assert_eq!(thac0(CombatAptitude::SemiMartial, 5), 17);
        assert_eq!(thac0(CombatAptitude::SemiMartial, 9), 14);
        assert_eq!(thac0(CombatAptitude::SemiMartial, 13), 12);
    }

    #[test]
    fn thac0_non_martial_progression() {
        assert_eq!(thac0(CombatAptitude::NonMartial, 5), 19);
        assert_eq!(thac0(CombatAptitude::NonMartial, 6), 17);
        assert_eq!(thac0(CombatAptitude::NonMartial, 11), 14);
    }

    #[test]
    fn create_fighter() {
        let abilities = [14, 10, 10, 12, 13, 10];
        let c = create_character_with(
            "Grond", "Fighter", abilities, Alignment::Neutral, &mut test_rng(),
        );
        assert_eq!(c.name, "Grond");
        assert_eq!(c.class, ClassId::new("Fighter"));
        assert_eq!(c.level, 1);
        assert_eq!(c.alignment, AlignmentId::from_enum(Alignment::Neutral));
        assert_eq!(c.thac0, 19);
        assert_eq!(c.movement_rate, 120);
        assert!(c.hp >= 1);
        assert!(c.gold_gp >= 30 && c.gold_gp <= 180);
        let saves = c.saving_throws.unwrap();
        assert_eq!(saves, SavingThrows::new(12, 13, 14, 15, 16));
    }

    #[test]
    fn create_magic_user() {
        let abilities = [8, 16, 10, 12, 10, 11];
        let c = create_character_with(
            "Elara", "Magic-User", abilities, Alignment::Chaotic, &mut test_rng(),
        );
        assert_eq!(c.class, ClassId::new("Magic-User"));
        assert!(c.hp >= 1 && c.hp <= 4);
        assert_eq!(c.thac0, 19);
        let saves = c.saving_throws.unwrap();
        assert_eq!(saves, SavingThrows::new(13, 14, 13, 16, 15));
    }

    #[test]
    fn create_dwarf_with_racial_mods() {
        let mut abilities = [14, 10, 10, 10, 14, 10];
        class::apply_racial_modifiers(&ClassId::new("Dwarf"), &mut abilities);
        assert_eq!(abilities[class::CON], 15); // +1
        assert_eq!(abilities[class::CHA], 9);  // -1
        let c = create_character_with(
            "Thorin", "Dwarf", abilities, Alignment::Lawful, &mut test_rng(),
        );
        assert_eq!(c.class, ClassId::new("Dwarf"));
        assert_eq!(c.abilities.constitution, 15);
        assert_eq!(c.abilities.charisma, 9);
        let saves = c.saving_throws.unwrap();
        assert_eq!(saves, SavingThrows::new(8, 9, 10, 13, 12));
    }

    #[test]
    fn create_thief_ac_with_dex() {
        // DEX 16 gives AC mod of -2 (improves AC by 2)
        let abilities = [10, 10, 10, 16, 10, 10];
        let c = create_character_with(
            "Shadow", "Thief", abilities, Alignment::Neutral, &mut test_rng(),
        );
        assert_eq!(c.ac, 7); // 9 (unarmoured) - 2 (DEX mod)
    }

    #[test]
    fn character_sheet_contains_key_info() {
        let abilities = [14, 10, 10, 12, 13, 10];
        let c = create_character_with(
            "Grond", "Fighter", abilities, Alignment::Neutral, &mut test_rng(),
        );
        let sheet = character_sheet(&c);
        assert!(sheet.contains("Grond"));
        assert!(sheet.contains("Fighter"));
        assert!(sheet.contains("Neutral"));
        assert!(sheet.contains("STR: 14"));
        assert!(sheet.contains("THAC0: 19"));
        assert!(sheet.contains("Movement: 120'"));
        assert!(sheet.contains("Death:"));
        assert!(sheet.contains("Gold:"));
    }

    #[test]
    fn chargen_full_pipeline() {
        let mut rng = test_rng();
        let abilities = roll_abilities_with(&mut rng);

        // Find eligible classes
        let eligible = class::eligible_classes(&abilities);
        assert!(!eligible.is_empty(), "at least one class should be eligible");

        // Pick first eligible class
        let chosen = &eligible[0];
        let mut final_abilities = abilities;
        class::apply_racial_modifiers(chosen, &mut final_abilities);

        // Verify requirements still met after modifiers
        assert!(
            class::meets_requirements(chosen, &final_abilities),
            "should still meet requirements after racial modifiers"
        );

        let c = create_character_with(
            "TestChar", chosen.clone(), final_abilities, Alignment::Neutral, &mut rng,
        );
        assert_eq!(c.level, 1);
        assert!(c.hp >= 1);
        assert!(c.gold_gp > 0);
        assert!(c.saving_throws.is_some());
    }

    #[test]
    fn serialization_with_new_fields() {
        let abilities = [10, 10, 10, 10, 10, 10];
        let c = create_character_with(
            "SerTest", "Fighter", abilities, Alignment::Lawful, &mut test_rng(),
        );
        let json = serde_json::to_string(&c).unwrap();
        let c2: Character = serde_json::from_str(&json).unwrap();
        assert_eq!(c.name, c2.name);
        assert_eq!(c.alignment, c2.alignment);
        assert_eq!(c.gold_gp, c2.gold_gp);
        assert_eq!(c.thac0, c2.thac0);
        assert_eq!(c.saving_throws, c2.saving_throws);
    }

    #[test]
    fn backward_compat_old_json() {
        // Simulate loading an old save without the new fields
        let old_json = r#"{
            "name": "OldChar",
            "class": "Fighter",
            "level": 1,
            "abilities": {"strength":10,"intelligence":10,"wisdom":10,"dexterity":10,"constitution":10,"charisma":10},
            "hp": 5, "max_hp": 5, "ac": 9, "xp": 0,
            "inventory": [], "spells": []
        }"#;
        let c: Character = serde_json::from_str(old_json).unwrap();
        assert_eq!(c.name, "OldChar");
        assert_eq!(c.alignment, AlignmentId::from_enum(Alignment::Neutral)); // default
        assert_eq!(c.gold_gp, 0);   // default
        assert_eq!(c.thac0, 0);     // default
        assert!(c.saving_throws.is_none());
    }
}
