use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use osr_ai_gm::persist::GameState;

const MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

pub enum WatcherUpdate {
    State(GameState),
    Image(PathBuf),
}

pub fn live_state_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(PathBuf::from(home).join(".osr_data").join("live_state.json"))
}

pub fn image_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".osr_data")
        .join("live")
        .join("image.png"))
}

fn try_read_state(path: &PathBuf) -> Option<GameState> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn image_size_ok(path: &PathBuf) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() <= MAX_IMAGE_SIZE)
        .unwrap_or(false)
}

pub fn spawn_watcher(tx: mpsc::Sender<WatcherUpdate>) -> Result<PathBuf, String> {
    let state_path = live_state_path()?;
    let img_path = image_path()?;
    let watch_dir = state_path
        .parent()
        .ok_or_else(|| "live_state.json has no parent dir".to_string())?
        .to_path_buf();

    // Send initial state if file already exists.
    if state_path.exists() {
        if let Some(state) = try_read_state(&state_path) {
            let _ = tx.send(WatcherUpdate::State(state));
        }
    }

    // Send initial image if file already exists and within size limit.
    if img_path.exists() && image_size_ok(&img_path) {
        let _ = tx.send(WatcherUpdate::Image(img_path.clone()));
    }

    let state_clone = state_path.clone();
    let img_clone = img_path;
    thread::spawn(move || {
        let tx = tx;
        let state_path = state_clone;
        let img_path = img_clone;

        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let _ = notify_tx.send(event);
                }
            })
            .expect("failed to create file watcher");

        // Watch the .osr_data directory recursively to catch both
        // live_state.json and live/image.png.
        std::fs::create_dir_all(&watch_dir).ok();
        if let Some(img_dir) = img_path.parent() {
            std::fs::create_dir_all(img_dir).ok();
        }
        watcher
            .watch(watch_dir.as_ref(), RecursiveMode::Recursive)
            .expect("failed to watch directory");

        for event in notify_rx {
            let dominated = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_)
            );
            if !dominated {
                continue;
            }

            // 10ms delay to let atomic rename settle.
            thread::sleep(Duration::from_millis(10));

            for path in &event.paths {
                if path.ends_with("live_state.json") {
                    if let Some(state) = try_read_state(&state_path) {
                        let _ = tx.send(WatcherUpdate::State(state));
                    }
                } else if path.ends_with("image.png")
                    && path.parent().and_then(|p| p.file_name())
                        == Some(std::ffi::OsStr::new("live"))
                {
                    if image_size_ok(&img_path) {
                        let _ = tx.send(WatcherUpdate::Image(img_path.clone()));
                    }
                }
            }
        }
    });

    Ok(state_path)
}
