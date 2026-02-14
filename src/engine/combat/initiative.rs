//! Initiative rolling, spell declaration, and disruption tracking.

use rand::Rng;

use crate::model::CombatState;

/// Roll group initiative (1d6 per side). Advances the round counter
/// and clears spell declarations/disruptions from the previous round.
pub fn roll_initiative(combat: &mut CombatState) -> (i32, i32) {
    roll_initiative_with(combat, &mut rand::thread_rng())
}

pub fn roll_initiative_with<R: Rng>(combat: &mut CombatState, rng: &mut R) -> (i32, i32) {
    let party = rng.gen_range(1..=6i32);
    let monsters = rng.gen_range(1..=6i32);
    combat.party_initiative = party;
    combat.monster_initiative = monsters;
    combat.round += 1;
    combat.spell_declarations.clear();
    combat.disrupted.clear();
    combat.phase = crate::model::CombatPhase::Morale;

    let winner = if party > monsters {
        "Party acts first"
    } else if monsters > party {
        "Monsters act first"
    } else {
        "Simultaneous actions"
    };
    let msg = format!("Round {} — Initiative: Party {} vs Monsters {} — {}",
        combat.round, party, monsters, winner);
    combat.log.push(msg);
    combat.log_len_at_initiative = combat.log.len();
    (party, monsters)
}

/// Declare a spell cast for a character (must be done during declaration phase).
/// If the caster takes damage before the magic phase, the spell is disrupted.
pub fn declare_spell(combat: &mut CombatState, character_name: &str, spell_name: &str) {
    combat.spell_declarations.push(character_name.to_string());
    combat.log.push(format!("{} declares: casting {}", character_name, spell_name));
}

/// Check if a character's spell was disrupted this round.
pub fn is_disrupted(combat: &CombatState, character_name: &str) -> bool {
    combat.disrupted.iter().any(|n| n.eq_ignore_ascii_case(character_name))
}

/// Mark a spell-casting character as disrupted (called internally when they take damage).
pub(super) fn disrupt_caster(combat: &mut CombatState, character_name: &str) {
    let is_casting = combat.spell_declarations.iter()
        .any(|n| n.eq_ignore_ascii_case(character_name));
    let already_disrupted = combat.disrupted.iter()
        .any(|n| n.eq_ignore_ascii_case(character_name));
    if is_casting && !already_disrupted {
        combat.disrupted.push(character_name.to_string());
        combat.log.push(format!("{}'s spell is DISRUPTED!", character_name));
    }
}
