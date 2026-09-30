//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;
use crate::session::CR_ARMOR_PENETRATION_LIKE_CPP;

#[path = "scenarios_spell_state_2/capability_unlearning.rs"]
mod capability_unlearning;

#[path = "scenarios_spell_state_2/offhand_unequip.rs"]
mod offhand_unequip;

#[path = "scenarios_spell_state_2/pvp_item_scaling.rs"]
mod pvp_item_scaling;
