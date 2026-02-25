use std::fmt;
use rand::Rng;

use crate::rules::ability;

/// Result of a surprise check.
#[derive(Debug, Clone, PartialEq)]
pub enum SurpriseResult {
    /// Neither side is surprised.
    None,
    /// Party surprises monsters.
    PartySurprises,
    /// Monsters surprise party.
    MonstersSurprise,
    /// Both sides are surprised (extremely rare, both roll 1-2).
    BothSurprised,
}

impl fmt::Display for SurpriseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SurpriseResult::None => write!(f, "Neither side is surprised."),
            SurpriseResult::PartySurprises => {
                write!(f, "The party surprises the monsters! Party gets a free round.")
            }
            SurpriseResult::MonstersSurprise => {
                write!(f, "The monsters surprise the party! Monsters get a free round.")
            }
            SurpriseResult::BothSurprised => {
                write!(f, "Both sides are surprised! A moment of mutual confusion.")
            }
        }
    }
}

/// Check for surprise. Each side rolls d6; 1-2 = surprised.
pub fn check_surprise() -> (SurpriseResult, u32, u32) {
    check_surprise_with(&mut rand::thread_rng())
}

/// Testable version. Returns (result, party_roll, monster_roll).
pub fn check_surprise_with<R: Rng>(rng: &mut R) -> (SurpriseResult, u32, u32) {
    let party_roll: u32 = rng.gen_range(1..=6);
    let monster_roll: u32 = rng.gen_range(1..=6);

    let party_surprised = party_roll <= 2;
    let monster_surprised = monster_roll <= 2;

    let result = match (party_surprised, monster_surprised) {
        (true, true) => SurpriseResult::BothSurprised,
        (true, false) => SurpriseResult::MonstersSurprise,
        (false, true) => SurpriseResult::PartySurprises,
        (false, false) => SurpriseResult::None,
    };

    (result, party_roll, monster_roll)
}

/// Encounter distance in feet.
/// Dungeon: 2d6 × 10 feet.
/// Wilderness: 4d6 × 10 yards (converted to feet × 3).
/// If surprised: encounter distance is reduced (1d4 × 10 feet dungeon).
pub fn encounter_distance_dungeon(surprised: bool) -> u32 {
    encounter_distance_dungeon_with(&mut rand::thread_rng(), surprised)
}

/// Testable version.
pub fn encounter_distance_dungeon_with<R: Rng>(rng: &mut R, surprised: bool) -> u32 {
    if surprised {
        let roll: u32 = rng.gen_range(1..=4);
        roll * 10
    } else {
        let d1: u32 = rng.gen_range(1..=6);
        let d2: u32 = rng.gen_range(1..=6);
        (d1 + d2) * 10
    }
}

/// Wilderness encounter distance.
pub fn encounter_distance_wilderness(surprised: bool) -> u32 {
    encounter_distance_wilderness_with(&mut rand::thread_rng(), surprised)
}

/// Testable version. Returns distance in yards.
pub fn encounter_distance_wilderness_with<R: Rng>(rng: &mut R, surprised: bool) -> u32 {
    if surprised {
        let d1: u32 = rng.gen_range(1..=4);
        d1 * 10
    } else {
        let mut total: u32 = 0;
        for _ in 0..4 {
            total += rng.gen_range(1..=6);
        }
        total * 10
    }
}

/// Reaction roll result (NPC/monster reaction to party).
#[derive(Debug, Clone, PartialEq)]
pub enum Reaction {
    /// 2: Immediate attack.
    Hostile,
    /// 3-5: Unfriendly, may attack.
    Unfriendly,
    /// 6-8: Uncertain, monster hesitates.
    Uncertain,
    /// 9-11: No attack, monster may negotiate.
    Indifferent,
    /// 12: Enthusiastically friendly.
    Friendly,
}

impl Reaction {
    pub fn from_roll(modified_roll: i32) -> Self {
        match modified_roll {
            i if i <= 2 => Reaction::Hostile,
            3..=5 => Reaction::Unfriendly,
            6..=8 => Reaction::Uncertain,
            9..=11 => Reaction::Indifferent,
            _ => Reaction::Friendly,
        }
    }
}

impl fmt::Display for Reaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reaction::Hostile => write!(f, "Hostile — immediate attack!"),
            Reaction::Unfriendly => write!(f, "Unfriendly — may attack if provoked."),
            Reaction::Uncertain => write!(f, "Uncertain — monster hesitates."),
            Reaction::Indifferent => write!(f, "Indifferent — no attack, may negotiate."),
            Reaction::Friendly => write!(f, "Friendly — enthusiastically well-disposed."),
        }
    }
}

/// Roll a reaction check. 2d6 + CHA modifier of the speaking character.
pub fn reaction_roll(cha_score: i32) -> (Reaction, i32, i32) {
    reaction_roll_with(&mut rand::thread_rng(), cha_score)
}

/// Testable version. Returns (reaction, raw_roll, modified_roll).
pub fn reaction_roll_with<R: Rng>(rng: &mut R, cha_score: i32) -> (Reaction, i32, i32) {
    let cha_mod = ability::cha_reaction_mod(cha_score);

    // DSL gate: use reaction_roll mechanic if available
    #[cfg(feature = "dsl-backend")]
    if let Some(result) = dsl_reaction_roll(cha_mod) {
        return result;
    }

    native_reaction_roll_with(rng, cha_mod)
}

fn native_reaction_roll_with<R: Rng>(rng: &mut R, cha_mod: i32) -> (Reaction, i32, i32) {
    let d1: i32 = rng.gen_range(1..=6);
    let d2: i32 = rng.gen_range(1..=6);
    let raw = d1 + d2;
    let modified = raw + cha_mod;

    let reaction = Reaction::from_roll(modified);
    (reaction, raw, modified)
}

/// Try evaluating reaction via DSL `reaction_roll` mechanic.
/// Returns None on DSL error (caller falls through to native).
#[cfg(feature = "dsl-backend")]
fn dsl_reaction_roll(cha_mod: i32) -> Option<(Reaction, i32, i32)> {
    use crate::backend::{self, MechanicGroup};
    if !backend::is_dsl(MechanicGroup::Combat) {
        return None;
    }
    let runtime = backend::dsl()?;
    let mut handler = backend::SimpleDiceHandler::new();
    use ttrpg_interp::value::Value;
    match runtime.evaluate_mechanic(
        &backend::NullState,
        &mut handler,
        "reaction_roll",
        vec![Value::Int(cha_mod as i64)],
    ) {
        Ok(Value::EnumVariant { ref variant, .. }) => {
            let reaction = match variant.as_str() {
                "rx_hostile" => Reaction::Hostile,
                "rx_unfriendly" => Reaction::Unfriendly,
                "rx_neutral" => Reaction::Uncertain,
                "rx_indifferent" => Reaction::Indifferent,
                "rx_friendly" => Reaction::Friendly,
                _ => return None,
            };
            // Extract raw 2d6 from handler (sum of the two dice)
            let raw = handler
                .rolls
                .first()
                .map(|r| r.unmodified as i32)
                .unwrap_or(0);
            let modified = raw + cha_mod;
            Some((reaction, raw, modified))
        }
        _ => None,
    }
}

/// Evasion attempt result.
#[derive(Debug, Clone, PartialEq)]
pub struct EvasionResult {
    pub escaped: bool,
    /// d100 roll (None if auto-escape due to faster movement).
    pub roll: Option<i32>,
    /// Percentage chance needed to escape (None if auto-escape).
    pub chance: Option<i32>,
}

impl EvasionResult {
    fn auto_escape() -> Self {
        Self { escaped: true, roll: None, chance: None }
    }

    fn from_roll(roll: i32, chance: i32) -> Self {
        Self { escaped: roll <= chance, roll: Some(roll), chance: Some(chance) }
    }
}

impl fmt::Display for EvasionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.escaped, self.roll, self.chance) {
            (true, None, _) => {
                write!(f, "The party successfully evades the encounter! (party faster — automatic escape)")
            }
            (true, Some(roll), Some(chance)) => {
                write!(f, "The party successfully evades the encounter! (rolled {} vs {}%)", roll, chance)
            }
            (false, Some(roll), Some(chance)) => {
                write!(f, "Evasion fails! The monsters catch up to the party. (rolled {} vs {}%)", roll, chance)
            }
            _ => write!(f, "Evasion result: escaped={}", self.escaped),
        }
    }
}

/// Attempt to evade an encounter per OSE evasion rules.
///
/// If the party is faster than the monsters, evasion automatically succeeds.
/// Otherwise, chance is based on the OSE party-size-vs-monster-number table:
///   Party 1-4:   party outnumbers monsters 70%, monsters outnumber party 50%
///   Party 5-12:  party outnumbers monsters 50%, monsters outnumber party 35%
///   Party 13-24: party outnumbers monsters 35%, monsters outnumber party 25%
///   Party 25+:   party outnumbers monsters 25%, monsters outnumber party 10%
pub fn attempt_evasion(
    party_size: u32,
    party_movement: u32,
    monster_count: u32,
    monster_movement: u32,
) -> EvasionResult {
    attempt_evasion_with(
        &mut rand::thread_rng(),
        party_size,
        party_movement,
        monster_count,
        monster_movement,
    )
}

/// Testable version.
pub fn attempt_evasion_with<R: Rng>(
    rng: &mut R,
    party_size: u32,
    party_movement: u32,
    monster_count: u32,
    monster_movement: u32,
) -> EvasionResult {
    // Per OSE: if party is faster, evasion automatically succeeds
    if party_movement > monster_movement {
        return EvasionResult::auto_escape();
    }

    // OSE evasion table: chance depends on party size and relative numbers
    let fewer_monsters = monster_count <= party_size;
    let chance: i32 = match party_size {
        1..=4 => if fewer_monsters { 70 } else { 50 },
        5..=12 => if fewer_monsters { 50 } else { 35 },
        13..=24 => if fewer_monsters { 35 } else { 25 },
        _ => if fewer_monsters { 25 } else { 10 },
    };

    let roll: i32 = rng.gen_range(1..=100);
    EvasionResult::from_roll(roll, chance)
}

/// Full encounter sequence result.
#[derive(Debug)]
pub struct EncounterSequence {
    pub surprise: SurpriseResult,
    pub party_surprise_roll: u32,
    pub monster_surprise_roll: u32,
    pub distance: u32,
    pub messages: Vec<String>,
}

impl fmt::Display for EncounterSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for msg in &self.messages {
            writeln!(f, "{}", msg)?;
        }
        Ok(())
    }
}

/// Roll the full encounter sequence: surprise, distance.
/// The caller should then use reaction_roll or start combat.
pub fn begin_encounter_dungeon() -> EncounterSequence {
    begin_encounter_dungeon_with(&mut rand::thread_rng())
}

/// Testable version.
pub fn begin_encounter_dungeon_with<R: Rng>(rng: &mut R) -> EncounterSequence {
    let (surprise, p_roll, m_roll) = check_surprise_with(rng);
    let is_surprised = matches!(
        surprise,
        SurpriseResult::MonstersSurprise | SurpriseResult::PartySurprises
    );
    let distance = encounter_distance_dungeon_with(rng, is_surprised);

    let mut messages = Vec::new();
    messages.push(format!(
        "Surprise: party rolled {}, monsters rolled {}.",
        p_roll, m_roll
    ));
    messages.push(surprise.to_string());
    messages.push(format!("Encounter distance: {}' feet.", distance));

    EncounterSequence {
        surprise,
        party_surprise_roll: p_roll,
        monster_surprise_roll: m_roll,
        distance,
        messages,
    }
}

/// Begin a wilderness encounter.
pub fn begin_encounter_wilderness() -> EncounterSequence {
    begin_encounter_wilderness_with(&mut rand::thread_rng())
}

/// Testable version.
pub fn begin_encounter_wilderness_with<R: Rng>(rng: &mut R) -> EncounterSequence {
    let (surprise, p_roll, m_roll) = check_surprise_with(rng);
    let is_surprised = matches!(
        surprise,
        SurpriseResult::MonstersSurprise | SurpriseResult::PartySurprises
    );
    let distance_yards = encounter_distance_wilderness_with(rng, is_surprised);

    let mut messages = Vec::new();
    messages.push(format!(
        "Surprise: party rolled {}, monsters rolled {}.",
        p_roll, m_roll
    ));
    messages.push(surprise.to_string());
    messages.push(format!("Encounter distance: {} yards.", distance_yards));

    EncounterSequence {
        surprise,
        party_surprise_roll: p_roll,
        monster_surprise_roll: m_roll,
        distance: distance_yards,
        messages,
    }
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
    fn surprise_produces_valid_results() {
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let (result, p, m) = check_surprise_with(&mut rng);
            assert!((1..=6).contains(&p));
            assert!((1..=6).contains(&m));
            match (p <= 2, m <= 2) {
                (true, true) => assert_eq!(result, SurpriseResult::BothSurprised),
                (true, false) => assert_eq!(result, SurpriseResult::MonstersSurprise),
                (false, true) => assert_eq!(result, SurpriseResult::PartySurprises),
                (false, false) => assert_eq!(result, SurpriseResult::None),
            }
        }
    }

    #[test]
    fn dungeon_distance_normal_bounds() {
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let d = encounter_distance_dungeon_with(&mut rng, false);
            assert!(d >= 20 && d <= 120, "got distance {}", d);
        }
    }

    #[test]
    fn dungeon_distance_surprised_bounds() {
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let d = encounter_distance_dungeon_with(&mut rng, true);
            assert!(d >= 10 && d <= 40, "got distance {}", d);
        }
    }

    #[test]
    fn wilderness_distance_bounds() {
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let d = encounter_distance_wilderness_with(&mut rng, false);
            assert!(d >= 40 && d <= 240, "got distance {}", d);
        }
    }

    #[test]
    fn reaction_hostile() {
        assert_eq!(Reaction::from_roll(2), Reaction::Hostile);
        assert_eq!(Reaction::from_roll(1), Reaction::Hostile);
    }

    #[test]
    fn reaction_unfriendly() {
        assert_eq!(Reaction::from_roll(3), Reaction::Unfriendly);
        assert_eq!(Reaction::from_roll(5), Reaction::Unfriendly);
    }

    #[test]
    fn reaction_uncertain() {
        assert_eq!(Reaction::from_roll(6), Reaction::Uncertain);
        assert_eq!(Reaction::from_roll(8), Reaction::Uncertain);
    }

    #[test]
    fn reaction_indifferent() {
        assert_eq!(Reaction::from_roll(9), Reaction::Indifferent);
        assert_eq!(Reaction::from_roll(11), Reaction::Indifferent);
    }

    #[test]
    fn reaction_friendly() {
        assert_eq!(Reaction::from_roll(12), Reaction::Friendly);
        assert_eq!(Reaction::from_roll(15), Reaction::Friendly);
    }

    #[test]
    fn reaction_roll_with_high_cha() {
        // CHA 18 gives +2 modifier
        let mut rng = test_rng();
        let (_reaction, raw, modified) = reaction_roll_with(&mut rng, 18);
        assert!(modified >= raw); // CHA 18 gives positive modifier
    }

    #[test]
    fn reaction_roll_with_low_cha() {
        // CHA 3 gives -2 modifier
        let mut rng = test_rng();
        let (_reaction, raw, modified) = reaction_roll_with(&mut rng, 3);
        assert!(modified <= raw); // CHA 3 gives negative modifier
    }

    #[test]
    fn evasion_faster_party_auto_success() {
        // Per OSE: faster party always escapes
        for seed in 0..50 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 4, 120, 6, 90);
            assert!(result.escaped, "faster party should always escape");
            assert!(result.roll.is_none(), "auto-escape should have no roll");
            assert!(result.chance.is_none(), "auto-escape should have no chance");
        }
    }

    #[test]
    fn evasion_small_party_vs_more_monsters() {
        // Party 1-4, more monsters than party: 50% chance
        let mut escaped = 0;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 4, 90, 6, 120);
            assert_eq!(result.chance, Some(50), "should be 50% chance");
            assert!(result.roll.is_some(), "non-auto should have a roll");
            if result.escaped {
                escaped += 1;
            }
        }
        // Should be around 50% (allow margin)
        assert!(escaped > 70 && escaped < 130, "expected ~50% escape rate, got {}", escaped);
    }

    #[test]
    fn evasion_small_party_vs_fewer_monsters() {
        // Party 1-4, fewer monsters: 70% chance
        let mut escaped = 0;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 4, 90, 2, 120);
            assert_eq!(result.chance, Some(70), "should be 70% chance");
            if result.escaped {
                escaped += 1;
            }
        }
        assert!(escaped > 100, "expected ~70% escape rate, got {}", escaped);
    }

    #[test]
    fn evasion_large_party_low_chance() {
        // Party 25+, more monsters: 10% chance
        let mut escaped = 0;
        for seed in 0..1000 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 30, 60, 50, 120);
            assert_eq!(result.chance, Some(10), "should be 10% chance");
            if result.escaped {
                escaped += 1;
            }
        }
        assert!(escaped > 0, "should have at least some chance of escape");
        assert!(escaped < 200, "expected ~10% escape rate, got {}", escaped);
    }

    #[test]
    fn begin_encounter_dungeon_sequence() {
        let mut rng = test_rng();
        let seq = begin_encounter_dungeon_with(&mut rng);
        assert!(!seq.messages.is_empty());
        assert!(seq.messages.iter().any(|m| m.contains("Surprise")));
        assert!(seq.messages.iter().any(|m| m.contains("distance")));
    }

    #[test]
    fn begin_encounter_wilderness_sequence() {
        let mut rng = test_rng();
        let seq = begin_encounter_wilderness_with(&mut rng);
        assert!(!seq.messages.is_empty());
        assert!(seq.messages.iter().any(|m| m.contains("yards")));
    }

    #[test]
    fn surprise_display() {
        let s = SurpriseResult::PartySurprises.to_string();
        assert!(s.contains("surprises"));
    }

    #[test]
    fn reaction_display() {
        let s = Reaction::Hostile.to_string();
        assert!(s.contains("Hostile"));
    }

    #[test]
    fn evasion_display() {
        let auto = EvasionResult::auto_escape();
        assert!(auto.to_string().contains("evades"));
        assert!(auto.to_string().contains("automatic escape"));

        let rolled_ok = EvasionResult::from_roll(30, 50);
        assert!(rolled_ok.to_string().contains("evades"));
        assert!(rolled_ok.to_string().contains("rolled 30 vs 50%"));

        let rolled_fail = EvasionResult::from_roll(80, 50);
        assert!(rolled_fail.to_string().contains("fails"));
        assert!(rolled_fail.to_string().contains("rolled 80 vs 50%"));
    }
}
