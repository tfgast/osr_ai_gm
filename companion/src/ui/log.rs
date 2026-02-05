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

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(paragraph, inner);
}
