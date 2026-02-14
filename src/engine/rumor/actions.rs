use crate::engine::result::EngineError;
use crate::rules::rumor;
use rand::Rng;

use super::results::{
    ListRumorTablesResult, LookupRumorTableResult, RollRumorResult, RumorData,
    RumorEntryData, RumorTableData, RumorTableInfo,
};

/// Roll a random rumor from a named table.
pub fn action_roll_rumor(table_name: &str) -> Result<RollRumorResult, EngineError> {
    let table = rumor::find_rumor_table(table_name).ok_or_else(|| {
        let available: Vec<&str> = rumor::all_rumor_tables().iter().map(|t| t.name.as_str()).collect();
        EngineError::InvalidInput(format!(
            "unknown rumor table '{}'. Available: {}",
            table_name,
            available.join(", ")
        ))
    })?;

    if table.entries.is_empty() {
        return Err(EngineError::Internal(format!(
            "rumor table '{}' has no entries",
            table_name
        )));
    }

    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..table.entries.len());
    let entry = &table.entries[idx];

    let truth_marker = if entry.true_rumor { "TRUE" } else { "FALSE" };
    let mut cli_output = String::new();
    cli_output.push_str(&format!("RUMOR ({})\n", table.name));
    cli_output.push_str("─────────────────────────────────\n");
    cli_output.push_str(&format!("\"{}\"", entry.text));
    cli_output.push('\n');
    cli_output.push_str(&format!("[GM: {} rumor]\n", truth_marker));
    if !entry.tags.is_empty() {
        cli_output.push_str(&format!("Tags: {}\n", entry.tags.join(", ")));
    }

    let message = format!(
        "rumor from {}: \"{}\" [{}]",
        table.name, entry.text, truth_marker
    );

    Ok(RollRumorResult {
        message,
        cli_output,
        data: RumorData {
            text: entry.text.clone(),
            true_rumor: entry.true_rumor,
            tags: entry.tags.clone(),
            table: table.name.clone(),
        },
    })
}

/// List all available rumor tables.
pub fn action_list_rumor_tables() -> Result<ListRumorTablesResult, EngineError> {
    let tables: Vec<RumorTableInfo> = rumor::all_rumor_tables()
        .iter()
        .map(|t| RumorTableInfo {
            name: t.name.clone(),
            description: t.description.clone(),
            entry_count: t.entries.len(),
        })
        .collect();

    Ok(ListRumorTablesResult { tables })
}

/// Look up a specific rumor table and show all its entries.
pub fn action_lookup_rumor_table(table_name: &str) -> Result<LookupRumorTableResult, EngineError> {
    let table = rumor::find_rumor_table(table_name).ok_or_else(|| {
        let available: Vec<&str> = rumor::all_rumor_tables().iter().map(|t| t.name.as_str()).collect();
        EngineError::InvalidInput(format!(
            "unknown rumor table '{}'. Available: {}",
            table_name,
            available.join(", ")
        ))
    })?;

    let entries: Vec<RumorEntryData> = table
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| RumorEntryData {
            index: i + 1,
            text: e.text.clone(),
            true_rumor: e.true_rumor,
            tags: e.tags.clone(),
        })
        .collect();

    let mut cli_output = String::new();
    cli_output.push_str(&format!("RUMOR TABLE: {}\n", table.name.to_uppercase()));
    if let Some(desc) = &table.description {
        cli_output.push_str(&format!("{}\n", desc));
    }
    cli_output.push_str("─────────────────────────────────\n");
    for entry in &entries {
        let truth = if entry.true_rumor { "T" } else { "F" };
        cli_output.push_str(&format!(
            "  {:>2}. [{}] {}\n",
            entry.index, truth, entry.text
        ));
    }
    cli_output.push_str("─────────────────────────────────\n");
    cli_output.push_str(&format!("{} rumors total\n", entries.len()));

    let message = format!(
        "rumor table '{}': {} entries.",
        table.name,
        entries.len()
    );

    Ok(LookupRumorTableResult {
        message,
        cli_output,
        data: RumorTableData {
            name: table.name.clone(),
            description: table.description.clone(),
            entries,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_rumor_tavern() {
        let result = action_roll_rumor("tavern").unwrap();
        assert!(!result.data.text.is_empty());
        assert_eq!(result.data.table, "tavern");
        assert!(result.cli_output.contains("RUMOR"));
    }

    #[test]
    fn roll_rumor_case_insensitive() {
        assert!(action_roll_rumor("TAVERN").is_ok());
        assert!(action_roll_rumor("Tavern").is_ok());
    }

    #[test]
    fn roll_rumor_unknown_table() {
        let result = action_roll_rumor("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            EngineError::InvalidInput(msg) => {
                assert!(msg.contains("unknown rumor table"));
                assert!(msg.contains("Available:"));
            }
            _ => panic!("expected InvalidInput"),
        }
    }

    #[test]
    fn list_rumor_tables() {
        let result = action_list_rumor_tables().unwrap();
        assert!(result.tables.len() >= 3);
        assert!(result.tables.iter().any(|t| t.name == "tavern"));
        assert!(result.tables.iter().any(|t| t.name == "market"));
        assert!(result.tables.iter().any(|t| t.name == "docks"));
        for t in &result.tables {
            assert!(t.entry_count > 0);
        }
    }

    #[test]
    fn lookup_rumor_table_tavern() {
        let result = action_lookup_rumor_table("tavern").unwrap();
        assert_eq!(result.data.name, "tavern");
        assert!(!result.data.entries.is_empty());
        assert!(result.cli_output.contains("RUMOR TABLE: TAVERN"));
        for entry in &result.data.entries {
            assert!(!entry.text.is_empty());
            assert!(entry.index > 0);
        }
    }

    #[test]
    fn lookup_rumor_table_unknown() {
        let result = action_lookup_rumor_table("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn roll_rumor_has_truth_marker() {
        // Roll many times to test both true and false
        for _ in 0..50 {
            let result = action_roll_rumor("tavern").unwrap();
            assert!(
                result.cli_output.contains("[GM: TRUE rumor]")
                    || result.cli_output.contains("[GM: FALSE rumor]")
            );
        }
    }
}
