//! Represented equipment (#597). Its average-item-level record moved to
//! `wow_world_inventory::item_session` in #1363 slice 12c; the loot
//! source-contract scenario still reads this file, and it mounts the F3 shims.

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment/f3_shims.rs"]
mod f3_shims;
