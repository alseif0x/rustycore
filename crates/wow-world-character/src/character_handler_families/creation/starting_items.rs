// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;

use wow_core::ObjectGuidGenerator;
use wow_packet::packets::{character::VisualItemInfo, item::equip_slot_for_inventory_type};
use wow_persistence::CharacterStartingItemPersistenceLikeCpp;
use wow_world_core::session::SessionCatalogs;

pub(super) fn project_starting_items_like_cpp(
    catalogs: &SessionCatalogs,
    race: u8,
    class: u8,
    item_guid_generator: &ObjectGuidGenerator,
) -> (Vec<CharacterStartingItemPersistenceLikeCpp>, String) {
    let mut items = Vec::new();
    let mut visuals = [VisualItemInfo::default(); 34];
    let mut item_slots = HashMap::new();
    let mut next_inventory_slot = 19;

    if let (Some(loadouts), Some(loadout_items)) = (
        catalogs.items.character_loadout_store.as_ref(),
        catalogs.items.character_loadout_item_store.as_ref(),
    ) {
        for loadout in loadouts.values().filter(|loadout| {
            loadout.purpose == 9
                && loadout.chr_class_id == class as i8
                && race > 0
                && loadout.race_mask & (1_i64 << (race - 1)) != 0
        }) {
            for relation in loadout_items
                .values()
                .filter(|relation| relation.character_loadout_id == loadout.id)
            {
                push_starting_item_like_cpp(
                    catalogs,
                    relation.item_id,
                    1,
                    item_guid_generator,
                    &mut items,
                    &mut visuals,
                    &mut item_slots,
                    &mut next_inventory_slot,
                );
            }
        }
    }

    for row in &catalogs.items.starting_item_overrides {
        if (row.race != 0 && row.race != race)
            || (row.class != 0 && row.class != class)
            || row.amount == 0
        {
            continue;
        }
        if row.amount > 0 {
            push_starting_item_like_cpp(
                catalogs,
                row.item,
                row.amount as u32,
                item_guid_generator,
                &mut items,
                &mut visuals,
                &mut item_slots,
                &mut next_inventory_slot,
            );
        } else {
            items.retain(|item| item.item_id != row.item);
            item_slots.retain(|slot, item| {
                if *item == row.item {
                    visuals[*slot as usize] = VisualItemInfo::default();
                    return false;
                }
                true
            });
        }
    }

    let equipment_cache = visuals
        .iter()
        .flat_map(|item| {
            [
                item.inv_type.to_string(),
                item.display_id.to_string(),
                item.display_enchant_id.to_string(),
                item.subclass.to_string(),
                item.secondary_item_modified_appearance_id.to_string(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" ");
    (items, format!("{equipment_cache} "))
}

fn push_starting_item_like_cpp(
    catalogs: &SessionCatalogs,
    item_id: u32,
    count: u32,
    item_guid_generator: &ObjectGuidGenerator,
    items: &mut Vec<CharacterStartingItemPersistenceLikeCpp>,
    visuals: &mut [VisualItemInfo; 34],
    item_slots: &mut HashMap<u8, u32>,
    next_inventory_slot: &mut u8,
) {
    let Some(sparse) = catalogs
        .items
        .stats_store
        .as_ref()
        .and_then(|store| store.sparse_template(item_id))
    else {
        return;
    };
    let Some(modified) = catalogs
        .items
        .modified_appearance_store
        .as_ref()
        .and_then(|store| store.get_default_for_item(item_id))
    else {
        return;
    };
    let Ok(appearance_id) = u32::try_from(modified.item_appearance_id) else {
        return;
    };
    let Some(display_id) = catalogs
        .items
        .appearance_store
        .as_ref()
        .and_then(|store| store.item_display_info_id(appearance_id))
    else {
        return;
    };
    let Some(basic) = catalogs
        .items
        .store
        .as_ref()
        .and_then(|store| store.get(item_id))
    else {
        return;
    };
    let inventory_type = sparse.inventory_type as u8;
    let occupied = item_slots.keys().map(|slot| (*slot, ())).collect();
    let slot = equip_slot_for_inventory_type(inventory_type, &occupied)
        .filter(|slot| !item_slots.contains_key(slot))
        .or_else(|| next_free_inventory_slot(item_slots, next_inventory_slot));
    let Some(slot) = slot else {
        return;
    };
    item_slots.insert(slot, item_id);
    let guid = item_guid_generator.generate() as u64;
    items.push(CharacterStartingItemPersistenceLikeCpp {
        guid,
        item_id,
        slot,
        count,
        durability: sparse.max_durability,
    });
    if slot < 19 || slot >= 30 {
        visuals[slot as usize] = VisualItemInfo {
            inv_type: inventory_type,
            display_id,
            display_enchant_id: 0,
            secondary_item_modified_appearance_id: 0,
            subclass: basic.subclass_id,
        };
    }
}

fn next_free_inventory_slot(item_slots: &HashMap<u8, u32>, next: &mut u8) -> Option<u8> {
    while item_slots.contains_key(next) {
        *next = next.saturating_add(1);
    }
    if *next >= 35 {
        return None;
    }
    let slot = *next;
    *next = next.saturating_add(1);
    Some(slot)
}

#[cfg(test)]
#[path = "starting_items_tests.rs"]
mod tests;
