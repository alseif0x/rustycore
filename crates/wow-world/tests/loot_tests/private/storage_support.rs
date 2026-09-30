//! Real application storage fixtures; each session retains the existing loot authority.
pub(super) use super::support::*;
pub(super) use super::quest_support::install_quest_bound_loot_objective_like_cpp;
pub(super) use super::inventory_port::{ControlledLootInventoryPort, install_storage_port};
pub(super) use std::collections::HashMap;
pub(super) use wow_loot::LootClaimPayload;
pub(super) use wow_world::session::mailbox::MasterLootGiveResult;
pub(super) use wow_world::session::InventoryItem;
pub(super) use wow_constants::ItemContext;
pub(super) use wow_core::ObjectGuidGenerator;
pub(super) use wow_persistence::PersistenceOutcomeLikeCpp;
pub(super) use wow_world::test_fixtures::loot::{
    applied_loot_item_quantity_for_test, request_master_loot_store_for_test, request_roll_loot_store_for_test,
    store_loot_materials_for_test, attach_loot_inventory_port_for_test, install_loot_inventory_item_for_test,
    loot_item_quest_allowed_for_test, mutate_loot_creature_for_test,
    two_sessions_with_money_loot_for_test, install_loot_wire_channel_for_test,
};
pub(super) const INVENTORY_SLOT_ITEM_START: u8 = 23;

pub(super) fn two_sessions_with_authoritative_creature_loot_like_cpp(loot: CreatureLoot) -> (WorldSession, flume::Receiver<Vec<u8>>, WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid, ObjectGuid, ObjectGuid) {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) = two_sessions_with_money_loot_for_test(loot);
    install_limited_test_item_template(&mut first, 25, 0);
    install_limited_test_item_template(&mut second, 25, 0);
    (first, first_rx, second, second_rx, owner, first_guid, second_guid)
}
pub(super) use super::recovery_support::authoritative_test_loot_like_cpp;

pub(super) fn represented_disenchant_test_outputs_like_cpp(winner_guid: ObjectGuid, item_id: u32) -> Vec<LootEntry> {
    (0..2).map(|loot_list_id| LootEntry {
        loot_list_id, item_id, quantity: 1, random_properties_id: 0, random_properties_seed: 0,
        item_context: 0, flags: LootEntryFlags { follow_loot_rules: true, ..Default::default() },
        allowed_looters: vec![winner_guid], roll_winner: winner_guid, ffa_looted_by: Vec::new(), taken: false,
    }).collect()
}
