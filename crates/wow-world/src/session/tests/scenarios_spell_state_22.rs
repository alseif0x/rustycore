//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_spell_state_22/battle_pet_experience.rs"]
mod battle_pet_experience;

#[path = "scenarios_spell_state_22/toy_casts.rs"]
mod toy_casts;

#[path = "scenarios_spell_state_22/transmog_sets.rs"]
mod transmog_sets;

#[path = "scenarios_spell_state_22/item_mods_and_sets.rs"]
mod item_mods_and_sets;
