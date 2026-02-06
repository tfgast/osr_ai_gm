use std::path::PathBuf;
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

    pub fn update_image(&mut self, path: &PathBuf) {
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
