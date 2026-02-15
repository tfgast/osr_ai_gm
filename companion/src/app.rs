use std::path::Path;
use std::time::Instant;

use osr_ai_gm::persist::GameState;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

pub struct App {
    pub state: Option<GameState>,
    pub quit: bool,
    pub show_help: bool,
    pub show_log: bool,
    pub show_image: bool,
    pub log_scroll: u16,
    pub auto_scroll: bool,
    pub last_update: Option<Instant>,
    pub picker: Option<Picker>,
    pub image_state: Option<StatefulProtocol>,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(picker: Option<Picker>) -> Self {
        App {
            state: None,
            quit: false,
            show_help: false,
            show_log: true,
            show_image: true,
            log_scroll: 0,
            auto_scroll: true,
            last_update: None,
            picker,
            image_state: None,
            status_message: None,
        }
    }

    pub fn update_state(&mut self, new_state: GameState) {
        self.state = Some(new_state);
        self.last_update = Some(Instant::now());
    }

    pub fn update_image(&mut self, path: &Path) {
        let picker = match self.picker.as_mut() {
            Some(p) => p,
            None => return,
        };
        let dyn_img = match image::open(path) {
            Ok(img) => img,
            Err(_) => return,
        };
        self.image_state = Some(picker.new_resize_protocol(dyn_img));
    }

    pub fn seconds_since_update(&self) -> Option<u64> {
        self.last_update.map(|t| t.elapsed().as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osr_ai_gm::persist::GameState;

    #[test]
    fn new_app_has_no_state() {
        let app = App::new(None);
        assert!(app.state.is_none());
        assert!(app.last_update.is_none());
        assert!(!app.quit);
        assert!(!app.show_help);
        assert!(app.show_log);
        assert!(app.show_image);
        assert_eq!(app.log_scroll, 0);
        assert!(app.auto_scroll);
    }

    #[test]
    fn update_state_sets_last_update() {
        let mut app = App::new(None);
        assert!(app.last_update.is_none());

        app.update_state(GameState::new());
        assert!(app.state.is_some());
        assert!(app.last_update.is_some());
    }

    #[test]
    fn seconds_since_update_none_when_no_update() {
        let app = App::new(None);
        assert_eq!(app.seconds_since_update(), None);
    }

    #[test]
    fn seconds_since_update_returns_elapsed() {
        let mut app = App::new(None);
        app.update_state(GameState::new());
        // Just after update, should be 0 seconds
        let secs = app.seconds_since_update().unwrap();
        assert!(secs < 2, "expected <2s, got {}", secs);
    }

    #[test]
    fn update_state_replaces_previous() {
        let mut app = App::new(None);

        let mut state1 = GameState::new();
        state1.notes.push("first".to_string());
        app.update_state(state1);

        let mut state2 = GameState::new();
        state2.notes.push("second".to_string());
        app.update_state(state2);

        let notes = &app.state.as_ref().unwrap().notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0], "second");
    }
}
