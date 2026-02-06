use serde::Serialize;

/// Typed success payload for `load_module`.
#[derive(Debug, Clone, Serialize)]
pub struct LoadModuleResult {
    pub message: String,
    pub module_name: String,
    pub level_range: (u32, u32),
    pub room_count: usize,
}
