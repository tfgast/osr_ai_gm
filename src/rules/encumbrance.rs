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
/// When `dsl-backend` is enabled and `OSR_BACKEND_ABILITY=dsl`, the
/// `encumbrance_level` and `movement_rate` functions delegate to DSL
/// evaluations. Other helpers remain native-only.
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

#[cfg(feature = "dsl-backend")]
use crate::backend::{is_dsl, MechanicGroup};

/// Returns true when the Ability mechanic group is using the DSL backend.
#[cfg(feature = "dsl-backend")]
#[inline]
fn use_dsl() -> bool {
    is_dsl(MechanicGroup::Ability)
}

/// Calculate encumbrance level from total weight in coins.
pub fn encumbrance_level(total_weight_cn: u32) -> EncumbranceLevel {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_gate::dsl_encumbrance_level(total_weight_cn) {
            return v;
        }
    }
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
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        let enc = encumbrance_level(total_weight_cn);
        if let Some(v) = dsl_gate::dsl_movement_rate(enc) {
            return v;
        }
    }
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

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::value::Value;

    use crate::backend::{self, NullState};
    use crate::bridge::handler::BridgeHandler;

    use super::EncumbranceLevel;

    fn variant_to_enc(variant: &str) -> Option<EncumbranceLevel> {
        match variant {
            "unencumbered" => Some(EncumbranceLevel::Unencumbered),
            "light" => Some(EncumbranceLevel::Light),
            "heavy" => Some(EncumbranceLevel::Heavy),
            "severe" => Some(EncumbranceLevel::Severe),
            "overloaded" => Some(EncumbranceLevel::Overloaded),
            _ => None,
        }
    }

    fn enc_to_value(enc: EncumbranceLevel) -> Value {
        let variant = match enc {
            EncumbranceLevel::Unencumbered => "unencumbered",
            EncumbranceLevel::Light => "light",
            EncumbranceLevel::Heavy => "heavy",
            EncumbranceLevel::Severe => "severe",
            EncumbranceLevel::Overloaded => "overloaded",
        };
        Value::EnumVariant {
            enum_name: "EncumbranceLevel".into(),
            variant: Name::from(variant),
            fields: BTreeMap::new(),
        }
    }

    pub fn dsl_encumbrance_level(weight: u32) -> Option<EncumbranceLevel> {
        let rt = backend::dsl()?;
        let mut handler = BridgeHandler::new();
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "encumbrance_level",
                vec![Value::Int(weight as i64)],
            )
            .ok()?;
        match result {
            Value::EnumVariant { variant, .. } => variant_to_enc(variant.as_str()),
            _ => None,
        }
    }

    pub fn dsl_movement_rate(enc: EncumbranceLevel) -> Option<u32> {
        let rt = backend::dsl()?;
        let mut handler = BridgeHandler::new();
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut handler,
                "movement_rate",
                vec![enc_to_value(enc)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
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

// ── DSL parity tests ────────────────────────────────────────────

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    /// Verify DSL encumbrance_level matches native for representative weights.
    #[test]
    fn dsl_encumbrance_level_matches_native() {
        let test_weights: &[u32] = &[
            0, 100, 400,     // unencumbered
            401, 500, 600,   // light
            601, 700, 800,   // heavy
            801, 1000, 1600, // severe
            1601, 2000,      // overloaded
        ];
        for &w in test_weights {
            let dsl_val = dsl_gate::dsl_encumbrance_level(w)
                .unwrap_or_else(|| panic!("DSL encumbrance_level({}) failed", w));
            assert_eq!(
                dsl_val,
                encumbrance_level_native(w),
                "encumbrance_level mismatch at weight {}",
                w,
            );
        }
    }

    /// Verify DSL movement_rate matches native for all encumbrance levels.
    #[test]
    fn dsl_movement_rate_matches_native() {
        let levels = [
            (EncumbranceLevel::Unencumbered, 120u32),
            (EncumbranceLevel::Light, 90),
            (EncumbranceLevel::Heavy, 60),
            (EncumbranceLevel::Severe, 30),
            (EncumbranceLevel::Overloaded, 0),
        ];
        for (enc, expected) in levels {
            let dsl_val = dsl_gate::dsl_movement_rate(enc)
                .unwrap_or_else(|| panic!("DSL movement_rate({:?}) failed", enc));
            assert_eq!(
                dsl_val, expected,
                "movement_rate mismatch for {:?}",
                enc,
            );
        }
    }

    // ── Native-only copy for parity comparison ────────────────

    fn encumbrance_level_native(weight: u32) -> EncumbranceLevel {
        match weight {
            0..=400 => EncumbranceLevel::Unencumbered,
            401..=600 => EncumbranceLevel::Light,
            601..=800 => EncumbranceLevel::Heavy,
            801..=1600 => EncumbranceLevel::Severe,
            _ => EncumbranceLevel::Overloaded,
        }
    }
}
