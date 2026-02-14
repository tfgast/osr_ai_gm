use crate::command::combat_cmds::{
    AttackCommand, CloseCommand, CombatLogCommand, DeclareSpellCommand, EndCombatCommand,
    InitiativeCommand, MonsterAttackCommand, MoraleCommand, RetreatCommand, StartCombatCommand,
    TurnUndeadCommand, WithdrawalCommand,
};
use crate::command::{Command, CommandResult};
use crate::engine::retainer::Retainer;
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{EncounterParams, GMCommand, GMRequest, GMResponse};
use crate::model::{Character, CombatState, Monster};
use crate::persist::GameState;
use crate::rules::class::Class;
use crate::state::game::GameMode;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct CliSnapshot {
    success: bool,
    output: String,
    state_after: Value,
}

#[derive(Debug, Serialize)]
struct ApiSnapshot {
    success: bool,
    message: String,
    error: Option<String>,
    data: Option<Value>,
    state_after: Value,
}

#[derive(Debug, Serialize)]
struct CommandParitySnapshot {
    command: &'static str,
    cli: CliSnapshot,
    api: ApiSnapshot,
}

fn capture_parity(
    command: &'static str,
    initial_state: GameState,
    cli_exec: impl FnOnce(&mut GameState) -> CommandResult,
    api_command: GMCommand,
) -> CommandParitySnapshot {
    let mut cli_state = initial_state.clone();
    let cli_result = cli_exec(&mut cli_state);
    let cli = CliSnapshot {
        success: !cli_result.output.starts_with("Error: "),
        output: cli_result.output,
        state_after: summarize_state(&cli_state),
    };

    let mut api_state = initial_state;
    let api_result = run_api(api_command, &mut api_state);
    let api = ApiSnapshot {
        success: api_result.success,
        message: api_result.message,
        error: api_result.error,
        data: api_result.data,
        state_after: summarize_state(&api_state),
    };

    CommandParitySnapshot { command, cli, api }
}

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "golden".to_string(),
        command,
    };
    handle_request(&request, state)
}

fn summarize_state(state: &GameState) -> Value {
    let party: Vec<Value> = state
        .party
        .members
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "hp": c.hp,
                "max_hp": c.max_hp,
                "alive": c.is_alive(),
            })
        })
        .collect();

    let retainers: Vec<Value> = state
        .retainers
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "hp": r.hp,
                "alive": r.is_alive(),
            })
        })
        .collect();

    let combat = state.combat.as_ref().map(|c| {
        let monsters: Vec<Value> = c
            .monsters
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "hp": m.hp,
                    "max_hp": m.max_hp,
                    "alive": m.is_alive(),
                    "helpless": m.helpless,
                    "turned": m.turned,
                    "morale": m.morale,
                })
            })
            .collect();

        json!({
            "round": c.round,
            "distance": c.distance,
            "party_initiative": c.party_initiative,
            "monster_initiative": c.monster_initiative,
            "log_len": c.log.len(),
            "spell_declarations": c.spell_declarations,
            "monsters": monsters,
        })
    });

    json!({
        "mode": state.mode.to_string(),
        "pre_combat_mode": state.pre_combat_mode.as_ref().map(|m| m.to_string()),
        "party": party,
        "retainers": retainers,
        "combat": combat,
        "notes_len": state.notes.len(),
    })
}

fn base_state() -> GameState {
    let mut state = GameState::new();

    let mut fighter = Character::new("Grond", Class::Fighter);
    fighter.hp = 12;
    fighter.max_hp = 12;
    fighter.ac = 4;
    fighter.level = 2;
    fighter.abilities.strength = 16;

    let mut cleric = Character::new("Aldric", Class::Cleric);
    cleric.hp = 9;
    cleric.max_hp = 9;
    cleric.ac = 5;
    cleric.level = 3;
    cleric.abilities.wisdom = 15;

    let mut magic_user = Character::new("Elara", Class::MagicUser);
    magic_user.hp = 5;
    magic_user.max_hp = 5;
    magic_user.ac = 7;
    magic_user.level = 2;
    magic_user.abilities.intelligence = 16;

    state.party.add_member(fighter);
    state.party.add_member(cleric);
    state.party.add_member(magic_user);
    state
}

fn mk_monster(name: &str, hd: &str, hp: i32, ac: i32, morale: u32, xp_value: u64) -> Monster {
    let mut monster = Monster::new(name, hd.parse().unwrap());
    monster.hp = hp;
    monster.max_hp = hp;
    monster.ac = ac;
    monster.damage = "1d6".to_string();
    monster.morale = morale;
    monster.xp_value = xp_value;
    monster.attacks = vec!["weapon".to_string()];
    monster
}

fn state_with_combat() -> GameState {
    let mut state = base_state();
    state.mode = GameMode::Combat;
    state.pre_combat_mode = Some(GameMode::Idle);
    state.combat = Some(CombatState::new(
        vec![
            mk_monster("Goblin 1", "1", 4, 6, 7, 5),
            mk_monster("Goblin 2", "1", 4, 6, 7, 5),
        ],
        60,
    ));
    state
}

fn state_with_helpless_target() -> GameState {
    let mut state = state_with_combat();
    if let Some(combat) = state.combat.as_mut() {
        combat.monsters[0].helpless = true;
    }
    state
}

fn state_with_morale_selector_divergence() -> GameState {
    let mut state = base_state();
    state.mode = GameMode::Combat;
    state.pre_combat_mode = Some(GameMode::Idle);
    state.combat = Some(CombatState::new(
        vec![
            mk_monster("Goblin Scout", "1", 4, 6, 6, 5),
            mk_monster("Orc Brute", "1+1", 7, 6, 10, 10),
        ],
        30,
    ));
    state
}

fn state_with_combat_log() -> GameState {
    let mut state = state_with_combat();
    if let Some(combat) = state.combat.as_mut() {
        combat.log.push("Grond attacks Goblin 1".to_string());
        combat.log.push("Goblin 1 misses Grond".to_string());
    }
    state
}

fn state_for_end_combat() -> GameState {
    let mut state = state_with_combat();
    if let Some(combat) = state.combat.as_mut() {
        combat.round = 2;
        combat.monsters[0].hp = 0;
        combat.monsters[0].xp_value = 25;
        combat.monsters[1].hp = 2;
        combat.monsters[1].xp_value = 25;
    }
    state.retainers.push(Retainer::new("Hob", Class::Fighter, 1, 6, 7, 25));
    state
}

#[test]
fn combat_command_parity_golden_scaffold_captures_snapshots() {
    let snapshots = vec![
        capture_parity(
            "start_combat",
            base_state(),
            |state| {
                StartCombatCommand.execute(
                    &["goblin", "2", "1", "6", "3", "1d6", "7", "60"],
                    state,
                )
            },
            GMCommand::SpawnEncounter(EncounterParams {
                name: "goblin".to_string(),
                count: 2,
                hit_dice: "1".parse().unwrap(),
                ac: 6,
                hp: 3,
                damage: "1d6".to_string(),
                morale: 7,
                distance: 60,
                xp_value: None,
            }),
        ),
        capture_parity(
            "initiative",
            state_with_combat(),
            |state| InitiativeCommand.execute(&[], state),
            GMCommand::RollInitiative,
        ),
        capture_parity(
            "attack",
            state_with_helpless_target(),
            |state| AttackCommand.execute(&["Grond", "0", "sword"], state),
            GMCommand::Attack {
                character: "Grond".to_string(),
                monster_idx: 0,
                weapon: "sword".to_string(),
            },
        ),
        capture_parity(
            "monster_attack",
            state_with_combat(),
            |state| MonsterAttackCommand.execute(&["0", "Grond"], state),
            GMCommand::MonsterAttack {
                monster_idx: 0,
                character: "Grond".to_string(),
            },
        ),
        capture_parity(
            "morale",
            state_with_morale_selector_divergence(),
            |state| MoraleCommand.execute(&["goblin"], state),
            GMCommand::CheckMorale,
        ),
        capture_parity(
            "turn_undead",
            state_with_combat(),
            |state| TurnUndeadCommand.execute(&["Aldric", "0"], state),
            GMCommand::TurnUndead {
                character: "Aldric".to_string(),
                monster_idx: 0,
            },
        ),
        capture_parity(
            "close",
            state_with_combat(),
            |state| CloseCommand.execute(&["Grond", "30"], state),
            GMCommand::Close {
                character: "Grond".to_string(),
                feet: Some(30),
            },
        ),
        capture_parity(
            "retreat",
            state_with_combat(),
            |state| RetreatCommand.execute(&["Grond"], state),
            GMCommand::Retreat {
                character: "Grond".to_string(),
            },
        ),
        capture_parity(
            "withdrawal",
            state_with_combat(),
            |state| WithdrawalCommand.execute(&["Grond"], state),
            GMCommand::FightingWithdrawal {
                character: "Grond".to_string(),
            },
        ),
        capture_parity(
            "combat_log",
            state_with_combat_log(),
            |state| CombatLogCommand.execute(&[], state),
            GMCommand::QueryCombatLog,
        ),
        capture_parity(
            "declare_spell",
            state_with_combat(),
            |state| DeclareSpellCommand.execute(&["Elara", "magic", "missile"], state),
            GMCommand::DeclareSpell {
                character: "Elara".to_string(),
                spell: "magic missile".to_string(),
            },
        ),
        capture_parity(
            "end_combat",
            state_for_end_combat(),
            |state| EndCombatCommand.execute(&[], state),
            GMCommand::EndCombat,
        ),
    ];

    assert_eq!(snapshots.len(), 12);
    for snapshot in &snapshots {
        assert!(!snapshot.command.is_empty());
        assert!(!snapshot.cli.output.is_empty());
        assert!(!snapshot.api.message.is_empty());
    }

    let serialized = serde_json::to_string_pretty(&snapshots).unwrap();
    assert!(serialized.contains("\"start_combat\""));
    assert!(serialized.contains("\"end_combat\""));

    // Useful when running with --nocapture to inspect parity drift quickly.
    println!("{serialized}");
}
