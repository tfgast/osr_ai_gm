//! JSONL REPL for the GM API.
//!
//! Reads one JSON `GMRequest` per line from stdin, processes it through
//! the same engine as the HTTP server, and writes the JSON `GMResponse`
//! to stdout. Designed for piping from AI agents (Codex, Claude, etc.)
//! that cannot open network sockets.
//!
//! Usage:
//!   echo '{"id":"1","command":{"type":"QueryState"}}' | gmrepl
//!   cat requests.jsonl | gmrepl

use osr_ai_gm::gmapi::interface::handle_request;
use osr_ai_gm::gmapi::protocol::{extract_request_id, parse_request, serialize_response, GMResponse};
use osr_ai_gm::persist::GameState;
use osr_ai_gm::state::game::GameMode;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut state = GameState::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let resp = GMResponse::err("", format!("stdin error: {}", e), GameMode::Idle);
                let _ = writeln!(stdout, "{}", serialize_response(&resp));
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let resp = match parse_request(&line) {
            Ok(req) => handle_request(&req, &mut state),
            Err(e) => GMResponse::err(&extract_request_id(&line), &e, GameMode::Idle),
        };

        if writeln!(stdout, "{}", serialize_response(&resp)).is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}
