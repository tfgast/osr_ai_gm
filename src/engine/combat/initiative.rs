//! Initiative rolling, spell declaration, and disruption tracking.

use rand::Rng;

use crate::model::CombatState;

/// Roll group initiative (1d6 per side). Advances the round counter
/// and clears disruptions from the previous round. (Spell declarations
/// and pending spells are cleared at the start of the declaration phase
/// so they survive the Declare → Initiative → Cast sequence.)
///
/// When the DSL backend is enabled, delegates to the `group_initiative`
/// mechanic for each side's roll. Falls back to native on DSL failure.
pub fn roll_initiative(combat: &mut CombatState) -> (i32, i32) {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some((p, m)) = dsl_group_initiative() {
            return apply_initiative(combat, p, m);
        }
        // DSL failed — fall through to native
    }

    roll_initiative_with(combat, &mut rand::thread_rng())
}

pub fn roll_initiative_with<R: Rng>(combat: &mut CombatState, rng: &mut R) -> (i32, i32) {
    let party = rng.gen_range(1..=6i32);
    let monsters = rng.gen_range(1..=6i32);
    apply_initiative(combat, party, monsters)
}

fn apply_initiative(combat: &mut CombatState, party: i32, monsters: i32) -> (i32, i32) {
    combat.party_initiative = party;
    combat.monster_initiative = monsters;
    combat.round += 1;
    combat.disrupted.clear();
    combat.monsters_attacked_this_round.clear();
    combat.characters_acted.clear();
    combat.phase = "Morale".to_string();

    let winner = if party > monsters {
        "Party acts first"
    } else if monsters > party {
        "Monsters act first"
    } else {
        "Simultaneous actions"
    };
    let msg = format!("Round {} — Initiative: Party {} vs Monsters {} — {}",
        combat.round, party, monsters, winner);
    combat.log_event(msg);
    combat.log_len_at_initiative = combat.log.len();
    (party, monsters)
}

// ── DSL backend ──────────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
fn dsl_group_initiative() -> Option<(i32, i32)> {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider};
    use ttrpg_interp::value::Value;

    use crate::bridge::handler::BridgeHandler;

    struct NullState;
    impl StateProvider for NullState {
        fn read_field(&self, _: &EntityRef, _: &str) -> Option<Value> { None }
        fn read_conditions(&self, _: &EntityRef) -> Option<Vec<ActiveCondition>> { None }
        fn read_turn_budget(&self, _: &EntityRef) -> Option<BTreeMap<Name, Value>> { None }
        fn read_enabled_options(&self) -> Vec<Name> { Vec::new() }
        fn position_eq(&self, _: &Value, _: &Value) -> bool { false }
        fn distance(&self, _: &Value, _: &Value) -> Option<i64> { None }
    }

    struct InitHandler {
        inner: BridgeHandler,
        roll_total: Option<i64>,
    }

    impl EffectHandler for InitHandler {
        fn handle(&mut self, effect: Effect) -> Response {
            let response = self.inner.handle(effect);
            if let Response::Rolled(ref result) = response {
                self.roll_total = Some(result.total);
            }
            response
        }
    }

    let runtime = crate::backend::dsl()?;

    // Roll for party
    let mut party_handler = InitHandler { inner: BridgeHandler::new(), roll_total: None };
    let party_result = runtime.evaluate_mechanic(
        &NullState, &mut party_handler, "group_initiative", vec![],
    ).ok()?;
    let party = match party_result {
        Value::Int(n) => n as i32,
        _ => return None,
    };

    // Roll for monsters
    let mut monster_handler = InitHandler { inner: BridgeHandler::new(), roll_total: None };
    let monster_result = runtime.evaluate_mechanic(
        &NullState, &mut monster_handler, "group_initiative", vec![],
    ).ok()?;
    let monsters = match monster_result {
        Value::Int(n) => n as i32,
        _ => return None,
    };

    Some((party, monsters))
}

/// Declare a spell cast for a character (must be done during declaration phase).
/// If the caster takes damage before the magic phase, the spell is disrupted.
pub fn declare_spell(combat: &mut CombatState, character_name: &str, spell_name: &str) {
    combat.spell_declarations.push(character_name.to_string());
    combat.pending_spells.push((character_name.to_string(), spell_name.to_string()));
    combat.log_event(format!("{} declares: casting {}", character_name, spell_name));
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
        combat.log_event(format!("{}'s spell is DISRUPTED!", character_name));
    }
}
