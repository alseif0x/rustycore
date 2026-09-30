//! Narrow observations and forwards for the inventory scenarios.
use crate::session::WorldSession;
use wow_constants::{EnchantmentSlot, WeaponAttackType};
use wow_core::ObjectGuid;

pub fn inventory_move_with_item_mods_for_test(session: &mut WorldSession, src: u8, dst: u8) -> Option<bool> {
    session.move_represented_direct_inventory_item_with_item_mods_like_cpp(src, dst)
}
pub fn inventory_set_spell_events_empty_for_test(session: &WorldSession) -> bool {
    session.represented_item_set_spell_events_like_cpp().is_empty()
}
pub fn inventory_refresh_enchantment_durations_for_test(session: &mut WorldSession, guid: ObjectGuid) {
    session.refresh_inventory_item_enchantment_duration_refs_like_cpp(guid);
}
pub fn inventory_enchantment_persistence_for_test(session: &WorldSession, guid: ObjectGuid, mainhand_only: bool) -> Option<(String, Vec<EnchantmentSlot>)> {
    session.inventory_remove_enchantment_persistence_like_cpp(guid, mainhand_only)
}
pub fn inventory_remove_side_effects_for_test(session: &mut WorldSession, bag: u8, slot: u8, guid: ObjectGuid, cleared: &[EnchantmentSlot]) -> bool {
    session.apply_inventory_item_remove_side_effects_like_cpp(bag, slot, guid, cleared)
}
pub fn inventory_item_object_for_test(session: &WorldSession, guid: ObjectGuid) -> Option<wow_entities::Item> {
    session.resolved_inventory_item_object_like_cpp(guid)
}
pub fn inventory_non_bank_count_for_test(session: &WorldSession, entry: u32) -> Option<u32> {
    session.represented_non_bank_item_count_like_cpp(entry)
}
pub fn inventory_apply_relocation_for_test(session: &mut WorldSession, src_bag: u8, src_slot: u8, dst_bag: u8, dst_slot: u8, count: u32) -> bool {
    session.apply_committed_inventory_item_relocation_like_cpp(src_bag, src_slot, dst_bag, dst_slot, count)
}
pub fn inventory_storage_fields_update_for_test(item: &wow_entities::Item, slot_changed: bool, equipped_changed: bool, cleared: &[EnchantmentSlot]) -> Option<wow_packet::packets::update::ItemDataValuesDeltaUpdate> {
    let update = crate::session::item_storage_fields_values_update_like_cpp(item, slot_changed, equipped_changed, cleared);
    crate::entity_update_bridge::item_values_update_to_packet(&update)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryCombatRecalculationForTest(crate::session::RepresentedCombatStatRecalculationLikeCpp);
impl InventoryCombatRecalculationForTest {
    pub fn expertise(attack: WeaponAttackType) -> Self {
        Self(crate::session::RepresentedCombatStatRecalculationLikeCpp::Expertise { attack })
    }
    pub fn rating(combat_rating: u8) -> Self {
        Self(crate::session::RepresentedCombatStatRecalculationLikeCpp::Rating { combat_rating })
    }
}
#[derive(Debug)]
pub struct InventoryCombatRecalculationsForTest<'a>(&'a [crate::session::RepresentedCombatStatRecalculationLikeCpp]);
impl<const N: usize> PartialEq<&[InventoryCombatRecalculationForTest; N]> for InventoryCombatRecalculationsForTest<'_> {
    fn eq(&self, other: &&[InventoryCombatRecalculationForTest; N]) -> bool {
        self.0.len() == N && self.0.iter().zip(other.iter()).all(|(actual, expected)| *actual == expected.0)
    }
}
pub fn inventory_combat_recalculations_for_test(session: &WorldSession) -> InventoryCombatRecalculationsForTest<'_> {
    InventoryCombatRecalculationsForTest(session.inventory_combat_recalculations_for_test())
}
pub fn inventory_player_snapshot_for_test(session: &WorldSession) -> Option<wow_entities::Player> {
    session.inventory_player_snapshot_for_test()
}
pub fn inventory_player_handle_present_for_test(session: &WorldSession) -> bool {
    session.inventory_player_handle_present_for_test()
}
pub fn inventory_capacities_for_test(session: &WorldSession) -> (Option<u8>, Option<u8>) {
    (session.resolved_player_inventory_slot_count_like_cpp(), session.resolved_player_bank_bag_slot_count_like_cpp())
}
pub fn set_inventory_capacities_for_test(session: &mut WorldSession, backpack: u8, bank_bags: u8) -> (bool, bool) {
    (session.set_player_inventory_slot_count_like_cpp(backpack), session.set_player_bank_bag_slot_count_like_cpp(bank_bags))
}
