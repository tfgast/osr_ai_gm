use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use osr_ai_gm::persist::GameState;

/// A tagged log entry with its original sequence number for sorting.
struct TaggedEntry {
    seq: u64,
    tag: &'static str,
    message: String,
}

/// Collect log entries from all active subsystems, sorted chronologically.
///
/// Each subsystem's log entries carry a monotonic sequence number (`seq`)
/// assigned from a shared counter.  Merging by `seq` produces true
/// chronological order instead of the old fixed-subsystem order.
///
/// Notes (GM annotations) don't carry sequence numbers, so they're
/// appended at the end.
fn collect_logs(state: &GameState) -> Vec<String> {
    let mut entries: Vec<TaggedEntry> = Vec::new();

    // Combat log
    if let Some(ref combat) = state.combat {
        for entry in &combat.log {
            entries.push(TaggedEntry {
                seq: entry.seq,
                tag: "[Combat]",
                message: entry.message.clone(),
            });
        }
    }

    // Dungeon exploration log
    if let Some(ref dungeon) = state.dungeon {
        for entry in &dungeon.log {
            entries.push(TaggedEntry {
                seq: entry.seq,
                tag: "[Explore]",
                message: entry.message.clone(),
            });
        }
    }

    // Wilderness log
    if let Some(ref wilderness) = state.wilderness {
        for entry in &wilderness.log {
            entries.push(TaggedEntry {
                seq: entry.seq,
                tag: "[Wild]",
                message: entry.message.clone(),
            });
        }
    }

    // Sort by sequence number (stable sort preserves insertion order for
    // entries with the same seq, e.g. from old saves where all seq == 0).
    entries.sort_by_key(|e| e.seq);

    let mut merged: Vec<String> = entries
        .into_iter()
        .map(|e| format!("{} {}", e.tag, e.message))
        .collect();

    // GM notes (no sequence numbers — always appended at the end)
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
