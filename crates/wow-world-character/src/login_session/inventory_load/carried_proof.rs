//! Validate and place the loaded carried topology without granting full Item-save authority.

use crate::login_session::world_entry::is_represented_bag_slot;
use wow_core::ObjectGuid;
use wow_entities::{INVENTORY_SLOT_BAG_0, Player, is_buyback_slot};
use wow_packet::packets::update::ItemCreateData;

pub(super) fn finish_carried_load_like_cpp(
    player: &mut Player,
    slots: &[ObjectGuid; 141],
    creates: &[ItemCreateData],
) -> bool {
    let runtime = player.inventory_runtime_like_cpp();
    // Source rows were checked before materialization. Recheck the actual
    // canonical objects and positions after every login await has completed.
    if runtime.item_objects().len() != creates.len() {
        return false;
    }
    for create in creates {
        let Some(item) = runtime.item_objects().get(&create.item_guid) else {
            return false;
        };
        if item.object().guid() != create.item_guid
            || item.owner_guid() != player.guid()
            || item.object().entry() != create.entry_id as u32
            || item.count() != create.stack_count
            || item.update_state() == wow_constants::ItemUpdateState::Removed
        {
            return false;
        }
    }
    for (slot, &guid) in slots.iter().enumerate() {
        let slot = slot as u8;
        let record = if is_buyback_slot(slot) {
            runtime.buyback_items().get(&slot)
        } else {
            runtime.inventory_items().get(&slot)
        };
        if record.map(|record| record.guid) != (!guid.is_empty()).then_some(guid) {
            return false;
        }
        let existing = player.get_item_by_pos(INVENTORY_SLOT_BAG_0, slot);
        if existing.is_some() && existing != Some(guid) {
            return false;
        }
        if guid.is_empty() {
            if player.inventory().bag_at(slot).is_some() {
                return false;
            }
            continue;
        }
        let Some(create) = creates.iter().find(|create| create.item_guid == guid) else {
            return false;
        };
        let item = &runtime.item_objects()[&guid];
        if item.slot() != slot
            || !item.container_guid().is_empty()
            || create.contained_in != player.guid()
            || record.is_none_or(|record| {
                record.entry_id != item.object().entry() || record.db_guid != guid.counter() as u64
            })
            || (is_represented_bag_slot(slot) && create.container_slots == 0)
            || create.container_slots > 36
        {
            return false;
        }
        if let Some(bag) = player.inventory().bag_at(slot) {
            if bag.bag_guid != guid || u32::from(bag.bag_size) != create.container_slots {
                return false;
            }
        }
        for (inner, &child) in create.container_item_guids.iter().enumerate() {
            if child.is_empty() {
                if player
                    .inventory()
                    .bag_at(slot)
                    .and_then(|bag| bag.slots[inner])
                    .is_some()
                {
                    return false;
                }
                continue;
            }
            let Some(item) = runtime.item_objects().get(&child) else {
                return false;
            };
            if inner >= create.container_slots as usize
                || item.slot() != inner as u8
                || item.container_guid() != guid
                || item.bag_slot() != slot
                || !creates
                    .iter()
                    .any(|create| create.item_guid == child && create.contained_in == guid)
                || player
                    .inventory()
                    .bag_at(slot)
                    .and_then(|bag| bag.slots[inner])
                    .is_some_and(|current| current != child)
            {
                return false;
            }
        }
    }
    // C++ Player.cpp:8034-8051 reads m_items and bag contents. Populate those
    // existing native positions only from the validated, successful load.
    for (slot, &guid) in slots.iter().enumerate() {
        let slot = slot as u8;
        if guid.is_empty() || is_buyback_slot(slot) {
            continue;
        }
        if player.store_top_level_item(slot, guid).is_err() {
            return false;
        }
        let create = creates
            .iter()
            .find(|create| create.item_guid == guid)
            .unwrap();
        if is_represented_bag_slot(slot) && create.container_slots > 0 {
            if player
                .register_bag_storage(slot, guid, create.container_slots as u8)
                .is_err()
            {
                return false;
            }
            for (inner, &child) in create.container_item_guids.iter().enumerate() {
                if !child.is_empty() && player.store_bag_item(slot, inner as u8, child).is_err() {
                    return false;
                }
            }
        }
    }
    player
        .inventory_runtime_mut_like_cpp()
        .set_carried_inventory_loaded_like_cpp(true);
    true
}
