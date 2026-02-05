use osr_ai_gm::persist::GameState;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

fn hp_color(hp: i32, max_hp: i32) -> Color {
    if max_hp <= 0 {
        return Color::Red;
    }
    let pct = (hp as f64 / max_hp as f64) * 100.0;
    if pct > 50.0 {
        Color::Green
    } else if pct > 25.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn hp_bar(hp: i32, max_hp: i32, width: usize) -> (String, Color) {
    let color = hp_color(hp, max_hp);
    if max_hp <= 0 {
        return ("DEAD".to_string(), Color::Red);
    }
    let ratio = (hp.max(0) as f64 / max_hp as f64).min(1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let bar = format!("[{}{}] {}/{}", "█".repeat(filled), "░".repeat(empty), hp, max_hp);
    (bar, color)
}

pub fn render_party(f: &mut Frame, area: Rect, state: &GameState) {
    let block = Block::default()
        .title(" Party ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if state.party.members.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no party members)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for c in &state.party.members {
            let alive = c.is_alive();
            let name_style = if alive {
                Style::default().bold()
            } else {
                Style::default().bold().fg(Color::DarkGray)
            };

            // Name line: "Aldric  Fighter L3  AC 3"
            let class_name = c.class.name();
            let header = Line::from(vec![
                Span::styled(&c.name, name_style),
                Span::raw("  "),
                Span::styled(
                    format!("{} L{}", class_name, c.level),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("AC {}", c.ac),
                    Style::default().fg(Color::White),
                ),
                if !alive {
                    Span::styled("  DEAD", Style::default().fg(Color::Red).bold())
                } else {
                    Span::raw("")
                },
            ]);
            lines.push(header);

            // HP bar line
            let bar_width = 12;
            let (bar_str, bar_color) = hp_bar(c.hp, c.max_hp, bar_width);
            lines.push(Line::from(vec![
                Span::raw("  HP "),
                Span::styled(bar_str, Style::default().fg(bar_color)),
            ]));

            lines.push(Line::raw(""));
        }
    }

    // Retainers section
    if !state.retainers.is_empty() {
        lines.push(Line::from(Span::styled(
            "── Retainers ──",
            Style::default().fg(Color::DarkGray),
        )));

        for r in &state.retainers {
            let alive = r.is_alive();
            let name_style = if alive {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let header = Line::from(vec![
                Span::styled(&r.name, name_style),
                Span::raw("  "),
                Span::styled(
                    format!("{} L{}", r.class, r.level),
                    Style::default().fg(Color::White),
                ),
                if !alive {
                    Span::styled("  DEAD", Style::default().fg(Color::Red).bold())
                } else {
                    Span::raw("")
                },
            ]);
            lines.push(header);

            if alive {
                let (bar_str, bar_color) = hp_bar(r.hp, r.max_hp, 10);
                lines.push(Line::from(vec![
                    Span::raw("  HP "),
                    Span::styled(bar_str, Style::default().fg(bar_color)),
                ]));
            }

            lines.push(Line::raw(""));
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
