//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_spell_state_23/equipped_item_replay.rs"]
mod equipped_item_replay;

#[path = "scenarios_spell_state_23/login_passives.rs"]
mod login_passives;

#[path = "scenarios_spell_state_23/item_set_auras.rs"]
mod item_set_auras;
