//! Equipment tables loaded from JSON data files.
//! Covers adventuring gear, weapons, armour, ammunition, poisons, mounts, vehicles, and more.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Weapon quality flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WeaponQualities {
    pub melee: bool,
    pub missile: bool,
    pub blunt: bool,
    pub two_handed: bool,
    pub slow: bool,
    pub brace: bool,
    pub charge: bool,
    pub reload: bool,
    pub splash: bool,
}

impl WeaponQualities {
    /// Create qualities from a list of quality strings.
    fn from_list(qualities: &[String]) -> Self {
        let mut q = WeaponQualities::default();
        for quality in qualities {
            match quality.as_str() {
                "melee" => q.melee = true,
                "missile" => q.missile = true,
                "blunt" => q.blunt = true,
                "two_handed" => q.two_handed = true,
                "slow" => q.slow = true,
                "brace" => q.brace = true,
                "charge" => q.charge = true,
                "reload" => q.reload = true,
                "splash" => q.splash = true,
                _ => {}
            }
        }
        q
    }
}

/// Cost in gold pieces with optional note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    pub gp: f64,
    #[serde(default)]
    pub note: Option<String>,
}

impl Cost {
    pub fn gp_value(&self) -> u32 {
        self.gp as u32
    }
}

/// Range bands for missile weapons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeBands {
    pub short: [u32; 2],
    pub medium: [u32; 2],
    pub long: [u32; 2],
}

impl RangeBands {
    /// Convert to the tuple format used by the old API: (short_max, medium_max, long_max)
    pub fn as_tuple(&self) -> (u32, u32, u32) {
        (self.short[1], self.medium[1], self.long[1])
    }
}

/// Weapon definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponDef {
    pub name: String,
    #[serde(default)]
    pub cost: Option<Cost>,
    #[serde(default)]
    pub weight_coins: Option<u32>,
    #[serde(default)]
    pub damage: Option<String>,
    #[serde(default)]
    pub qualities: Vec<String>,
    #[serde(default)]
    pub range: Option<RangeBands>,
    pub category: String,
}

impl WeaponDef {
    /// Get cost in GP.
    pub fn cost_gp(&self) -> u32 {
        self.cost.as_ref().map(|c| c.gp_value()).unwrap_or(0)
    }

    /// Get weight in coins.
    pub fn weight(&self) -> u32 {
        self.weight_coins.unwrap_or(0)
    }

    /// Get damage dice.
    pub fn damage_dice(&self) -> &str {
        self.damage.as_deref().unwrap_or("1d4")
    }

    /// Get weapon qualities as flags.
    pub fn weapon_qualities(&self) -> WeaponQualities {
        WeaponQualities::from_list(&self.qualities)
    }

    /// Get range as tuple (short_max, medium_max, long_max). Returns (0,0,0) for melee-only.
    pub fn range_tuple(&self) -> (u32, u32, u32) {
        self.range.as_ref().map(|r| r.as_tuple()).unwrap_or((0, 0, 0))
    }
}

/// Check if a weapon name indicates a magical weapon.
///
/// Detects "+N" patterns (e.g., "Sword +1", "Dagger +2") and
/// "silver" material weapons (e.g., "Silver Dagger").
pub fn is_magical_weapon(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Check for +N bonus pattern
    if lower.contains("+1") || lower.contains("+2") || lower.contains("+3")
        || lower.contains("+4") || lower.contains("+5")
    {
        return true;
    }
    // Silver weapons can harm some immune monsters
    if lower.contains("silver") {
        return true;
    }
    false
}

/// Armour class definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmourClass {
    #[serde(default)]
    pub descending: Option<i32>,
    #[serde(default)]
    pub ascending: Option<i32>,
    #[serde(default)]
    pub bonus: Option<i32>,
    #[serde(default)]
    pub is_shield: bool,
}

/// Armour definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmourDef {
    pub name: String,
    pub ac: ArmourClass,
    pub cost: Cost,
    #[serde(default)]
    pub weight_coins: Option<u32>,
    pub category: String,
}

impl ArmourDef {
    /// Get descending AC value.
    pub fn ac_descending(&self) -> i32 {
        self.ac.descending.unwrap_or(9)
    }

    /// Get ascending AC value.
    pub fn ac_ascending(&self) -> i32 {
        self.ac.ascending.unwrap_or(10)
    }

    /// Check if this is a shield.
    pub fn is_shield(&self) -> bool {
        self.ac.is_shield
    }

    /// Get cost in GP.
    pub fn cost_gp(&self) -> u32 {
        self.cost.gp_value()
    }

    /// Get weight in coins.
    pub fn weight(&self) -> u32 {
        self.weight_coins.unwrap_or(0)
    }
}

/// Adventuring gear item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GearDef {
    pub name: String,
    pub cost: Cost,
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl GearDef {
    pub fn cost_gp(&self) -> u32 {
        self.cost.gp_value()
    }
}

/// Ammunition definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoDef {
    pub name: String,
    pub cost: Cost,
    pub category: String,
}

impl AmmoDef {
    pub fn cost_gp(&self) -> u32 {
        self.cost.gp_value()
    }
}

/// Poison type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoisonType {
    Bloodstream,
    Ingested,
}

/// Poison definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoisonDef {
    pub name: String,
    #[serde(rename = "type")]
    pub poison_type: PoisonType,
    pub tier: String,
    pub cost: Cost,
    pub save_modifier: String,
    pub detection_chance: String,
    pub onset_time: String,
    pub effect_on_save: String,
    pub effect_on_fail: String,
    pub category: String,
}

/// Mount/animal of burden definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountDef {
    pub name: String,
    pub cost: Cost,
    #[serde(default)]
    pub unencumbered: Option<LoadStats>,
    #[serde(default)]
    pub encumbered: Option<LoadStats>,
    pub category: String,
}

/// Load statistics for mounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadStats {
    pub miles_per_day: String,
    pub movement_rate: String,
    pub max_load_coins: String,
}

/// Land vehicle definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandVehicleDef {
    pub name: String,
    pub cost: Cost,
    pub miles_per_day: String,
    pub movement_rate: String,
    pub minimum_animals: String,
    pub min_load_coins: String,
    #[serde(default)]
    pub extra_animals: Option<String>,
    #[serde(default)]
    pub max_load_coins: Option<String>,
    pub category: String,
}

/// Water vessel definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterVesselDef {
    pub name: String,
    pub cost: Cost,
    pub cargo_capacity_coins: String,
    pub usage: String,
    pub length: String,
    pub beam: String,
    pub draft: String,
    pub seaworthy: bool,
    pub category: String,
}

/// Simple equipment item (tack, ship weapons).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleItemDef {
    pub name: String,
    pub cost: Cost,
    pub category: String,
}

/// Equipment categories in the JSON file.
#[derive(Debug, Deserialize)]
struct EquipmentCategories {
    #[serde(default)]
    gear: Vec<GearDef>,
    #[serde(default)]
    weapons: Vec<WeaponDef>,
    #[serde(default)]
    ammunition: Vec<AmmoDef>,
    #[serde(default)]
    armour: Vec<ArmourDef>,
    #[serde(default)]
    poisons: Vec<PoisonDef>,
    #[serde(default)]
    mounts: Vec<MountDef>,
    #[serde(default)]
    dogs: Vec<SimpleItemDef>,
    #[serde(default)]
    tack: Vec<SimpleItemDef>,
    #[serde(default)]
    land_vehicles: Vec<LandVehicleDef>,
    #[serde(default)]
    water_vessels: Vec<WaterVesselDef>,
    #[serde(default)]
    ship_weapons: Vec<SimpleItemDef>,
}

/// JSON file format for equipment data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EquipmentFile {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    total: Option<usize>,
    equipment: EquipmentCategories,
}

/// Registry holding all loaded equipment.
struct EquipmentRegistry {
    gear: Vec<GearDef>,
    weapons: Vec<WeaponDef>,
    ammunition: Vec<AmmoDef>,
    armour: Vec<ArmourDef>,
    poisons: Vec<PoisonDef>,
    mounts: Vec<MountDef>,
    dogs: Vec<SimpleItemDef>,
    tack: Vec<SimpleItemDef>,
    land_vehicles: Vec<LandVehicleDef>,
    water_vessels: Vec<WaterVesselDef>,
    ship_weapons: Vec<SimpleItemDef>,
    // Indexes
    weapons_by_name: HashMap<String, usize>,
    armour_by_name: HashMap<String, usize>,
    gear_by_name: HashMap<String, usize>,
}

impl EquipmentRegistry {
    fn new() -> Self {
        Self {
            gear: Vec::new(),
            weapons: Vec::new(),
            ammunition: Vec::new(),
            armour: Vec::new(),
            poisons: Vec::new(),
            mounts: Vec::new(),
            dogs: Vec::new(),
            tack: Vec::new(),
            land_vehicles: Vec::new(),
            water_vessels: Vec::new(),
            ship_weapons: Vec::new(),
            weapons_by_name: HashMap::new(),
            armour_by_name: HashMap::new(),
            gear_by_name: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<usize, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let file: EquipmentFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let eq = file.equipment;

        // Load and index weapons
        for weapon in eq.weapons {
            let name_lower = weapon.name.to_lowercase();
            let idx = self.weapons.len();
            self.weapons_by_name.insert(name_lower, idx);
            self.weapons.push(weapon);
        }

        // Load and index armour
        for armour in eq.armour {
            let name_lower = armour.name.to_lowercase();
            let idx = self.armour.len();
            self.armour_by_name.insert(name_lower, idx);
            self.armour.push(armour);
        }

        // Load and index gear
        for gear in eq.gear {
            let name_lower = gear.name.to_lowercase();
            let idx = self.gear.len();
            self.gear_by_name.insert(name_lower, idx);
            self.gear.push(gear);
        }

        // Load other categories
        self.ammunition = eq.ammunition;
        self.poisons = eq.poisons;
        self.mounts = eq.mounts;
        self.dogs = eq.dogs;
        self.tack = eq.tack;
        self.land_vehicles = eq.land_vehicles;
        self.water_vessels = eq.water_vessels;
        self.ship_weapons = eq.ship_weapons;

        let total = self.weapons.len()
            + self.armour.len()
            + self.gear.len()
            + self.ammunition.len()
            + self.poisons.len()
            + self.mounts.len()
            + self.dogs.len()
            + self.tack.len()
            + self.land_vehicles.len()
            + self.water_vessels.len()
            + self.ship_weapons.len();

        Ok(total)
    }

    fn find_weapon(&self, name: &str) -> Option<&WeaponDef> {
        let lower = name.to_lowercase();
        // Exact match first
        if let Some(&idx) = self.weapons_by_name.get(&lower) {
            return Some(&self.weapons[idx]);
        }
        // Fuzzy match: find a weapon whose name appears in the input
        // (handles "Fairy Longsword" → Longsword, "Dagger +1" → Dagger, etc.)
        let mut best: Option<&WeaponDef> = None;
        for weapon in &self.weapons {
            let wname = weapon.name.to_lowercase();
            if lower.contains(&wname) {
                // Prefer longest match (e.g., "long sword" over "sword")
                if best.map_or(true, |b| wname.len() > b.name.len()) {
                    best = Some(weapon);
                }
            }
        }
        best
    }

    fn find_armour(&self, name: &str) -> Option<&ArmourDef> {
        self.armour_by_name
            .get(&name.to_lowercase())
            .map(|&idx| &self.armour[idx])
    }

    fn find_gear(&self, name: &str) -> Option<&GearDef> {
        self.gear_by_name
            .get(&name.to_lowercase())
            .map(|&idx| &self.gear[idx])
    }
}

/// Global equipment registry.
static REGISTRY: OnceLock<EquipmentRegistry> = OnceLock::new();

/// Initialize the equipment registry by loading data files.
fn init_registry() -> EquipmentRegistry {
    let mut registry = EquipmentRegistry::new();

    if let Some(path) = crate::manifest::game_data_file("equipment") {
        if path.exists() {
            match registry.load_file(&path) {
                Ok(count) => {
                    eprintln!("Loaded {} equipment items from {}", count, path.display());
                    return registry;
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                }
            }
        }
    }

    eprintln!("Warning: No equipment data file found in game system manifest.");

    registry
}

/// Get the global equipment registry.
fn registry() -> &'static EquipmentRegistry {
    REGISTRY.get_or_init(init_registry)
}

// ============================================================================
// Public API - maintains backward compatibility with the old module
// ============================================================================

/// Get all weapons.
pub fn weapons() -> &'static [WeaponDef] {
    &registry().weapons
}

/// Get all armour.
pub fn armour() -> &'static [ArmourDef] {
    &registry().armour
}

/// Get all adventuring gear.
pub fn gear() -> &'static [GearDef] {
    &registry().gear
}

/// Get all ammunition.
pub fn ammunition() -> &'static [AmmoDef] {
    &registry().ammunition
}

/// Get all poisons.
pub fn poisons() -> &'static [PoisonDef] {
    &registry().poisons
}

/// Get all mounts.
pub fn mounts() -> &'static [MountDef] {
    &registry().mounts
}

/// Get all land vehicles.
pub fn land_vehicles() -> &'static [LandVehicleDef] {
    &registry().land_vehicles
}

/// Get all water vessels.
pub fn water_vessels() -> &'static [WaterVesselDef] {
    &registry().water_vessels
}

/// Look up a weapon by name (case-insensitive).
pub fn find_weapon(name: &str) -> Option<&'static WeaponDef> {
    registry().find_weapon(name)
}

/// Look up armour by name (case-insensitive).
pub fn find_armour(name: &str) -> Option<&'static ArmourDef> {
    registry().find_armour(name)
}

/// Look up gear by name (case-insensitive).
pub fn find_gear(name: &str) -> Option<&'static GearDef> {
    registry().find_gear(name)
}

/// Calculate AC from armour and shield, plus DEX modifier.
/// AC is descending: lower = better.
pub fn calculate_ac(armour_ac: i32, has_shield: bool, dex_mod: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_calc_ac(armour_ac, has_shield, dex_mod) {
            return v;
        }
    }
    native_calculate_ac(armour_ac, has_shield, dex_mod)
}

fn native_calculate_ac(armour_ac: i32, has_shield: bool, dex_mod: i32) -> i32 {
    let mut ac = armour_ac;
    if has_shield {
        ac -= 1;
    }
    ac - dex_mod
}

#[cfg(feature = "dsl-backend")]
fn dsl_calc_ac(armour_ac: i32, has_shield: bool, dex_mod: i32) -> Option<i32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    let shield_bonus = if has_shield { 1i64 } else { 0i64 };
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "calc_ac",
        vec![
            Value::Int(armour_ac as i64),
            Value::Int(shield_bonus),
            Value::Int(dex_mod as i64),
        ],
    ) {
        Ok(Value::Int(v)) => Some(v as i32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        let w = weapons();
        assert!(w.len() >= 19, "Should have at least 19 weapons, got {}", w.len());
    }

    #[test]
    fn weapon_count() {
        // The new data has more weapons (includes improvised like torch, oil flask)
        assert!(weapons().len() >= 19);
    }

    #[test]
    fn armour_count() {
        assert_eq!(armour().len(), 4, "Should have 4 armour types");
    }

    #[test]
    fn gear_count() {
        assert_eq!(gear().len(), 24, "Should have 24 gear items");
    }

    #[test]
    fn find_sword() {
        let w = find_weapon("Sword").unwrap();
        assert_eq!(w.cost_gp(), 10);
        assert_eq!(w.damage_dice(), "1d8");
        let q = w.weapon_qualities();
        assert!(q.melee);
        assert!(!q.missile);
    }

    #[test]
    fn find_dagger() {
        let w = find_weapon("dagger").unwrap();
        assert_eq!(w.cost_gp(), 3);
        let q = w.weapon_qualities();
        assert!(q.melee);
        assert!(q.missile);
        let range = w.range_tuple();
        assert_eq!(range, (10, 20, 30));
    }

    #[test]
    fn find_leather() {
        let a = find_armour("leather").unwrap();
        assert_eq!(a.ac_descending(), 7);
        assert_eq!(a.cost_gp(), 20);
    }

    #[test]
    fn find_plate() {
        let a = find_armour("Plate mail").unwrap();
        assert_eq!(a.ac_descending(), 3);
    }

    #[test]
    fn ac_unarmoured() {
        assert_eq!(calculate_ac(9, false, 0), 9);
    }

    #[test]
    fn ac_plate_shield_dex() {
        // Plate (3) + shield (-1) + DEX +2 mod (-2) = 0
        assert_eq!(calculate_ac(3, true, 2), 0);
    }

    #[test]
    fn ac_leather_no_shield_low_dex() {
        // Leather (7) + no shield + DEX -1 = 8
        assert_eq!(calculate_ac(7, false, -1), 8);
    }

    #[test]
    fn staff_qualities() {
        let w = find_weapon("Staff").unwrap();
        let q = w.weapon_qualities();
        assert!(q.blunt);
        assert!(q.two_handed);
        // Note: Advanced Fantasy source shows Staff as slow, but Basic OSE does not.
        // The JSON data reflects the Advanced Fantasy source.
        assert!(q.slow, "Staff has slow quality in Advanced Fantasy");
    }

    #[test]
    fn blunt_weapons() {
        let blunt: Vec<_> = weapons()
            .iter()
            .filter(|w| w.weapon_qualities().blunt)
            .collect();
        let names: Vec<&str> = blunt.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"Club"));
        assert!(names.contains(&"Mace"));
        assert!(names.contains(&"Sling"));
        assert!(names.contains(&"Staff"));
        assert!(names.contains(&"War hammer"));
    }

    #[test]
    fn poisons_loaded() {
        let p = poisons();
        assert_eq!(p.len(), 9, "Should have 9 poisons (4 bloodstream + 5 ingested)");
    }

    #[test]
    fn mounts_loaded() {
        let m = mounts();
        assert_eq!(m.len(), 5, "Should have 5 mounts");
    }

    #[test]
    fn water_vessels_loaded() {
        let v = water_vessels();
        assert_eq!(v.len(), 16, "Should have 16 water vessels");
    }

    #[test]
    fn crossbow_has_reload() {
        let w = find_weapon("Crossbow").unwrap();
        let q = w.weapon_qualities();
        assert!(q.reload, "Crossbow should have reload quality");
        assert!(q.slow, "Crossbow should have slow quality");
    }

    /// hq-6c541: Fuzzy weapon matching for magical/named variants.
    #[test]
    fn find_weapon_fuzzy_magic_variant() {
        // "Fairy Longsword" should match "Longsword"
        let w = find_weapon("Fairy Longsword").unwrap();
        assert!(w.name.to_lowercase().contains("sword"), "should match a sword variant");

        // "Dagger +1" should match "Dagger"
        let w = find_weapon("Dagger +1").unwrap();
        assert_eq!(w.name.to_lowercase(), "dagger");

        // Exact match still works
        let w = find_weapon("Sword").unwrap();
        assert_eq!(w.name, "Sword");
    }

    #[test]
    fn is_magical_weapon_detection() {
        assert!(is_magical_weapon("Sword +1"));
        assert!(is_magical_weapon("Dagger +2"));
        assert!(is_magical_weapon("Mace +3"));
        assert!(is_magical_weapon("Silver Dagger"));
        assert!(is_magical_weapon("silver dagger"));
        assert!(!is_magical_weapon("Sword"));
        assert!(!is_magical_weapon("Dagger"));
        assert!(!is_magical_weapon("Mace"));
        assert!(!is_magical_weapon("Short Bow"));
    }
}
