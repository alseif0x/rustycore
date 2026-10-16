//! Represented player item and inventory responsibility, separated from the Session
//! root under #597. Each submodule owns one complete operation group; the
//! canonical Player keeps the authority for the state they read and write.

use super::*;

mod appearance;
mod bank;
mod catalog;
mod durability;
mod enchantment;
mod equipment_sets;
mod items;
mod modifiers;
mod persistence;
mod persistence_load;
mod publication;
mod storage;
mod storage_move_host;
mod storage_slots;
#[cfg(test)]
pub(crate) mod test_fixtures;
mod valuation;

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment/f3_shims.rs"]
mod equipment_f3_shims;
