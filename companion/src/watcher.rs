use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use osr_ai_gm::persist::GameState;

const MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

pub enum WatcherUpdate {
    State(Box<GameState>),
    Image(PathBuf),
    Error(String),
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

fn try_read_state(path: &Path) -> Option<GameState> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn image_size_ok(path: &Path) -> bool {
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
            let _ = tx.send(WatcherUpdate::State(Box::new(state)));
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
        let mut watcher = match notify::recommended_watcher(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let _ = notify_tx.send(event);
                }
            },
        ) {
            Ok(w) => w,
            Err(e) => {
                let _ = tx.send(WatcherUpdate::Error(format!(
                    "failed to create file watcher: {}",
                    e
                )));
                return;
            }
        };

        // Watch the .osr_data directory recursively to catch both
        // live_state.json and live/image.png.
        std::fs::create_dir_all(&watch_dir).ok();
        if let Some(img_dir) = img_path.parent() {
            std::fs::create_dir_all(img_dir).ok();
        }
        if let Err(e) = watcher.watch(watch_dir.as_ref(), RecursiveMode::Recursive) {
            let _ = tx.send(WatcherUpdate::Error(format!(
                "failed to watch directory: {}",
                e
            )));
            return;
        }

        for event in notify_rx {
            let is_write_event = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_)
            );
            if !is_write_event {
                continue;
            }

            // 10ms delay to let atomic rename settle.
            thread::sleep(Duration::from_millis(10));

            for path in &event.paths {
                if path.ends_with("live_state.json") {
                    if let Some(state) = try_read_state(&state_path) {
                        let _ = tx.send(WatcherUpdate::State(Box::new(state)));
                    }
                } else if path.ends_with("image.png")
                    && path.parent().and_then(|p| p.file_name())
                        == Some(std::ffi::OsStr::new("live"))
                    && image_size_ok(&img_path) {
                        let _ = tx.send(WatcherUpdate::Image(img_path.clone()));
                    }
            }
        }
    });

    Ok(state_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn try_read_state_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let state = GameState::new();
        let json = serde_json::to_string(&state).unwrap();
        std::fs::write(&path, &json).unwrap();

        let result = try_read_state(&path);
        assert!(result.is_some());
    }

    #[test]
    fn try_read_state_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(&path, "not valid json {{{").unwrap();

        let result = try_read_state(&path);
        assert!(result.is_none());
    }

    #[test]
    fn try_read_state_missing_file() {
        let path = Path::new("/tmp/nonexistent_osr_test_state.json");
        let result = try_read_state(path);
        assert!(result.is_none());
    }

    #[test]
    fn image_size_ok_small_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.png");
        std::fs::write(&path, b"tiny").unwrap();
        assert!(image_size_ok(&path));
    }

    #[test]
    fn image_size_ok_at_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("limit.png");
        let mut f = std::fs::File::create(&path).unwrap();
        // Write exactly MAX_IMAGE_SIZE bytes
        let buf = vec![0u8; MAX_IMAGE_SIZE as usize];
        f.write_all(&buf).unwrap();
        assert!(image_size_ok(&path));
    }

    #[test]
    fn image_size_ok_over_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.png");
        let mut f = std::fs::File::create(&path).unwrap();
        let buf = vec![0u8; (MAX_IMAGE_SIZE + 1) as usize];
        f.write_all(&buf).unwrap();
        assert!(!image_size_ok(&path));
    }

    #[test]
    fn image_size_ok_missing_file() {
        let path = Path::new("/tmp/nonexistent_osr_test_image.png");
        assert!(!image_size_ok(path));
    }

    #[test]
    fn live_state_path_returns_expected() {
        // This test relies on HOME being set, which is true in normal environments
        let result = live_state_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with(".osr_data/live_state.json"));
    }

    #[test]
    fn image_path_returns_expected() {
        let result = image_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with(".osr_data/live/image.png"));
    }
}
