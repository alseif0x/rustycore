// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor sale transaction, buyback creation and publication.

use tracing::{debug, info, warn};
use wow_constants::SellResult;
use wow_core::ObjectGuid;
use wow_entities::PlayerInventoryItem as InventoryItem;
use wow_packet::packets::misc::SellItem;
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate};

use super::{
    VendorSessionHostLikeCpp, item_is_currently_looted_like_cpp, item_is_not_empty_bag_like_cpp,
    player_money_gain_like_cpp,
};
use crate::item_spell_charges_db_string;
use crate::vendor::{SellItemAmountAction, sell_item_amount_action};

/// Handle CMSG_SELL_ITEM — player sells an item to a vendor.
///
/// C++ ref: `HandleSellItemOpcode` (`Handlers/ItemHandler.cpp:365+`).
pub async fn handle_sell_item_with_generator_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    sell: SellItem,
) {
    use wow_packet::packets::update::UpdateObject;

    debug!(
        "SellItem: item={:?} from account {}",
        sell.item_guid,
        host.core().account_id
    );

    let player_guid = match host.player_guid() {
        Some(g) => g,
        None => return,
    };
    let map_id = host.core().player_map_id_like_cpp();

    // ── Find item in inventory by GUID ──
    let Some(inventory_items) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_items_like_cpp(hub)
    }) else {
        return;
    };
    let (slot, item) = match inventory_items
        .iter()
        .find(|(_, item)| item.guid == sell.item_guid)
        .map(|(&s, item)| (s, item.clone()))
    {
        Some(pair) => pair,
        None => {
            warn!("SellItem: item {:?} not in inventory", sell.item_guid);
            host.core().send_sell_error(
                SellResult::YouDontOwnThatItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }
    };

    // Equipped items (slots 0-18) can't be sold without unequipping first
    if slot < 19 {
        host.core().send_sell_error(
            SellResult::CantSellItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    }

    let vendor_trade_port = match host.lifecycle().vendor_trade_persistence_port_like_cpp() {
        Some(port) => port,
        None => return,
    };

    let Some(runtime_item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item.guid)
    }) else {
        host.core().send_sell_error(
            SellResult::CantFindItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    };
    let item_inventory_type = host
        .hub_ref()
        .catalogs
        .item_storage_template(item.entry_id)
        .map(|template| template.inventory_type);
    if item_is_not_empty_bag_like_cpp(item_inventory_type, {
        let (state, hub) = host.split_inventory_ref();
        state.direct_item_contains_items(hub, item.guid)
    }) {
        host.core().send_sell_error(
            SellResult::CantSellItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    }
    if host.is_active_loot_guid(item.guid) || item_is_currently_looted_like_cpp(&runtime_item) {
        host.core().send_sell_error(
            SellResult::CantSellItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    }
    if runtime_item.is_refundable() {
        return;
    }
    let sell_amount = match sell_item_amount_action(runtime_item.count(), sell.amount) {
        SellItemAmountAction::Invalid => {
            host.core().send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }
        action => action,
    };
    let sold_count = match sell_amount {
        SellItemAmountAction::FullStack { amount }
        | SellItemAmountAction::PartialStack { amount, .. } => amount,
        SellItemAmountAction::Invalid => unreachable!(),
    };

    // ── Get sell price from item_sparse directly ──
    let sell_price: u64 = {
        let port = match host.lifecycle().vendor_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        match port.load_item_sell_price_like_cpp(item.entry_id).await {
            wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(price) => price,
            _ => 0,
        }
    };
    if sell_price == 0 {
        host.core().send_sell_error(
            SellResult::CantSellItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    }

    let money = sell_price.saturating_mul(u64::from(sold_count));
    let Some(money_persistence) = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
    else {
        return;
    };
    let Some(old_gold) = host.resolved_player_money_like_cpp() else {
        return;
    };
    let Some(new_gold) = player_money_gain_like_cpp(old_gold, money) else {
        host.core().send_sell_error(
            SellResult::CantSellItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    };
    let Some(buyback_slot) = ({
        let (s, h) = host.split_inventory_ref();
        s.select_buyback_slot_cpp(h)
    }) else {
        return;
    };
    let Some(buyback_items) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_buyback_items_like_cpp(hub)
    }) else {
        return;
    };
    let old_buyback = buyback_items.get(&buyback_slot).cloned();
    let buyback_price = sell_price
        .saturating_mul(u64::from(sold_count))
        .min(u64::from(u32::MAX)) as u32;
    let buyback_timestamp = host
        .lifecycle()
        .login_time_like_cpp()
        .map(|login_time| login_time.elapsed().as_secs())
        .unwrap_or(0)
        .saturating_add(30 * 3600)
        .min(u64::from(u32::MAX)) as i64;

    let mut new_buyback_stack = None;
    let persistence_sold_item = match sell_amount {
        SellItemAmountAction::FullStack { .. } => {
            wow_persistence::VendorSaleItemPersistenceLikeCpp::FullStack {
                item_guid: item.db_guid,
                buyback_slot,
            }
        }
        SellItemAmountAction::PartialStack { remaining, amount } => {
            let Some((new_db_guid, new_item_guid)) = host
                .core()
                .allocate_item_instance_guids_with_generator_like_cpp(item_guid_generator, 1)
                .and_then(|mut allocated| allocated.pop())
            else {
                warn!("SellItem: process-wide item GUID allocator is unavailable");
                host.core().send_sell_error(
                    SellResult::CantSellItem,
                    Some(sell.vendor_guid),
                    sell.item_guid,
                );
                return;
            };
            let cloned_item =
                runtime_item.clone_item_for_store(new_item_guid, Some(player_guid), amount);
            let cloned_data = cloned_item.data();
            let charges = item_spell_charges_db_string(
                &cloned_data.spell_charges,
                host.hub_ref()
                    .catalogs
                    .item_effect_count_like_cpp(item.entry_id),
            );
            let Some((enchantments, _)) = ({
                let (state, hub) = host.split_inventory_ref();
                state.inventory_remove_enchantment_persistence_like_cpp(hub, item.guid, false)
            }) else {
                host.core().send_sell_error(
                    SellResult::CantSellItem,
                    Some(sell.vendor_guid),
                    sell.item_guid,
                );
                return;
            };

            let persistence = wow_persistence::VendorSaleItemPersistenceLikeCpp::PartialStack {
                source_item_guid: item.db_guid,
                source_count_after: remaining,
                sold_clone: wow_persistence::VendorSoldClonePersistenceLikeCpp {
                    item_guid: new_db_guid,
                    item_entry: item.entry_id,
                    owner_guid: player_guid.counter() as u64,
                    creator_guid: cloned_data.creator.counter() as u64,
                    gift_creator_guid: cloned_data.gift_creator.counter() as u64,
                    count: cloned_item.count(),
                    expiration: cloned_data.expiration,
                    charges,
                    enchantments,
                    flags: cloned_data.dynamic_flags,
                    durability: cloned_data.durability,
                    create_played_time: cloned_data.create_played_time,
                    random_properties_id: cloned_data.random_properties_id,
                    property_seed: cloned_data.property_seed,
                    context: u8::try_from(cloned_data.context).unwrap_or(0),
                    buyback_slot,
                },
            };

            new_buyback_stack = Some((new_db_guid, cloned_item, remaining));
            persistence
        }
        SellItemAmountAction::Invalid => unreachable!(),
    };

    let persistence_request = wow_persistence::VendorTradePersistenceRequestLikeCpp::Sale(
        wow_persistence::VendorSalePersistenceLikeCpp {
            player_guid: player_guid.counter() as u64,
            money_before: old_gold,
            money_after: new_gold,
            evicted_buyback_item_guid: old_buyback.as_ref().map(|item| item.db_guid),
            sold_item: persistence_sold_item,
        },
    );
    let Some(money_persistence) = host
        .await_exclusive_player_money_transaction_outcome_like_cpp(
            money_persistence,
            vendor_trade_port.persist_vendor_trade_like_cpp(persistence_request),
            old_gold,
            new_gold,
            "vendor item sale",
        )
        .await
    else {
        warn!("SellItem: transaction did not commit");
        host.core().send_sell_error(
            SellResult::CantSellItem,
            Some(sell.vendor_guid),
            sell.item_guid,
        );
        return;
    };

    // C++ mutates money, inventory and buyback state as one in-memory
    // operation. Our durable-first adaptation must publish the same whole
    // state before reopening admission or reaching a cancellation point.
    if !host.stage_player_money_change_like_cpp(old_gold, new_gold) {
        host.core_mut()
            .kick("canonical Player money owner became unavailable after vendor sale COMMIT");
        return;
    }
    if let Some(old_buyback) = old_buyback {
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.remove_buyback_item_like_cpp(&mut hub, buyback_slot)
        };
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.remove_inventory_item_object(&mut hub, old_buyback.guid)
        };
    }
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.set_buyback_slot_metadata_like_cpp(
            &mut hub,
            buyback_slot,
            buyback_price,
            buyback_timestamp,
        )
    };
    {
        let (s, mut h) = host.split_inventory_mut();
        s.advance_buyback_slot_cpp(&mut h)
    };

    let mut created_buyback_item = None;
    let mut stack_update = None;
    if let Some((new_db_guid, cloned_item, remaining)) = new_buyback_stack {
        let new_item_guid = cloned_item.object().guid();
        let stack_count = cloned_item.count();
        let durability = cloned_item.data().durability;
        let max_durability = cloned_item.data().max_durability;
        let _ = {
            let (state, mut hub) = host.split_inventory_mut();
            state.apply_inventory_item_object_updates_like_cpp(
                &mut hub,
                item.guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(remaining)],
            )
        };
        stack_update = Some((item.guid, remaining));
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.insert_buyback_item_like_cpp(
                &mut hub,
                buyback_slot,
                InventoryItem {
                    guid: new_item_guid,
                    entry_id: item.entry_id,
                    db_guid: new_db_guid,
                    inventory_type: item.inventory_type,
                },
            )
        };
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.insert_inventory_item_object(&mut hub, cloned_item)
        };
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.set_inventory_item_object_slot(&mut hub, new_item_guid, buyback_slot)
        };
        created_buyback_item = Some((new_item_guid, stack_count, durability, max_durability));
    } else {
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.remove_inventory_item_like_cpp(&mut hub, slot)
        };
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.insert_buyback_item_like_cpp(
                &mut hub,
                buyback_slot,
                InventoryItem {
                    guid: item.guid,
                    entry_id: item.entry_id,
                    db_guid: item.db_guid,
                    inventory_type: item.inventory_type,
                },
            )
        };
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.set_inventory_item_object_slot(&mut hub, item.guid, buyback_slot)
        };
    }
    drop(money_persistence);

    host.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;

    info!(
        "SellItem: player {:?} sold {}x item {} from slot {} for {} copper (total: {})",
        player_guid, sold_count, item.entry_id, slot, money, new_gold
    );

    if let Some((item_guid, stack_count, durability, max_durability)) = created_buyback_item {
        host.send_packet(&UpdateObject::create_items(
            vec![ItemCreateData {
                item_guid,
                entry_id: item.entry_id as i32,
                owner_guid: player_guid,
                contained_in: player_guid,
                stack_count,
                dynamic_flags: 0,
                durability,
                max_durability,
                random_properties_seed: 0,
                random_properties_id: 0,
                enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                gems: Vec::new(),
                context: 0,
                container_slots: 0,
                container_item_guids: [ObjectGuid::EMPTY; 36],
            }],
            map_id,
        ));
    }
    if let Some((item_guid, new_count)) = stack_update {
        host.send_packet(&UpdateObject::item_stack_count_update(
            item_guid, map_id, new_count,
        ));
    }

    let mut inv_slot_changes = Vec::new();
    if matches!(sell_amount, SellItemAmountAction::FullStack { .. }) {
        inv_slot_changes.push((slot, ObjectGuid::EMPTY));
    }
    let buyback_guid = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_buyback_items_like_cpp(hub)
    })
    .and_then(|items| items.get(&buyback_slot).cloned())
    .map(|item| item.guid)
    .unwrap_or(ObjectGuid::EMPTY);
    inv_slot_changes.push((buyback_slot, buyback_guid));
    host.send_player_values_update_from_entity_bridge(
        &inv_slot_changes,
        &[],
        &[],
        &[(buyback_slot, buyback_price, buyback_timestamp)],
        Some(new_gold),
    );
}
