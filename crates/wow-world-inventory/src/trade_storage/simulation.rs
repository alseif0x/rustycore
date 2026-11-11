use std::collections::HashMap;

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{
    BagTemplateRef, CanStoreItemArgs, INVENTORY_SLOT_BAG_0, Item, ItemPosCount, ItemSlotRef,
    ItemStorageRef, ItemStorageTemplate, ItemStorageView, NULL_BAG, NULL_SLOT, PLAYER_SLOT_END,
    PlayerInventoryItem, is_bag_pos, is_buyback_slot, make_item_pos,
};

use crate::is_represented_bag_slot;

use super::{
    TradeStorageSnapshot, TradeStoreActionLikeCpp, TradeStoreCatalogsLikeCpp, TradeStorePlanLikeCpp,
};

pub(super) fn plan_trade_receive_snapshot_like_cpp(
    mut snapshot: TradeStorageSnapshot,
    outgoing: &[ObjectGuid],
    incoming: &[Item],
    catalogs: &TradeStoreCatalogsLikeCpp<'_>,
) -> Result<Vec<TradeStorePlanLikeCpp>, (InventoryResult, u32)> {
    // `Player::CanStoreItem` addresses bags through the native bag registry.
    // Runtime-only bag records are registered on this disposable storage copy.
    let runtime_bags: Vec<(u8, ObjectGuid, u32)> = snapshot
        .inventory_runtime
        .inventory_items()
        .iter()
        .filter(|(slot, _)| is_represented_bag_slot(**slot))
        .map(|(slot, record)| (*slot, record.guid, record.entry_id))
        .collect();
    for (slot, bag_guid, entry) in runtime_bags {
        let registered = snapshot
            .inventory
            .bags
            .get(usize::from(slot))
            .is_some_and(Option::is_some);
        let size = (catalogs.template)(entry).map_or(0, |template| template.container_slots);
        if !registered && size > 0 {
            let _ = snapshot
                .inventory
                .register_bag_storage(slot, bag_guid, size);
        }
    }

    for guid in outgoing {
        let _ = take_item_like_cpp(&mut snapshot, *guid);
    }

    let mut plans = Vec::with_capacity(incoming.len());
    for item in incoming {
        let entry = item.object().entry();
        let proto = (catalogs.template)(entry);
        let limit_category = proto
            .as_ref()
            .and_then(|proto| (catalogs.limit_category)(proto.item_limit_category));
        let mut dest = Vec::new();
        let outcome = {
            let views = StorageViewsLikeCpp::of(&snapshot, catalogs);
            let slot_items = views.slot_items();
            let stored_items = views.stored_items();
            let bag_templates = views.bag_templates();
            ItemStorageView::new(
                snapshot.owner_guid,
                &snapshot.inventory,
                snapshot.backpack_slots,
            )
            .can_store_item(
                &mut dest,
                CanStoreItemArgs {
                    bag: NULL_BAG,
                    slot: NULL_SLOT,
                    entry,
                    count: item.count(),
                    proto: proto.as_ref(),
                    source_item: Some(item),
                    source_is_not_empty_bag: false,
                    source_bop_trade_allowed_for_player: item
                        .is_soulbound_trade_allowed_for(snapshot.owner_guid),
                    swap: false,
                    limit_category: limit_category.as_ref(),
                    slot_items: &slot_items,
                    stored_items: &stored_items,
                    bag_templates: &bag_templates,
                },
            )
        };
        if outcome.result != InventoryResult::Ok {
            return Err((outcome.result, entry));
        }
        let plan = apply_destinations_like_cpp(&mut snapshot, item, &dest)
            .ok_or((InventoryResult::InternalBagError, entry))?;
        plans.push(plan);
    }
    Ok(plans)
}

fn take_item_like_cpp(snapshot: &mut TradeStorageSnapshot, item_guid: ObjectGuid) -> Option<Item> {
    let (bag, slot) = snapshot
        .inventory_runtime
        .trade_item_position_like_cpp(&snapshot.inventory, item_guid)?;
    if bag == INVENTORY_SLOT_BAG_0 {
        if snapshot
            .inventory
            .items
            .get(slot as usize)
            .copied()
            .flatten()
            == Some(item_guid)
        {
            snapshot.inventory.items[slot as usize] = None;
            if is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)) {
                snapshot.inventory.bags[slot as usize] = None;
            }
        }
        snapshot
            .inventory_runtime
            .remove_item_from_slot_like_cpp(slot);
    } else {
        let native = snapshot
            .inventory
            .bags
            .get(bag as usize)
            .and_then(Option::as_ref)
            .and_then(|storage| storage.slots.get(slot as usize).copied().flatten());
        if native == Some(item_guid) {
            let _ = snapshot.inventory.remove_bag_item(bag, slot);
        }
    }
    snapshot
        .inventory_runtime
        .remove_item_object_like_cpp(item_guid)
}

fn apply_destinations_like_cpp(
    snapshot: &mut TradeStorageSnapshot,
    item: &Item,
    dest: &[ItemPosCount],
) -> Option<TradeStorePlanLikeCpp> {
    let mut actions = Vec::with_capacity(dest.len());
    let mut stored = false;
    for position in dest {
        let bag = (position.pos >> 8) as u8;
        let slot = (position.pos & 0xFF) as u8;
        if let Some(existing) =
            snapshot
                .inventory_runtime
                .trade_item_at_like_cpp(&snapshot.inventory, bag, slot)
        {
            let target_guid = existing.object().guid();
            let new_count = existing.count().checked_add(position.count)?;
            set_stack_count_like_cpp(snapshot, target_guid, new_count);
            actions.push(TradeStoreActionLikeCpp::Merge {
                target_guid,
                added: position.count,
                new_count,
            });
        } else {
            // The represented plan permits one source item at most one
            // empty position.
            if stored {
                return None;
            }
            stored = true;
            if !store_item_like_cpp(snapshot, item, position.count, bag, slot) {
                return None;
            }
            actions.push(TradeStoreActionLikeCpp::Store {
                bag,
                slot,
                count: position.count,
            });
        }
    }
    Some(TradeStorePlanLikeCpp {
        item_guid: item.object().guid(),
        actions,
    })
}

fn set_stack_count_like_cpp(
    snapshot: &mut TradeStorageSnapshot,
    item_guid: ObjectGuid,
    count: u32,
) -> bool {
    let Some(item) = snapshot
        .inventory_runtime
        .item_objects_mut()
        .get_mut(&item_guid)
    else {
        return false;
    };
    item.set_count(count);
    true
}

fn store_item_like_cpp(
    snapshot: &mut TradeStorageSnapshot,
    item: &Item,
    count: u32,
    bag: u8,
    slot: u8,
) -> bool {
    if snapshot
        .inventory_runtime
        .trade_item_at_like_cpp(&snapshot.inventory, bag, slot)
        .is_some()
    {
        return false;
    }
    let owner = snapshot.owner_guid;
    let item_guid = item.object().guid();
    let mut placed = item.clone();
    placed.set_count(count);
    placed.set_owner_guid(owner);
    if bag == INVENTORY_SLOT_BAG_0 {
        if slot as usize >= PLAYER_SLOT_END {
            return false;
        }
        snapshot.inventory.items[slot as usize] = Some(item_guid);
        placed.set_container_guid(ObjectGuid::EMPTY);
        placed.set_contained_in(owner);
        placed.set_slot(slot);
        let entry_id = placed.object().entry();
        let _ = snapshot.inventory_runtime.store_item_in_slot_like_cpp(
            slot,
            PlayerInventoryItem {
                guid: item_guid,
                entry_id,
                db_guid: 0,
                inventory_type: None,
            },
        );
    } else {
        let Some(bag_guid) = snapshot
            .inventory_runtime
            .trade_bag_guid_like_cpp(&snapshot.inventory, bag)
        else {
            return false;
        };
        let has_native_bag = snapshot
            .inventory
            .bags
            .get(bag as usize)
            .is_some_and(Option::is_some);
        if has_native_bag
            && snapshot
                .inventory
                .store_bag_item(bag, slot, item_guid)
                .is_err()
        {
            return false;
        }
        placed.set_container_guid_and_slot(bag_guid, bag);
        placed.set_contained_in(bag_guid);
        placed.set_slot(slot);
    }
    let _ = snapshot
        .inventory_runtime
        .store_item_object_like_cpp(placed);
    true
}

/// The `CanStoreItem` views of one receiving-side inventory snapshot.
struct StorageViewsLikeCpp<'p> {
    positioned: Vec<(u8, u8, &'p Item)>,
    templates: HashMap<u32, ItemStorageTemplate>,
    bags: Vec<(u8, u32)>,
}

impl<'p> StorageViewsLikeCpp<'p> {
    fn of(snapshot: &'p TradeStorageSnapshot, catalogs: &TradeStoreCatalogsLikeCpp<'_>) -> Self {
        let runtime = &snapshot.inventory_runtime;
        let objects = runtime.item_objects();
        let mut positioned = Vec::new();
        let mut bags = Vec::new();
        let mut templates = HashMap::new();
        for (&slot, record) in runtime.inventory_items() {
            if is_buyback_slot(slot) {
                continue;
            }
            let Some(item) = objects.get(&record.guid) else {
                continue;
            };
            positioned.push((INVENTORY_SLOT_BAG_0, slot, item));
            if is_represented_bag_slot(slot) {
                bags.push((slot, record.entry_id));
            }
        }
        for item in objects.values() {
            if item.container_guid().is_empty() {
                continue;
            }
            positioned.push((item.bag_slot(), item.slot(), item));
        }
        for (_, _, item) in &positioned {
            let entry = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(vacant) = templates.entry(entry)
                && let Some(template) = (catalogs.template)(entry)
            {
                vacant.insert(template);
            }
        }
        Self {
            positioned,
            templates,
            bags,
        }
    }

    fn slot_items(&self) -> Vec<ItemSlotRef<'_>> {
        self.positioned
            .iter()
            .map(|(bag, slot, item)| ItemSlotRef::new(*bag, *slot, item))
            .collect()
    }

    fn stored_items(&self) -> Vec<ItemStorageRef<'_>> {
        self.positioned
            .iter()
            .map(|(bag, slot, item)| {
                ItemStorageRef::new(
                    *bag,
                    *slot,
                    item,
                    self.templates.get(&item.object().entry()),
                )
            })
            .collect()
    }

    fn bag_templates(&self) -> Vec<BagTemplateRef<'_>> {
        self.bags
            .iter()
            .filter_map(|(slot, entry)| {
                let template = self.templates.get(entry)?;
                (template.container_slots > 0).then(|| BagTemplateRef::new(*slot, template))
            })
            .collect()
    }
}
