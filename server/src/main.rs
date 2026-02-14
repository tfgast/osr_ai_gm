use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use osr_ai_gm::auth::TokenStore;
use osr_ai_gm::gmapi::interface::handle_request;
use osr_ai_gm::gmapi::protocol::{self, GMResponse};
use osr_ai_gm::persist::{self, GameState};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct AppState {
    game: Mutex<GameState>,
    tokens: TokenStore,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Subcommand: token management
    if args.len() >= 2 {
        match args[1].as_str() {
            "token" => {
                handle_token_command(&args[2..]);
                return;
            }
            "help" | "--help" | "-h" => {
                print_usage();
                return;
            }
            _ => {}
        }
    }

    // Load token store
    let token_path = token_path_from_args(&args);
    let tokens = match TokenStore::load(&token_path) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("error: failed to load token store from {}: {}", token_path.display(), e);
            std::process::exit(1);
        }
    };

    if tokens.tokens.is_empty() {
        eprintln!("warning: no API tokens configured. All requests will be rejected.");
        eprintln!("  Create a token: osr-gm-server token create <name>");
    }

    let port = port_from_args(&args);
    let bind_addr = bind_addr_from_args(&args);

    let state = Arc::new(AppState {
        game: Mutex::new(GameState::new()),
        tokens,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/gm", post(gm_endpoint))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", bind_addr, port)
        .parse()
        .unwrap_or_else(|_| {
            eprintln!("error: invalid bind address");
            std::process::exit(1);
        });

    eprintln!("OSR GM Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| {
        eprintln!("error: failed to bind to {}: {}", addr, e);
        std::process::exit(1);
    });
    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("error: server failed: {}", e);
        std::process::exit(1);
    });
}

async fn health() -> &'static str {
    "ok"
}

async fn gm_endpoint(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    // Extract and validate bearer token.
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "missing or malformed Authorization header; expected: Bearer <token>",
            );
        }
    };

    if !state.tokens.validate(token) {
        return error_response(StatusCode::UNAUTHORIZED, "invalid API token");
    }

    // Parse the GM request.
    let req = match protocol::parse_request(&body) {
        Ok(r) => r,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &e);
        }
    };

    // Execute the command against game state.
    let response = {
        let mut game = state.game.lock().unwrap_or_else(|e| {
            // Poisoned mutex — recover by taking the inner value.
            e.into_inner()
        });
        let resp = handle_request(&req, &mut game);
        // Export live state for the companion TUI (best-effort, ignore errors).
        let _ = persist::export_live_state(&game);
        resp
    };

    let json = protocol::serialize_response(&response);
    json_response(StatusCode::OK, json)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get("authorization")?.to_str().ok()?;
    let stripped = value.strip_prefix("Bearer ")?;
    if stripped.is_empty() {
        return None;
    }
    Some(stripped)
}

fn json_response(status: StatusCode, body: String) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    (status, [(header::CONTENT_TYPE, "application/json")], body)
}

fn error_response(status: StatusCode, msg: &str) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    let resp = GMResponse::err("", msg, osr_ai_gm::state::game::GameMode::Idle);
    json_response(status, protocol::serialize_response(&resp))
}

// ---------------------------------------------------------------------------
// CLI helpers
// ---------------------------------------------------------------------------

fn print_usage() {
    eprintln!(
        "OSR GM Server — authenticated HTTP API for the OSR AI Game Master

USAGE:
    osr-gm-server [OPTIONS]              Start the server
    osr-gm-server token create <name>    Generate a new API token
    osr-gm-server token list             List configured tokens
    osr-gm-server token revoke <name>    Revoke a token by name

OPTIONS:
    --port <PORT>           Listen port (default: 3000, or OSR_GM_PORT env)
    --bind <ADDR>           Bind address (default: 127.0.0.1, or OSR_GM_BIND env)
    --token-file <PATH>     Token store path (default: ~/.osr_data/api_tokens.json)"
    );
}

fn port_from_args(args: &[String]) -> u16 {
    for i in 0..args.len() {
        if args[i] == "--port" {
            if let Some(val) = args.get(i + 1) {
                return val.parse().unwrap_or_else(|_| {
                    eprintln!("error: invalid port number");
                    std::process::exit(1);
                });
            }
        }
    }
    std::env::var("OSR_GM_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000)
}

fn bind_addr_from_args(args: &[String]) -> String {
    for i in 0..args.len() {
        if args[i] == "--bind" {
            if let Some(val) = args.get(i + 1) {
                return val.clone();
            }
        }
    }
    std::env::var("OSR_GM_BIND")
        .ok()
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn token_path_from_args(args: &[String]) -> PathBuf {
    for i in 0..args.len() {
        if args[i] == "--token-file" {
            if let Some(val) = args.get(i + 1) {
                return PathBuf::from(val);
            }
        }
    }
    TokenStore::default_path()
}

fn handle_token_command(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: osr-gm-server token <create|list|revoke> [args]");
        std::process::exit(1);
    }

    let token_path = TokenStore::default_path();

    match args[0].as_str() {
        "create" => {
            let name = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            let mut store = TokenStore::load(&token_path).unwrap_or_else(|e| {
                eprintln!("error loading tokens: {}", e);
                std::process::exit(1);
            });
            let token = store.create_token(name);
            store.save(&token_path).unwrap_or_else(|e| {
                eprintln!("error saving tokens: {}", e);
                std::process::exit(1);
            });
            println!("{}", token.token);
            eprintln!("Token '{}' created and saved to {}", name, token_path.display());
        }
        "list" => {
            let store = TokenStore::load(&token_path).unwrap_or_else(|e| {
                eprintln!("error loading tokens: {}", e);
                std::process::exit(1);
            });
            if store.tokens.is_empty() {
                eprintln!("No tokens configured.");
            } else {
                for t in &store.tokens {
                    // Show only first 8 chars of token for security.
                    println!("  {} ({}...)", t.name, &t.token[..8]);
                }
            }
        }
        "revoke" => {
            let name = match args.get(1) {
                Some(n) => n.as_str(),
                None => {
                    eprintln!("usage: osr-gm-server token revoke <name>");
                    std::process::exit(1);
                }
            };
            let mut store = TokenStore::load(&token_path).unwrap_or_else(|e| {
                eprintln!("error loading tokens: {}", e);
                std::process::exit(1);
            });
            if store.revoke(name) {
                store.save(&token_path).unwrap_or_else(|e| {
                    eprintln!("error saving tokens: {}", e);
                    std::process::exit(1);
                });
                eprintln!("Token '{}' revoked.", name);
            } else {
                eprintln!("No token named '{}' found.", name);
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("unknown token subcommand: {}", other);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn extract_bearer_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer abc123"));
        assert_eq!(extract_bearer_token(&headers), Some("abc123"));
    }

    #[test]
    fn extract_bearer_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn extract_bearer_no_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("abc123"));
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn extract_bearer_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer "));
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
