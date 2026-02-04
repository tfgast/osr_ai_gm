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
    let d1: i32 = rng.gen_range(1..=6);
    let d2: i32 = rng.gen_range(1..=6);
    let raw = d1 + d2;
    let cha_mod = ability::cha_reaction_mod(cha_score);
    let modified = raw + cha_mod;

    let reaction = Reaction::from_roll(modified);
    (reaction, raw, modified)
}

/// Evasion attempt result.
#[derive(Debug, Clone, PartialEq)]
pub enum EvasionResult {
    /// Party successfully evades.
    Escaped,
    /// Evasion fails, encounter proceeds.
    Caught,
}

impl fmt::Display for EvasionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvasionResult::Escaped => write!(f, "The party successfully evades the encounter!"),
            EvasionResult::Caught => {
                write!(f, "Evasion fails! The monsters catch up to the party.")
            }
        }
    }
}

/// Attempt to evade an encounter.
/// Success based on party size vs monster number and relative speed.
///
/// OSE evasion: if party is smaller, 50% chance (d% <= 50) per round of chase.
/// If party is faster, automatic success after initial check.
/// Simplified: roll d% — if party movement >= monster movement, succeed on 1-70.
/// If party is slower, succeed on 1-50. If greatly outnumbered, -25%.
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
    let mut chance: i32 = 50;

    // Faster party gets +20%
    if party_movement > monster_movement {
        chance += 20;
    }
    // Slower party gets -20%
    if party_movement < monster_movement {
        chance -= 20;
    }
    // Smaller party gets +10% (easier to hide)
    if party_size < monster_count {
        chance += 10;
    }
    // Greatly outnumbered (4:1+) gets -10%
    if monster_count >= party_size * 4 {
        chance -= 10;
    }

    chance = chance.max(5).min(95); // Always at least 5%, at most 95%

    let roll: i32 = rng.gen_range(1..=100);
    if roll <= chance {
        EvasionResult::Escaped
    } else {
        EvasionResult::Caught
    }
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
    messages.push(format!("{}", surprise));
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
    messages.push(format!("{}", surprise));
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
    fn evasion_faster_party() {
        let mut escaped = 0;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 4, 120, 6, 90);
            if result == EvasionResult::Escaped {
                escaped += 1;
            }
        }
        // Faster party should escape more often (70% base)
        assert!(escaped > 50, "faster party should escape frequently, got {}", escaped);
    }

    #[test]
    fn evasion_slower_party() {
        let mut escaped = 0;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 4, 90, 6, 120);
            if result == EvasionResult::Escaped {
                escaped += 1;
            }
        }
        // Slower party should escape less often (30% base)
        assert!(escaped < 60, "slower party should not escape too often, got {}", escaped);
    }

    #[test]
    fn evasion_always_has_some_chance() {
        // Even worst case should have at least 5%
        let mut escaped = 0;
        for seed in 0..1000 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = attempt_evasion_with(&mut rng, 1, 60, 100, 180);
            if result == EvasionResult::Escaped {
                escaped += 1;
            }
        }
        assert!(escaped > 0, "should have at least some chance of escape");
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
        let s = format!("{}", SurpriseResult::PartySurprises);
        assert!(s.contains("surprises"));
    }

    #[test]
    fn reaction_display() {
        let s = format!("{}", Reaction::Hostile);
        assert!(s.contains("Hostile"));
    }

    #[test]
    fn evasion_display() {
        let s = format!("{}", EvasionResult::Escaped);
        assert!(s.contains("evades"));
    }
}
