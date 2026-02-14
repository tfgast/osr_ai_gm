use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

/// A failed command entry for playtesting telemetry.
#[derive(Debug, Clone, Serialize)]
pub struct FailedCommand {
    pub timestamp: u64,
    pub raw_input: String,
    pub category: &'static str,
    pub error_message: String,
    pub game_mode: String,
}

/// Reconstruct the player's input from parsed command name and arguments.
pub fn reconstruct_input(name: &str, args: &[&str]) -> String {
    if args.is_empty() {
        name.to_string()
    } else {
        format!("{} {}", name, args.join(" "))
    }
}

/// Log a failed command to the telemetry file. Silently ignores all errors.
pub fn log_failed_command(entry: &FailedCommand) {
    if let Some(dir) = telemetry_dir() {
        let _ = log_to_path(entry, &dir.join("commands.jsonl"));
    }
}

fn telemetry_dir() -> Option<PathBuf> {
    crate::persist::data_dir().ok().map(|d| d.join("telemetry"))
}

fn log_to_path(entry: &FailedCommand, path: &Path) -> io::Result<()> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(entry).map_err(io::Error::other)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::lock_env;

    #[test]
    fn reconstruct_input_no_args() {
        assert_eq!(reconstruct_input("help", &[]), "help");
    }

    #[test]
    fn reconstruct_input_with_args() {
        assert_eq!(
            reconstruct_input("attack", &["Aldric", "0", "sword"]),
            "attack Aldric 0 sword"
        );
    }

    #[test]
    fn failed_command_serializes_to_json() {
        let entry = FailedCommand {
            timestamp: 1700000000,
            raw_input: "xyzzy".to_string(),
            category: "unknown_command",
            error_message: "Error: unknown command: 'xyzzy'. Type 'help' for commands."
                .to_string(),
            game_mode: "exploration".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"category\":\"unknown_command\""));
        assert!(json.contains("\"raw_input\":\"xyzzy\""));
        assert!(json.contains("\"game_mode\":\"exploration\""));
    }

    #[test]
    fn log_to_path_writes_jsonl() {
        let dir = std::env::temp_dir().join("osr_telemetry_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("commands.jsonl");

        let entry = FailedCommand {
            timestamp: 1700000000,
            raw_input: "go north".to_string(),
            category: "unknown_command",
            error_message: "Error: unknown command: 'go'.".to_string(),
            game_mode: "exploration".to_string(),
        };

        log_to_path(&entry, &path).unwrap();
        log_to_path(&entry, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed["category"], "unknown_command");
            assert_eq!(parsed["raw_input"], "go north");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_does_not_panic_on_missing_home() {
        let original = std::env::var("HOME").ok();
        unsafe { std::env::remove_var("HOME") };

        let entry = FailedCommand {
            timestamp: 0,
            raw_input: "test".to_string(),
            category: "unknown_command",
            error_message: "Error: test".to_string(),
            game_mode: "idle".to_string(),
        };
        log_failed_command(&entry); // must not panic

        if let Some(home) = original {
            unsafe { std::env::set_var("HOME", home) };
        }
    }

    #[test]
    fn telemetry_dir_respects_osr_data_dir() {
        let _env = lock_env();
        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::set_var("OSR_DATA_DIR", "/tmp/custom_osr") };

        let dir = telemetry_dir().unwrap();
        assert_eq!(dir, std::path::PathBuf::from("/tmp/custom_osr/telemetry"));

        match orig {
            Some(v) => unsafe { std::env::set_var("OSR_DATA_DIR", v) },
            None => unsafe { std::env::remove_var("OSR_DATA_DIR") },
        }
    }
}
