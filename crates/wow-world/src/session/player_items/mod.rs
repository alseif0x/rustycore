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
mod storage;
mod storage_bags;
mod storage_slots;
mod valuation;
