use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Native,
    Dsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MechanicGroup {
    Combat,
    Morale,
    TurnUndead,
    Saves,
    Ability,
    Thief,
}

impl MechanicGroup {
    fn env_suffix(&self) -> &'static str {
        match self {
            MechanicGroup::Combat => "COMBAT",
            MechanicGroup::Morale => "MORALE",
            MechanicGroup::TurnUndead => "TURN_UNDEAD",
            MechanicGroup::Saves => "SAVES",
            MechanicGroup::Ability => "ABILITY",
            MechanicGroup::Thief => "THIEF",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub combat: Backend,
    pub morale: Backend,
    pub turn_undead: Backend,
    pub saves: Backend,
    pub ability: Backend,
    pub thief: Backend,
}

impl BackendConfig {
    fn from_env() -> Self {
        let global = parse_backend_env("OSR_BACKEND");

        BackendConfig {
            combat: parse_backend_env("OSR_BACKEND_COMBAT").or(global).unwrap_or(Backend::Native),
            morale: parse_backend_env("OSR_BACKEND_MORALE").or(global).unwrap_or(Backend::Native),
            turn_undead: parse_backend_env("OSR_BACKEND_TURN_UNDEAD").or(global).unwrap_or(Backend::Native),
            saves: parse_backend_env("OSR_BACKEND_SAVES").or(global).unwrap_or(Backend::Native),
            ability: parse_backend_env("OSR_BACKEND_ABILITY").or(global).unwrap_or(Backend::Native),
            thief: parse_backend_env("OSR_BACKEND_THIEF").or(global).unwrap_or(Backend::Native),
        }
    }

    pub fn get(&self, group: MechanicGroup) -> Backend {
        match group {
            MechanicGroup::Combat => self.combat,
            MechanicGroup::Morale => self.morale,
            MechanicGroup::TurnUndead => self.turn_undead,
            MechanicGroup::Saves => self.saves,
            MechanicGroup::Ability => self.ability,
            MechanicGroup::Thief => self.thief,
        }
    }
}

fn parse_backend_env(var: &str) -> Option<Backend> {
    match std::env::var(var).ok()?.to_lowercase().as_str() {
        "dsl" => Some(Backend::Dsl),
        "native" => Some(Backend::Native),
        _ => None,
    }
}

static CONFIG: OnceLock<BackendConfig> = OnceLock::new();

/// Get the global backend configuration (lazily initialized from environment).
pub fn config() -> &'static BackendConfig {
    CONFIG.get_or_init(BackendConfig::from_env)
}

/// Check if a mechanic group is configured to use the DSL backend.
pub fn is_dsl(group: MechanicGroup) -> bool {
    config().get(group) == Backend::Dsl
}

// ── DSL Runtime ──────────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
pub use dsl_runtime::*;

#[cfg(feature = "dsl-backend")]
mod dsl_runtime {
    use std::sync::OnceLock;

    use ttrpg_ast::ast::Program;
    use ttrpg_ast::{FileId, Severity};
    use ttrpg_checker::CheckResult;
    use ttrpg_interp::value::Value;
    use ttrpg_interp::{Interpreter, RuntimeError};
    use ttrpg_interp::state::StateProvider;
    use ttrpg_interp::effect::EffectHandler;

    /// Owns the parsed AST + type environment. Interpreter created per-call (lightweight).
    pub struct DslRuntime {
        program: Program,
        check_result: CheckResult,
    }

    impl DslRuntime {
        /// Load all .ttrpg files from the given directory, parse, lower, and type-check.
        pub fn load(dir: &str) -> Result<Self, String> {
            let mut sources = Vec::new();

            let entries = std::fs::read_dir(dir)
                .map_err(|e| format!("Failed to read DSL directory '{}': {}", dir, e))?;

            let mut paths: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map_or(false, |ext| ext == "ttrpg")
                })
                .collect();

            // Sort for deterministic load order
            paths.sort_by_key(|e| e.path());

            for entry in &paths {
                let path = entry.path();
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
                sources.push(content);
            }

            if sources.is_empty() {
                return Err(format!("No .ttrpg files found in '{}'", dir));
            }

            let combined = sources.join("\n");

            // Parse
            let (program, mut diags) = ttrpg_parser::parse(&combined, FileId(0));

            // Lower moves (desugar)
            let program = ttrpg_parser::lower_moves(program, &mut diags);

            // Check for parse errors
            let errors: Vec<_> = diags
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .collect();
            if !errors.is_empty() {
                return Err(format!(
                    "DSL parse errors: {}",
                    errors
                        .iter()
                        .map(|d| d.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }

            // Type-check
            let check_result = ttrpg_checker::check(&program);

            let check_errors: Vec<_> = check_result
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .collect();
            if !check_errors.is_empty() {
                return Err(format!(
                    "DSL type errors: {}",
                    check_errors
                        .iter()
                        .map(|d| d.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }

            Ok(DslRuntime {
                program,
                check_result,
            })
        }

        /// Evaluate a derive function by name with the given arguments.
        pub fn evaluate_derive(
            &self,
            state: &dyn StateProvider,
            handler: &mut dyn EffectHandler,
            name: &str,
            args: Vec<Value>,
        ) -> Result<Value, RuntimeError> {
            let interp = Interpreter::new(&self.program, &self.check_result.env)?;
            interp.evaluate_derive(state, handler, name, args)
        }

        /// Evaluate a mechanic function by name with the given arguments.
        pub fn evaluate_mechanic(
            &self,
            state: &dyn StateProvider,
            handler: &mut dyn EffectHandler,
            name: &str,
            args: Vec<Value>,
        ) -> Result<Value, RuntimeError> {
            let interp = Interpreter::new(&self.program, &self.check_result.env)?;
            interp.evaluate_mechanic(state, handler, name, args)
        }

        /// Create a fresh interpreter (for advanced use by bridge engine).
        pub fn interpreter(&self) -> Result<Interpreter<'_>, RuntimeError> {
            Interpreter::new(&self.program, &self.check_result.env)
        }
    }

    static DSL: OnceLock<Option<DslRuntime>> = OnceLock::new();

    /// Get the global DSL runtime (lazily loaded from data/dsl/).
    /// Returns None if loading fails (logged to stderr).
    pub fn dsl() -> Option<&'static DslRuntime> {
        DSL.get_or_init(|| {
            match DslRuntime::load("data/dsl") {
                Ok(runtime) => Some(runtime),
                Err(e) => {
                    eprintln!("DSL backend failed to load: {}", e);
                    None
                }
            }
        })
        .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_all_native() {
        // Don't use the OnceLock; test from_env directly with clean env
        let config = BackendConfig::from_env();
        assert_eq!(config.combat, Backend::Native);
        assert_eq!(config.morale, Backend::Native);
        assert_eq!(config.turn_undead, Backend::Native);
        assert_eq!(config.saves, Backend::Native);
        assert_eq!(config.ability, Backend::Native);
        assert_eq!(config.thief, Backend::Native);
    }

    #[test]
    fn mechanic_group_env_suffix() {
        assert_eq!(MechanicGroup::Combat.env_suffix(), "COMBAT");
        assert_eq!(MechanicGroup::TurnUndead.env_suffix(), "TURN_UNDEAD");
    }

    #[test]
    fn backend_config_get() {
        let config = BackendConfig {
            combat: Backend::Dsl,
            morale: Backend::Native,
            turn_undead: Backend::Native,
            saves: Backend::Dsl,
            ability: Backend::Native,
            thief: Backend::Native,
        };
        assert_eq!(config.get(MechanicGroup::Combat), Backend::Dsl);
        assert_eq!(config.get(MechanicGroup::Morale), Backend::Native);
        assert_eq!(config.get(MechanicGroup::Saves), Backend::Dsl);
    }
}
