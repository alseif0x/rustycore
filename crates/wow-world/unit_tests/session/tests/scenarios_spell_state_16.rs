//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_spell_state_16/heal_absorb.rs"]
mod heal_absorb;
#[path = "scenarios_spell_state_16/healing_and_health_leech.rs"]
mod healing_and_health_leech;
#[path = "scenarios_spell_state_16/honor_effects.rs"]
mod honor_effects;
#[path = "scenarios_spell_state_16/kill_credit.rs"]
mod kill_credit;
#[path = "scenarios_spell_state_16/pet_dismissal.rs"]
mod pet_dismissal;
