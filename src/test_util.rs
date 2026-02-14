use std::sync::{Mutex, MutexGuard};

/// Mutex for tests that read or write the `OSR_DATA_DIR` env var.
///
/// Rust runs unit tests in parallel within the same process. Since
/// `env::set_var` mutates process-global state, tests that modify
/// `OSR_DATA_DIR` can race with tests that read it (via `data_dir()`).
/// All such tests must hold this lock.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the environment variable lock. Hold the returned guard for the
/// duration of any test that reads or writes `OSR_DATA_DIR`.
pub fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
