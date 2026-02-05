use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use osr_ai_gm::persist::GameState;

pub fn live_state_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".osr_data").join("live_state.json")
}

fn try_read(path: &PathBuf) -> Option<GameState> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn spawn_watcher(tx: mpsc::Sender<GameState>) -> PathBuf {
    let path = live_state_path();
    let watch_dir = path.parent().expect("live_state.json has no parent dir").to_path_buf();

    // Send initial state if file already exists.
    if path.exists() {
        if let Some(state) = try_read(&path) {
            let _ = tx.send(state);
        }
    }

    let path_clone = path.clone();
    thread::spawn(move || {
        let tx = tx;
        let path = path_clone;

        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let _ = notify_tx.send(event);
            }
        })
        .expect("failed to create file watcher");

        std::fs::create_dir_all(&watch_dir).ok();
        watcher
            .watch(watch_dir.as_ref(), RecursiveMode::NonRecursive)
            .expect("failed to watch directory");

        for event in notify_rx {
            let dominated = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_)
            );
            if dominated && event.paths.iter().any(|p| p.ends_with("live_state.json")) {
                // 10ms delay to let atomic rename settle.
                thread::sleep(Duration::from_millis(10));
                if let Some(state) = try_read(&path) {
                    let _ = tx.send(state);
                }
            }
        }
    });

    path
}
