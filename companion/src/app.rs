use std::time::Instant;

use osr_ai_gm::persist::GameState;

pub struct App {
    pub state: Option<GameState>,
    pub quit: bool,
    pub show_help: bool,
    pub show_log: bool,
    pub log_scroll: u16,
    pub last_update: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        App {
            state: None,
            quit: false,
            show_help: false,
            show_log: true,
            log_scroll: 0,
            last_update: None,
        }
    }

    pub fn update_state(&mut self, new_state: GameState) {
        self.state = Some(new_state);
        self.last_update = Some(Instant::now());
    }

    pub fn seconds_since_update(&self) -> Option<u64> {
        self.last_update.map(|t| t.elapsed().as_secs())
    }
}
