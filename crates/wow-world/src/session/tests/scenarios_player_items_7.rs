//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_player_items_7/durability_repair.rs"]
mod durability_repair;

#[path = "scenarios_player_items_7/weapon_bonuses.rs"]
mod weapon_bonuses;

#[path = "scenarios_player_items_7/item_removal.rs"]
mod item_removal;

#[path = "scenarios_player_items_7/item_sets.rs"]
mod item_sets;
