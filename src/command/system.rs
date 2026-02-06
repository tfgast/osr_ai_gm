use super::{Command, CommandResult};
use crate::engine::system;
use crate::persist::GameState;

pub struct RollCommand;
impl Command for RollCommand {
    fn name(&self) -> &str { "roll" }
    fn help(&self) -> &str { "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)" }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: roll <dice expression>");
        }
        let notation = args.join("");
        match system::action_roll_dice(&notation) {
            Ok(result) => CommandResult::ok(result.rendered),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct SaveCommand;
impl Command for SaveCommand {
    fn name(&self) -> &str { "save" }
    fn help(&self) -> &str { "Save game state (e.g., save mycamp)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let filename = args.first().copied().unwrap_or("save");
        match system::action_save_game(state, filename) {
            Ok(result) => CommandResult::ok(format!("Game saved to {}", result.path.display())),
            Err(e) => CommandResult::error(format!("save failed: {}", e)),
        }
    }
}

pub struct LoadCommand;
impl Command for LoadCommand {
    fn name(&self) -> &str { "load" }
    fn help(&self) -> &str { "Load game state (e.g., load mycamp)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let filename = args.first().copied().unwrap_or("save");
        match system::action_load_game(state, filename) {
            Ok(result) => {
                let msg = format!(
                    "Loaded: turn {}, dungeon level {}, {} party members{}",
                    result.turn,
                    result.dungeon_level,
                    result.party_members,
                    if result.combat_active {
                        ", combat active"
                    } else {
                        ""
                    }
                );
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
        match system::action_help(&[]) {
            Ok(result) => CommandResult::ok(result.output),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        if state.notes.is_empty() {
            return CommandResult::error("no notes to delete");
        }
        let n: usize = match args[0].parse() {
            Ok(n) => n,
            Err(_) => return CommandResult::error("index must be a positive integer"),
        };
        if n < 1 {
            return CommandResult::error("notes use 1-based indexing; first note is index 1");
        }
        if n > state.notes.len() {
            return CommandResult::error(format!(
                "index {} out of range; have {} note{}",
                n,
                state.notes.len(),
                if state.notes.len() == 1 { "" } else { "s" }
            ));
        }
        let removed = state.notes.remove(n - 1);
        CommandResult::ok(format!("Deleted note [{}]: {}", n, removed))
    }
}

pub struct QuitCommand;
impl Command for QuitCommand {
    fn name(&self) -> &str { "quit" }
    fn help(&self) -> &str { "Exit the game" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        match system::action_quit() {
            Ok(_) => CommandResult::quit(),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        assert!(result.output.contains("1-based indexing"));
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
