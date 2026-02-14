use std::str::FromStr;
use serde::{Deserialize, Serialize};

use crate::log_entry::LogEntry;

/// Capitalize the first letter of a string.
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

/// Light source types with their duration in dungeon turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightSourceKind {
    #[serde(alias = "torch")]
    Torch,   // 6 turns
    #[serde(alias = "lantern")]
    Lantern, // 24 turns
}

impl FromStr for LightSourceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "torch" => Ok(LightSourceKind::Torch),
            "lantern" => Ok(LightSourceKind::Lantern),
            _ => Err(format!("invalid light source '{}': must be torch or lantern", s)),
        }
    }
}

impl LightSourceKind {
    /// Maximum duration in dungeon turns.
    pub fn max_turns(self) -> u32 {
        match self {
            LightSourceKind::Torch => 6,
            LightSourceKind::Lantern => 24,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LightSourceKind::Torch => "torch",
            LightSourceKind::Lantern => "lantern",
        }
    }
}

/// An active light source being tracked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveLight {
    pub kind: LightSourceKind,
    pub remaining_turns: u32,
    pub carrier: String,
}

impl ActiveLight {
    pub fn new(kind: LightSourceKind, carrier: &str) -> Self {
        ActiveLight {
            kind,
            remaining_turns: kind.max_turns(),
            carrier: carrier.to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_turns == 0
    }
}

/// Tracks dungeon time: turns, rounds within a turn, and days.
///
/// OSE time structure:
/// - 1 turn = 10 minutes (dungeon exploration unit)
/// - 1 round = 10 seconds (combat unit, 6 rounds per turn for movement)
/// - 1 day = 24 hours = 144 turns
///
/// Rest requirement: 1 turn rest per 5 turns of activity (rest on the 6th).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTracker {
    /// Total turns elapsed since start of exploration.
    pub total_turns: u32,
    /// Turns since last rest (for forced rest tracking).
    pub turns_since_rest: u32,
    /// Active light sources.
    pub lights: Vec<ActiveLight>,
    /// Log of time-related events.
    pub log: Vec<LogEntry>,
    /// Monotonic sequence counter for log entry ordering.
    #[serde(default)]
    pub log_seq: u64,
}

impl TimeTracker {
    /// Maximum log entries retained before oldest are dropped.
    const MAX_LOG_ENTRIES: usize = 1000;

    pub fn new() -> Self {
        TimeTracker {
            total_turns: 0,
            turns_since_rest: 0,
            lights: Vec::new(),
            log: Vec::new(),
            log_seq: 0,
        }
    }

    /// Append a message to the log, capping at MAX_LOG_ENTRIES.
    fn log(&mut self, msg: String) {
        if self.log.len() >= Self::MAX_LOG_ENTRIES {
            let drain = self.log.len() - Self::MAX_LOG_ENTRIES / 2;
            self.log.drain(..drain);
        }
        self.log_seq += 1;
        self.log.push(LogEntry::new(self.log_seq, msg));
    }

    /// Advance time by one dungeon turn (10 minutes).
    /// Decrements all light sources. Returns expired light source messages.
    pub fn advance_turn(&mut self) -> Vec<String> {
        self.total_turns += 1;
        self.turns_since_rest += 1;

        let mut messages = Vec::new();

        // Tick all light sources
        for light in &mut self.lights {
            if light.remaining_turns > 0 {
                light.remaining_turns -= 1;
                if light.remaining_turns == 1 {
                    let msg = format!(
                        "{}'s {} sputters — 1 turn remaining!",
                        light.carrier,
                        light.kind.name()
                    );
                    messages.push(msg);
                } else if light.remaining_turns == 0 {
                    let msg = format!(
                        "{}'s {} goes out!",
                        light.carrier,
                        light.kind.name()
                    );
                    messages.push(msg);
                }
            }
        }

        // Remove expired lights
        self.lights.retain(|l| !l.is_expired());

        for msg in &messages {
            self.log(msg.clone());
        }

        messages
    }

    /// Add a new light source.
    pub fn light(&mut self, kind: LightSourceKind, carrier: &str) {
        let light = ActiveLight::new(kind, carrier);
        self.log(format!(
            "{} lights a {} ({} turns).",
            carrier,
            kind.name(),
            kind.max_turns()
        ));
        self.lights.push(light);
    }

    /// Check if the party has any active light.
    /// Since `advance_turn` already removes expired lights, this only
    /// needs to check whether any lights remain.
    pub fn has_light(&self) -> bool {
        !self.lights.is_empty()
    }

    /// Whether rest is required (after 5 turns of activity).
    pub fn needs_rest(&self) -> bool {
        self.turns_since_rest >= 5
    }

    /// Per OSE, -1 penalty to attack and damage when rest is overdue.
    pub fn rest_penalty(&self) -> i32 {
        if self.needs_rest() { -1 } else { 0 }
    }

    /// Record a rest turn. Resets activity counter.
    pub fn rest(&mut self) {
        self.log(format!("Turn {}: Party rests for one turn.", self.total_turns));
        // Resting still consumes a turn for light sources
        self.advance_turn();
        // Reset activity counter after the turn advance
        self.turns_since_rest = 0;
    }

    /// Current day (1-indexed).
    pub fn current_day(&self) -> u32 {
        self.total_turns / 144 + 1
    }

    /// One-line light summary for exploration output.
    /// Returns None when no lights are active (caller handles darkness).
    pub fn light_summary(&self) -> Option<String> {
        if self.lights.is_empty() {
            return None;
        }
        if self.lights.len() == 1 {
            let l = &self.lights[0];
            let plural = if l.remaining_turns == 1 { "" } else { "s" };
            return Some(format!(
                "{}: {} turn{} remaining",
                capitalize(l.kind.name()),
                l.remaining_turns,
                plural,
            ));
        }
        // Multiple light sources
        let parts: Vec<String> = self.lights
            .iter()
            .map(|l| format!("{}'s {} ({} turns)", l.carrier, l.kind.name(), l.remaining_turns))
            .collect();
        Some(format!("Light: {}", parts.join(", ")))
    }

    /// Format a status display.
    pub fn status(&self) -> String {
        let mut out = format!(
            "Turn: {}  Day: {}  Activity: {}/5",
            self.total_turns, self.current_day(), self.turns_since_rest
        );
        if self.needs_rest() {
            out.push_str("  [REST REQUIRED]");
        }
        if !self.lights.is_empty() {
            out.push_str("\nLight sources:");
            for l in &self.lights {
                out.push_str(&format!(
                    "\n  {} ({}, {} turns left)",
                    l.carrier,
                    l.kind.name(),
                    l.remaining_turns
                ));
            }
        } else {
            out.push_str("\n  No light sources active — DARKNESS!");
        }
        out
    }
}

impl Default for TimeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_duration() {
        assert_eq!(LightSourceKind::Torch.max_turns(), 6);
    }

    #[test]
    fn lantern_duration() {
        assert_eq!(LightSourceKind::Lantern.max_turns(), 24);
    }

    #[test]
    fn advance_turn_decrements_light() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        assert_eq!(tracker.lights[0].remaining_turns, 6);
        tracker.advance_turn();
        assert_eq!(tracker.lights[0].remaining_turns, 5);
        assert_eq!(tracker.total_turns, 1);
    }

    #[test]
    fn torch_expires_after_6_turns() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        for _ in 0..6 {
            tracker.advance_turn();
        }
        assert!(!tracker.has_light());
        assert!(tracker.lights.is_empty());
    }

    #[test]
    fn lantern_lasts_24_turns() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Lantern, "Brin");
        for _ in 0..23 {
            tracker.advance_turn();
        }
        assert!(tracker.has_light());
        tracker.advance_turn();
        assert!(!tracker.has_light());
    }

    #[test]
    fn sputtering_warning_at_1_turn() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        // Advance 5 turns — remaining will be 1
        for _ in 0..4 {
            tracker.advance_turn();
        }
        let msgs = tracker.advance_turn();
        assert!(msgs.iter().any(|m| m.contains("sputters")));
    }

    #[test]
    fn rest_requirement_after_5_turns() {
        let mut tracker = TimeTracker::new();
        for _ in 0..4 {
            tracker.advance_turn();
        }
        assert!(!tracker.needs_rest());
        tracker.advance_turn();
        assert!(tracker.needs_rest());
    }

    #[test]
    fn rest_resets_counter() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Lantern, "Brin");
        for _ in 0..5 {
            tracker.advance_turn();
        }
        assert!(tracker.needs_rest());
        let turns_before = tracker.total_turns;
        tracker.rest();
        assert!(!tracker.needs_rest());
        assert_eq!(tracker.turns_since_rest, 0);
        // Rest itself advances a turn (for light tracking)
        assert_eq!(tracker.total_turns, turns_before + 1);
    }

    #[test]
    fn rest_penalty_applied_when_overdue() {
        let mut tracker = TimeTracker::new();
        assert_eq!(tracker.rest_penalty(), 0);
        for _ in 0..5 {
            tracker.advance_turn();
        }
        assert_eq!(tracker.rest_penalty(), -1, "should have -1 penalty when rest is overdue");
        tracker.light(LightSourceKind::Lantern, "Test");
        tracker.rest();
        assert_eq!(tracker.rest_penalty(), 0, "penalty should clear after rest");
    }

    #[test]
    fn no_light_status() {
        let tracker = TimeTracker::new();
        assert!(!tracker.has_light());
        assert!(tracker.status().contains("DARKNESS"));
    }

    #[test]
    fn day_calculation() {
        let mut tracker = TimeTracker::new();
        assert_eq!(tracker.current_day(), 1);
        for _ in 0..144 {
            tracker.advance_turn();
        }
        assert_eq!(tracker.current_day(), 2);
    }

    #[test]
    fn light_summary_none_when_no_lights() {
        let tracker = TimeTracker::new();
        assert!(tracker.light_summary().is_none());
    }

    #[test]
    fn light_summary_single_torch() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        let summary = tracker.light_summary().unwrap();
        assert_eq!(summary, "Torch: 6 turns remaining");
    }

    #[test]
    fn light_summary_single_torch_after_advance() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        tracker.advance_turn();
        let summary = tracker.light_summary().unwrap();
        assert_eq!(summary, "Torch: 5 turns remaining");
    }

    #[test]
    fn light_summary_singular_turn() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        // 6-turn torch: 5 advances brings remaining to 1
        for _ in 0..5 {
            tracker.advance_turn();
        }
        let summary = tracker.light_summary().unwrap();
        assert_eq!(summary, "Torch: 1 turn remaining");
    }

    #[test]
    fn light_summary_single_lantern() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Lantern, "Brin");
        let summary = tracker.light_summary().unwrap();
        assert_eq!(summary, "Lantern: 24 turns remaining");
    }

    #[test]
    fn light_summary_multiple_sources() {
        let mut tracker = TimeTracker::new();
        tracker.light(LightSourceKind::Torch, "Arden");
        tracker.light(LightSourceKind::Lantern, "Brin");
        let summary = tracker.light_summary().unwrap();
        assert!(summary.starts_with("Light: "));
        assert!(summary.contains("Arden's torch (6 turns)"));
        assert!(summary.contains("Brin's lantern (24 turns)"));
    }
}
