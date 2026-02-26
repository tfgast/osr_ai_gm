use osr_ai_gm::model::PHASE_SEQUENCE;
use osr_ai_gm::persist::GameState;
use osr_ai_gm::state::dungeon::DoorState;
use osr_ai_gm::state::game::GameMode;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

fn door_state_style(state: &DoorState) -> (Color, &'static str) {
    match state {
        DoorState::Open => (Color::Green, "open"),
        DoorState::Spiked => (Color::Green, "spiked open"),
        DoorState::Closed => (Color::Yellow, "closed"),
        DoorState::Stuck => (Color::Yellow, "stuck"),
        DoorState::Locked => (Color::Red, "locked"),
        DoorState::Secret => (Color::Magenta, "secret"),
    }
}

use super::hp_bar;

pub fn render_location(f: &mut Frame, area: Rect, state: &GameState) {
    let block = Block::default()
        .title(" Location ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = match &state.mode {
        GameMode::Exploration => render_exploration(state),
        GameMode::Combat => render_combat(state),
        GameMode::Wilderness => render_wilderness(state),
        _ => render_idle(state),
    };

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn render_exploration(state: &GameState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let dungeon = match &state.dungeon {
        Some(d) => d,
        None => {
            lines.push(Line::from(Span::styled(
                "(no dungeon loaded)",
                Style::default().fg(Color::DarkGray),
            )));
            return lines;
        }
    };

    // Room header
    let (room_name, room_id_str) = match dungeon.current_room {
        Some(id) => {
            let name = dungeon
                .find_room(id)
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "unknown".to_string());
            (name, format!("#{}", id))
        }
        None => ("No room".to_string(), "—".to_string()),
    };

    lines.push(Line::from(vec![
        Span::styled(room_name, Style::default().fg(Color::White).bold()),
        Span::raw("  "),
        Span::styled(room_id_str, Style::default().fg(Color::DarkGray)),
    ]));

    // Level
    lines.push(Line::from(vec![
        Span::styled("Level ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            dungeon.level.to_string(),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} explored", dungeon.explored.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Time tracker
    if let Some(ref time) = state.time {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Turn ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                time.total_turns.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Activity {}/5", time.turns_since_rest),
                Style::default().fg(if time.needs_rest() {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            ),
        ]));

        // Light sources
        match time.light_summary() {
            Some(summary) => {
                lines.push(Line::from(Span::styled(
                    summary,
                    Style::default().fg(Color::Yellow),
                )));
            }
            None => {
                lines.push(Line::from(Span::styled(
                    "No light — DARKNESS!",
                    Style::default().fg(Color::Red).bold(),
                )));
            }
        }
    }

    // Exits
    let doors = dungeon.doors_from_current();
    if let Some(current) = dungeon.current_room {
      if !doors.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "── Exits ──",
            Style::default().fg(Color::DarkGray),
        )));

        for d in &doors {
            let dest = if d.room_a == current {
                d.room_b
            } else {
                d.room_a
            };
            let dest_name = dungeon
                .find_room(dest)
                .map(|r| r.name.as_str())
                .unwrap_or("?");
            let (color, label) = door_state_style(&d.state);

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  → {}", dest_name),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  "),
                Span::styled(format!("[{}]", label), Style::default().fg(color)),
            ]));
        }
      }
    }

    lines
}

/// Short label for each phase used in the progression bar.
fn phase_short(phase: &str) -> &'static str {
    match phase {
        "Declaration" => "Decl",
        "Initiative" => "Init",
        "Morale" => "Moral",
        "Movement" => "Move",
        "Missile" => "Miss",
        "Magic" => "Magic",
        "Melee" => "Melee",
        "EndOfRound" => "End",
        _ => "???",
    }
}

fn render_phase_bar(current: &str) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    for (i, &phase) in PHASE_SEQUENCE.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" › ", Style::default().fg(Color::DarkGray)));
        }
        let label = phase_short(phase);
        if phase == current {
            spans.push(Span::styled(
                format!("[{}]", label),
                Style::default().fg(Color::Yellow).bold(),
            ));
        } else {
            spans.push(Span::styled(
                label.to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    Line::from(spans)
}

fn render_combat(state: &GameState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let combat = match &state.combat {
        Some(c) => c,
        None => {
            lines.push(Line::from(Span::styled(
                "(no active combat)",
                Style::default().fg(Color::DarkGray),
            )));
            return lines;
        }
    };

    // Round header
    lines.push(Line::from(vec![
        Span::styled("COMBAT", Style::default().fg(Color::Red).bold()),
        Span::raw("  "),
        Span::styled(
            format!("Round {}", combat.round),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{}'", combat.distance),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Phase progression bar
    lines.push(render_phase_bar(&combat.phase));

    // Initiative winner callout (only after initiative has been rolled)
    if combat.round > 0 {
        let (winner_label, winner_color) =
            if combat.party_initiative > combat.monster_initiative {
                ("PARTY FIRST", Color::Cyan)
            } else if combat.monster_initiative > combat.party_initiative {
                ("MONSTERS FIRST", Color::Red)
            } else {
                ("SIMULTANEOUS", Color::Yellow)
            };

        lines.push(Line::from(vec![
            Span::styled("Initiative  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Party {}", combat.party_initiative),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" vs ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Monsters {}", combat.monster_initiative),
                Style::default().fg(Color::Red),
            ),
            Span::raw("  "),
            Span::styled(
                format!("▸ {}", winner_label),
                Style::default().fg(winner_color).bold(),
            ),
        ]));
    }

    // Party action tracker
    let alive_members: Vec<_> = state
        .party
        .members
        .iter()
        .filter(|c| c.is_alive())
        .collect();
    if !alive_members.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "── Party Actions ──",
            Style::default().fg(Color::DarkGray),
        )));
        for c in &alive_members {
            let acted = combat.characters_acted.contains(&c.name);
            let (marker, marker_color) = if acted {
                ("✓", Color::Green)
            } else {
                ("·", Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), Style::default().fg(marker_color)),
                Span::styled(
                    c.name.clone(),
                    if acted {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]));
        }
    }

    // Spell declarations
    if !combat.spell_declarations.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "── Spells Declared ──",
            Style::default().fg(Color::DarkGray),
        )));
        for caster in &combat.spell_declarations {
            let is_disrupted = combat.disrupted.contains(caster);
            let style = if is_disrupted {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Magenta)
            };
            let suffix = if is_disrupted { " DISRUPTED" } else { "" };
            lines.push(Line::from(Span::styled(
                format!("  {}{}", caster, suffix),
                style,
            )));
        }
    }

    // Monsters with attack tracking
    let living: Vec<_> = combat.living_monsters();
    if !living.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "── Monsters ({}/{}) ──",
                living.len(),
                combat.monsters.len()
            ),
            Style::default().fg(Color::DarkGray),
        )));

        for (idx, m) in &living {
            let mut spans = vec![Span::styled(
                format!("  {}", m.name),
                Style::default().fg(Color::White),
            )];

            if m.turned {
                spans.push(Span::styled(
                    " [turned]",
                    Style::default().fg(Color::Yellow),
                ));
            }
            if m.helpless {
                spans.push(Span::styled(
                    " [helpless]",
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Attack usage indicator
            let total_attacks = m.attack_routines.len().max(1);
            let used = combat
                .monsters_attacked_this_round
                .get(idx)
                .copied()
                .unwrap_or(0);
            if used > 0 || total_attacks > 1 {
                spans.push(Span::styled(
                    format!("  [{}/{}]", used, total_attacks),
                    Style::default().fg(if used >= total_attacks {
                        Color::DarkGray
                    } else {
                        Color::White
                    }),
                ));
            }

            lines.push(Line::from(spans));

            // HP bar
            let (bar_str, bar_color) = hp_bar(m.hp, m.max_hp, 10);
            lines.push(Line::from(vec![
                Span::raw("    HP "),
                Span::styled(bar_str, Style::default().fg(bar_color)),
            ]));
        }
    }

    lines
}

fn render_wilderness(state: &GameState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let wild = match &state.wilderness {
        Some(w) => w,
        None => {
            lines.push(Line::from(Span::styled(
                "(no wilderness loaded)",
                Style::default().fg(Color::DarkGray),
            )));
            return lines;
        }
    };

    // Terrain and position
    let terrain_name = wild
        .current_hex()
        .map(|h| h.terrain.name())
        .unwrap_or("unknown");

    lines.push(Line::from(vec![
        Span::styled(
            terrain_name.to_string(),
            Style::default().fg(Color::White).bold(),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Hex ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("({}, {})", wild.current_x, wild.current_y),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} explored", wild.explored.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Travel day
    lines.push(Line::from(vec![
        Span::styled("Day ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            wild.travel_day.to_string(),
            Style::default().fg(Color::White),
        ),
    ]));

    // Lost status
    if wild.lost {
        lines.push(Line::from(Span::styled(
            "LOST",
            Style::default().fg(Color::Red).bold(),
        )));
    }

    lines
}

fn render_idle(state: &GameState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "No active exploration",
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::raw(""));

    // Party summary
    let alive = state.party.members.iter().filter(|c| c.is_alive()).count();
    let total = state.party.members.len();

    if total > 0 {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}/{} alive", alive, total),
                Style::default().fg(if alive == total {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            ),
        ]));
    }

    let retainer_count = state.retainers.len();
    if retainer_count > 0 {
        let alive_r = state.retainers.iter().filter(|r| r.is_alive()).count();
        lines.push(Line::from(Span::styled(
            format!("{} retainer{}", alive_r, if retainer_count == 1 { "" } else { "s" }),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}
