// Require at least one backend to be compiled in.
#[cfg(not(any(feature = "dsl-backend", feature = "legacy-native")))]
compile_error!(
    "At least one backend feature must be enabled. \
     Enable 'legacy-native' for built-in Rust rule tables, \
     'dsl-backend' for the rule DSL, or both."
);

pub mod auth;
pub mod backend;
#[cfg(feature = "dsl-backend")]
pub mod bridge;
pub mod command;
pub mod dice;
pub mod engine;
pub mod gmapi;
pub mod log_entry;
pub mod manifest;
pub mod model;
pub mod pathutil;
pub mod persist;
pub mod rules;
pub mod session;
pub mod state;
pub mod telemetry;

#[cfg(test)]
pub(crate) mod test_util;
