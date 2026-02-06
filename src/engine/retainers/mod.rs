mod actions;
pub mod results;

pub use actions::{
    action_dismiss_retainer, action_hire_retainer, action_list_retainers, action_loyalty_check,
    action_retainer_morale,
};

#[cfg(test)]
mod golden_tests;
