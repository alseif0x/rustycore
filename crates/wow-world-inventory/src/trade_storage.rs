// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Storage planning of a completed trade (#38).
//!
//! C++ checks `Player::CanStoreItems` for both participants while the traded
//! items are flagged in-trade, so their slots count as free
//! (`TradeHandler.cpp:445-447`, `Player.cpp:10328-10352`), and `moveItems`
//! then stores each item with `CanStoreItem(NULL_BAG, NULL_SLOT, ...)` after
//! every traded item left both inventories (`TradeHandler.cpp:118-121`).
//! The plan runs that sequence on a detached copy of the receiving canonical
//! Player: the outgoing items leave it first, then every incoming item is
//! planned and stored in trade-slot order, so a later item sees the earlier
//! ones. The copy is never written back; it only produces positions.

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{
    BagTemplateRef, CanStoreItemArgs, INVENTORY_SLOT_BAG_0, Item, ItemLimitCategoryTemplate,
    ItemPosCount, ItemSlotRef, ItemStorageRef, ItemStorageTemplate, NULL_BAG, NULL_SLOT, Player,
    is_buyback_slot,
};

use crate::is_represented_bag_slot;

/// Where one incoming item ends up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeStoreActionLikeCpp {
    /// `_StoreItem` into an existing stack.
    Merge {
        target_guid: ObjectGuid,
        added: u32,
        new_count: u32,
    },
    /// The traded item itself is stored with `count` at an empty position.
    Store { bag: u8, slot: u8, count: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeStorePlanLikeCpp {
    pub item_guid: ObjectGuid,
    pub actions: Vec<TradeStoreActionLikeCpp>,
}

impl TradeStorePlanLikeCpp {
    /// The item merged completely and its own row goes away.
    #[must_use]
    pub fn fully_merged(&self) -> bool {
        !self
            .actions
            .iter()
            .any(|action| matches!(action, TradeStoreActionLikeCpp::Store { .. }))
    }
}

/// Catalog lookups the plan needs.
pub struct TradeStoreCatalogsLikeCpp<'a> {
    pub template: &'a dyn Fn(u32) -> Option<ItemStorageTemplate>,
    pub limit_category: &'a dyn Fn(u32) -> Option<ItemLimitCategoryTemplate>,
}

/// Plan C++ `moveItems` for `receiver`: `outgoing` leave first, `incoming`
/// are stored in order. `Err((result, entry))` is the first item that does
/// not fit, as `CanStoreItems` reports `BagResult` and `ItemID`.
pub fn plan_trade_receive_like_cpp(
    receiver: &Player,
    outgoing: &[ObjectGuid],
    incoming: &[Item],
    catalogs: &TradeStoreCatalogsLikeCpp<'_>,
) -> Result<Vec<TradeStorePlanLikeCpp>, (InventoryResult, u32)> {
    let mut simulated = receiver.clone();
    // `Player::CanStoreItem` addresses bags through the native bag registry.
    // A Player whose bags exist only as inventory-runtime records (the
    // production login shape) gets them registered on this detached copy,
    // as the inventory projection does for its own CanStoreItem.
    let runtime_bags: Vec<(u8, ObjectGuid, u32)> = simulated
        .inventory_runtime_like_cpp()
        .inventory_items()
        .iter()
        .filter(|(slot, _)| is_represented_bag_slot(**slot))
        .map(|(slot, record)| (*slot, record.guid, record.entry_id))
        .collect();
    for (slot, bag_guid, entry) in runtime_bags {
        let registered = simulated
            .inventory()
            .bags
            .get(usize::from(slot))
            .is_some_and(Option::is_some);
        let size = (catalogs.template)(entry).map_or(0, |template| template.container_slots);
        if !registered && size > 0 {
            let _ = simulated.register_bag_storage(slot, bag_guid, size);
        }
    }
    for guid in outgoing {
        let _ = simulated.trade_take_item_like_cpp(*guid);
    }
    let receiver_guid = receiver.guid();
    let mut plans = Vec::with_capacity(incoming.len());
    for item in incoming {
        let entry = item.object().entry();
        let proto = (catalogs.template)(entry);
        let limit_category = proto
            .as_ref()
            .and_then(|proto| (catalogs.limit_category)(proto.item_limit_category));
        let mut dest = Vec::new();
        let outcome = {
            let views = StorageViewsLikeCpp::of(&simulated, catalogs);
            let slot_items = views.slot_items();
            let stored_items = views.stored_items();
            let bag_templates = views.bag_templates();
            simulated.can_store_item(
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
                        .is_soulbound_trade_allowed_for(receiver_guid),
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
        let plan = apply_destinations_like_cpp(&mut simulated, item, &dest)
            .ok_or((InventoryResult::InternalBagError, entry))?;
        plans.push(plan);
    }
    Ok(plans)
}

fn apply_destinations_like_cpp(
    simulated: &mut Player,
    item: &Item,
    dest: &[ItemPosCount],
) -> Option<TradeStorePlanLikeCpp> {
    let mut actions = Vec::with_capacity(dest.len());
    let mut stored = false;
    for position in dest {
        let bag = (position.pos >> 8) as u8;
        let slot = (position.pos & 0xFF) as u8;
        if let Some(existing) = simulated.trade_item_at_like_cpp(bag, slot) {
            let target_guid = existing.object().guid();
            let new_count = existing.count().checked_add(position.count)?;
            simulated.trade_set_stack_count_like_cpp(target_guid, new_count);
            actions.push(TradeStoreActionLikeCpp::Merge {
                target_guid,
                added: position.count,
                new_count,
            });
        } else {
            // `CanStoreItem` gives one source item at most one empty
            // position: a stack never exceeds its maximum.
            if stored {
                return None;
            }
            stored = true;
            let mut placed = item.clone();
            placed.set_count(position.count);
            if !simulated.trade_store_item_like_cpp(placed, 0, None, bag, slot) {
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

/// The `CanStoreItem` views of one Player, built the way the session's
/// direct-inventory planner builds them from its canonical owner.
struct StorageViewsLikeCpp<'p> {
    positioned: Vec<(u8, u8, &'p Item)>,
    templates: std::collections::HashMap<u32, ItemStorageTemplate>,
    bags: Vec<(u8, u32)>,
}

impl<'p> StorageViewsLikeCpp<'p> {
    fn of(player: &'p Player, catalogs: &TradeStoreCatalogsLikeCpp<'_>) -> Self {
        let runtime = player.inventory_runtime_like_cpp();
        let objects = runtime.item_objects();
        let mut positioned = Vec::new();
        let mut bags = Vec::new();
        let mut templates = std::collections::HashMap::new();
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

#[cfg(test)]
#[path = "../unit_tests/trade_storage_tests.rs"]
mod tests;
