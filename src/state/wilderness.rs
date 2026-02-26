use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::log_entry::LogEntry;

#[cfg(feature = "dsl-backend")]
use crate::backend::{is_dsl, MechanicGroup};

/// Returns true when the Wilderness mechanic group is using the DSL backend.
#[cfg(feature = "dsl-backend")]
#[inline]
fn use_dsl() -> bool {
    is_dsl(MechanicGroup::Wilderness)
}

/// Terrain types for wilderness hexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Terrain {
    #[serde(alias = "clear")]
    #[default]
    Clear,
    #[serde(alias = "forest")]
    Forest,
    #[serde(alias = "hills")]
    Hills,
    #[serde(alias = "mountains")]
    Mountains,
    #[serde(alias = "desert")]
    Desert,
    #[serde(alias = "swamp")]
    Swamp,
    #[serde(alias = "jungle")]
    Jungle,
    #[serde(alias = "ocean")]
    Ocean,
    #[serde(alias = "river")]
    River,
    #[serde(alias = "barren")]
    Barren,
    #[serde(alias = "city")]
    City,
}

impl fmt::Display for Terrain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Terrain {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "clear" => Ok(Terrain::Clear),
            "forest" => Ok(Terrain::Forest),
            "hills" => Ok(Terrain::Hills),
            "mountains" => Ok(Terrain::Mountains),
            "desert" => Ok(Terrain::Desert),
            "swamp" => Ok(Terrain::Swamp),
            "jungle" => Ok(Terrain::Jungle),
            "ocean" => Ok(Terrain::Ocean),
            "river" => Ok(Terrain::River),
            "barren" => Ok(Terrain::Barren),
            "city" => Ok(Terrain::City),
            _ => Err(format!(
                "invalid terrain '{}': must be clear, forest, hills, mountains, desert, swamp, jungle, ocean, river, barren, or city",
                s
            )),
        }
    }
}

impl Terrain {
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Clear => "Clear",
            Terrain::Forest => "Forest",
            Terrain::Hills => "Hills",
            Terrain::Mountains => "Mountains",
            Terrain::Desert => "Desert",
            Terrain::Swamp => "Swamp",
            Terrain::Jungle => "Jungle",
            Terrain::Ocean => "Ocean",
            Terrain::River => "River",
            Terrain::Barren => "Barren",
            Terrain::City => "City",
        }
    }

    /// Movement cost multiplier (fraction of daily movement used per hex).
    /// Base movement: 1 hex per day at movement rate 120'.
    /// Returns (numerator, denominator) for the cost fraction.
    ///
    /// When `dsl-backend` is enabled and `OSR_BACKEND_WILDERNESS=dsl`, delegates
    /// to the `terrain_move_cost_num` / `terrain_move_cost_den` DSL tables.
    pub fn movement_cost(self) -> (u32, u32) {
        #[cfg(feature = "dsl-backend")]
        if use_dsl() {
            if let Some(v) = dsl_gate::dsl_movement_cost(self) {
                return v;
            }
        }
        match self {
            Terrain::Clear | Terrain::City => (1, 1),
            Terrain::Barren | Terrain::Hills => (3, 2),    // 1.5x cost
            Terrain::Forest | Terrain::River => (3, 2),     // 1.5x cost
            Terrain::Desert | Terrain::Jungle => (2, 1),     // 2x cost
            Terrain::Mountains => (3, 1),                   // 3x cost (~1 hex/day at 120')
            Terrain::Swamp => (2, 1),                       // 2x cost
            Terrain::Ocean => (1, 1),                       // ship travel
        }
    }

    /// Chance of getting lost (X-in-6). 0 means never lost.
    ///
    /// When `dsl-backend` is enabled and `OSR_BACKEND_WILDERNESS=dsl`, delegates
    /// to the `terrain_lost_chance` DSL table.
    pub fn lost_chance(self) -> u32 {
        #[cfg(feature = "dsl-backend")]
        if use_dsl() {
            if let Some(v) = dsl_gate::dsl_lost_chance(self) {
                return v;
            }
        }
        match self {
            Terrain::Clear | Terrain::City | Terrain::Barren
            | Terrain::Hills | Terrain::Mountains => 1,
            Terrain::Forest | Terrain::River | Terrain::Desert
            | Terrain::Ocean | Terrain::Swamp | Terrain::Jungle => 2,
        }
    }

    /// Whether foraging is possible in this terrain.
    ///
    /// When `dsl-backend` is enabled and `OSR_BACKEND_WILDERNESS=dsl`, delegates
    /// to the `terrain_can_forage` DSL derive.
    pub fn can_forage(self) -> bool {
        #[cfg(feature = "dsl-backend")]
        if use_dsl() {
            if let Some(v) = dsl_gate::dsl_can_forage(self) {
                return v;
            }
        }
        !matches!(self, Terrain::Ocean | Terrain::City | Terrain::Barren)
    }

    /// Chance of successful foraging (X-in-6).
    ///
    /// When `dsl-backend` is enabled and `OSR_BACKEND_WILDERNESS=dsl`, delegates
    /// to the `terrain_forage_chance` DSL table.
    pub fn forage_chance(self) -> u32 {
        #[cfg(feature = "dsl-backend")]
        if use_dsl() {
            if let Some(v) = dsl_gate::dsl_forage_chance(self) {
                return v;
            }
        }
        match self {
            Terrain::Forest | Terrain::Jungle | Terrain::River => 2,
            Terrain::Clear | Terrain::Hills => 1,
            Terrain::Swamp => 1,
            Terrain::Desert | Terrain::Mountains => 1,
            _ => 0,
        }
    }
}

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::value::Value;

    use crate::backend::{self, NullState};
    use crate::bridge::handler::BridgeHandler;

    use super::Terrain;

    /// Convert a Rust `Terrain` to a DSL `WildernessTerrainKind` enum value.
    fn terrain_to_value(terrain: Terrain) -> Value {
        let variant = match terrain {
            Terrain::Clear     => "wt_clear",
            Terrain::Forest    => "wt_forest",
            Terrain::Hills     => "wt_hills",
            Terrain::Mountains => "wt_mountains",
            Terrain::Desert    => "wt_desert",
            Terrain::Swamp     => "wt_swamp",
            Terrain::Jungle    => "wt_jungle",
            Terrain::Ocean     => "wt_ocean",
            Terrain::River     => "wt_river",
            Terrain::Barren    => "wt_barren",
            Terrain::City      => "wt_city",
        };
        Value::EnumVariant {
            enum_name: "WildernessTerrainKind".into(),
            variant: Name::from(variant),
            fields: BTreeMap::new(),
        }
    }

    /// Call `terrain_lost_chance` DSL table.
    pub fn dsl_lost_chance(terrain: Terrain) -> Option<u32> {
        let rt = backend::dsl()?;
        let mut handler = BridgeHandler::new();
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "terrain_lost_chance",
                vec![terrain_to_value(terrain)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    /// Call `terrain_forage_chance` DSL table.
    pub fn dsl_forage_chance(terrain: Terrain) -> Option<u32> {
        let rt = backend::dsl()?;
        let mut handler = BridgeHandler::new();
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "terrain_forage_chance",
                vec![terrain_to_value(terrain)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    /// Call `terrain_can_forage` DSL derive.
    pub fn dsl_can_forage(terrain: Terrain) -> Option<bool> {
        let rt = backend::dsl()?;
        let mut handler = BridgeHandler::new();
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "terrain_can_forage",
                vec![terrain_to_value(terrain)],
            )
            .ok()?;
        match result {
            Value::Bool(v) => Some(v),
            _ => None,
        }
    }

    /// Call `terrain_move_cost_num` and `terrain_move_cost_den` DSL tables.
    /// Returns (numerator, denominator) for the movement cost fraction.
    pub fn dsl_movement_cost(terrain: Terrain) -> Option<(u32, u32)> {
        let rt = backend::dsl()?;
        let mut handler = BridgeHandler::new();
        let v = terrain_to_value(terrain);
        let num = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "terrain_move_cost_num",
                vec![v.clone()],
            )
            .ok()?;
        let den = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "terrain_move_cost_den",
                vec![v],
            )
            .ok()?;
        match (num, den) {
            (Value::Int(n), Value::Int(d)) => Some((n as u32, d as u32)),
            _ => None,
        }
    }
}

/// A hex cell on the wilderness map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexCell {
    pub x: i32,
    pub y: i32,
    pub terrain: Terrain,
    pub description: String,
}

impl HexCell {
    pub fn new(x: i32, y: i32, terrain: Terrain) -> Self {
        HexCell {
            x,
            y,
            terrain,
            description: String::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn coord(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

/// Tracks wilderness exploration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildernessState {
    pub hexes: Vec<HexCell>,
    /// Current hex coordinates.
    pub current_x: i32,
    pub current_y: i32,
    /// Hexes that have been visited.
    pub explored: HashSet<(i32, i32)>,
    /// Days spent travelling.
    pub travel_day: u32,
    /// Whether the party is currently lost.
    pub lost: bool,
    /// Log of travel events.
    pub log: Vec<LogEntry>,
    /// Monotonic sequence counter for log entry ordering.
    #[serde(default)]
    pub log_seq: u64,
}

impl WildernessState {
    /// Maximum log entries retained before oldest are dropped.
    const MAX_LOG_ENTRIES: usize = 1000;

    pub fn new() -> Self {
        WildernessState {
            hexes: Vec::new(),
            current_x: 0,
            current_y: 0,
            explored: HashSet::new(),
            travel_day: 1,
            lost: false,
            log: Vec::new(),
            log_seq: 0,
        }
    }

    /// Add a hex to the map. Returns an error if a hex at the same
    /// coordinates already exists.
    pub fn add_hex(&mut self, hex: HexCell) -> Result<(), String> {
        if self.hexes.iter().any(|h| h.x == hex.x && h.y == hex.y) {
            return Err(format!("duplicate hex at ({}, {})", hex.x, hex.y));
        }
        self.hexes.push(hex);
        Ok(())
    }

    /// Append a message to the log, capping at MAX_LOG_ENTRIES.
    pub fn log(&mut self, msg: String) {
        if self.log.len() >= Self::MAX_LOG_ENTRIES {
            let drain = self.log.len() - Self::MAX_LOG_ENTRIES / 2;
            self.log.drain(..drain);
        }
        self.log_seq += 1;
        self.log.push(LogEntry::new(self.log_seq, msg));
    }

    /// Find a hex by coordinates.
    pub fn find_hex(&self, x: i32, y: i32) -> Option<&HexCell> {
        self.hexes.iter().find(|h| h.x == x && h.y == y)
    }

    /// Get the current hex.
    pub fn current_hex(&self) -> Option<&HexCell> {
        self.find_hex(self.current_x, self.current_y)
    }

    /// Move to a hex by coordinates. Destination must be adjacent
    /// to the current hex (prevents teleportation).
    pub fn move_to(&mut self, x: i32, y: i32) -> Result<(), String> {
        if !self.hexes.iter().any(|h| h.x == x && h.y == y) {
            return Err(format!("hex ({}, {}) does not exist on the map", x, y));
        }
        if !Self::is_adjacent(self.current_x, self.current_y, x, y) {
            return Err(format!(
                "hex ({}, {}) is not adjacent to ({}, {})",
                x, y, self.current_x, self.current_y
            ));
        }
        self.current_x = x;
        self.current_y = y;
        self.explored.insert((x, y));
        Ok(())
    }

    /// Check if two hex coordinates are adjacent (including same hex).
    /// Uses offset coordinates where adjacent hexes differ by at most 1
    /// in each axis.
    fn is_adjacent(x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        dx <= 1 && dy <= 1
    }

    /// Daily travel speed in hexes based on party movement rate and terrain.
    /// Base: movement_rate / 120 hexes per day (on clear terrain).
    /// A party with 120' movement covers 1 hex/day on clear terrain,
    /// or 24 miles / 6-mile hex = 4 hexes at scale, but OSE uses 1 hex = 6 miles,
    /// and base overland travel = 24 miles/day for 120' movement = 4 hexes.
    /// However, different scales exist. We use: movement_rate / 30 = miles/day,
    /// then miles/day / 6 = hexes/day (for 6-mile hexes).
    pub fn hexes_per_day(movement_rate: u32, terrain: Terrain) -> u32 {
        // Base miles per day = movement_rate * 24 / 120 (OSE overland: 24 miles at 120')
        // = movement_rate / 5 miles per day
        // Hexes per day at 6 miles/hex = miles / 6
        let base_miles = movement_rate as u64 / 5;
        let (cost_num, cost_den) = terrain.movement_cost();
        // Effective miles = base_miles * cost_den / cost_num
        let effective_miles = base_miles * cost_den as u64 / cost_num as u64;
        let hexes = effective_miles / 6;
        if hexes < 1 { 1 } else { hexes as u32 }
    }

    /// Status display.
    pub fn status(&self) -> String {
        let terrain_name = self.current_hex()
            .map(|h| h.terrain.name())
            .unwrap_or("unknown");
        let mut out = format!(
            "Position: ({}, {})  Terrain: {}  Day: {}",
            self.current_x, self.current_y, terrain_name, self.travel_day
        );
        if self.lost {
            out.push_str("  [LOST]");
        }
        out.push_str(&format!("\nExplored: {} hexes", self.explored.len()));
        out
    }
}

impl Default for WildernessState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_movement_costs() {
        assert_eq!(Terrain::Clear.movement_cost(), (1, 1));
        assert_eq!(Terrain::Forest.movement_cost(), (3, 2));
        assert_eq!(Terrain::Mountains.movement_cost(), (3, 1));
    }

    #[test]
    fn terrain_lost_chances() {
        assert_eq!(Terrain::Clear.lost_chance(), 1);
        assert_eq!(Terrain::Hills.lost_chance(), 1);
        assert_eq!(Terrain::Mountains.lost_chance(), 1);
        assert_eq!(Terrain::Forest.lost_chance(), 2);
        assert_eq!(Terrain::Swamp.lost_chance(), 2);
        assert_eq!(Terrain::Jungle.lost_chance(), 2);
    }

    #[test]
    fn terrain_foraging() {
        assert!(Terrain::Forest.can_forage());
        assert!(!Terrain::Ocean.can_forage());
        assert_eq!(Terrain::Forest.forage_chance(), 2);
    }

    #[test]
    fn hex_movement() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(1, 0, Terrain::Forest)).unwrap();
        ws.move_to(1, 0).unwrap();
        assert_eq!(ws.current_x, 1);
        assert!(ws.explored.contains(&(1, 0)));
    }

    #[test]
    fn hex_movement_invalid() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        assert!(ws.move_to(5, 5).is_err());
    }

    #[test]
    fn hex_movement_nonadjacent_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(3, 0, Terrain::Hills)).unwrap();
        // (3,0) exists but is not adjacent to (0,0)
        assert!(ws.move_to(3, 0).is_err());
    }

    #[test]
    fn duplicate_hex_rejected() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        assert!(ws.add_hex(HexCell::new(0, 0, Terrain::Forest)).is_err());
    }

    #[test]
    fn travel_speed_clear() {
        // 120' movement on clear terrain: 24 miles/day / 6 = 4 hexes
        let hexes = WildernessState::hexes_per_day(120, Terrain::Clear);
        assert_eq!(hexes, 4);
    }

    #[test]
    fn travel_speed_forest() {
        // 120' on forest: 24 * 2/3 = 16 miles / 6 = 2.6 -> 2 hexes
        let hexes = WildernessState::hexes_per_day(120, Terrain::Forest);
        assert_eq!(hexes, 2);
    }

    #[test]
    fn travel_speed_mountains() {
        // 120' on mountains: 24 / 3 = 8 miles / 6 = 1.3 -> 1 hex
        let hexes = WildernessState::hexes_per_day(120, Terrain::Mountains);
        assert_eq!(hexes, 1);
    }

    #[test]
    fn travel_speed_slow_party() {
        // 60' movement on clear: 12 miles/day / 6 = 2 hexes
        let hexes = WildernessState::hexes_per_day(60, Terrain::Clear);
        assert_eq!(hexes, 2);
    }

    #[test]
    fn status_display() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        let s = ws.status();
        assert!(s.contains("Clear"));
        assert!(s.contains("(0, 0)"));
    }
}

// ── DSL parity tests ────────────────────────────────────────────

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    const ALL_TERRAINS: &[Terrain] = &[
        Terrain::Clear,
        Terrain::Forest,
        Terrain::Hills,
        Terrain::Mountains,
        Terrain::Desert,
        Terrain::Swamp,
        Terrain::Jungle,
        Terrain::Ocean,
        Terrain::River,
        Terrain::Barren,
        Terrain::City,
    ];

    /// Verify DSL terrain_lost_chance matches native for every terrain.
    #[test]
    fn dsl_lost_chance_matches_native() {
        for &terrain in ALL_TERRAINS {
            let dsl_val = dsl_gate::dsl_lost_chance(terrain)
                .unwrap_or_else(|| panic!("DSL terrain_lost_chance({:?}) failed", terrain));
            assert_eq!(
                dsl_val,
                lost_chance_native(terrain),
                "terrain_lost_chance mismatch for {:?}",
                terrain,
            );
        }
    }

    /// Verify DSL terrain_forage_chance matches native for every terrain.
    #[test]
    fn dsl_forage_chance_matches_native() {
        for &terrain in ALL_TERRAINS {
            let dsl_val = dsl_gate::dsl_forage_chance(terrain)
                .unwrap_or_else(|| panic!("DSL terrain_forage_chance({:?}) failed", terrain));
            assert_eq!(
                dsl_val,
                forage_chance_native(terrain),
                "terrain_forage_chance mismatch for {:?}",
                terrain,
            );
        }
    }

    /// Verify DSL terrain_can_forage matches native for every terrain.
    #[test]
    fn dsl_can_forage_matches_native() {
        for &terrain in ALL_TERRAINS {
            let dsl_val = dsl_gate::dsl_can_forage(terrain)
                .unwrap_or_else(|| panic!("DSL terrain_can_forage({:?}) failed", terrain));
            assert_eq!(
                dsl_val,
                can_forage_native(terrain),
                "terrain_can_forage mismatch for {:?}",
                terrain,
            );
        }
    }

    /// Verify DSL terrain_move_cost_num/den matches native for every terrain.
    #[test]
    fn dsl_movement_cost_matches_native() {
        for &terrain in ALL_TERRAINS {
            let dsl_val = dsl_gate::dsl_movement_cost(terrain)
                .unwrap_or_else(|| panic!("DSL movement_cost({:?}) failed", terrain));
            assert_eq!(
                dsl_val,
                movement_cost_native(terrain),
                "movement_cost mismatch for {:?}",
                terrain,
            );
        }
    }

    // ── Native-only copies for parity comparison ──────────────

    fn lost_chance_native(terrain: Terrain) -> u32 {
        match terrain {
            Terrain::Clear | Terrain::City | Terrain::Barren
            | Terrain::Hills | Terrain::Mountains => 1,
            Terrain::Forest | Terrain::River | Terrain::Desert
            | Terrain::Ocean | Terrain::Swamp | Terrain::Jungle => 2,
        }
    }

    fn forage_chance_native(terrain: Terrain) -> u32 {
        match terrain {
            Terrain::Forest | Terrain::Jungle | Terrain::River => 2,
            Terrain::Clear | Terrain::Hills => 1,
            Terrain::Swamp => 1,
            Terrain::Desert | Terrain::Mountains => 1,
            _ => 0,
        }
    }

    fn can_forage_native(terrain: Terrain) -> bool {
        !matches!(terrain, Terrain::Ocean | Terrain::City | Terrain::Barren)
    }

    fn movement_cost_native(terrain: Terrain) -> (u32, u32) {
        match terrain {
            Terrain::Clear | Terrain::City => (1, 1),
            Terrain::Barren | Terrain::Hills => (3, 2),
            Terrain::Forest | Terrain::River => (3, 2),
            Terrain::Desert | Terrain::Jungle => (2, 1),
            Terrain::Mountains => (3, 1),
            Terrain::Swamp => (2, 1),
            Terrain::Ocean => (1, 1),
        }
    }
}
