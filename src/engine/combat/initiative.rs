//! Initiative rolling, spell declaration, and disruption tracking.
//!
//! Supports two initiative models selectable per game system via the DSL
//! `initiative_model` derive:
//! - "group" (default): 1d6 per side, winning side acts first
//! - "individual": 1d6 + DEX mod per combatant, sorted highest-first

use rand::Rng;

use crate::model::{Character, CombatState, InitiativeEntry};

/// Roll initiative using the DSL-selected model. For group initiative,
/// returns (party_roll, monster_roll). For individual initiative, returns
/// (0, 0) since ordering is stored in `combat.initiative_order`.
///
/// Advances the round counter and clears per-round state. (Spell declarations
/// and pending spells are cleared at the start of the declaration phase
/// so they survive the Declare → Initiative → Cast sequence.)
pub fn roll_initiative(combat: &mut CombatState, party: &[Character]) -> (i32, i32) {
    let model = crate::model::get_initiative_model();

    if model == "individual" {
        return roll_individual_initiative(combat, party);
    }

    // Default: group initiative
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some((p, m)) = dsl_group_initiative() {
            return apply_group_initiative(combat, p, m);
        }
        // DSL failed — fall through to native
    }

    roll_initiative_with(combat, party, &mut rand::thread_rng())
}

pub fn roll_initiative_with<R: Rng>(combat: &mut CombatState, _party: &[Character], rng: &mut R) -> (i32, i32) {
    let party_roll = rng.gen_range(1..=6i32);
    let monsters = rng.gen_range(1..=6i32);
    apply_group_initiative(combat, party_roll, monsters)
}

/// Roll a single individual initiative (1d6 + dex_mod), using DSL if available.
fn roll_one_individual(dex_mod: i32, _use_dsl: bool) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if _use_dsl {
        if let Some(r) = dsl_individual_initiative(dex_mod) {
            return r;
        }
    }
    rand::thread_rng().gen_range(1..=6i32) + dex_mod
}

fn clear_round_state(combat: &mut CombatState) {
    combat.round += 1;
    combat.disrupted.clear();
    combat.monsters_attacked_this_round.clear();
    combat.characters_acted.clear();
    combat.initiative_order.clear();
    combat.action_budget_used.clear();
    combat.phase = "Morale".to_string();
}

fn apply_group_initiative(combat: &mut CombatState, party: i32, monsters: i32) -> (i32, i32) {
    combat.party_initiative = party;
    combat.monster_initiative = monsters;
    clear_round_state(combat);

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

/// Roll individual initiative for all living combatants.
fn roll_individual_initiative(combat: &mut CombatState, party: &[Character]) -> (i32, i32) {
    #[cfg(feature = "dsl-backend")]
    let use_dsl = crate::backend::is_dsl(crate::backend::MechanicGroup::Combat);
    #[cfg(not(feature = "dsl-backend"))]
    let use_dsl = false;

    let mut order = Vec::new();

    // Roll for each living party member
    for (i, c) in party.iter().enumerate() {
        if !c.is_alive() {
            continue;
        }
        let dex_mod = crate::rules::ability::dex_init_mod(c.abilities.dexterity);
        let roll = roll_one_individual(dex_mod, use_dsl);
        order.push(InitiativeEntry {
            name: c.name.clone(),
            side: "character".to_string(),
            index: i,
            roll,
        });
    }

    // Roll for each living monster
    for (i, m) in combat.monsters.iter().enumerate() {
        if !m.is_alive() {
            continue;
        }
        // Monsters have no DEX score in OSE, so dex_mod = 0
        let roll = roll_one_individual(0, use_dsl);
        order.push(InitiativeEntry {
            name: m.name.clone(),
            side: "monster".to_string(),
            index: i,
            roll,
        });
    }

    // Sort highest roll first (ties broken by name for stability)
    order.sort_by(|a, b| b.roll.cmp(&a.roll).then_with(|| a.name.cmp(&b.name)));

    // Clear round state
    combat.party_initiative = 0;
    combat.monster_initiative = 0;
    clear_round_state(combat);

    // Build initiative order log message
    let order_desc: Vec<String> = order.iter()
        .map(|e| format!("{} ({})", e.name, e.roll))
        .collect();
    let msg = format!("Round {} — Individual Initiative: {}",
        combat.round, order_desc.join(", "));
    combat.log_event(msg);

    combat.initiative_order = order;
    combat.log_len_at_initiative = combat.log.len();
    (0, 0)
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

/// Roll individual initiative via DSL `individual_initiative` mechanic.
#[cfg(feature = "dsl-backend")]
fn dsl_individual_initiative(dex_mod: i32) -> Option<i32> {
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
    }

    impl EffectHandler for InitHandler {
        fn handle(&mut self, effect: Effect) -> Response {
            self.inner.handle(effect)
        }
    }

    let runtime = crate::backend::dsl()?;
    let mut handler = InitHandler { inner: BridgeHandler::new() };
    let result = runtime.evaluate_mechanic(
        &NullState, &mut handler, "individual_initiative",
        vec![Value::Int(dex_mod as i64)],
    ).ok()?;

    match result {
        Value::Int(n) => Some(n as i32),
        _ => None,
    }
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
