// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage: items operations (#1363 slice 4c).

use super::*;

/// C++ passes packet `uint32 DstSlot` to helpers taking `uint8`, so the
/// language conversion truncates before the 160-slot range check.
pub fn void_storage_swap_destination_slot_like_cpp(dst_slot: u32) -> u8 {
    dst_slot as u8
}

pub(crate) fn void_storage_item_write_like_cpp(
    item: &RepresentedVoidStorageItemLikeCpp,
) -> wow_persistence::VoidStorageItemWriteLikeCpp {
    wow_persistence::VoidStorageItemWriteLikeCpp {
        item_id: item.item_id,
        item_entry: item.item_entry,
        creator_guid: item.creator_guid.counter() as u64,
        fixed_scaling_level: item.fixed_scaling_level,
        random_properties_id: item.random_properties_id,
        random_properties_seed: item.random_properties_seed,
        context: item.context,
    }
}

pub fn effective_void_storage_random_properties_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    random_properties_id: i32,
    random_properties_seed: i32,
) -> EffectiveVoidStorageRandomPropertiesLikeCpp {
    let (state, hub) = host.split_inventory_ref();
    state.effective_void_storage_random_properties_like_cpp(
        hub,
        random_properties_id,
        random_properties_seed,
    )
}

pub fn void_storage_enchantments_db_string_like_cpp(
    enchantment_ids: &[i32; wow_entities::MAX_ENCHANTMENT_SLOT],
) -> String {
    enchantment_ids
        .iter()
        .flat_map(|id| [id.to_string(), "0".to_string(), "0".to_string()])
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn overwrite_void_storage_random_property_enchantments_like_cpp(
    enchantments: &str,
    random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
) -> String {
    let mut fields = enchantments
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if fields.len() != wow_entities::MAX_ENCHANTMENT_SLOT * 3 {
        fields = vec!["0".to_string(); wow_entities::MAX_ENCHANTMENT_SLOT * 3];
    }
    let slots = if random_properties.id > 0 {
        EnchantmentSlot::Property2 as usize..=EnchantmentSlot::Property4 as usize
    } else if random_properties.id < 0 {
        EnchantmentSlot::Property0 as usize..=EnchantmentSlot::Property2 as usize
    } else {
        return fields.join(" ");
    };
    for slot in slots {
        let base = slot * 3;
        fields[base] = random_properties.enchantment_ids[slot].to_string();
        fields[base + 1] = "0".to_string();
        fields[base + 2] = "0".to_string();
    }
    fields.join(" ")
}

pub fn void_storage_merged_item_write_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    inventory_item: &InventoryItem,
    item: &wow_entities::Item,
    enchantments: &str,
) -> wow_persistence::VoidStorageMergedInventoryItemWriteLikeCpp {
    let (state, hub) = host.split_inventory_ref();
    state.void_storage_merged_item_write_like_cpp(hub, inventory_item, item, enchantments)
}

pub fn apply_effective_void_storage_random_properties_like_cpp(
    item: &mut wow_entities::Item,
    random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
) {
    if random_properties.id == 0 {
        return;
    }
    item.set_random_properties_id(random_properties.id);
    item.set_property_seed(random_properties.seed);
    let slots = if random_properties.id > 0 {
        EnchantmentSlot::Property2 as usize..=EnchantmentSlot::Property4 as usize
    } else {
        EnchantmentSlot::Property0 as usize..=EnchantmentSlot::Property2 as usize
    };
    for slot_index in slots {
        if let Some(slot) = EnchantmentSlot::from_usize(slot_index) {
            item.set_enchantment(slot, random_properties.enchantment_ids[slot_index], 0, 0);
        }
    }
}

pub fn plan_void_storage_destroyed_items_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    bag: u8,
    slot: u8,
    inventory_item: InventoryItem,
    cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
) -> Option<Vec<PlannedVoidDestroyedInventoryItemLikeCpp>> {
    let (state, hub) = host.split_inventory_ref();
    state.plan_void_storage_destroyed_items_like_cpp(
        hub,
        bag,
        slot,
        inventory_item,
        cleared_mainhand_enchantments,
    )
}

pub fn clear_item_publication_changes_like_cpp(item: &mut wow_entities::Item) {
    item.clear_item_data_changes();
    item.object_mut().clear_update_mask(false);
}

pub fn apply_committed_void_storage_destroyed_items_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &mut H,
    destroyed_items: &[PlannedVoidDestroyedInventoryItemLikeCpp],
) -> Option<(Vec<wow_core::ObjectGuid>, Vec<u32>)> {
    let mut destroyed_guids = Vec::with_capacity(destroyed_items.len());
    let mut changed_quest_ids = Vec::new();
    for destroyed in destroyed_items {
        let _ = host.apply_inventory_item_remove_side_effects_like_cpp(
            destroyed.bag,
            destroyed.slot,
            destroyed.inventory_item.guid,
            &destroyed.cleared_mainhand_enchantments,
        );
        let removed = host.apply_committed_inventory_item_removal_like_cpp(
            destroyed.bag,
            destroyed.slot,
            destroyed.inventory_item.guid,
        );
        debug_assert!(removed);
        destroyed_guids.push(destroyed.inventory_item.guid);
        // C++ recursive DestroyItem runs ItemRemovedQuestCheck after each
        // child/parent removal, preserving intermediate objective updates.
        changed_quest_ids.extend(wow_world_quest::apply_quest_item_removed_like_cpp(
            host,
            destroyed.inventory_item.entry_id,
        )?);
    }
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    Some((destroyed_guids, changed_quest_ids))
}
