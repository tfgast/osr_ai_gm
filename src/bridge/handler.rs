use ttrpg_interp::effect::{Effect, EffectHandler, Response};

use crate::bridge::dice::roll_interp_dice;

/// Effect handler that bridges ttrpg_interp effects to the oag system.
///
/// Handles dice rolls via oag's dice engine and logs effects.
pub struct BridgeHandler {
    /// Human-readable log of effects processed during execution.
    pub effect_log: Vec<String>,
}

impl BridgeHandler {
    pub fn new() -> Self {
        BridgeHandler {
            effect_log: Vec::new(),
        }
    }
}

impl Default for BridgeHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHandler for BridgeHandler {
    fn handle(&mut self, effect: Effect) -> Response {
        match effect {
            Effect::RollDice { expr } => {
                let result = roll_interp_dice(&expr);
                self.effect_log.push(format!(
                    "Rolled {}d{}+{} = {} (dice: {:?})",
                    expr.count, expr.sides, expr.modifier, result.total, result.dice
                ));
                Response::Rolled(result)
            }

            Effect::MutateField {
                entity,
                ref path,
                ref value,
                ..
            } => {
                let path_str: Vec<String> = path
                    .iter()
                    .map(|seg| match seg {
                        ttrpg_interp::effect::FieldPathSegment::Field(f) => f.to_string(),
                        ttrpg_interp::effect::FieldPathSegment::Index(v) => format!("[{:?}]", v),
                    })
                    .collect();
                self.effect_log.push(format!(
                    "Mutate {:?}.{} = {:?}",
                    entity,
                    path_str.join("."),
                    value
                ));
                Response::Acknowledged
            }

            Effect::ApplyCondition {
                target,
                ref condition,
                ..
            } => {
                self.effect_log.push(format!(
                    "Apply condition '{}' to {:?}",
                    condition, target
                ));
                Response::Acknowledged
            }

            Effect::RemoveCondition {
                target,
                ref condition,
                ..
            } => {
                self.effect_log.push(format!(
                    "Remove condition '{}' from {:?}",
                    condition, target
                ));
                Response::Acknowledged
            }

            Effect::ActionStarted {
                ref name, actor, ..
            } => {
                self.effect_log
                    .push(format!("Action '{}' started by {:?}", name, actor));
                Response::Acknowledged
            }

            Effect::ActionCompleted { ref name, actor } => {
                self.effect_log
                    .push(format!("Action '{}' completed by {:?}", name, actor));
                Response::Acknowledged
            }

            Effect::DeductCost {
                actor,
                ref token,
                ref budget_field,
            } => {
                self.effect_log.push(format!(
                    "Deduct cost: {:?} spends {} ({})",
                    actor, token, budget_field
                ));
                Response::Acknowledged
            }

            Effect::RequiresCheck { passed, .. } => {
                if passed {
                    Response::Acknowledged
                } else {
                    Response::Vetoed
                }
            }

            Effect::ResolvePrompt { ref suggest, .. } => {
                // Auto-accept the suggestion if one is provided
                if let Some(val) = suggest {
                    Response::PromptResult(val.clone())
                } else {
                    Response::PromptResult(ttrpg_interp::value::Value::None)
                }
            }

            Effect::MutateTurnField { .. }
            | Effect::GrantGroup { .. }
            | Effect::RevokeGroup { .. }
            | Effect::ModifyApplied { .. } => Response::Acknowledged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttrpg_interp::value::DiceExpr;

    #[test]
    fn handle_roll_dice() {
        let mut handler = BridgeHandler::new();
        let effect = Effect::RollDice {
            expr: DiceExpr {
                count: 1,
                sides: 20,
                filter: None,
                modifier: 5,
            },
        };

        let response = handler.handle(effect);
        match response {
            Response::Rolled(result) => {
                assert!(result.total >= 6 && result.total <= 25);
                assert_eq!(result.modifier, 5);
                assert_eq!(result.dice.len(), 1);
            }
            _ => panic!("expected Rolled response"),
        }
        assert_eq!(handler.effect_log.len(), 1);
        assert!(handler.effect_log[0].contains("Rolled 1d20"));
    }

    #[test]
    fn handle_action_lifecycle() {
        use ttrpg_interp::effect::ActionKind;
        use ttrpg_interp::state::EntityRef;

        let mut handler = BridgeHandler::new();

        handler.handle(Effect::ActionStarted {
            name: "Attack".into(),
            kind: ActionKind::Action,
            actor: EntityRef(0),
            params: vec![],
        });
        handler.handle(Effect::ActionCompleted {
            name: "Attack".into(),
            actor: EntityRef(0),
        });

        assert_eq!(handler.effect_log.len(), 2);
        assert!(handler.effect_log[0].contains("started"));
        assert!(handler.effect_log[1].contains("completed"));
    }

    #[test]
    fn requires_check_pass_and_fail() {
        let mut handler = BridgeHandler::new();

        let r = handler.handle(Effect::RequiresCheck {
            action: "test".into(),
            passed: true,
            reason: None,
        });
        assert!(matches!(r, Response::Acknowledged));

        let r = handler.handle(Effect::RequiresCheck {
            action: "test".into(),
            passed: false,
            reason: Some("failed".into()),
        });
        assert!(matches!(r, Response::Vetoed));
    }
}
