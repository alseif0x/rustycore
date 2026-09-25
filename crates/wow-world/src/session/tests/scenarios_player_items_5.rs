//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_player_items_5/appearance_admission_and_updates.rs"]
mod appearance_admission_and_updates;
#[path = "scenarios_player_items_5/appearance_queries.rs"]
mod appearance_queries;
#[path = "scenarios_player_items_5/can_add_appearance_gates.rs"]
mod can_add_appearance_gates;
#[path = "scenarios_player_items_5/quest_reward_appearances.rs"]
mod quest_reward_appearances;
