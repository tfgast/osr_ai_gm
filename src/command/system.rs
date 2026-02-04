use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::{dice, persist};

pub struct RollCommand;
impl Command for RollCommand {
    fn name(&self) -> &str { "roll" }
    fn help(&self) -> &str { "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)" }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: roll <dice expression>");
        }
        let notation = args.join("");
        match dice::roll_str(&notation) {
            Ok(result) => CommandResult::ok(format!("{}", result)),
            Err(e) => CommandResult::error(format!("{}", e)),
        }
    }
}

pub struct SaveCommand;
impl Command for SaveCommand {
    fn name(&self) -> &str { "save" }
    fn help(&self) -> &str { "Save game state (e.g., save game.json)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::save(state, std::path::Path::new(path)) {
            Ok(()) => CommandResult::ok(format!("Game saved to {}", path)),
            Err(e) => CommandResult::error(format!("save failed: {}", e)),
        }
    }
}

pub struct LoadCommand;
impl Command for LoadCommand {
    fn name(&self) -> &str { "load" }
    fn help(&self) -> &str { "Load game state (e.g., load game.json)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::load(std::path::Path::new(path)) {
            Ok(loaded) => {
                let msg = format!(
                    "Loaded: turn {}, dungeon level {}, {} party members{}",
                    loaded.turn(), loaded.dungeon_level, loaded.party.members.len(),
                    if loaded.combat.is_some() { ", combat active" } else { "" }
                );
                *state = loaded;
                CommandResult::ok(msg)
            }
            Err(e) => CommandResult::error(format!("load failed: {}", e)),
        }
    }
}

pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn help(&self) -> &str { "Show available commands" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        // Help is handled by CommandRegistry::dispatch() which has access to
        // the full command list. This execute() is only reached if the command
        // is invoked outside the registry (which shouldn't happen in practice).
        CommandResult::ok("Type 'help' for available commands.")
    }
}

pub struct NoteCommand;
impl Command for NoteCommand {
    fn name(&self) -> &str { "note" }
    fn help(&self) -> &str { "Add a session note (note <text>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: note <text>");
        }
        let text = args.join(" ");
        state.notes.push(text.clone());
        CommandResult::ok(format!("Note added: {}", text))
    }
}

pub struct NotesCommand;
impl Command for NotesCommand {
    fn name(&self) -> &str { "notes" }
    fn help(&self) -> &str { "List all session notes" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        if state.notes.is_empty() {
            return CommandResult::ok("No notes yet.".to_string());
        }
        let mut out = String::from("Session notes:\n");
        for (i, note) in state.notes.iter().enumerate() {
            out.push_str(&format!("  [{}] {}\n", i + 1, note));
        }
        CommandResult::ok(out)
    }
}

pub struct NoteDeleteCommand;
impl Command for NoteDeleteCommand {
    fn name(&self) -> &str { "note_delete" }
    fn help(&self) -> &str { "Delete a note by index (note_delete <index>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: note_delete <index>");
        }
        let index: usize = match args[0].parse() {
            Ok(n) if n >= 1 && n <= state.notes.len() => n,
            _ => return CommandResult::error(format!(
                "index must be 1-{}", state.notes.len()
            )),
        };
        let removed = state.notes.remove(index - 1);
        CommandResult::ok(format!("Deleted note [{}]: {}", index, removed))
    }
}

pub struct QuitCommand;
impl Command for QuitCommand {
    fn name(&self) -> &str { "quit" }
    fn help(&self) -> &str { "Exit the game" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        CommandResult::quit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_add() {
        let mut state = GameState::new();
        let cmd = NoteCommand;
        let result = cmd.execute(&["Found", "a", "secret", "door"], &mut state);
        assert!(result.output.contains("Note added"));
        assert_eq!(state.notes.len(), 1);
        assert_eq!(state.notes[0], "Found a secret door");
    }

    #[test]
    fn note_empty_args() {
        let mut state = GameState::new();
        let cmd = NoteCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
        assert!(state.notes.is_empty());
    }

    #[test]
    fn notes_empty() {
        let mut state = GameState::new();
        let cmd = NotesCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("No notes yet"));
    }

    #[test]
    fn notes_lists_all() {
        let mut state = GameState::new();
        state.notes.push("Clue one".to_string());
        state.notes.push("Clue two".to_string());
        let cmd = NotesCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("[1] Clue one"));
        assert!(result.output.contains("[2] Clue two"));
    }

    #[test]
    fn note_delete_valid() {
        let mut state = GameState::new();
        state.notes.push("Keep".to_string());
        state.notes.push("Remove".to_string());
        state.notes.push("Also keep".to_string());
        let cmd = NoteDeleteCommand;
        let result = cmd.execute(&["2"], &mut state);
        assert!(result.output.contains("Deleted note [2]: Remove"));
        assert_eq!(state.notes.len(), 2);
        assert_eq!(state.notes[0], "Keep");
        assert_eq!(state.notes[1], "Also keep");
    }

    #[test]
    fn note_delete_out_of_range() {
        let mut state = GameState::new();
        state.notes.push("Only note".to_string());
        let cmd = NoteDeleteCommand;
        let result = cmd.execute(&["5"], &mut state);
        assert!(result.output.contains("Error"));
        assert_eq!(state.notes.len(), 1);
    }

    #[test]
    fn note_delete_zero() {
        let mut state = GameState::new();
        state.notes.push("Only note".to_string());
        let cmd = NoteDeleteCommand;
        let result = cmd.execute(&["0"], &mut state);
        assert!(result.output.contains("Error"));
        assert_eq!(state.notes.len(), 1);
    }

    #[test]
    fn note_delete_no_args() {
        let mut state = GameState::new();
        let cmd = NoteDeleteCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn note_delete_not_a_number() {
        let mut state = GameState::new();
        state.notes.push("Note".to_string());
        let cmd = NoteDeleteCommand;
        let result = cmd.execute(&["abc"], &mut state);
        assert!(result.output.contains("Error"));
        assert_eq!(state.notes.len(), 1);
    }
}
