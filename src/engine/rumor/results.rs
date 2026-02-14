use serde::Serialize;

/// Data for a single rolled rumor.
#[derive(Debug, Clone, Serialize)]
pub struct RumorData {
    pub text: String,
    pub true_rumor: bool,
    pub tags: Vec<String>,
    pub table: String,
}

/// Result of rolling a rumor from a table.
#[derive(Debug, Clone)]
pub struct RollRumorResult {
    pub message: String,
    pub cli_output: String,
    pub data: RumorData,
}

/// Data for a single rumor table entry in a list.
#[derive(Debug, Clone, Serialize)]
pub struct RumorTableInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entry_count: usize,
}

/// Result of listing all rumor tables.
#[derive(Debug, Clone, Serialize)]
pub struct ListRumorTablesResult {
    pub tables: Vec<RumorTableInfo>,
}

/// Data for a full rumor table lookup.
#[derive(Debug, Clone, Serialize)]
pub struct RumorTableData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entries: Vec<RumorEntryData>,
}

/// Data for a single rumor entry in a table lookup.
#[derive(Debug, Clone, Serialize)]
pub struct RumorEntryData {
    pub index: usize,
    pub text: String,
    pub true_rumor: bool,
    pub tags: Vec<String>,
}

/// Result of looking up a rumor table.
#[derive(Debug, Clone)]
pub struct LookupRumorTableResult {
    pub message: String,
    pub cli_output: String,
    pub data: RumorTableData,
}
