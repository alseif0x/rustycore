//! Session scenarios exercising the represented world-entity responsibility.
//!
//! Split out of `session_tests.rs` under #626. Focused creature-melee children
//! retain access to the shared parent session-test fixtures.

use super::*;

#[path = "scenarios_world_entities_28/creature_melee_admission.rs"]
mod creature_melee_admission;
#[path = "scenarios_world_entities_28/creature_melee_damage.rs"]
mod creature_melee_damage;
