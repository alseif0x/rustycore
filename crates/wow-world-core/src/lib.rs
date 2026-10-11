//! Session-independent world owners shared by `wow-world` and its tests.

pub mod battle_pet_account;
pub mod canonical_player_access;
pub mod catalogs;
pub mod corpse_load;
pub mod creature_movement;
pub mod creature_projection;
pub mod creature_respawn;
#[cfg(test)]
#[path = "../unit_tests/creature_runtime_tests.rs"]
mod creature_runtime_tests;
pub mod entity_update_bridge;
pub mod loot_persistence;
pub mod pathfinder;
pub mod phasing;
mod player;
#[path = "session/directory.rs"]
pub mod player_directory;
pub mod runtime_plan;
pub mod session;
pub mod session_policy;
pub mod terrain;
