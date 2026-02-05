mod app;
mod ui;
mod watcher;

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;

fn main() -> io::Result<()> {
    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Spawn file watcher
    let (tx, rx) = mpsc::channel();
    let _watch_path = watcher::spawn_watcher(tx);

    let mut app = app::App::new();

    // Main event loop
    loop {
        // Drain state updates from watcher
        while let Ok(state) = rx.try_recv() {
            app.update_state(state);
        }

        // Draw
        terminal.draw(|f| ui::draw(f, &app))?;

        // Poll for keyboard events (100ms timeout)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.quit = true,
                    KeyCode::Char('?') => app.show_help = !app.show_help,
                    KeyCode::Char('l') => app.show_log = !app.show_log,
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.log_scroll = app.log_scroll.saturating_add(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.log_scroll = app.log_scroll.saturating_sub(1);
                    }
                    KeyCode::Esc => {
                        if app.show_help {
                            app.show_help = false;
                        } else {
                            app.quit = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
