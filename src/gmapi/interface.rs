use crate::dice;
use crate::engine::{chargen, combat, encounter_engine, exploration, retainer, wilderness_engine, xp};
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::model::{CombatState, Monster};
use crate::persist::{self, GameState};
use crate::rules::{ability, class, encumbrance, equipment, monster, spell_data, thief};
use crate::rules::alignment::Alignment;
use crate::rules::attack::HitDice;
use crate::rules::class::Class;
use crate::state::dungeon::{Door, DoorState, DungeonState, Room};
use crate::state::game::GameMode;
use crate::state::time::{LightSourceKind, TimeTracker};
use crate::state::wilderness::{HexCell, Terrain, WildernessState};

/// Process a GMRequest against the current GameState and return a GMResponse.
pub fn handle_request(req: &GMRequest, state: &mut GameState) -> GMResponse {
    let id = &req.id;
    match &req.command {
        // -- State queries --
        GMCommand::QueryState => query_state(id, state),
        GMCommand::QueryMode => GMResponse::ok_with_data(
            id, format!("current mode: {}", state.mode), state.mode.clone(),
            serde_json::json!({ "mode": format!("{}", state.mode) }),
        ),
        GMCommand::QueryParty => query_party(id, state),
        GMCommand::QueryCombat => query_combat(id, state),
        GMCommand::QueryExploration => query_exploration(id, state),
        GMCommand::QueryWilderness => query_wilderness(id, state),

        // -- Character management --
        GMCommand::CreateCharacter { name, class, alignment, abilities } => {
            create_character(id, state, name, *class, *alignment, abilities.as_ref())
        }

        // -- Combat --
        GMCommand::SpawnEncounter { name, count, hit_dice, ac, hp, damage, morale, distance, xp_value } => {
            spawn_encounter(id, state, name, *count, hit_dice.clone(), *ac, *hp, damage, *morale, *distance, *xp_value)
        }
        GMCommand::RollInitiative => roll_initiative(id, state),
        GMCommand::Attack { character, monster_idx, weapon } => {
            attack(id, state, character, *monster_idx, weapon)
        }
        GMCommand::MonsterAttack { monster_idx, character } => {
            monster_attack(id, state, *monster_idx, character)
        }
        GMCommand::CheckMorale => check_morale(id, state),
        GMCommand::TurnUndead { character, monster_idx } => {
            turn_undead(id, state, character, *monster_idx)
        }
        GMCommand::Close { character, feet } => close(id, state, character, *feet),
        GMCommand::Retreat { character } => retreat(id, state, character),
        GMCommand::FightingWithdrawal { character } => fighting_withdrawal(id, state, character),
        GMCommand::EndCombat => end_combat(id, state),

        // -- Exploration --
        GMCommand::EnterDungeon { level, room_name } => {
            enter_dungeon(id, state, *level, room_name)
        }
        GMCommand::AdvanceTurn => advance_turn(id, state),
        GMCommand::AddRoom { id: room_id, name } => add_room(id, state, *room_id, name),
        GMCommand::AddDoor { id: door_id, room_a, room_b, state: door_state } => {
            add_door(id, state, *door_id, *room_a, *room_b, *door_state)
        }
        GMCommand::MoveRoom { door_id } => move_room(id, state, *door_id),
        GMCommand::Search { is_elf } => search(id, state, *is_elf),
        GMCommand::Light { source, carrier } => light(id, state, *source, carrier),

        // -- Wilderness --
        GMCommand::EnterWilderness { terrain } => enter_wilderness(id, state, *terrain),
        GMCommand::AddHex { x, y, terrain } => add_hex(id, state, *x, *y, *terrain),
        GMCommand::Travel { x, y } => travel(id, state, *x, *y),
        GMCommand::Orient => orient(id, state),

        // -- Encounter resolution --
        GMCommand::RollSurprise => roll_surprise(id, state),
        GMCommand::RollReaction { character } => roll_reaction(id, state, character),

        // -- Management --
        GMCommand::AwardXp { character, xp } => award_xp(id, state, character, *xp),
        GMCommand::AwardTreasureXp { character, treasure_gp, monster_xp } => {
            award_treasure_xp(id, state, character, *treasure_gp, *monster_xp)
        }
        GMCommand::ThiefSkillCheck { character, skill } => {
            thief_skill_check(id, state, character, skill)
        }
        GMCommand::Backstab { character, monster_idx, weapon } => {
            backstab(id, state, character, *monster_idx, weapon)
        }
        GMCommand::QueryEncumbrance { character } => query_encumbrance(id, state, character),
        GMCommand::SpawnMonster { name, count, distance } => {
            spawn_monster(id, state, name, *count, *distance)
        }
        GMCommand::LookupSpell { name, list } => lookup_spell(id, state, name, list),
        GMCommand::HireRetainer { employer, retainer_name, retainer_class, retainer_level } => {
            hire_retainer(id, state, employer, retainer_name, *retainer_class, *retainer_level)
        }
        GMCommand::LoyaltyCheck { retainer_name, loyalty } => {
            loyalty_check(id, state, retainer_name, *loyalty)
        }
        GMCommand::LevelUp { character } => level_up(id, state, character),
        GMCommand::Ruling { text } => ruling(id, state, text),

        // -- System --
        GMCommand::Save { path } => save_game(id, state, path),
        GMCommand::Load { path } => load_game(id, state, path),
        GMCommand::Roll { notation } => roll_dice(id, state, notation),
        GMCommand::Quit => GMResponse::ok(id, "session ended.", state.mode.clone()),
    }
}

// =============================================================================
// Query handlers
// =============================================================================

fn query_state(id: &str, state: &GameState) -> GMResponse {
    let data = serde_json::json!({
        "mode": format!("{}", state.mode),
        "turn": state.turn(),
        "dungeon_level": state.dungeon_level,
        "party_size": state.party.members.len(),
        "has_combat": state.combat.is_some(),
        "has_dungeon": state.dungeon.is_some(),
        "has_wilderness": state.wilderness.is_some(),
        "notes": state.notes,
    });
    GMResponse::ok_with_data(id, "game state summary", state.mode.clone(), data)
}

fn query_party(id: &str, state: &GameState) -> GMResponse {
    if state.party.members.is_empty() {
        return GMResponse::ok_with_data(
            id, "no party members.", state.mode.clone(),
            serde_json::json!({ "members": [] }),
        );
    }
    let members: Vec<serde_json::Value> = state.party.members.iter().map(|c| {
        serde_json::json!({
            "name": c.name,
            "class": c.class.name(),
            "level": c.level,
            "hp": c.hp,
            "max_hp": c.max_hp,
            "ac": c.ac,
            "thac0": c.thac0,
            "xp": c.xp,
            "alive": c.is_alive(),
            "alignment": c.alignment.name(),
            "movement_rate": c.movement_rate,
        })
    }).collect();
    GMResponse::ok_with_data(
        id, format!("{} party members.", members.len()), state.mode.clone(),
        serde_json::json!({ "members": members }),
    )
}

fn query_combat(id: &str, state: &GameState) -> GMResponse {
    match &state.combat {
        Some(combat_state) => {
            let status = combat::combat_status(combat_state, &state.party.members);
            let monsters: Vec<serde_json::Value> = combat_state.monsters.iter().enumerate().map(|(i, m)| {
                serde_json::json!({
                    "index": i,
                    "name": m.name,
                    "hp": m.hp,
                    "max_hp": m.max_hp,
                    "ac": m.ac,
                    "alive": m.is_alive(),
                })
            }).collect();
            GMResponse::ok_with_data(
                id, status, state.mode.clone(),
                serde_json::json!({
                    "round": combat_state.round,
                    "distance": combat_state.distance,
                    "party_initiative": combat_state.party_initiative,
                    "monster_initiative": combat_state.monster_initiative,
                    "monsters": monsters,
                }),
            )
        }
        None => GMResponse::err(id, "no active combat.", state.mode.clone()),
    }
}

fn query_exploration(id: &str, state: &GameState) -> GMResponse {
    let time = match &state.time {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match &state.dungeon {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let status = exploration::exploration_status(time, dungeon);
    GMResponse::ok_with_data(
        id, status, state.mode.clone(),
        serde_json::json!({
            "dungeon_level": dungeon.level,
            "current_room": dungeon.current_room,
            "total_turns": time.total_turns,
            "has_light": time.has_light(),
        }),
    )
}

fn query_wilderness(id: &str, state: &GameState) -> GMResponse {
    let ws = match &state.wilderness {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let status = wilderness_engine::wilderness_status(ws, &state.party, party_movement);
    GMResponse::ok_with_data(
        id, status, state.mode.clone(),
        serde_json::json!({
            "current_x": ws.current_x,
            "current_y": ws.current_y,
            "travel_day": ws.travel_day,
            "lost": ws.lost,
        }),
    )
}

// =============================================================================
// Character management
// =============================================================================

fn create_character(id: &str, state: &mut GameState, name: &str, class: Class, alignment: Alignment, provided_abilities: Option<&[i32; 6]>) -> GMResponse {
    // Validate provided abilities if present
    if let Some(abs) = provided_abilities {
        for &score in abs {
            if !(3..=18).contains(&score) {
                return GMResponse::err(
                    id,
                    format!("ability scores must be 3-18, got {}.", score),
                    state.mode.clone(),
                );
            }
        }
    }

    let mut abilities = provided_abilities.copied().unwrap_or_else(chargen::roll_abilities);
    let def = class::class_def(class);
    if !def.racial_modifiers.is_empty() {
        class::apply_racial_modifiers(class, &mut abilities);
    }
    if !class::meets_requirements(class, &abilities) {
        let source = if provided_abilities.is_some() { "provided" } else { "rolled" };
        return GMResponse::err(
            id,
            format!("{} abilities do not meet requirements for {}.", source, class.name()),
            state.mode.clone(),
        );
    }

    let c = chargen::create_character(name, class, abilities, alignment);
    let sheet = chargen::character_sheet(&c);
    state.party.add_member(c);

    GMResponse::ok_with_data(
        id,
        format!("{} created and added to party.", name),
        state.mode.clone(),
        serde_json::json!({ "character_sheet": sheet }),
    )
}

// =============================================================================
// Combat
// =============================================================================

fn spawn_encounter(
    id: &str, state: &mut GameState,
    name: &str, count: u32, hit_dice: HitDice, ac: i32, hp: i32,
    damage: &str, morale: u32, distance: u32, xp_value: Option<u64>,
) -> GMResponse {
    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }
    if !(2..=12).contains(&morale) {
        return GMResponse::err(id, "morale must be 2-12.", state.mode.clone());
    }
    // XP: use explicit value if provided, otherwise look up from monster database
    let xp = xp_value.unwrap_or_else(|| {
        crate::rules::monster::find_monster(name)
            .map(|m| m.xp())
            .unwrap_or(0)
    });
    let mut monsters = Vec::new();
    for i in 0..count {
        let monster_name = if count > 1 {
            format!("{} {}", name, i + 1)
        } else {
            name.to_string()
        };
        let hd_str = hit_dice.to_string();
        let mut m = Monster::new(&monster_name, &hd_str);
        m.hp = hp;
        m.max_hp = hp;
        m.ac = ac;
        m.damage = damage.to_string();
        m.morale = morale;
        m.xp_value = xp;
        m.attacks = vec!["attack".to_string()];
        monsters.push(m);
    }

    let combat_state = CombatState::new(monsters, distance);
    let status = combat::combat_status(&combat_state, &state.party.members);
    state.combat = Some(combat_state);
    state.pre_combat_mode = Some(state.mode.clone());
    state.mode = GameMode::Combat;

    GMResponse::ok_with_data(
        id,
        format!("combat started: {} {}(s) at {}' distance.", count, name, distance),
        state.mode.clone(),
        serde_json::json!({ "status": status }),
    )
}

fn roll_initiative(id: &str, state: &mut GameState) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    let (p, m) = combat::roll_initiative(combat_state);
    let winner = if p > m { "party" } else if m > p { "monsters" } else { "simultaneous" };
    GMResponse::ok_with_data(
        id,
        format!("round {} initiative: party {} vs monsters {} — {} acts first.",
            combat_state.round, p, m, winner),
        state.mode.clone(),
        serde_json::json!({
            "round": combat_state.round,
            "party_initiative": p,
            "monster_initiative": m,
            "winner": winner,
        }),
    )
}

fn attack(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize, weapon_name: &str) -> GMResponse {
    let weapon = match equipment::find_weapon(weapon_name) {
        Some(w) => w,
        None => return GMResponse::err(id, format!("unknown weapon '{}'.", weapon_name), state.mode.clone()),
    };
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let rest_penalty = state.time.as_ref().map(|t| t.rest_penalty()).unwrap_or(0);
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };

    match combat::resolve_character_attack(combat_state, &character, monster_idx, &weapon, rest_penalty) {
        Ok(result) => GMResponse::ok(id, format!("{}", result), state.mode.clone()),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

fn monster_attack(id: &str, state: &mut GameState, monster_idx: usize, char_name: &str) -> GMResponse {
    {
        let combat_state = match state.combat.as_ref() {
            Some(c) => c,
            None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
        };
        if monster_idx >= combat_state.monsters.len() {
            return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
        }
        if !combat_state.monsters[monster_idx].is_alive() {
            return GMResponse::err(id, format!("{} is dead.", combat_state.monsters[monster_idx].name), state.mode.clone());
        }
    }
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !character.is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", character.name), state.mode.clone());
    }
    let combat_state = state.combat.as_mut().unwrap();
    let result = combat::monster_attack(combat_state, monster_idx, character);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn check_morale(id: &str, state: &mut GameState) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if combat_state.living_monster_count() == 0 {
        return GMResponse::err(id, "no living monsters.", state.mode.clone());
    }
    let morale_score = combat_state.monsters.iter()
        .filter(|m| m.is_alive())
        .map(|m| m.morale)
        .max()
        .unwrap_or(6);
    let result = combat::check_morale(combat_state, morale_score);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn turn_undead(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    // Only Clerics and Paladins can turn undead per OSE rules
    if !matches!(character.class, Class::Cleric | Class::Paladin) {
        return GMResponse::err(id, format!("{} ({}) cannot turn undead. Only Clerics and Paladins can turn undead.", character.name, character.class.name()), state.mode.clone());
    }
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(id, "target is already dead.", state.mode.clone());
    }
    let result = combat::resolve_turn_undead(combat_state, &character, character.level, monster_idx);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn close(id: &str, state: &mut GameState, char_name: &str, feet: Option<u32>) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    match combat::close(combat_state, &character, feet) {
        Ok(msg) => GMResponse::ok_with_data(
            id, msg, state.mode.clone(),
            serde_json::json!({ "distance": combat_state.distance }),
        ),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

fn retreat(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    if state.combat.is_none() {
        return GMResponse::err(id, "no active combat.", state.mode.clone());
    }
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !character.is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", character.name), state.mode.clone());
    }
    let combat_state = state.combat.as_mut().unwrap();
    let result = combat::retreat(combat_state, character);
    GMResponse::ok_with_data(
        id,
        format!("{}", result),
        state.mode.clone(),
        serde_json::json!({
            "retreater": result.retreater,
            "distance_moved": result.distance_moved,
            "new_distance": result.new_distance,
            "free_attacks": result.free_attacks.iter().map(|a| serde_json::json!({
                "attacker": a.attacker,
                "target": a.target,
                "roll": a.roll,
                "modifiers": a.modifiers,
                "target_number": a.target_number,
                "hit": a.hit,
                "damage": a.damage,
                "target_hp_after": a.target_hp_after,
                "target_killed": a.target_killed,
            })).collect::<Vec<_>>(),
        }),
    )
}

fn fighting_withdrawal(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    let msg = combat::fighting_withdrawal(combat_state, &character);
    GMResponse::ok(id, msg, state.mode.clone())
}

fn end_combat(id: &str, state: &mut GameState) -> GMResponse {
    if state.combat.is_none() {
        return GMResponse::err(id, "no active combat.", state.mode.clone());
    }
    let combat_state = state.combat.take().unwrap();
    let dead_monsters = combat_state.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_xp: u64 = combat_state.monsters.iter()
        .filter(|m| !m.is_alive())
        .map(|m| m.xp_value)
        .sum();
    let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
    state.mode = state.pre_combat_mode.take().unwrap_or(GameMode::Idle);

    GMResponse::ok_with_data(
        id,
        format!("combat ended after {} rounds. {} of {} monsters defeated.",
            combat_state.round, dead_monsters, combat_state.monsters.len()),
        state.mode.clone(),
        serde_json::json!({
            "rounds": combat_state.round,
            "monsters_defeated": dead_monsters,
            "total_xp": total_xp,
            "party_casualties": dead_party,
        }),
    )
}

// =============================================================================
// Exploration
// =============================================================================

fn enter_dungeon(id: &str, state: &mut GameState, level: u32, room_name: &str) -> GMResponse {
    if level == 0 {
        return GMResponse::err(id, "level must be a positive integer.", state.mode.clone());
    }
    let mut dungeon = DungeonState::new(level);
    dungeon.add_room(Room::new(0, room_name)).unwrap();
    dungeon.explore_current();
    state.dungeon = Some(dungeon);
    state.time = Some(TimeTracker::new());
    state.dungeon_level = level;
    state.mode = GameMode::Exploration;

    GMResponse::ok(
        id,
        format!("entered dungeon level {}. starting room: {}.", level, room_name),
        state.mode.clone(),
    )
}

fn advance_turn(id: &str, state: &mut GameState) -> GMResponse {
    let level = state.dungeon_level;
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let result = exploration::advance_dungeon_turn(time, dungeon, level);
    let has_encounter = result.encounter.is_some();
    let mut data = serde_json::json!({
        "messages": result.messages,
        "has_encounter": has_encounter,
    });
    if let Some(enc) = &result.encounter {
        data["encounter"] = serde_json::json!({
            "name": enc.name,
            "number": enc.number,
        });
    }
    GMResponse::ok_with_data(id, format!("{}", result), state.mode.clone(), data)
}

fn add_room(id: &str, state: &mut GameState, room_id: u32, name: &str) -> GMResponse {
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    if let Err(e) = dungeon.add_room(Room::new(room_id, name)) {
        return GMResponse::err(id, e, state.mode.clone());
    }
    GMResponse::ok(id, format!("added room {}: {}.", room_id, name), state.mode.clone())
}

fn add_door(id: &str, state: &mut GameState, door_id: u32, room_a: u32, room_b: u32, door_state: DoorState) -> GMResponse {
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let door = match Door::new(door_id, room_a, room_b, door_state) {
        Ok(d) => d,
        Err(e) => return GMResponse::err(id, e, state.mode.clone()),
    };
    if let Err(e) = dungeon.add_door(door) {
        return GMResponse::err(id, e, state.mode.clone());
    }
    GMResponse::ok(
        id,
        format!("added door {} between rooms {} and {} ({}).", door_id, room_a, room_b, door_state),
        state.mode.clone(),
    )
}

fn move_room(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    let dungeon_level = state.dungeon_level;
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    match exploration::move_through_door(time, dungeon, dungeon_level, door_id) {
        Ok(result) => GMResponse::ok(id, format!("{}", result), state.mode.clone()),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

fn search(id: &str, state: &mut GameState, is_elf: bool) -> GMResponse {
    let dungeon_level = state.dungeon_level;
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let result = exploration::search_room(time, dungeon, dungeon_level, is_elf);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn light(id: &str, state: &mut GameState, source: LightSourceKind, carrier: &str) -> GMResponse {
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    time.light(source, carrier);
    GMResponse::ok(id, format!("{} lights a {}.", carrier, source.name()), state.mode.clone())
}

// =============================================================================
// Wilderness
// =============================================================================

fn enter_wilderness(id: &str, state: &mut GameState, terrain: Terrain) -> GMResponse {
    let mut ws = WildernessState::new();
    ws.add_hex(HexCell::new(0, 0, terrain)).unwrap();
    state.wilderness = Some(ws);
    state.mode = GameMode::Wilderness;
    GMResponse::ok(
        id,
        format!("entered wilderness. starting hex: (0, 0) — {}.", terrain.name()),
        state.mode.clone(),
    )
}

fn add_hex(id: &str, state: &mut GameState, x: i32, y: i32, terrain: Terrain) -> GMResponse {
    let ws = match state.wilderness.as_mut() {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    if let Err(e) = ws.add_hex(HexCell::new(x, y, terrain)) {
        return GMResponse::err(id, e, state.mode.clone());
    }
    GMResponse::ok(id, format!("added hex ({}, {}) — {}.", x, y, terrain.name()), state.mode.clone())
}

fn travel(id: &str, state: &mut GameState, x: i32, y: i32) -> GMResponse {
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    if state.wilderness.is_none() {
        return GMResponse::err(id, "not in wilderness mode.", state.mode.clone());
    }
    let ws = state.wilderness.as_mut().unwrap();
    let result = wilderness_engine::travel_day(ws, &mut state.party, x, y, party_movement);
    let has_encounter = !result.encounters.is_empty();
    let encounters_json: Vec<serde_json::Value> = result.encounters.iter().map(|enc| {
        serde_json::json!({
            "name": enc.name,
            "number": enc.number,
        })
    }).collect();
    let data = serde_json::json!({
        "messages": result.messages,
        "lost": result.lost,
        "has_encounter": has_encounter,
        "encounters": encounters_json,
        "rations_consumed": result.rations_consumed,
        "starving": result.starving,
        "rations_remaining": state.party.rations,
    });
    GMResponse::ok_with_data(id, format!("{}", result), state.mode.clone(), data)
}

fn orient(id: &str, state: &mut GameState) -> GMResponse {
    let ws = match state.wilderness.as_mut() {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    let result = wilderness_engine::orient(ws);
    let data = serde_json::json!({
        "success": result.success,
        "terrain": result.terrain.name(),
        "lost": ws.lost,
        "travel_day": ws.travel_day,
    });
    GMResponse::ok_with_data(id, result.message, state.mode.clone(), data)
}

// =============================================================================
// Encounter resolution
// =============================================================================

fn roll_surprise(id: &str, state: &GameState) -> GMResponse {
    let (result, p, m) = encounter_engine::check_surprise();
    GMResponse::ok_with_data(
        id,
        format!("party roll: {} monster roll: {} — {}", p, m, result),
        state.mode.clone(),
        serde_json::json!({
            "party_roll": p,
            "monster_roll": m,
            "result": format!("{}", result),
        }),
    )
}

fn roll_reaction(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let cha = character.abilities.charisma;
    let (reaction, raw, modified) = encounter_engine::reaction_roll(cha);
    let cha_mod = ability::cha_reaction_mod(cha);
    GMResponse::ok_with_data(
        id,
        format!("{} speaks (CHA {}, modifier {:+}). reaction roll: {} {:+} = {} — {}",
            character.name, cha, cha_mod, raw, cha_mod, modified, reaction),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "charisma": cha,
            "cha_modifier": cha_mod,
            "raw_roll": raw,
            "modified_roll": modified,
            "reaction": format!("{}", reaction),
        }),
    )
}

// =============================================================================
// Management
// =============================================================================

fn award_xp(id: &str, state: &mut GameState, char_name: &str, xp_amount: u64) -> GMResponse {
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    character.xp += xp_amount;
    GMResponse::ok_with_data(
        id,
        format!("{} awarded {} XP (total: {}).", character.name, xp_amount, character.xp),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "xp_awarded": xp_amount,
            "total_xp": character.xp,
        }),
    )
}

fn award_treasure_xp(id: &str, state: &mut GameState, char_name: &str, treasure_gp: u64, monster_xp: u64) -> GMResponse {
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let result = xp::award_xp(character, treasure_gp, monster_xp);
    let mut msg = format!(
        "{}: base {}xp ({}gp treasure + {}xp monsters), {:+}% prime req modifier = {} adjusted XP. Total: {}.",
        character.name, result.base_xp, treasure_gp, monster_xp,
        result.modifier_pct, result.adjusted_xp, result.new_total,
    );
    if result.leveled_up {
        msg.push_str(&format!(
            " LEVEL UP to {}! Gained {} HP.",
            result.new_level, result.hp_gained,
        ));
    }
    GMResponse::ok_with_data(
        id, msg, state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "base_xp": result.base_xp,
            "modifier_pct": result.modifier_pct,
            "adjusted_xp": result.adjusted_xp,
            "total_xp": result.new_total,
            "leveled_up": result.leveled_up,
            "new_level": result.new_level,
            "hp_gained": result.hp_gained,
        }),
    )
}

fn thief_skill_check(id: &str, state: &GameState, char_name: &str, skill_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !thief::has_thief_skills(character.class) {
        return GMResponse::err(id, format!("{} ({}) does not have thief skills.", character.name, character.class.name()), state.mode.clone());
    }
    let skill = match skill_name.to_lowercase().replace([' ', '_', '-'], "").as_str() {
        "climbwalls" | "climb" => thief::ThiefSkill::ClimbWalls,
        "findtraps" | "traps" => thief::ThiefSkill::FindTraps,
        "hearnoise" | "hear" | "listen" => thief::ThiefSkill::HearNoise,
        "hideshadows" | "hide" | "hideinshadows" => thief::ThiefSkill::HideShadows,
        "movesilently" | "sneak" | "stealth" => thief::ThiefSkill::MoveSilently,
        "openlocks" | "pick" | "lockpick" => thief::ThiefSkill::OpenLocks,
        "pickpockets" | "pickpocket" | "steal" => thief::ThiefSkill::PickPockets,
        "readlanguages" | "read" => thief::ThiefSkill::ReadLanguages,
        _ => return GMResponse::err(id, format!("unknown thief skill '{}'.", skill_name), state.mode.clone()),
    };
    let target = thief::skill_chance(skill, character.level);
    let roll: u32 = if skill.is_d6() {
        rand::Rng::gen_range(&mut rand::thread_rng(), 1..=6)
    } else {
        rand::Rng::gen_range(&mut rand::thread_rng(), 1..=100)
    };
    let result = thief::check_skill(skill, character.level, roll);
    let die_type = if skill.is_d6() { "d6" } else { "d%" };
    GMResponse::ok_with_data(
        id,
        format!("{} attempts {} (level {}): target {}, rolled {} ({}) — {}.",
            character.name, skill.name(), character.level,
            target, roll, die_type,
            if result.success { "SUCCESS" } else { "FAILURE" }),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "skill": skill.name(),
            "target": target,
            "roll": roll,
            "success": result.success,
        }),
    )
}

fn backstab(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize, weapon_name: &str) -> GMResponse {
    let weapon = match equipment::find_weapon(weapon_name) {
        Some(w) => w,
        None => return GMResponse::err(id, format!("unknown weapon '{}'.", weapon_name), state.mode.clone()),
    };
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !thief::can_backstab(character.class) {
        return GMResponse::err(id, format!("{} ({}) cannot backstab.", character.name, character.class.name()), state.mode.clone());
    }
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", combat_state.monsters[monster_idx].name), state.mode.clone());
    }

    let multiplier = thief::backstab_multiplier(character.level);
    let str_mod = ability::str_melee_mod(character.abilities.strength);
    let attack_bonus = thief::BACKSTAB_ATTACK_BONUS;

    // Roll attack with backstab bonus
    let target_ac = combat_state.monsters[monster_idx].ac;
    let target_number = (character.thac0 as i32 - target_ac - attack_bonus - str_mod).max(2).min(20);
    let attack_roll: i32 = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=20);

    let hit = attack_roll == 20 || (attack_roll != 1 && attack_roll >= target_number);

    if hit {
        // Roll damage and multiply
        let base_damage = match dice::roll_str(weapon.damage) {
            Ok(r) => r.total.max(1),
            Err(_) => 1,
        };
        let total_damage = (base_damage + str_mod).max(1) * multiplier as i32;
        combat_state.monsters[monster_idx].hp -= total_damage;
        let monster_name = combat_state.monsters[monster_idx].name.clone();
        let alive = combat_state.monsters[monster_idx].is_alive();
        combat_state.log.push(format!(
            "{} backstabs {} for {} damage (x{}){}",
            character.name, monster_name, total_damage, multiplier,
            if !alive { " — KILLED!" } else { "" }
        ));
        GMResponse::ok_with_data(
            id,
            format!("{} backstabs {} (+{} to hit, x{} damage)! Rolled {} vs target {}: HIT for {} damage{}.",
                character.name, monster_name, attack_bonus, multiplier,
                attack_roll, target_number, total_damage,
                if !alive { " — KILLED!" } else { "" }),
            state.mode.clone(),
            serde_json::json!({
                "hit": true,
                "attack_roll": attack_roll,
                "target_number": target_number,
                "damage": total_damage,
                "multiplier": multiplier,
                "monster_alive": alive,
            }),
        )
    } else {
        combat_state.log.push(format!("{} backstab attempt on {} missed", character.name, combat_state.monsters[monster_idx].name));
        GMResponse::ok_with_data(
            id,
            format!("{} backstab attempt: rolled {} vs target {} — MISS.",
                character.name, attack_roll, target_number),
            state.mode.clone(),
            serde_json::json!({
                "hit": false,
                "attack_roll": attack_roll,
                "target_number": target_number,
            }),
        )
    }
}

fn query_encumbrance(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let item_weights: Vec<u32> = character.inventory.iter()
        .map(|item| (item.weight * 10.0) as u32) // weight is in pounds, convert to coins (10 cn = 1 lb)
        .collect();
    let total = encumbrance::total_weight(&item_weights, character.gold_gp);
    let level = encumbrance::encumbrance_level(total);
    let movement = encumbrance::movement_rate(total);
    GMResponse::ok_with_data(
        id,
        format!("{}: {} cn total, {} (movement {}').",
            character.name, total, level.name(), movement),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "total_weight_cn": total,
            "encumbrance_level": level.name(),
            "movement_rate": movement,
            "max_capacity": encumbrance::MAX_CAPACITY_CN,
        }),
    )
}

fn spawn_monster(id: &str, state: &mut GameState, name: &str, count: u32, distance: u32) -> GMResponse {
    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }
    let def = match monster::find_monster(name) {
        Some(d) => d,
        None => return GMResponse::err(id, format!("unknown monster '{}'. Use SpawnEncounter for custom monsters.", name), state.mode.clone()),
    };

    let mut monsters = Vec::new();
    for i in 0..count {
        let monster_name = if count > 1 {
            format!("{} {}", def.name, i + 1)
        } else {
            def.name.to_string()
        };
        let mut m = Monster::new(&monster_name, &def.hit_dice);
        // Roll HP from hit dice
        let hd = crate::rules::attack::parse_monster_hd(&def.hit_dice);
        let hp = if hd == 0 {
            // Half HD monsters (kobolds, etc): 1d4
            match dice::roll_str("1d4") {
                Ok(r) => r.total.max(1),
                Err(_) => 2,
            }
        } else {
            match dice::roll_str(&format!("{}d8", hd)) {
                Ok(r) => r.total.max(1),
                Err(_) => (hd as i32 * 4).max(1),
            }
        };
        m.hp = hp;
        m.max_hp = hp;
        m.ac = def.ac();
        m.damage = def.damage();
        m.morale = def.morale;
        m.xp_value = def.xp();
        m.attacks = def.attack_names();
        monsters.push(m);
    }

    let combat_state = CombatState::new(monsters, distance);
    let status = combat::combat_status(&combat_state, &state.party.members);
    state.combat = Some(combat_state);
    state.pre_combat_mode = Some(state.mode.clone());
    state.mode = GameMode::Combat;

    let mut msg = format!("combat started: {} {}(s) at {}' distance.", count, def.name, distance);
    let special = def.special();
    if !special.is_empty() {
        msg.push_str(&format!(" Special: {}", special));
    }

    GMResponse::ok_with_data(
        id, msg, state.mode.clone(),
        serde_json::json!({
            "status": status,
            "monster": def.name,
            "hit_dice": def.hit_dice,
            "ac": def.ac(),
            "special": def.special(),
        }),
    )
}

fn lookup_spell(id: &str, state: &GameState, name: &str, list_name: &str) -> GMResponse {
    let list = if list_name.is_empty() {
        None
    } else {
        match list_name.to_lowercase().as_str() {
            "cleric" => Some(spell_data::SpellList::Cleric),
            "magicuser" | "magic-user" | "magic_user" | "mu" | "mage" => Some(spell_data::SpellList::MagicUser),
            "druid" => Some(spell_data::SpellList::Druid),
            "illusionist" => Some(spell_data::SpellList::Illusionist),
            _ => return GMResponse::err(id, format!("unknown spell list '{}'.", list_name), state.mode.clone()),
        }
    };
    match spell_data::find_spell(name, list) {
        Some(spell) => GMResponse::ok_with_data(
            id,
            format!("{} ({}L{}) — Range: {}, Duration: {}: {}",
                spell.name, spell.list.name(), spell.level,
                spell.range, spell.duration, spell.description),
            state.mode.clone(),
            serde_json::json!({
                "name": spell.name,
                "list": spell.list.name(),
                "level": spell.level,
                "range": spell.range,
                "duration": spell.duration,
                "description": spell.description,
            }),
        ),
        None => GMResponse::err(id, format!("spell '{}' not found.", name), state.mode.clone()),
    }
}

fn hire_retainer(id: &str, state: &GameState, employer_name: &str, ret_name: &str, ret_class: Class, ret_level: u32) -> GMResponse {
    let employer = match state.party.find_member(employer_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", employer_name), state.mode.clone()),
    };
    let cha = employer.abilities.charisma;
    let max = retainer::max_retainers(cha);
    let base_loyalty = retainer::base_loyalty(cha);
    let reaction = retainer::hiring_reaction(cha);
    let wage = retainer::standard_wage(ret_level);

    let hired = matches!(reaction, retainer::HireReaction::Accepts | retainer::HireReaction::Eager);
    let bonus_loyalty = matches!(reaction, retainer::HireReaction::Eager);
    let loyalty = if bonus_loyalty { base_loyalty + 1 } else { base_loyalty };

    GMResponse::ok_with_data(
        id,
        format!("{} attempts to hire {} ({} L{}, {}gp/month). CHA {} (max {} retainers, loyalty {}). Reaction: {} — {}.",
            employer.name, ret_name, ret_class.name(), ret_level, wage,
            cha, max, base_loyalty, reaction.name(),
            if hired { "HIRED" } else { "NOT HIRED" }),
        state.mode.clone(),
        serde_json::json!({
            "employer": employer.name,
            "retainer": ret_name,
            "class": ret_class.name(),
            "level": ret_level,
            "reaction": reaction.name(),
            "hired": hired,
            "loyalty": loyalty,
            "wage_gp": wage,
            "max_retainers": max,
        }),
    )
}

fn loyalty_check(id: &str, state: &GameState, ret_name: &str, loyalty: u32) -> GMResponse {
    let result = retainer::loyalty_check(loyalty);
    let result_name = match result {
        retainer::LoyaltyResult::Loyal => "Loyal",
        retainer::LoyaltyResult::Wavering => "Wavering",
        retainer::LoyaltyResult::Disloyal => "Disloyal",
    };
    GMResponse::ok_with_data(
        id,
        format!("{} loyalty check (loyalty {}): {}.", ret_name, loyalty, result_name),
        state.mode.clone(),
        serde_json::json!({
            "retainer": ret_name,
            "loyalty": loyalty,
            "result": result_name,
        }),
    )
}

fn level_up(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let cls = character.class;
    match crate::rules::xp::check_level_up(cls, character.level, character.xp) {
        Some(_next_level) => {
            let result = xp::award_xp(character, 0, 0);
            if result.leveled_up {
                GMResponse::ok_with_data(
                    id,
                    format!("{} leveled up to {}! Gained {} HP. HP: {}/{}.",
                        character.name, result.new_level, result.hp_gained,
                        character.hp, character.max_hp),
                    state.mode.clone(),
                    serde_json::json!({
                        "character": character.name,
                        "new_level": result.new_level,
                        "hp_gained": result.hp_gained,
                        "hp": character.hp,
                        "max_hp": character.max_hp,
                    }),
                )
            } else {
                GMResponse::ok(id, format!("{} is already at level {} and cannot advance further.", character.name, character.level), state.mode.clone())
            }
        }
        None => {
            let needed = crate::rules::xp::xp_for_level(cls, character.level + 1);
            if needed == u64::MAX {
                GMResponse::err(id, format!("{} is at maximum level ({}).", character.name, character.level), state.mode.clone())
            } else {
                GMResponse::err(id, format!("{} needs {} XP for level {} (has {}).", character.name, needed, character.level + 1, character.xp), state.mode.clone())
            }
        }
    }
}

fn ruling(id: &str, state: &mut GameState, text: &str) -> GMResponse {
    state.notes.push(format!("[RULING] {}", text));
    GMResponse::ok(
        id,
        format!("ruling recorded: {}", text),
        state.mode.clone(),
    )
}

// =============================================================================
// System
// =============================================================================

fn save_game(id: &str, state: &GameState, path: &str) -> GMResponse {
    match persist::save(state, std::path::Path::new(path)) {
        Ok(()) => GMResponse::ok(id, format!("game saved to {}.", path), state.mode.clone()),
        Err(e) => GMResponse::err(id, format!("save failed: {}.", e), state.mode.clone()),
    }
}

fn load_game(id: &str, state: &mut GameState, path: &str) -> GMResponse {
    match persist::load(std::path::Path::new(path)) {
        Ok(loaded) => {
            let msg = format!(
                "loaded: turn {}, dungeon level {}, {} party members.",
                loaded.turn(), loaded.dungeon_level, loaded.party.members.len(),
            );
            *state = loaded;
            GMResponse::ok(id, msg, state.mode.clone())
        }
        Err(e) => GMResponse::err(id, format!("load failed: {}.", e), state.mode.clone()),
    }
}

fn roll_dice(id: &str, state: &GameState, notation: &str) -> GMResponse {
    match dice::roll_str(notation) {
        Ok(result) => GMResponse::ok_with_data(
            id,
            format!("{}", result),
            state.mode.clone(),
            serde_json::json!({ "total": result.total }),
        ),
        Err(e) => GMResponse::err(id, format!("{}", e), state.mode.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 3,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 60,
            xp_value: None,
        }), &mut state);
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
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 13,
            distance: 60,
            xp_value: None,
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn combat_already_active() {
        let mut state = GameState::new();
        state.combat = Some(CombatState::new(vec![], 0));
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "orc".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 4,
            damage: "1d6".to_string(),
            morale: 8,
            distance: 30,
            xp_value: None,
        }), &mut state);
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
    fn end_combat_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EndCombat), &mut state);
        assert!(!resp.success);
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
        let resp = handle_request(&make_req("2", GMCommand::SpawnEncounter {
            name: "Goblin".to_string(),
            count: 2,
            hit_dice: "1-1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Combat);

        // Retreat
        let resp = handle_request(&make_req("3", GMCommand::Retreat {
            character: "Aldric".to_string(),
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
        let resp = handle_request(&make_req("2", GMCommand::SpawnEncounter {
            name: "Goblin".to_string(),
            count: 1,
            hit_dice: "1-1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 10,
            xp_value: Some(5),
        }), &mut state);
        assert!(resp.success);

        // Fighting withdrawal
        let resp = handle_request(&make_req("3", GMCommand::FightingWithdrawal {
            character: "Aldric".to_string(),
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
            character: "Nobody".to_string(),
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.message.contains("no active combat"), "should error: no active combat");
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
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 5,
            xp_value: None,
        }), &mut state);
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
        let resp = handle_request(&make_req("4", GMCommand::EndCombat), &mut state);
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
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".parse().unwrap(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 100,
            xp_value: None,
        }), &mut state);
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
}
