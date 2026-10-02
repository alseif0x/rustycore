//! Session-independent world owners shared by `wow-world` and its tests.

pub mod battle_pet_account;
pub mod canonical_player_access;
pub mod catalogs;
pub mod loot_persistence;
pub mod map_manager;
pub mod phasing;
mod player;
#[path = "session/directory.rs"]
pub mod player_directory;
pub mod session;
pub mod session_policy;
