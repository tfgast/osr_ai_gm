/// Basic encumbrance system per OSE Rules Tome.
/// Uses the "Basic Encumbrance" option: determined by armour + treasure carried.
///
/// Weight is measured in coins (cn). 10 coins = 1 pound.
///
/// Movement rates (feet per turn):
/// - Unencumbered (≤400 cn): 120'
/// - Lightly encumbered (401-600 cn): 90'
/// - Heavily encumbered (601-800 cn): 60'
/// - Severely encumbered (801-1600 cn): 30'
/// - Over maximum (>1600 cn): 0' (cannot move)
///
/// Encumbrance category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum EncumbranceLevel {
    Unencumbered,
    Light,
    Heavy,
    Severe,
    Overloaded,
}

impl EncumbranceLevel {
    pub fn name(self) -> &'static str {
        match self {
            EncumbranceLevel::Unencumbered => "Unencumbered",
            EncumbranceLevel::Light => "Lightly Encumbered",
            EncumbranceLevel::Heavy => "Heavily Encumbered",
            EncumbranceLevel::Severe => "Severely Encumbered",
            EncumbranceLevel::Overloaded => "Overloaded",
        }
    }
}

/// Maximum carrying capacity in coins.
pub const MAX_CAPACITY_CN: u32 = 1600;

/// Calculate encumbrance level from total weight in coins.
pub fn encumbrance_level(total_weight_cn: u32) -> EncumbranceLevel {
    match total_weight_cn {
        0..=400 => EncumbranceLevel::Unencumbered,
        401..=600 => EncumbranceLevel::Light,
        601..=800 => EncumbranceLevel::Heavy,
        801..=1600 => EncumbranceLevel::Severe,
        _ => EncumbranceLevel::Overloaded,
    }
}

/// Calculate movement rate (feet per turn) based on total weight.
pub fn movement_rate(total_weight_cn: u32) -> u32 {
    match encumbrance_level(total_weight_cn) {
        EncumbranceLevel::Unencumbered => 120,
        EncumbranceLevel::Light => 90,
        EncumbranceLevel::Heavy => 60,
        EncumbranceLevel::Severe => 30,
        EncumbranceLevel::Overloaded => 0,
    }
}

/// Calculate total weight from inventory items.
/// Each item weight is in coins (cn). Gold pieces also count as 1 cn each.
pub fn total_weight(item_weights_cn: &[u32], gold_pieces: u32) -> u32 {
    let items: u32 = item_weights_cn.iter().copied().fold(0u32, u32::saturating_add);
    items.saturating_add(gold_pieces)
}

/// Armour weight contribution for basic encumbrance.
/// In the basic system, armour type determines the base encumbrance.
pub fn armour_weight(armour_name: &str) -> u32 {
    let lower = armour_name.to_lowercase();
    if lower.contains("plate") {
        500
    } else if lower.contains("chain") {
        400
    } else if lower.contains("leather") {
        200
    } else if lower.contains("shield") {
        100
    } else {
        0 // unarmoured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unencumbered() {
        assert_eq!(encumbrance_level(0), EncumbranceLevel::Unencumbered);
        assert_eq!(encumbrance_level(400), EncumbranceLevel::Unencumbered);
        assert_eq!(movement_rate(0), 120);
        assert_eq!(movement_rate(400), 120);
    }

    #[test]
    fn light_encumbrance() {
        assert_eq!(encumbrance_level(401), EncumbranceLevel::Light);
        assert_eq!(encumbrance_level(600), EncumbranceLevel::Light);
        assert_eq!(movement_rate(500), 90);
    }

    #[test]
    fn heavy_encumbrance() {
        assert_eq!(encumbrance_level(601), EncumbranceLevel::Heavy);
        assert_eq!(encumbrance_level(800), EncumbranceLevel::Heavy);
        assert_eq!(movement_rate(700), 60);
    }

    #[test]
    fn severe_encumbrance() {
        assert_eq!(encumbrance_level(801), EncumbranceLevel::Severe);
        assert_eq!(encumbrance_level(1600), EncumbranceLevel::Severe);
        assert_eq!(movement_rate(1000), 30);
    }

    #[test]
    fn overloaded() {
        assert_eq!(encumbrance_level(1601), EncumbranceLevel::Overloaded);
        assert_eq!(movement_rate(2000), 0);
    }

    #[test]
    fn total_weight_calculation() {
        assert_eq!(total_weight(&[200, 60, 30], 100), 390);
    }

    #[test]
    fn total_weight_with_gold() {
        // Leather armour (200) + sword (60) + 200gp = 460 = lightly encumbered
        let weight = total_weight(&[200, 60], 200);
        assert_eq!(weight, 460);
        assert_eq!(encumbrance_level(weight), EncumbranceLevel::Light);
    }

    #[test]
    fn armour_weights() {
        assert_eq!(armour_weight("Plate mail"), 500);
        assert_eq!(armour_weight("Chain mail"), 400);
        assert_eq!(armour_weight("Leather"), 200);
        assert_eq!(armour_weight("Shield"), 100);
        assert_eq!(armour_weight("None"), 0);
    }

    #[test]
    fn fighter_in_plate_with_loot() {
        // Plate (500) + shield (100) + sword (60) + 200gp = 860 = severely encumbered
        let weight = total_weight(&[500, 100, 60], 200);
        assert_eq!(weight, 860);
        assert_eq!(encumbrance_level(weight), EncumbranceLevel::Severe);
        assert_eq!(movement_rate(weight), 30);
    }

    #[test]
    fn thief_in_leather() {
        // Leather (200) + short sword (30) + thieves tools (10) = 240 = unencumbered
        let weight = total_weight(&[200, 30, 10], 0);
        assert_eq!(weight, 240);
        assert_eq!(encumbrance_level(weight), EncumbranceLevel::Unencumbered);
        assert_eq!(movement_rate(weight), 120);
    }

    #[test]
    fn encumbrance_level_names() {
        assert_eq!(EncumbranceLevel::Unencumbered.name(), "Unencumbered");
        assert_eq!(EncumbranceLevel::Overloaded.name(), "Overloaded");
    }
}
