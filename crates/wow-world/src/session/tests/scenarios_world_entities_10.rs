//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_world_entities_10/creature_aura_application.rs"]
mod creature_aura_application;
#[path = "scenarios_world_entities_10/creature_kill_and_death.rs"]
mod creature_kill_and_death;
#[path = "scenarios_world_entities_10/spell_damage_death.rs"]
mod spell_damage_death;
#[path = "scenarios_world_entities_10/spell_healing.rs"]
mod spell_healing;
#[path = "scenarios_world_entities_10/spell_threat.rs"]
mod spell_threat;
