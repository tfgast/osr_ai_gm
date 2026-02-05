use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use osr_ai_gm::persist::GameState;

fn live_state_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".osr_data").join("live_state.json")
}

fn print_summary(state: &GameState) {
    println!("=== OSR AI GM — Live State ===");
    println!("Mode: {}", state.mode);
    println!(
        "Party: {} member{}",
        state.party.members.len(),
        if state.party.members.len() == 1 { "" } else { "s" }
    );
    for c in &state.party.members {
        println!(
            "  {} — {:?} L{} | HP {}/{} | AC {} | XP {}",
            c.name, c.class, c.level, c.hp, c.max_hp, c.ac, c.xp
        );
    }
    if !state.retainers.is_empty() {
        println!("Retainers: {}", state.retainers.len());
    }
    if let Some(ref combat) = state.combat {
        println!(
            "Combat: round {} | {} monster{} alive | distance {}",
            combat.round,
            combat.living_monster_count(),
            if combat.living_monster_count() == 1 { "" } else { "s" },
            combat.distance,
        );
    }
    println!("Dungeon level: {}", state.dungeon_level);
    println!("Turn: {}", state.turn());
    println!("Gold: {} gp", state.party.gold);
    if !state.notes.is_empty() {
        println!("Notes: {}", state.notes.len());
    }
    println!();
}

fn try_read_and_print(path: &PathBuf) {
    match std::fs::read_to_string(path) {
        Ok(data) => match serde_json::from_str::<GameState>(&data) {
            Ok(state) => print_summary(&state),
            Err(e) => eprintln!("Parse error: {e}"),
        },
        Err(e) => eprintln!("Read error: {e}"),
    }
}

fn main() {
    let path = live_state_path();
    let watch_dir = path.parent().expect("live_state.json has no parent dir");

    println!("Companion watching: {}", path.display());
    println!();

    // Print current state if file already exists.
    if path.exists() {
        try_read_and_print(&path);
    }

    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })
    .expect("failed to create file watcher");

    // Watch the parent directory so we catch atomic renames into the target.
    std::fs::create_dir_all(watch_dir).ok();
    watcher
        .watch(watch_dir.as_ref(), RecursiveMode::NonRecursive)
        .expect("failed to watch directory");

    for event in rx {
        let dominated = matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_)
        );
        if dominated && event.paths.iter().any(|p| p.ends_with("live_state.json")) {
            // 10ms delay to let atomic rename settle.
            thread::sleep(Duration::from_millis(10));
            try_read_and_print(&path);
        }
    }
}
