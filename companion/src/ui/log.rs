use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use osr_ai_gm::persist::GameState;

/// Collect and deduplicate log entries from all active subsystems.
fn collect_logs(state: &GameState) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();

    // Combat log
    if let Some(ref combat) = state.combat {
        for entry in &combat.log {
            merged.push(format!("[Combat] {}", entry));
        }
    }

    // Dungeon exploration log
    if let Some(ref dungeon) = state.dungeon {
        for entry in &dungeon.log {
            merged.push(format!("[Explore] {}", entry));
        }
    }

    // Wilderness log
    if let Some(ref wilderness) = state.wilderness {
        for entry in &wilderness.log {
            merged.push(format!("[Wild] {}", entry));
        }
    }

    // GM notes
    for note in &state.notes {
        merged.push(format!("[Note] {}", note));
    }

    // Deduplicate consecutive identical entries
    merged.dedup();

    merged
}

pub fn render_log(f: &mut Frame, area: Rect, state: &GameState, scroll: u16) {
    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let logs = collect_logs(state);

    if logs.is_empty() {
        let p = Paragraph::new("(no events)")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    let lines: Vec<Line> = logs
        .iter()
        .map(|entry| {
            let (tag_end, color) = if entry.starts_with("[Combat]") {
                (8, Color::Red)
            } else if entry.starts_with("[Explore]") {
                (9, Color::Yellow)
            } else if entry.starts_with("[Wild]") {
                (6, Color::Cyan)
            } else if entry.starts_with("[Note]") {
                (6, Color::Magenta)
            } else {
                (0, Color::White)
            };

            if tag_end > 0 {
                Line::from(vec![
                    Span::styled(&entry[..tag_end], Style::default().fg(color)),
                    Span::raw(&entry[tag_end..]),
                ])
            } else {
                Line::raw(entry.as_str())
            }
        })
        .collect();

    let max_scroll = (logs.len() as u16).saturating_sub(inner.height);
    let clamped_scroll = scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((clamped_scroll, 0));

    f.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use osr_ai_gm::model::CombatState;
    use osr_ai_gm::state::dungeon::DungeonState;
    use osr_ai_gm::state::wilderness::WildernessState;

    #[test]
    fn collect_logs_empty_state() {
        let state = GameState::new();
        let logs = collect_logs(&state);
        assert!(logs.is_empty());
    }

    #[test]
    fn collect_logs_combat_entries() {
        let mut state = GameState::new();
        let mut combat = CombatState::new(vec![], 30);
        combat.log.push("Aldric attacks Goblin".to_string());
        combat.log.push("Goblin takes 5 damage".to_string());
        state.combat = Some(combat);

        let logs = collect_logs(&state);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0], "[Combat] Aldric attacks Goblin");
        assert_eq!(logs[1], "[Combat] Goblin takes 5 damage");
    }

    #[test]
    fn collect_logs_dungeon_entries() {
        let mut state = GameState::new();
        let mut dungeon = DungeonState::new(1);
        dungeon.log.push("Entered room 3".to_string());
        state.dungeon = Some(dungeon);

        let logs = collect_logs(&state);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], "[Explore] Entered room 3");
    }

    #[test]
    fn collect_logs_wilderness_entries() {
        let mut state = GameState::new();
        let mut wild = WildernessState::new();
        wild.log.push("Day 1 travel begins".to_string());
        state.wilderness = Some(wild);

        let logs = collect_logs(&state);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], "[Wild] Day 1 travel begins");
    }

    #[test]
    fn collect_logs_notes() {
        let mut state = GameState::new();
        state.notes.push("Found a secret passage".to_string());

        let logs = collect_logs(&state);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], "[Note] Found a secret passage");
    }

    #[test]
    fn collect_logs_merges_all_sources() {
        let mut state = GameState::new();

        let mut combat = CombatState::new(vec![], 30);
        combat.log.push("combat entry".to_string());
        state.combat = Some(combat);

        let mut dungeon = DungeonState::new(1);
        dungeon.log.push("dungeon entry".to_string());
        state.dungeon = Some(dungeon);

        let mut wild = WildernessState::new();
        wild.log.push("wilderness entry".to_string());
        state.wilderness = Some(wild);

        state.notes.push("a note".to_string());

        let logs = collect_logs(&state);
        assert_eq!(logs.len(), 4);
        // Verify ordering: combat, dungeon, wilderness, notes
        assert!(logs[0].starts_with("[Combat]"));
        assert!(logs[1].starts_with("[Explore]"));
        assert!(logs[2].starts_with("[Wild]"));
        assert!(logs[3].starts_with("[Note]"));
    }

    #[test]
    fn collect_logs_deduplicates_consecutive() {
        let mut state = GameState::new();
        state.notes.push("same entry".to_string());
        state.notes.push("same entry".to_string());
        state.notes.push("different entry".to_string());
        state.notes.push("same entry".to_string());

        let logs = collect_logs(&state);
        // dedup removes consecutive duplicates: "[Note] same entry" x2 -> 1
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0], "[Note] same entry");
        assert_eq!(logs[1], "[Note] different entry");
        assert_eq!(logs[2], "[Note] same entry");
    }
}
