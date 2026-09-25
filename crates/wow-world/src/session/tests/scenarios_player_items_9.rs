//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_player_items_9/durability_scenarios.rs"]
mod durability_scenarios;
#[path = "scenarios_player_items_9/inventory_position_and_open_item.rs"]
mod inventory_position_and_open_item;
#[path = "scenarios_player_items_9/item_template_rules.rs"]
mod item_template_rules;
