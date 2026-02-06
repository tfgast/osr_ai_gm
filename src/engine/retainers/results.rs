use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HireRetainerMode {
    RecruitToParty,
    AssessOnly,
}

#[derive(Debug, Clone, Serialize)]
pub struct HireRetainerResult {
    pub message: String,
    pub employer: String,
    pub retainer: String,
    pub class: String,
    pub level: u32,
    pub reaction: String,
    pub hired: bool,
    pub loyalty: u32,
    pub wage_gp: u32,
    pub max_retainers: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    pub bonus_loyalty: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetainerSummary {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub loyalty: u32,
    pub wage_gp: u32,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListRetainersResult {
    pub retainers: Vec<RetainerSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DismissRetainerResult {
    pub name: String,
    pub class: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetainerMoraleCheck {
    pub name: String,
    pub loyalty: u32,
    pub result: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetainerMoraleResult {
    pub message: String,
    pub checks: Vec<RetainerMoraleCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoyaltyCheckResult {
    pub message: String,
    pub retainer: String,
    pub loyalty: u32,
    pub result: String,
}
