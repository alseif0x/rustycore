//! Scalar forwards to the original Character helpers.
use super::super::*;

pub fn make_bank_fixture_for_test(capacity: usize) -> (
    WorldSession, flume::Receiver<Vec<u8>>, std::sync::Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    super::super::bank_test_support::make_bank_slot_session(capacity)
}
pub fn insert_inventory_binder_creature_for_test(manager: &std::sync::Arc<std::sync::Mutex<wow_map::MapManager>>, guid: ObjectGuid, npc_flags: u32) {
    super::super::bank_test_support::insert_binder_creature(manager, guid, npc_flags);
}

pub fn loaded_inventory_slot_count_for_test(saved_slots: u8) -> u8 {
    loaded_inventory_slot_count_with_legacy_rust_compat(saved_slots)
}
pub fn player_money_gain_for_test(current: u64, amount: u64) -> Option<u64> {
    player_money_gain_like_cpp(current, amount)
}
pub fn item_is_not_empty_bag_for_test(kind: Option<InventoryType>, contains_items: bool) -> bool {
    item_is_not_empty_bag_like_cpp(kind, contains_items)
}
pub fn bind_inventory_destination_for_test(item: &mut wow_entities::Item, destination: u16) {
    bind_inventory_item_for_destination_like_cpp(item, destination)
}
pub fn item_dynamic_flags_changed_for_test(before: &wow_entities::Item, after: &wow_entities::Item) -> bool {
    item_dynamic_flags_changed_like_cpp(before, after)
}
pub fn parse_equipment_cache_for_test(cache: &str) -> [VisualItemInfo; 34] {
    parse_equipment_cache(cache)
}
pub fn item_is_currently_looted_for_test(item: &wow_entities::Item) -> bool {
    item_is_currently_looted_like_cpp(item)
}
pub fn bank_store_destination_applies_obtain_spells_for_test(bag: u8) -> bool {
    bank_store_destination_applies_obtain_spells_like_cpp(bag)
}
pub fn bank_store_item_added_quest_count_for_test(plan: &InventoryStorageMovePlanLikeCpp) -> u32 {
    bank_store_item_added_quest_count_like_cpp(plan)
}
pub fn item_spell_charges_db_string_for_test(charges: &[i32], effect_count: usize) -> String {
    item_spell_charges_db_string(charges, effect_count)
}
pub fn item_storage_mutable_persistence_for_test(db_guid: u64, item: &wow_entities::Item, count: u32, flags: u32, enchantments: String, effect_count: usize) -> wow_persistence::InventoryItemMutablePersistenceLikeCpp {
    item_storage_mutable_persistence_like_cpp(db_guid, item, count, flags, enchantments, effect_count)
}
