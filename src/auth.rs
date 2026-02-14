use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A stored API token with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    /// The token value (64-char hex string).
    pub token: String,
    /// Human-readable label for this token.
    pub name: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// Persistent store for API tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStore {
    pub tokens: Vec<ApiToken>,
}

impl TokenStore {
    /// Default path for the token store file.
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".osr_data").join("api_tokens.json")
    }

    /// Load tokens from a file, returning an empty store if the file doesn't exist.
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => {
                let store: TokenStore = serde_json::from_str(&contents)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(store)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(TokenStore { tokens: vec![] }),
            Err(e) => Err(e),
        }
    }

    /// Save tokens to a file (atomic write).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(io::Error::other)?;

        // Atomic write: write to temp file then rename.
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Check if a token value exists in the store (constant-time comparison).
    pub fn validate(&self, candidate: &str) -> bool {
        let candidate_bytes = candidate.as_bytes();
        self.tokens.iter().any(|t| constant_time_eq(t.token.as_bytes(), candidate_bytes))
    }

    /// Add a new token and return it.
    pub fn create_token(&mut self, name: &str) -> ApiToken {
        let token = generate_token();
        let api_token = ApiToken {
            token,
            name: name.to_string(),
            created_at: now_iso8601(),
        };
        self.tokens.push(api_token.clone());
        api_token
    }

    /// Revoke a token by name. Returns true if found and removed.
    pub fn revoke(&mut self, name: &str) -> bool {
        let before = self.tokens.len();
        self.tokens.retain(|t| t.name != name);
        self.tokens.len() < before
    }
}

/// Generate a cryptographically random 32-byte (64-char hex) token.
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn now_iso8601() -> String {
    // Simple timestamp without external deps - uses seconds since epoch.
    // In production you'd use chrono, but we keep deps minimal.
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}Z", d.as_secs()),
        Err(_) => "0Z".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_token_unique() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
    }

    #[test]
    fn constant_time_eq_same() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn token_store_create_and_validate() {
        let mut store = TokenStore { tokens: vec![] };
        let token = store.create_token("test");
        assert!(store.validate(&token.token));
        assert!(!store.validate("not-a-real-token"));
    }

    #[test]
    fn token_store_revoke() {
        let mut store = TokenStore { tokens: vec![] };
        let token = store.create_token("test");
        assert!(store.validate(&token.token));
        assert!(store.revoke("test"));
        assert!(!store.validate(&token.token));
    }

    #[test]
    fn token_store_revoke_nonexistent() {
        let mut store = TokenStore { tokens: vec![] };
        assert!(!store.revoke("nonexistent"));
    }

    #[test]
    fn token_store_roundtrip() {
        let dir = std::env::temp_dir().join("osr_auth_test");
        let path = dir.join("tokens.json");
        let _ = fs::remove_dir_all(&dir);

        let mut store = TokenStore { tokens: vec![] };
        store.create_token("roundtrip-test");
        store.save(&path).unwrap();

        let loaded = TokenStore::load(&path).unwrap();
        assert_eq!(loaded.tokens.len(), 1);
        assert_eq!(loaded.tokens[0].name, "roundtrip-test");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn token_store_load_missing_file() {
        let store = TokenStore::load(Path::new("/tmp/nonexistent_osr_tokens.json")).unwrap();
        assert!(store.tokens.is_empty());
    }
}
