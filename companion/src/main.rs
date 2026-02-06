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
use ratatui_image::picker::Picker;

/// Guard that restores the terminal on drop, even if the event loop exits via `?`.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
    }
}

fn main() -> io::Result<()> {
    // Query terminal for graphics protocol before entering alt screen.
    let picker = Picker::from_query_stdio().ok();

    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let _guard = TerminalGuard;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Spawn file watcher
    let (tx, rx) = mpsc::channel();
    let _watch_path = watcher::spawn_watcher(tx)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut app = app::App::new(picker);

    // Main event loop
    loop {
        // Drain updates from watcher
        while let Ok(update) = rx.try_recv() {
            match update {
                watcher::WatcherUpdate::State(state) => app.update_state(state),
                watcher::WatcherUpdate::Image(path) => app.update_image(&path),
                watcher::WatcherUpdate::Error(msg) => {
                    app.status_message = Some(msg);
                }
            }
        }

        // Draw
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Poll for keyboard events (100ms timeout)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.quit = true,
                    KeyCode::Char('i') => app.show_image = !app.show_image,
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

    // _guard's Drop handles terminal restore, but do it explicitly too for clarity.
    drop(_guard);
    Ok(())
}
