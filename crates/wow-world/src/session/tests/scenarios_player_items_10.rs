//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_player_items_10/open_item.rs"]
mod open_item;

#[path = "scenarios_player_items_10/inventory_mutation.rs"]
mod inventory_mutation;

#[path = "scenarios_player_items_10/inventory_runtime.rs"]
mod inventory_runtime;

#[path = "scenarios_player_items_10/storage_planning.rs"]
mod storage_planning;
