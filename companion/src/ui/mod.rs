mod image;
mod location;
mod log;
mod party;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: body + footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    let body = outer[0];
    let footer_area = outer[1];

    // 2x2 grid: two rows, two columns
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bot_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    // Top-left: Party panel
    if let Some(ref state) = app.state {
        party::render_party(f, top_cols[0], state);
    } else {
        let block = Block::default()
            .title(" Party ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(top_cols[0]);
        f.render_widget(block, top_cols[0]);
        let p = Paragraph::new("(waiting for state...)")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
    }

    // Top-right: Location panel
    if let Some(ref state) = app.state {
        location::render_location(f, top_cols[1], state);
    } else {
        let block = Block::default()
            .title(" Location ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(top_cols[1]);
        f.render_widget(block, top_cols[1]);
        let p = Paragraph::new("(waiting for state...)")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
    }

    // Bottom-left: Log panel
    if app.show_log {
        if let Some(ref state) = app.state {
            log::render_log(f, bot_cols[0], state, app.log_scroll);
        } else {
            render_placeholder(f, bot_cols[0], " Log ", Color::Green);
        }
    } else {
        let block = Block::default()
            .title(" Log [hidden] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(bot_cols[0]);
        f.render_widget(block, bot_cols[0]);
        let p = Paragraph::new("(press l to show)")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(p, inner);
    }

    // Bottom-right: Image panel
    image::render_image(f, bot_cols[1], app);

    // Footer bar
    render_footer(f, footer_area, app);

    // Help overlay
    if app.show_help {
        render_help_overlay(f, size);
    }
}

fn render_placeholder(f: &mut Frame, area: Rect, title: &str, color: Color) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let p = Paragraph::new("(waiting for state...)")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(p, inner);
}

pub(crate) fn hp_color(hp: i32, max_hp: i32) -> Color {
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

pub(crate) fn hp_bar(hp: i32, max_hp: i32, width: usize) -> (String, Color) {
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

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let update_str = match app.seconds_since_update() {
        Some(s) => format!("{}s ago", s),
        None => "never".to_string(),
    };

    let mode_str = match &app.state {
        Some(s) => s.mode.to_string(),
        None => "—".to_string(),
    };

    let footer = Line::from(vec![
        Span::styled(" [Watching: state.json] ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("[Last update: {}] ", update_str),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("[Mode: {}] ", mode_str),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("[i=Image ?=Help q=Quit]", Style::default().fg(Color::DarkGray)),
    ]);

    let p = Paragraph::new(footer);
    f.render_widget(p, area);
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let w = 40u16.min(area.width.saturating_sub(4));
    let h = 16u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let popup = Rect::new(x, y, w, h);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    let text = vec![
        Line::raw(""),
        Line::raw("  q        Quit"),
        Line::raw("  i        Toggle image panel"),
        Line::raw("  ?        Toggle this help"),
        Line::raw("  l        Toggle log panel"),
        Line::raw("  j / Down Scroll log down"),
        Line::raw("  k / Up   Scroll log up"),
        Line::raw(""),
        Line::styled(
            "  OSR AI GM Companion TUI",
            Style::default().fg(Color::Cyan),
        ),
        Line::styled(
            "  Watches ~/.osr_data/live_state.json",
            Style::default().fg(Color::DarkGray),
        ),
        Line::styled(
            "  Image:  ~/.osr_data/live/image.png",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup);
}
