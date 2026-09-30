//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_player_items_11/storage_planning.rs"]
mod storage_planning;

#[path = "scenarios_player_items_11/enchantment_durations.rs"]
mod enchantment_durations;

#[path = "scenarios_player_items_11/enchantment_replay.rs"]
mod enchantment_replay;

#[path = "scenarios_player_items_11/item_push.rs"]
mod item_push;
