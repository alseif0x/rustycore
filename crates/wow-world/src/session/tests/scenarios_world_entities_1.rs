//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_world_entities_1/combat_catalog_and_fanout.rs"]
mod combat_catalog_and_fanout;
#[path = "scenarios_world_entities_1/creature_attack_commands.rs"]
mod creature_attack_commands;
#[path = "scenarios_world_entities_1/creature_melee_commands.rs"]
mod creature_melee_commands;
#[path = "scenarios_world_entities_1/durable_runtime_rail.rs"]
mod durable_runtime_rail;
#[path = "scenarios_world_entities_1/visibility_refresh_commands.rs"]
mod visibility_refresh_commands;
