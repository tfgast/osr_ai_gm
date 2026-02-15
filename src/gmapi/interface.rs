use crate::engine::{gm, party, retainers, system};
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::persist::GameState;
use crate::rules::alignment::Alignment;
use crate::rules::class::Class;
use super::{combat_handlers, exploration_handlers, query_handlers};
use super::ok_with_typed_data;
use serde::Serialize;

/// Process a GMRequest against the current GameState and return a GMResponse.
pub fn handle_request(req: &GMRequest, state: &mut GameState) -> GMResponse {
    let id = &req.id;
    let response = match &req.command {
        // -- State queries --
        GMCommand::QueryState => query_handlers::query_state(id, state),
        GMCommand::QueryMode => GMResponse::ok_with_data(
            id, format!("current mode: {}", state.mode), state.mode,
            serde_json::json!({ "mode": state.mode.to_string() }),
        ),
        GMCommand::QueryParty => query_handlers::query_party(id, state),
        GMCommand::QueryCombat => query_handlers::query_combat(id, state),
        GMCommand::QueryExploration => query_handlers::query_exploration(id, state),
        GMCommand::Look => exploration_handlers::look(id, state),
        GMCommand::QueryWilderness => query_handlers::query_wilderness(id, state),

        // -- Character management --
        GMCommand::CreateCharacter { name, class, alignment, abilities } => {
            create_character(id, state, name, *class, *alignment, abilities.as_ref())
        }

        // -- Combat --
        GMCommand::SpawnEncounter(params) => combat_handlers::spawn_encounter(id, state, params),
        GMCommand::AddMonster(params) => combat_handlers::add_monster(id, state, params),
        GMCommand::RollInitiative => combat_handlers::roll_initiative(id, state),
        GMCommand::Attack { character, monster_idx, weapon } => {
            combat_handlers::attack(id, state, character, *monster_idx, weapon)
        }
        GMCommand::MonsterAttack { monster_idx, character } => {
            combat_handlers::monster_attack(id, state, *monster_idx, character)
        }
        GMCommand::CheckMorale => combat_handlers::check_morale(id, state),
        GMCommand::TurnUndead { character, monster_idx } => {
            combat_handlers::turn_undead(id, state, character, *monster_idx)
        }
        GMCommand::Close { character, feet } => combat_handlers::close(id, state, character, *feet),
        GMCommand::Retreat { character } => combat_handlers::retreat(id, state, character.as_deref()),
        GMCommand::FightingWithdrawal { character } => combat_handlers::fighting_withdrawal(id, state, character.as_deref()),
        GMCommand::QueryCombatLog => combat_handlers::query_combat_log(id, state),
        GMCommand::DeclareSpell { character, spell } => {
            combat_handlers::declare_spell(id, state, character, spell)
        }
        GMCommand::CastSpell { character } => {
            combat_handlers::cast_spell(id, state, character)
        }
        GMCommand::NextPhase => combat_handlers::next_phase(id, state),
        GMCommand::EndCombat { skip_xp } => combat_handlers::end_combat(id, state, *skip_xp),

        // -- Exploration --
        GMCommand::EnterDungeon { level, room_name } => {
            exploration_handlers::enter_dungeon(id, state, *level, room_name)
        }
        GMCommand::AdvanceTurn => exploration_handlers::advance_turn(id, state),
        GMCommand::AddRoom { id: room_id, name } => exploration_handlers::add_room(id, state, *room_id, name),
        GMCommand::AddDoor { id: door_id, room_a, room_b, state: door_state } => {
            exploration_handlers::add_door(id, state, *door_id, *room_a, *room_b, *door_state)
        }
        GMCommand::MoveRoom { door_id } => exploration_handlers::move_room(id, state, *door_id),
        GMCommand::Search { is_elf } => exploration_handlers::search(id, state, *is_elf),
        GMCommand::Light { source, carrier } => exploration_handlers::light(id, state, *source, carrier),
        GMCommand::LoadModule { path } => exploration_handlers::load_module(id, state, path),
        GMCommand::OpenDoor { door_id } => exploration_handlers::open_door(id, state, *door_id),
        GMCommand::ForceDoor { door_id, character } => exploration_handlers::force_door(id, state, *door_id, character),
        GMCommand::PickLock { door_id, character } => exploration_handlers::pick_lock(id, state, *door_id, character),
        GMCommand::Listen { is_demihuman } => exploration_handlers::listen(id, state, *is_demihuman),
        GMCommand::Rest => exploration_handlers::rest(id, state),

        // -- Wilderness --
        GMCommand::EnterWilderness { terrain } => exploration_handlers::enter_wilderness(id, state, *terrain),
        GMCommand::LeaveWilderness => exploration_handlers::leave_wilderness(id, state),
        GMCommand::AddHex { x, y, terrain } => exploration_handlers::add_hex(id, state, *x, *y, *terrain),
        GMCommand::Travel { x, y } => exploration_handlers::travel(id, state, *x, *y),
        GMCommand::Orient => exploration_handlers::orient(id, state),
        GMCommand::Forage => exploration_handlers::forage(id, state),
        GMCommand::Hunt => exploration_handlers::hunt(id, state),
        GMCommand::RollEncounter => combat_handlers::roll_encounter(id, state),
        GMCommand::Evade { monster_count, monster_movement } => {
            combat_handlers::evade(id, state, *monster_count, *monster_movement)
        }

        // -- Encounter resolution --
        GMCommand::RollSurprise => combat_handlers::roll_surprise(id, state),
        GMCommand::RollReaction { character } => combat_handlers::roll_reaction(id, state, character),

        // -- Management --
        GMCommand::AwardXp { character, xp } => award_xp(id, state, character, *xp),
        GMCommand::AwardTreasureXp { character, treasure_gp, monster_xp } => {
            award_treasure_xp(id, state, character, *treasure_gp, *monster_xp)
        }
        GMCommand::ThiefSkillCheck { character, skill } => {
            thief_skill_check(id, state, character, skill)
        }
        GMCommand::Backstab { character, monster_idx, weapon } => {
            combat_handlers::backstab(id, state, character, *monster_idx, weapon)
        }
        GMCommand::QueryEncumbrance { character } => query_handlers::query_encumbrance(id, state, character),
        GMCommand::SpawnMonster { name, count, distance } => {
            combat_handlers::spawn_monster(id, state, name, *count, *distance)
        }
        GMCommand::SpawnNpcParty { party_type, distance } => {
            combat_handlers::spawn_npc_party(id, state, party_type, *distance)
        }
        GMCommand::LookupSpell { name, list } => query_handlers::lookup_spell(id, state, name, list),
        GMCommand::HireRetainer { employer, retainer_name, retainer_class, retainer_level } => {
            hire_retainer(id, state, employer, retainer_name, *retainer_class, *retainer_level)
        }
        GMCommand::LoyaltyCheck { retainer_name, loyalty } => {
            loyalty_check(id, state, retainer_name, *loyalty)
        }
        GMCommand::LevelUp { character } => level_up(id, state, character),
        GMCommand::Ruling { text } => ruling(id, state, text),
        GMCommand::ListNotes => list_notes(id, state),
        GMCommand::DeleteNote { index } => delete_note(id, state, *index),
        GMCommand::ListRetainers => list_retainers(id, state),
        GMCommand::DismissRetainer { name } => dismiss_retainer(id, state, name),

        // -- Fiat commands --
        GMCommand::Heal { character, amount } => heal(id, state, character, *amount),
        GMCommand::Damage { character, amount } => damage(id, state, character, *amount),
        GMCommand::SetHp { character, hp } => set_hp(id, state, character, *hp),
        GMCommand::SetHelpless { monster_idx, helpless } => {
            combat_handlers::set_helpless(id, state, *monster_idx, *helpless)
        }
        GMCommand::Kill { character, monster_idx } => {
            combat_handlers::kill(id, state, character, *monster_idx)
        }
        GMCommand::SetRations { amount } => set_rations(id, state, *amount),
        GMCommand::AddRations { amount } => add_rations(id, state, *amount),
        GMCommand::AddGold { character, amount } => add_gold(id, state, character, *amount),

        // -- Inventory --
        GMCommand::ListEquipment { category } => exploration_handlers::list_equipment(id, state, category),
        GMCommand::Buy { character, item_name } => exploration_handlers::buy(id, state, character, item_name),
        GMCommand::Drop { character, item_name } => exploration_handlers::drop(id, state, character, item_name),
        GMCommand::Equip { character, item_name } => exploration_handlers::equip(id, state, character, item_name),
        GMCommand::Unequip { character, item_name } => exploration_handlers::unequip(id, state, character, item_name),
        GMCommand::Loot { character, item_name, value_gp } => exploration_handlers::loot(id, state, character, item_name, *value_gp),

        // -- Rumors --
        GMCommand::RollRumor { table } => query_handlers::roll_rumor(id, state, table),
        GMCommand::LookupRumorTable { table } => query_handlers::lookup_rumor_table(id, state, table),
        GMCommand::ListRumorTables => query_handlers::list_rumor_tables(id, state),

        // -- Lookup & reference --
        GMCommand::LookupItem { name } => query_handlers::lookup_item(id, state, name),
        GMCommand::SearchItems { query } => query_handlers::search_items(id, state, query),
        GMCommand::LookupTreasureType { letter } => query_handlers::lookup_treasure_type(id, state, letter),
        GMCommand::RollTreasure { letter } => query_handlers::roll_treasure(id, state, letter),
        GMCommand::ListClasses => query_handlers::list_classes(id, state),
        GMCommand::EligibleClasses { abilities } => query_handlers::eligible_classes(id, state, abilities),

        // -- System --
        GMCommand::Save { path } => save_game(id, state, path),
        GMCommand::Load { path } => load_game(id, state, path),
        GMCommand::Roll { notation } => roll_dice(id, state, notation),
        GMCommand::Quit => quit_session(id, state),
    };
    state.assert_mode_invariants();
    response
}

// =============================================================================
// Character management
// =============================================================================

#[derive(Serialize)]
struct CreateCharacterData {
    character_sheet: String,
}

fn create_character(id: &str, state: &mut GameState, name: &str, class: Class, alignment: Alignment, provided_abilities: Option<&[i32; 6]>) -> GMResponse {
    match party::action_create_character(state, name, class, alignment, provided_abilities.copied()) {
        Ok(result) => {
            if !result.created {
                let source = if result.used_provided_abilities { "provided" } else { "rolled" };
                return GMResponse::err(
                    id,
                    format!("{} abilities do not meet requirements for {}.", source, class.name()),
                    state.mode,
                );
            }
            ok_with_typed_data(
                id,
                state,
                format!("{} created and added to party.", name),
                CreateCharacterData {
                    character_sheet: result.character_sheet.unwrap_or_default(),
                },
            )
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

// =============================================================================
// Management
// =============================================================================

fn award_xp(id: &str, state: &mut GameState, char_name: &str, xp_amount: u64) -> GMResponse {
    match gm::action_award_xp(state, char_name, xp_amount, false) {
        Ok(result) => GMResponse::ok_with_data(
            id,
            format!("{} awarded {} XP (total: {}).", result.character, result.base_xp, result.total_xp),
            state.mode,
            serde_json::json!({
                "character": result.character,
                "xp_awarded": result.base_xp,
                "total_xp": result.total_xp,
            }),
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn award_treasure_xp(id: &str, state: &mut GameState, char_name: &str, treasure_gp: u64, monster_xp: u64) -> GMResponse {
    match gm::action_award_treasure_xp(state, char_name, treasure_gp, monster_xp) {
        Ok(result) => {
            let mut msg = format!(
                "{}: base {}xp ({}gp treasure + {}xp monsters), {:+}% prime req modifier = {} adjusted XP. Total: {}.",
                result.character, result.base_xp, result.treasure_gp, result.monster_xp,
                result.modifier_pct, result.adjusted_xp, result.total_xp,
            );
            if result.ready_to_train {
                msg.push_str(" Ready to train!");
            }
            ok_with_typed_data(id, state, msg, result)
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn thief_skill_check(id: &str, state: &GameState, char_name: &str, skill_name: &str) -> GMResponse {
    match gm::action_thief_skill_check(state, char_name, skill_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn hire_retainer(id: &str, state: &mut GameState, employer_name: &str, ret_name: &str, ret_class: Class, ret_level: u32) -> GMResponse {
    match retainers::action_hire_retainer(
        state,
        ret_name,
        ret_class,
        Some(employer_name),
        ret_level,
        retainers::results::HireRetainerMode::AssessOnly,
    ) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn loyalty_check(id: &str, state: &GameState, ret_name: &str, loyalty: u32) -> GMResponse {
    match retainers::action_loyalty_check(ret_name, loyalty) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn level_up(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    match gm::action_level_up(state, char_name) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("{} leveled up to {}! Gained {} HP. HP: {}/{}.",
                result.character, result.new_level, result.hp_gained,
                result.current_hp, result.new_hp),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn heal(id: &str, state: &mut GameState, char_name: &str, amount: i32) -> GMResponse {
    match gm::action_heal(state, char_name, amount) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("{} healed {} HP ({} -> {}/{}).", result.character, result.healed, result.old_hp, result.hp, result.max_hp),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn damage(id: &str, state: &mut GameState, char_name: &str, amount: i32) -> GMResponse {
    match gm::action_damage(state, char_name, amount) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("{} takes {} damage ({} -> {}/{}). Status: {}.", result.character, result.damage, result.old_hp, result.hp, result.max_hp, result.status),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn set_hp(id: &str, state: &mut GameState, char_name: &str, hp: i32) -> GMResponse {
    match gm::action_set_hp(state, char_name, hp) {
        Ok(result) => {
            let mut msg = format!("{} HP set to {} (was {}). Max HP: {}. Status: {}.", result.character, result.hp, result.old_hp, result.max_hp, result.status);
            if let Some(warning) = &result.warning {
                msg.push_str(&format!(" WARNING: {}", warning));
            }
            ok_with_typed_data(id, state, msg, result)
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn set_rations(id: &str, state: &mut GameState, amount: u32) -> GMResponse {
    match gm::action_set_rations(state, amount) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("rations set to {} person-days (was {}).", result.rations, result.old_rations),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn add_rations(id: &str, state: &mut GameState, amount: u32) -> GMResponse {
    match gm::action_add_rations(state, amount) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("added {} rations. Total: {} person-days.", result.added, result.rations),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn add_gold(id: &str, state: &mut GameState, char_name: &str, amount: u32) -> GMResponse {
    match gm::action_add_gold(state, char_name, amount) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("{} gained {}gp ({} → {}).", result.character, result.added, result.old_gold_gp, result.gold_gp),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn ruling(id: &str, state: &mut GameState, text: &str) -> GMResponse {
    match gm::action_ruling(state, text) {
        Ok(result) => GMResponse::ok(
            id,
            format!("ruling recorded: {}", result.text),
            state.mode,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn list_notes(id: &str, state: &GameState) -> GMResponse {
    match gm::action_list_notes(state) {
        Ok(result) => {
            if result.notes.is_empty() {
                return GMResponse::ok_with_data(
                    id, "no notes yet.", state.mode,
                    serde_json::json!({ "notes": [] }),
                );
            }

            let mut msg = String::from("Session notes:\n");
            for note in &result.notes {
                msg.push_str(&format!("  [{}] {}\n", note.index, note.text));
            }

            ok_with_typed_data(id, state, msg, result)
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn delete_note(id: &str, state: &mut GameState, index: usize) -> GMResponse {
    match gm::action_delete_note(state, index) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("deleted note [{}]: {}", result.index, result.deleted),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn list_retainers(id: &str, state: &GameState) -> GMResponse {
    match retainers::action_list_retainers(state) {
        Ok(result) => {
            if result.retainers.is_empty() {
                return GMResponse::ok_with_data(
                    id, "no retainers.", state.mode,
                    serde_json::json!({ "retainers": [] }),
                );
            }

            let mut msg = format!("Retainers ({}):\n", result.retainers.len());
            for r in &result.retainers {
                let status = if r.alive {
                    format!("HP {}/{}, Loyalty {}, Wage {} gp/mo", r.hp, r.max_hp, r.loyalty, r.wage_gp)
                } else {
                    "DEAD".to_string()
                };
                msg.push_str(&format!("  {} ({} L{}) — {}\n", r.name, r.class, r.level, status));
            }

            ok_with_typed_data(id, state, msg, result)
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn dismiss_retainer(id: &str, state: &mut GameState, name: &str) -> GMResponse {
    match retainers::action_dismiss_retainer(state, name) {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("{} ({}) dismissed from service.", result.name, result.class),
            result,
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

// =============================================================================
// System
// =============================================================================

fn quit_session(id: &str, state: &GameState) -> GMResponse {
    match system::action_quit() {
        Ok(_) => GMResponse::ok(id, "session ended.", state.mode),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

fn save_game(id: &str, state: &GameState, path: &str) -> GMResponse {
    match system::action_save_game(state, path) {
        Ok(result) => GMResponse::ok(
            id,
            format!("game saved to {}.", result.path.display()),
            state.mode,
        ),
        Err(e) => GMResponse::err(id, format!("save failed: {}.", e), state.mode),
    }
}

fn load_game(id: &str, state: &mut GameState, path: &str) -> GMResponse {
    match system::action_load_game(state, path) {
        Ok(result) => {
            let msg = format!(
                "loaded: turn {}, dungeon level {}, {} party members.",
                result.turn, result.dungeon_level, result.party_members,
            );
            GMResponse::ok(id, msg, state.mode)
        }
        Err(e) => GMResponse::err(id, format!("load failed: {}.", e), state.mode),
    }
}

fn roll_dice(id: &str, state: &GameState, notation: &str) -> GMResponse {
    match system::action_roll_dice(notation) {
        Ok(result) => GMResponse::ok_with_data(
            id,
            result.rendered,
            state.mode,
            serde_json::json!({ "total": result.total }),
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmapi::protocol::EncounterParams;
    use crate::model::CombatState;
    use crate::state::game::GameMode;
    use crate::state::time::LightSourceKind;
    use crate::state::wilderness::Terrain;

    fn make_req(id: &str, command: GMCommand) -> GMRequest {
        GMRequest { id: id.to_string(), command }
    }

    #[test]
    fn query_state_empty() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryState), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["party_size"], 0);
        assert_eq!(data["mode"], "idle");
    }

    #[test]
    fn query_mode() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryMode), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("idle"));
    }

    #[test]
    fn query_party_empty() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryParty), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("no party"));
    }

    #[test]
    fn query_combat_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryCombat), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn create_character_and_query() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::CreateCharacter {
            name: "Aldric".to_string(),
            class: Class::Fighter,
            alignment: Alignment::Lawful,
            abilities: None,
        }), &mut state);
        // Character creation might fail due to random ability rolls not meeting requirements.
        // So we just verify the response is well-formed.
        assert!(!resp.id.is_empty());

        // If it succeeded, verify party has a member.
        if resp.success {
            let resp2 = handle_request(&make_req("2", GMCommand::QueryParty), &mut state);
            assert!(resp2.success);
            let data = resp2.data.unwrap();
            let members = data["members"].as_array().unwrap();
            assert_eq!(members.len(), 1);
        }
    }

    #[test]
    fn spawn_encounter_and_query() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 3,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 60,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Combat);

        let resp2 = handle_request(&make_req("2", GMCommand::QueryCombat), &mut state);
        assert!(resp2.success);
        let data = resp2.data.unwrap();
        assert_eq!(data["monsters"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn spawn_encounter_invalid_morale() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 13,
            distance: 60,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn combat_already_active() {
        let mut state = GameState::new();
        state.combat = Some(CombatState::new(vec![], 0));
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "orc".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 4,
            damage: "1d6".to_string(),
            morale: 8,
            distance: 30,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("already active"));
    }

    #[test]
    fn initiative_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::RollInitiative), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn initiative_twice_without_action_rejected() {
        let mut state = GameState::new();
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        state.party.add_member(c);

        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "Goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 5,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        let resp = handle_request(&make_req("2", GMCommand::RollInitiative), &mut state);
        assert!(resp.success, "first initiative should succeed");

        let resp = handle_request(&make_req("3", GMCommand::RollInitiative), &mut state);
        assert!(!resp.success, "second initiative without action should fail");
        assert!(resp.error.unwrap().contains("initiative already rolled"));
        assert_eq!(state.combat.as_ref().unwrap().round, 1, "round should not advance");
    }

    #[test]
    fn end_combat_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EndCombat { skip_xp: false }), &mut state);
        assert!(!resp.success);
    }

    /// Regression: EndCombat when no combat active must not change the game mode.
    /// Bug oag-uee9i: calling EndCombat in Exploration mode (no combat) changed
    /// the mode from Exploration to Idle.
    #[test]
    fn end_combat_no_combat_preserves_mode() {
        use crate::state::dungeon::DungeonState;

        let mut state = GameState::new();

        // Set up exploration mode
        let dungeon = DungeonState::new(1);
        state.enter_exploration(dungeon, 1);
        assert_eq!(state.mode, GameMode::Exploration);

        // Add a character so we can enter combat
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        state.party.add_member(c);

        // Enter and end a valid combat
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Combat);

        let resp = handle_request(&make_req("2", GMCommand::EndCombat { skip_xp: false }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Exploration, "after first EndCombat mode should be Exploration");

        // Now call EndCombat again with no active combat
        let resp = handle_request(&make_req("3", GMCommand::EndCombat { skip_xp: false }), &mut state);
        assert!(!resp.success, "EndCombat with no combat should fail");
        assert_eq!(
            state.mode, GameMode::Exploration,
            "failed EndCombat must not change mode from Exploration to Idle"
        );
        assert_eq!(
            resp.mode, GameMode::Exploration,
            "response mode must reflect actual state mode"
        );
    }

    #[test]
    fn retreat_auto_resolves_free_attacks() {
        let mut state = GameState::new();
        // Create a character
        let resp = handle_request(&make_req("1", GMCommand::CreateCharacter {
            name: "Aldric".to_string(),
            class: Class::Fighter,
            alignment: Alignment::Lawful,
            abilities: None,
        }), &mut state);
        assert!(resp.success);

        // Give character lots of HP so they survive the free attacks
        if let Some(c) = state.party.find_member_mut("Aldric") {
            c.hp = 100;
            c.max_hp = 100;
        }

        // Spawn goblins for combat
        let resp = handle_request(&make_req("2", GMCommand::SpawnEncounter(EncounterParams {
            name: "Goblin".to_string(),
            count: 2,
            hit_dice: "1-1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Combat);

        // Retreat
        let resp = handle_request(&make_req("3", GMCommand::Retreat {
            character: Some("Aldric".to_string()),
        }), &mut state);
        assert!(resp.success, "retreat should succeed: {}", resp.message);

        // Check the response data contains free attacks
        let data = resp.data.unwrap();
        assert!(data.get("free_attacks").is_some(), "response should contain free_attacks");
        let free_attacks = data["free_attacks"].as_array().unwrap();
        assert_eq!(free_attacks.len(), 2, "both goblins should get free attacks");

        // Each attack should have +2 modifier
        for atk in free_attacks {
            assert_eq!(atk["modifiers"], 2, "free attack should be at +2");
        }

        // Distance should have increased (fighter moves 40' per encounter)
        assert!(data["new_distance"].as_u64().unwrap() > 10, "distance should increase");
    }

    #[test]
    fn fighting_withdrawal_no_free_attacks() {
        let mut state = GameState::new();
        // Create a character
        let resp = handle_request(&make_req("1", GMCommand::CreateCharacter {
            name: "Aldric".to_string(),
            class: Class::Fighter,
            alignment: Alignment::Lawful,
            abilities: None,
        }), &mut state);
        assert!(resp.success);

        // Spawn goblin for combat
        let resp = handle_request(&make_req("2", GMCommand::SpawnEncounter(EncounterParams {
            name: "Goblin".to_string(),
            count: 1,
            hit_dice: "1-1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        // Fighting withdrawal
        let resp = handle_request(&make_req("3", GMCommand::FightingWithdrawal {
            character: Some("Aldric".to_string()),
        }), &mut state);
        assert!(resp.success, "fighting withdrawal should succeed: {}", resp.message);

        // Message should indicate fighting withdrawal, not free attacks
        assert!(resp.message.contains("fighting withdrawal"), "message should mention fighting withdrawal");
        assert!(!resp.message.contains("free attack"), "fighting withdrawal should NOT mention free attacks");
    }

    #[test]
    fn retreat_no_combat_error() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Retreat {
            character: Some("Nobody".to_string()),
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.message.contains("no active combat"), "should error: no active combat");
    }

    #[test]
    fn retreat_without_character_auto_selects_sole_member() {
        let mut state = GameState::new();
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 100;
        c.max_hp = 100;
        state.party.add_member(c);

        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(crate::gmapi::protocol::EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        // Retreat without specifying character — should auto-select Aldric
        let resp = handle_request(&make_req("2", GMCommand::Retreat {
            character: None,
        }), &mut state);
        assert!(resp.success, "retreat without character should auto-select sole member: {}", resp.message);
        assert!(resp.message.contains("Aldric"), "should retreat Aldric");
    }

    #[test]
    fn retreat_without_character_errors_with_multiple_members() {
        let mut state = GameState::new();
        let mut c1 = crate::model::Character::new("Aldric", Class::Fighter);
        c1.hp = 10;
        c1.max_hp = 10;
        state.party.add_member(c1);
        let mut c2 = crate::model::Character::new("Bran", Class::Fighter);
        c2.hp = 10;
        c2.max_hp = 10;
        state.party.add_member(c2);

        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(crate::gmapi::protocol::EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        // Retreat without specifying character — should error with multiple alive members
        let resp = handle_request(&make_req("2", GMCommand::Retreat {
            character: None,
        }), &mut state);
        assert!(!resp.success, "should require character when multiple alive members");
        assert!(resp.message.contains("multiple alive party members"), "should list members: {}", resp.message);
    }

    #[test]
    fn fighting_withdrawal_without_character_auto_selects() {
        let mut state = GameState::new();
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        state.party.add_member(c);

        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(crate::gmapi::protocol::EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        let resp = handle_request(&make_req("2", GMCommand::FightingWithdrawal {
            character: None,
        }), &mut state);
        assert!(resp.success, "withdrawal without character should auto-select: {}", resp.message);
        assert!(resp.message.contains("Aldric"), "should withdraw Aldric");
    }

    #[test]
    fn enter_dungeon_and_advance() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterDungeon {
            level: 1,
            room_name: "Entry Hall".to_string(),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Exploration);

        let resp2 = handle_request(&make_req("2", GMCommand::AdvanceTurn), &mut state);
        assert!(resp2.success);
    }

    #[test]
    fn enter_dungeon_level_zero() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterDungeon {
            level: 0,
            room_name: "Test".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn enter_wilderness_and_add_hex() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterWilderness {
            terrain: Terrain::Forest,
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Wilderness);

        let resp2 = handle_request(&make_req("2", GMCommand::AddHex {
            x: 1,
            y: 0,
            terrain: Terrain::Hills,
        }), &mut state);
        assert!(resp2.success);
    }

    #[test]
    fn enter_wilderness_invalid_terrain() {
        // Invalid terrain is now caught at JSON deserialization.
        let json = r#"{"id":"1","command":{"type":"EnterWilderness","params":{"terrain":"lava"}}}"#;
        let result = crate::gmapi::protocol::parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn award_xp_no_character() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::AwardXp {
            character: "Nobody".to_string(),
            xp: 100,
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn ruling_recorded() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Ruling {
            text: "The bridge can hold 3 people at once.".to_string(),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.notes.len(), 1);
        assert!(state.notes[0].contains("bridge"));
    }

    #[test]
    fn roll_dice_valid() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Roll {
            notation: "2d6+3".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        let total = data["total"].as_i64().unwrap();
        assert!((5..=15).contains(&total));
    }

    #[test]
    fn roll_dice_invalid() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Roll {
            notation: "invalid".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn surprise_roll() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::RollSurprise), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["party_roll"].as_u64().is_some());
        assert!(data["monster_roll"].as_u64().is_some());
    }

    #[test]
    fn query_exploration_not_exploring() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryExploration), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn query_wilderness_not_in_wilderness() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryWilderness), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn quit_response() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Quit), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("ended"));
    }

    #[test]
    fn add_room_no_dungeon() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::AddRoom {
            id: 1,
            name: "Test".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn light_not_exploring() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Light {
            source: LightSourceKind::Torch,
            carrier: "Aldric".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn light_invalid_source() {
        // Invalid light source is now caught at JSON deserialization.
        let json = r#"{"id":"1","command":{"type":"Light","params":{"source":"candle","carrier":"Aldric"}}}"#;
        let result = crate::gmapi::protocol::parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn full_combat_sequence() {
        let mut state = GameState::new();
        // Add a character
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        c.thac0 = 19;
        c.abilities.strength = 12;
        state.party.add_member(c);

        // Spawn encounter
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 5,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        // Roll initiative
        let resp = handle_request(&make_req("2", GMCommand::RollInitiative), &mut state);
        assert!(resp.success);

        // Attack
        let resp = handle_request(&make_req("3", GMCommand::Attack {
            character: "Aldric".to_string(),
            monster_idx: 0,
            weapon: "sword".to_string(),
        }), &mut state);
        assert!(resp.success);

        // End combat
        let resp = handle_request(&make_req("4", GMCommand::EndCombat { skip_xp: false }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Idle);
    }

    #[test]
    fn close_command_integration() {
        let mut state = GameState::new();
        // Add a character
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        c.thac0 = 19;
        c.movement_rate = 120; // encounter move = 40
        state.party.add_member(c);

        // Spawn encounter at 100' distance
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 100,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.combat.as_ref().unwrap().distance, 100);

        // Close 30 feet
        let resp = handle_request(&make_req("2", GMCommand::Close {
            character: "Aldric".to_string(),
            feet: Some(30),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.combat.as_ref().unwrap().distance, 70);

        // Close without specifying feet (uses full encounter move)
        let resp = handle_request(&make_req("3", GMCommand::Close {
            character: "Aldric".to_string(),
            feet: None,
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.combat.as_ref().unwrap().distance, 30);

        // Try to close too far
        let resp = handle_request(&make_req("4", GMCommand::Close {
            character: "Aldric".to_string(),
            feet: Some(50),
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("too far"));
    }

    #[test]
    fn load_module_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LoadModule {
            path: "data/modules/sample_crypt/module.json".to_string(),
        }), &mut state);
        assert!(resp.success, "load_module should succeed: {:?}", resp.error);
        assert_eq!(state.mode, GameMode::Exploration);
        assert!(state.dungeon.is_some());
        assert!(state.time.is_some());

        let dungeon = state.dungeon.as_ref().unwrap();
        assert_eq!(dungeon.rooms.len(), 3);
        assert_eq!(dungeon.doors.len(), 2);
        assert_eq!(dungeon.current_room, Some(0));

        // Verify entry room is named correctly
        let entry = dungeon.find_room(0).unwrap();
        assert_eq!(entry.name, "Crypt Entrance");

        // Check response data
        let data = resp.data.unwrap();
        assert_eq!(data["module_name"], "Sample Crypt");
        assert_eq!(data["room_count"], 3);
    }

    #[test]
    fn load_module_not_found() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LoadModule {
            path: "data/modules/nonexistent/module.json".to_string(),
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("not found"));
    }

    #[test]
    fn load_module_path_traversal_rejected() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LoadModule {
            path: "data/modules/../../etc/passwd".to_string(),
        }), &mut state);
        assert!(!resp.success);
        assert!(
            resp.error.unwrap().contains("must be within the modules directory"),
            "path traversal should be blocked"
        );
    }

    #[test]
    fn spawn_npc_party_basic() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SpawnNpcParty {
            party_type: "basic".to_string(),
            distance: 60,
        }), &mut state);
        assert!(resp.success, "spawn_npc_party should succeed: {:?}", resp.error);
        assert_eq!(state.mode, GameMode::Combat);
        assert!(state.combat.is_some());
        let combat = state.combat.as_ref().unwrap();
        assert!(combat.monsters.len() >= 5); // 1d4+4 = 5-8
        assert!(combat.monsters.len() <= 8);
        assert_eq!(combat.distance, 60);
        let data = resp.data.unwrap();
        assert_eq!(data["party_type"], "Basic Adventurers");
    }

    #[test]
    fn spawn_npc_party_invalid_type() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SpawnNpcParty {
            party_type: "invalid".to_string(),
            distance: 60,
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("unknown party type"));
    }

    #[test]
    fn lookup_item_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LookupItem {
            name: "Bag of Holding".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["name"], "Bag of Holding");
    }

    #[test]
    fn lookup_item_not_found() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LookupItem {
            name: "Nonexistent XYZ".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn search_items_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SearchItems {
            query: "healing".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn lookup_treasure_type_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LookupTreasureType {
            letter: "A".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["letter"], "A");
    }

    #[test]
    fn lookup_treasure_type_invalid() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LookupTreasureType {
            letter: "Z".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn roll_treasure_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::RollTreasure {
            letter: "P".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(!data["items"].as_array().unwrap().is_empty());
    }

    #[test]
    fn roll_treasure_invalid() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::RollTreasure {
            letter: "Z".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn list_classes_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::ListClasses), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["classes"].as_array().unwrap().len(), 22);
    }

    #[test]
    fn eligible_classes_success() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EligibleClasses {
            abilities: [16, 10, 10, 12, 14, 12],
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["eligible"].as_array().unwrap().iter().any(|c| c == "Fighter"));
    }

    #[test]
    fn eligible_classes_invalid_score() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EligibleClasses {
            abilities: [20, 10, 10, 10, 10, 10],
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn end_combat_marks_room_monsters_cleared() {
        use crate::state::dungeon::{DungeonState, Room, PlacedMonsterInstance};

        let mut state = GameState::new();

        // Set up dungeon with a room that has placed monsters
        let mut dungeon = DungeonState::new(1);
        let monster_room = Room::new(0, "Monster Lair")
            .with_placed_monsters(vec![PlacedMonsterInstance::new("skeleton", 3)]);
        dungeon.add_room(monster_room).unwrap();
        state.enter_exploration(dungeon, 1);

        // Verify monsters_cleared starts as false
        assert!(
            !state.dungeon.as_ref().unwrap().find_room(0).unwrap().monsters_cleared,
            "monsters_cleared should start false"
        );

        // Add a character for combat
        let mut c = crate::model::Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        state.party.add_member(c);

        // Spawn encounter (simulating monsters from the room)
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "skeleton".to_string(),
            count: 3,
            hit_dice: "1".parse().unwrap(),
            ac: 7,
            hp: 4,
            damage: "1d6".to_string(),
            morale: 12,
            distance: 10,
            xp_value: Some(10),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Combat);

        // End combat
        let resp = handle_request(&make_req("2", GMCommand::EndCombat { skip_xp: false }), &mut state);
        assert!(resp.success);

        // Verify monsters_cleared is now true
        assert!(
            state.dungeon.as_ref().unwrap().find_room(0).unwrap().monsters_cleared,
            "monsters_cleared should be true after EndCombat"
        );
    }

    #[test]
    fn add_monster_to_active_combat() {
        let mut state = GameState::new();
        // Spawn initial encounter
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 3,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 60,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 3);

        // Add more monsters
        let resp = handle_request(&make_req("2", GMCommand::AddMonster(EncounterParams {
            name: "orc".to_string(),
            count: 2,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 4,
            damage: "1d6".to_string(),
            morale: 8,
            distance: 30, // ignored for AddMonster (distance is per-combat)
            xp_value: Some(10),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success, "add_monster should succeed: {:?}", resp.error);
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 5);

        // Verify the new monsters are at indices 3 and 4
        let combat = state.combat.as_ref().unwrap();
        assert_eq!(combat.monsters[3].name, "orc 1");
        assert_eq!(combat.monsters[4].name, "orc 2");
        assert_eq!(combat.monsters[3].hp, 4);
        assert_eq!(combat.monsters[3].ac, 6);
        assert_eq!(combat.monsters[3].xp_value, 10);

        // Verify response data
        let data = resp.data.unwrap();
        assert_eq!(data["count"], 2);
        assert_eq!(data["total_monsters"], 5);
    }

    #[test]
    fn add_monster_no_combat_error() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::AddMonster(EncounterParams {
            name: "orc".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 4,
            damage: "1d6".to_string(),
            morale: 8,
            distance: 30,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("no active combat"));
    }

    #[test]
    fn add_monster_single_named_correctly() {
        let mut state = GameState::new();
        // Start combat
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter(EncounterParams {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 60,
            xp_value: None,
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);

        // Add a single monster — should not get a number suffix
        let resp = handle_request(&make_req("2", GMCommand::AddMonster(EncounterParams {
            name: "ogre".to_string(),
            count: 1,
            hit_dice: "4+1".parse().unwrap(),
            ac: 5,
            hp: 19,
            damage: "1d10".to_string(),
            morale: 10,
            distance: 30,
            xp_value: Some(125),
            undead: None,
            immune_to_normal_weapons: None,
        })), &mut state);
        assert!(resp.success);
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 2);
        assert_eq!(state.combat.as_ref().unwrap().monsters[1].name, "ogre");
    }

    #[test]
    fn heal_dead_character_rejected() {
        let mut state = GameState::new();
        let mut c = crate::model::Character::new("Shadow", Class::Fighter);
        c.hp = -1;
        c.max_hp = 6;
        state.party.add_member(c);

        let resp = handle_request(&make_req("1", GMCommand::Heal {
            character: "Shadow".to_string(),
            amount: 10,
        }), &mut state);
        assert!(!resp.success, "healing a dead character should fail");
        assert!(resp.message.contains("dead"), "error should mention dead: {}", resp.message);

        // HP should be unchanged
        assert_eq!(state.party.find_member("Shadow").unwrap().hp, -1);
    }

    #[test]
    fn list_equipment_all() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::ListEquipment {
            category: None,
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["total"].as_u64().unwrap() > 0);
        assert!(!data["weapons"].as_array().unwrap().is_empty());
        assert!(!data["armour"].as_array().unwrap().is_empty());
        assert!(!data["gear"].as_array().unwrap().is_empty());
        assert!(!data["ammunition"].as_array().unwrap().is_empty());
    }

    #[test]
    fn list_equipment_weapons_only() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::ListEquipment {
            category: Some("weapons".to_string()),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(!data["weapons"].as_array().unwrap().is_empty());
        assert!(data["armour"].as_array().unwrap().is_empty());
        assert!(data["gear"].as_array().unwrap().is_empty());
        assert!(data["ammunition"].as_array().unwrap().is_empty());
    }

    #[test]
    fn look_shows_room_description() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LoadModule {
            path: "data/modules/sample_crypt/module.json".to_string(),
        }), &mut state);
        assert!(resp.success, "load module should succeed: {:?}", resp.error);

        let resp = handle_request(&make_req("2", GMCommand::Look), &mut state);
        assert!(resp.success, "look should succeed: {:?}", resp.error);
        assert!(resp.message.contains("Crypt Entrance"), "should show room name");
        let data = resp.data.unwrap();
        assert_eq!(data["room_id"], 0);
        assert_eq!(data["room_name"], "Crypt Entrance");
        assert!(!data["description"].as_str().unwrap().is_empty(), "description should not be empty");
    }

    #[test]
    fn look_not_exploring() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Look), &mut state);
        assert!(!resp.success, "look without dungeon should fail");
    }

    #[test]
    fn look_shows_exits() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::LoadModule {
            path: "data/modules/sample_crypt/module.json".to_string(),
        }), &mut state);
        assert!(resp.success);

        let resp = handle_request(&make_req("2", GMCommand::Look), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("Exits:"), "should show exits");
        assert!(resp.message.contains("Door"), "should show door info");
    }

    #[test]
    fn list_equipment_invalid_category() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::ListEquipment {
            category: Some("potions".to_string()),
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.message.contains("unknown category"));
    }
}
