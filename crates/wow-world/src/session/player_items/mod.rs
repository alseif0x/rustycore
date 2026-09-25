//! Represented player item and inventory responsibility, separated from the Session
//! root under #597. Each submodule owns one complete operation group; the
//! canonical Player keeps the authority for the state they read and write.

use super::*;

mod appearance;
mod bank;
mod catalog;
mod durability;
mod enchantment;
mod equipment;
mod equipment_sets;
mod equipment_slots;
mod items;
mod modifiers;
mod offhand;
mod persistence;
mod persistence_load;
mod publication;
pub(crate) use publication::item_push_result_from_send_new_item_plan;
mod storage;
mod storage_bags;
mod storage_slots;
#[cfg(test)]
pub(crate) mod test_fixtures;
mod valuation;
