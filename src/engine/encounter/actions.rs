use crate::dice;
use crate::engine::encounter::results::{
    EvadeResult, RollEncounterResult, RollReactionResult, RollSurpriseResult,
};
use crate::engine::encounter_engine;
use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::rules::ability;
use crate::rules::encounter as encounter_tables;
use crate::state::game::GameMode;

/// Roll number appearing. Handles both dice notation ("2d4") and plain integers ("1").
fn roll_number_appearing(notation: &str) -> Result<i32, EngineError> {
    if let Ok(n) = notation.parse::<i32>() {
        return Ok(n);
    }
    dice::roll_str(notation)
        .map(|r| r.total)
        .map_err(|e| EngineError::Internal(format!("bad dice expr '{}': {}", notation, e)))
}

pub fn action_roll_encounter(state: &mut GameState) -> Result<RollEncounterResult, EngineError> {
    let mut rng = rand::thread_rng();
    let table_roll: u32 = rand::Rng::gen_range(&mut rng, 1..=20);

    match state.mode {
        GameMode::Exploration => {
            let level = state.dungeon_level;
            if level == 0 {
                return Err(EngineError::WrongState(
                    "dungeon level not set. Use EnterDungeon first.".to_string(),
                ));
            }
            let entry = encounter_tables::dungeon_encounter_d40(level, table_roll).ok_or_else(|| {
                EngineError::Internal("no encounter found for this roll.".to_string())
            })?;
            let num_appearing = roll_number_appearing(&entry.number)?;
            let seq = encounter_engine::begin_encounter_dungeon();

            Ok(RollEncounterResult {
                message: format!(
                    "ENCOUNTER — Dungeon Level {}\n\
                     Table roll: {} → {}\n\
                     Number appearing: {} → {}\n\
                     Surprise: party {}, monsters {} — {}\n\
                     Distance: {}' feet",
                    level,
                    table_roll,
                    entry.name,
                    entry.number,
                    num_appearing,
                    seq.party_surprise_roll,
                    seq.monster_surprise_roll,
                    seq.surprise,
                    seq.distance,
                ),
                context: "dungeon".to_string(),
                level: Some(level),
                terrain: None,
                table_roll,
                monster_name: entry.name,
                number_notation: entry.number,
                number_appearing: num_appearing,
                party_surprise_roll: seq.party_surprise_roll,
                monster_surprise_roll: seq.monster_surprise_roll,
                surprise: seq.surprise.to_string(),
                distance: seq.distance,
            })
        }
        GameMode::Wilderness => {
            let ws = state
                .wilderness
                .as_ref()
                .ok_or_else(|| EngineError::WrongState("no wilderness state.".to_string()))?;
            let hex = ws
                .current_hex()
                .ok_or_else(|| EngineError::WrongState("no current hex.".to_string()))?;
            let terrain = hex.terrain;
            let entry = encounter_tables::wilderness_encounter_simple(terrain, table_roll)
                .ok_or_else(|| EngineError::Internal("no encounter found for this terrain.".to_string()))?;
            let num_appearing = roll_number_appearing(&entry.number)?;
            let seq = encounter_engine::begin_encounter_wilderness();

            Ok(RollEncounterResult {
                message: format!(
                    "ENCOUNTER — Wilderness ({})\n\
                     Table roll: {} → {}\n\
                     Number appearing: {} → {}\n\
                     Surprise: party {}, monsters {} — {}\n\
                     Distance: {} yards",
                    terrain.name(),
                    table_roll,
                    entry.name,
                    entry.number,
                    num_appearing,
                    seq.party_surprise_roll,
                    seq.monster_surprise_roll,
                    seq.surprise,
                    seq.distance,
                ),
                context: "wilderness".to_string(),
                level: None,
                terrain: Some(terrain.name().to_string()),
                table_roll,
                monster_name: entry.name,
                number_notation: entry.number,
                number_appearing: num_appearing,
                party_surprise_roll: seq.party_surprise_roll,
                monster_surprise_roll: seq.monster_surprise_roll,
                surprise: seq.surprise.to_string(),
                distance: seq.distance,
            })
        }
        _ => Err(EngineError::WrongState(
            "encounter requires exploration or wilderness mode.".to_string(),
        )),
    }
}

pub fn action_roll_surprise(_state: &GameState) -> Result<RollSurpriseResult, EngineError> {
    let (result, party_roll, monster_roll) = encounter_engine::check_surprise();
    Ok(RollSurpriseResult {
        message: format!(
            "party roll: {} monster roll: {} — {}",
            party_roll, monster_roll, result
        ),
        party_roll,
        monster_roll,
        result: result.to_string(),
    })
}

pub fn action_roll_reaction(
    state: &GameState,
    char_name: &str,
) -> Result<RollReactionResult, EngineError> {
    let character = state.party.find_member(char_name).ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    let charisma = character.abilities.charisma;
    let (reaction, raw_roll, modified_roll) = encounter_engine::reaction_roll(charisma);
    let cha_modifier = ability::cha_reaction_mod(charisma);

    Ok(RollReactionResult {
        message: format!(
            "{} speaks (CHA {}, modifier {:+}). reaction roll: {} {:+} = {} — {}",
            character.name, charisma, cha_modifier, raw_roll, cha_modifier, modified_roll, reaction
        ),
        character: character.name.clone(),
        charisma,
        cha_modifier,
        raw_roll,
        modified_roll,
        reaction: reaction.to_string(),
    })
}

pub fn action_evade(
    state: &GameState,
    monster_count: u32,
    monster_movement: u32,
) -> Result<EvadeResult, EngineError> {
    let party_size = state.party.members.iter().filter(|c| c.is_alive()).count() as u32;
    if party_size == 0 {
        return Err(EngineError::InvalidInput(
            "no living party members.".to_string(),
        ));
    }

    let party_movement = state
        .party
        .members
        .iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let result = encounter_engine::attempt_evasion(
        party_size,
        party_movement,
        monster_count,
        monster_movement,
    );
    let escaped = matches!(result, encounter_engine::EvasionResult::Escaped);

    Ok(EvadeResult {
        message: format!(
            "Party ({} members, {}' movement) vs {} monsters ({}' movement)\n{}",
            party_size, party_movement, monster_count, monster_movement, result
        ),
        escaped,
        party_size,
        party_movement,
        monster_count,
        monster_movement,
    })
}
