//! Session scenarios exercising the represented combat responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_combat_1/canonical_player_ownership.rs"]
mod canonical_player_ownership;
#[path = "scenarios_combat_1/combat_rating_and_swing_bonuses.rs"]
mod combat_rating_and_swing_bonuses;
#[path = "scenarios_combat_1/combat_reach_and_visibility.rs"]
mod combat_reach_and_visibility;
#[path = "scenarios_combat_1/combat_tick_admission.rs"]
mod combat_tick_admission;
#[path = "scenarios_combat_1/death_and_healing.rs"]
mod death_and_healing;
#[path = "scenarios_combat_1/quest_kill_rewards.rs"]
mod quest_kill_rewards;
