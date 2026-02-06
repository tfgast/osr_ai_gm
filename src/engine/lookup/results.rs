use serde::Serialize;
use std::collections::BTreeMap;

use crate::rules::magic_item::MagicItemDef;
use crate::rules::spell_data::SpellList;

#[derive(Debug, Clone, Serialize)]
pub struct MagicItemPropertyData {
    pub key: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagicItemData {
    pub name: String,
    pub category: String,
    pub cursed: bool,
    pub description: Option<String>,
    pub properties: Vec<MagicItemPropertyData>,
}

impl MagicItemData {
    pub fn from_def(item: &MagicItemDef) -> Self {
        Self {
            name: item.name.clone(),
            category: item.category.name().to_string(),
            cursed: item.cursed,
            description: item.description.clone(),
            properties: item
                .properties
                .iter()
                .map(|p| MagicItemPropertyData {
                    key: p.key.clone(),
                    value: p.value.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LookupItemMatch {
    Single(MagicItemData),
    Multiple(Vec<String>),
    TooMany(usize),
}

#[derive(Debug, Clone)]
pub struct LookupItemResult {
    pub query: String,
    pub item_match: LookupItemMatch,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LookupItemPayload {
    Item(MagicItemData),
    Matches { matches: Vec<String>, count: usize },
    Count { count: usize },
}

impl LookupItemResult {
    pub fn cli_output(&self) -> String {
        match &self.item_match {
            LookupItemMatch::Single(item) => format_magic_item_cli(item),
            LookupItemMatch::Multiple(names) => {
                let mut out = format!("Multiple items match '{}'. Did you mean:\n", self.query);
                for name in names {
                    out.push_str(&format!("  - {name}\n"));
                }
                out
            }
            LookupItemMatch::TooMany(count) => {
                format!(
                    "Found {} items matching '{}'. Please be more specific.",
                    count, self.query
                )
            }
        }
    }

    pub fn api_message(&self) -> String {
        match &self.item_match {
            LookupItemMatch::Single(item) => {
                let mut msg = format!("{} ({})", item.name, item.category);
                if item.cursed {
                    msg.push_str(" [CURSED]");
                }
                if let Some(desc) = &item.description {
                    msg.push_str(&format!(": {desc}"));
                }
                msg
            }
            LookupItemMatch::Multiple(names) => format!(
                "multiple items match '{}'. Did you mean: {}?",
                self.query,
                names.join(", ")
            ),
            LookupItemMatch::TooMany(count) => {
                format!(
                    "found {} items matching '{}'. Please be more specific.",
                    count, self.query
                )
            }
        }
    }

    pub fn api_payload(&self) -> LookupItemPayload {
        match &self.item_match {
            LookupItemMatch::Single(item) => LookupItemPayload::Item(item.clone()),
            LookupItemMatch::Multiple(names) => LookupItemPayload::Matches {
                matches: names.clone(),
                count: names.len(),
            },
            LookupItemMatch::TooMany(count) => LookupItemPayload::Count { count: *count },
        }
    }
}

fn format_magic_item_cli(item: &MagicItemData) -> String {
    let mut out = format!("=== {} ===\nCategory: {}", item.name, item.category);

    if item.cursed {
        out.push_str(" [CURSED]");
    }
    out.push('\n');

    if let Some(desc) = &item.description {
        out.push_str(&format!("\n{desc}\n"));
    }

    if !item.properties.is_empty() {
        out.push_str("\nProperties:\n");
        for prop in &item.properties {
            if let Some(key) = &prop.key {
                out.push_str(&format!("  {key}: {}\n", prop.value));
            } else {
                out.push_str(&format!("  - {}\n", prop.value));
            }
        }
    }

    out
}

#[derive(Debug, Clone)]
pub struct SearchItemEntry {
    pub name: String,
    pub cursed: bool,
}

#[derive(Debug, Clone)]
pub struct SearchItemsResult {
    pub query: String,
    pub by_category: BTreeMap<String, Vec<SearchItemEntry>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SearchItemsPayload {
    Empty {
        matches: Vec<String>,
        count: usize,
    },
    Results {
        count: usize,
        by_category: BTreeMap<String, Vec<String>>,
    },
}

impl SearchItemsResult {
    pub fn count(&self) -> usize {
        self.by_category.values().map(Vec::len).sum()
    }

    pub fn cli_output(&self) -> String {
        let count = self.count();
        if count == 0 {
            return format!("No magic items found matching '{}'.", self.query);
        }

        let mut out = format!("Found {} item(s) matching '{}':\n\n", count, self.query);
        for (category, items) in &self.by_category {
            out.push_str(&format!("{category}:\n"));
            for item in items {
                let cursed = if item.cursed { " [CURSED]" } else { "" };
                out.push_str(&format!("  - {}{}\n", item.name, cursed));
            }
            out.push('\n');
        }

        out
    }

    pub fn api_message(&self) -> String {
        let count = self.count();
        if count == 0 {
            return format!("no magic items found matching '{}'.", self.query);
        }
        format!("found {} item(s) matching '{}'.", count, self.query)
    }

    pub fn api_payload(&self) -> SearchItemsPayload {
        let count = self.count();
        if count == 0 {
            return SearchItemsPayload::Empty {
                matches: Vec::new(),
                count: 0,
            };
        }

        let by_category = self
            .by_category
            .iter()
            .map(|(category, items)| {
                let names = items
                    .iter()
                    .map(|item| item.name.clone())
                    .collect::<Vec<_>>();
                (category.clone(), names)
            })
            .collect();

        SearchItemsPayload::Results { count, by_category }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TreasureTypeEntryData {
    pub chance: u32,
    pub quantity: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub restriction: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LookupTreasureTypeResult {
    pub letter: String,
    pub category: String,
    pub average_gp: f64,
    pub entries: Vec<TreasureTypeEntryData>,
    pub has_coins: bool,
    pub has_gems: bool,
    pub has_jewellery: bool,
    pub has_magic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LookupTreasureTypePayload {
    pub letter: String,
    pub category: String,
    pub average_gp: f64,
    pub entries: Vec<TreasureTypeEntryData>,
}

impl LookupTreasureTypeResult {
    pub fn cli_output(&self) -> String {
        let mut out = format!(
            "=== Treasure Type {} ===\n\
             Category: {}\n\
             Average Value: {} gp\n\n\
             Contents:\n",
            self.letter, self.category, self.average_gp
        );

        for entry in &self.entries {
            let restriction = entry
                .restriction
                .as_ref()
                .map(|r| format!(" ({r})"))
                .unwrap_or_default();
            let note = entry
                .note
                .as_ref()
                .map(|n| format!(" [{n}]"))
                .unwrap_or_default();

            out.push_str(&format!(
                "  {:3}%: {} {}{}{}\n",
                entry.chance, entry.quantity, entry.item_type, restriction, note
            ));
        }

        out.push_str("\nPossible contents:\n");
        if self.has_coins {
            out.push_str("  - Coins (copper, silver, electrum, gold, platinum)\n");
        }
        if self.has_gems {
            out.push_str("  - Gems (10-1000 gp each, rolled on d20 table)\n");
        }
        if self.has_jewellery {
            out.push_str("  - Jewellery (3d6 x 100 gp each)\n");
        }
        if self.has_magic {
            out.push_str("  - Magic items\n");
        }

        out
    }

    pub fn api_message(&self) -> String {
        let mut msg = format!(
            "treasure type {} ({}), avg {} gp.",
            self.letter, self.category, self.average_gp
        );

        let mut contents = Vec::new();
        if self.has_coins {
            contents.push("coins");
        }
        if self.has_gems {
            contents.push("gems");
        }
        if self.has_jewellery {
            contents.push("jewellery");
        }
        if self.has_magic {
            contents.push("magic items");
        }
        if !contents.is_empty() {
            msg.push_str(&format!(" May contain: {}.", contents.join(", ")));
        }

        msg
    }

    pub fn api_payload(&self) -> LookupTreasureTypePayload {
        LookupTreasureTypePayload {
            letter: self.letter.clone(),
            category: self.category.clone(),
            average_gp: self.average_gp,
            entries: self.entries.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LookupSpellResult {
    pub query: String,
    pub name: String,
    pub list: SpellList,
    pub level: u32,
    pub range: String,
    pub duration: String,
    pub description: String,
    pub reversible: bool,
    pub reversed_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LookupSpellPayload {
    pub name: String,
    pub list: String,
    pub level: u32,
    pub range: String,
    pub duration: String,
    pub description: String,
}

impl LookupSpellResult {
    pub fn cli_output(&self) -> String {
        let mut out = format!(
            "=== {} ===\nList: {} (Level {})\nRange: {}\nDuration: {}\n",
            self.name,
            self.list.name(),
            self.level,
            self.range,
            self.duration,
        );
        if self.reversible {
            if let Some(rev_name) = &self.reversed_name {
                out.push_str(&format!("Reversible: {rev_name}\n"));
            } else {
                out.push_str("Reversible: yes\n");
            }
        }
        out.push_str(&format!("\n{}", self.description));
        out
    }

    pub fn api_message(&self) -> String {
        format!(
            "{} ({}L{}) — Range: {}, Duration: {}: {}",
            self.name,
            self.list.name(),
            self.level,
            self.range,
            self.duration,
            self.description
        )
    }

    pub fn api_payload(&self) -> LookupSpellPayload {
        LookupSpellPayload {
            name: self.name.clone(),
            list: self.list.name().to_string(),
            level: self.level,
            range: self.range.clone(),
            duration: self.duration.clone(),
            description: self.description.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum GpValue {
    Int(i32),
    Float(f64),
}

#[derive(Debug, Clone, Serialize)]
pub struct RolledTreasureItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub quantity: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gp_value: Option<GpValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_gp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollTreasureResult {
    pub letter: String,
    pub category: String,
    pub items: Vec<RolledTreasureItem>,
    pub total_gp: f64,
}

impl RollTreasureResult {
    pub fn api_message(&self) -> String {
        if self.items.is_empty() {
            return format!("rolled on treasure type {}: nothing found.", self.letter);
        }
        format!(
            "rolled on treasure type {}: {} item(s), {:.0} gp total value.",
            self.letter,
            self.items.len(),
            self.total_gp
        )
    }
}
