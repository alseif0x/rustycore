// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure vendor admission, catalog, price, stock, and cost rules.

use wow_constants::BuyResult;
use wow_constants::{
    InventoryResult, ItemBondingType, ItemExtendedCostFlags, ItemFieldFlags, ItemFlags, ItemFlags2,
    Team,
};
use wow_core::ObjectGuid;
use wow_data::{
    CurrencyTypesStore, ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
    is_player_meeting_condition_like_cpp,
};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, MAX_BAG_SIZE, MAX_MONEY_AMOUNT, NULL_BAG, NULL_SLOT, is_equipment_pos,
    is_inventory_pos,
};
use wow_packet::packets::misc::BuyItem;

use super::super::{player_class_mask, player_team_for_race_cpp};

const MAX_VENDOR_ITEMS_CPP: usize = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) struct VendorBuyItem {
    pub(in crate::handlers::character) item_id: u32,
    pub(in crate::handlers::character) item_type: i32,
    pub(in crate::handlers::character) max_count: u32,
    pub(in crate::handlers::character) incr_time: u32,
    pub(in crate::handlers::character) player_condition_id: u32,
    pub(in crate::handlers::character) has_vendor_conditions: bool,
    pub(in crate::handlers::character) extended_cost: u32,
    pub(in crate::handlers::character) buy_price: u64,
    pub(in crate::handlers::character) max_durability: u32,
    pub(in crate::handlers::character) buy_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) enum VendorBuyTemplateBlock {
    BuyError(BuyResult),
    Silent,
}

pub(in crate::handlers::character) fn vendor_buy_quantity_and_price(
    buy_price: u64,
    buy_count: u32,
    quantity: u32,
) -> (u32, u64) {
    if buy_price == 0 || quantity == 0 {
        return (quantity, 0);
    }

    let buy_price_per_item = buy_price as f64 / buy_count.max(1) as f64;
    let max_count = (MAX_MONEY_AMOUNT as f64 / buy_price_per_item) as u32;
    let quantity = quantity.min(max_count);
    let price = ((buy_price_per_item * quantity as f64) as u64).max(1);

    (quantity, price)
}

pub(in crate::handlers::character) fn vendor_buy_coinage_update_like_cpp(
    buy_price: u64,
    remaining_gold: u64,
) -> Option<u64> {
    // C++ `_StoreOrEquipNewItem` calls `ModifyMoney(-price)`, whose first
    // branch returns without dirtying `ActivePlayerData::Coinage` when the
    // amount is zero (`Player.cpp::ModifyMoney`).
    (buy_price != 0).then_some(remaining_gold)
}

pub(in crate::handlers::character) fn vendor_stored_new_item_flags_like_cpp(
    template: Option<&wow_entities::ItemStorageTemplate>,
    bag: u8,
    slot: u8,
) -> u32 {
    // C++ `StoreNewItem` marks the object new before `_StoreItem`, then the
    // store path applies the template bonding rule at its destination.
    let mut item = wow_entities::Item::new(0);
    if let Some(template) = template {
        item.set_bonding(template.bonding);
    }
    item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
        bag, slot,
    )));
    item.item_flags_bits()
}

pub(in crate::handlers::character) fn vendor_buy_packet_quantity_to_cpp_count(
    quantity: i32,
) -> u32 {
    u32::from((quantity as u8).max(1))
}

pub(in crate::handlers::character) fn vendor_buy_currency_packet_quantity_to_cpp_count(
    quantity: i32,
) -> u32 {
    (quantity as u32).max(1)
}

pub(in crate::handlers::character) fn vendor_list_reaches_cpp_item_limit(count: usize) -> bool {
    count >= MAX_VENDOR_ITEMS_CPP
}

pub(in crate::handlers::character) fn vendor_list_should_skip_currency_row(
    currency_store: Option<&CurrencyTypesStore>,
    item_id: i32,
    extended_cost: i32,
) -> bool {
    if extended_cost == 0 {
        return true;
    }

    !vendor_currency_type_is_known(currency_store, item_id as u32)
}

pub(in crate::handlers::character) fn vendor_currency_type_is_known(
    currency_store: Option<&CurrencyTypesStore>,
    currency_id: u32,
) -> bool {
    currency_store.is_some_and(|store| store.has_record(currency_id))
}

pub(in crate::handlers::character) fn vendor_buy_currency_quantity_block_result(
    max_count: u32,
    quantity: u32,
) -> Option<InventoryResult> {
    if max_count == 0 || quantity % max_count != 0 {
        Some(InventoryResult::CantBuyQuantity)
    } else {
        None
    }
}

pub(in crate::handlers::character) fn vendor_buy_muid_to_cpp_slot(muid: i32) -> Option<u32> {
    let muid = muid as u32;
    if muid > 0 { Some(muid - 1) } else { None }
}

pub(in crate::handlers::character) fn vendor_player_condition_failed_id_like_cpp(
    player_condition_id: u32,
    store: Option<&PlayerConditionStore>,
    context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> i32 {
    if player_condition_id == 0 {
        return 0;
    }

    let (Some(store), Some(context)) = (store, context) else {
        return player_condition_id as i32;
    };

    let Some(condition) = store.get(player_condition_id) else {
        return 0;
    };

    if is_player_meeting_condition_like_cpp(condition, &context) {
        0
    } else {
        player_condition_id as i32
    }
}

pub(in crate::handlers::character) fn vendor_buy_player_condition_block_result_like_cpp(
    player_condition_id: u32,
    store: Option<&PlayerConditionStore>,
    context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> Option<InventoryResult> {
    if vendor_player_condition_failed_id_like_cpp(player_condition_id, store, context) == 0 {
        None
    } else {
        Some(InventoryResult::ItemLocked)
    }
}

pub(in crate::handlers::character) fn vendor_conditions_block_result(
    has_vendor_conditions: bool,
) -> Option<BuyResult> {
    if has_vendor_conditions {
        Some(BuyResult::CantFindItem)
    } else {
        None
    }
}

pub(in crate::handlers::character) fn vendor_buy_required_reputation_block_result(
    required_reputation_faction: Option<u16>,
    required_reputation_rank: Option<i32>,
    player_reputation_rank: i32,
) -> Option<BuyResult> {
    if required_reputation_faction.unwrap_or(0) != 0
        && player_reputation_rank < required_reputation_rank.unwrap_or(0)
    {
        Some(BuyResult::ReputationRequire)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) enum VendorExtendedCostBlock {
    Equip(InventoryResult),
    Buy(BuyResult),
    Silent,
}

pub(in crate::handlers::character) fn vendor_buy_extended_cost_block_result(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    currency_store: Option<&CurrencyTypesStore>,
    has_item_count: impl Fn(u32, u32) -> bool,
    has_currency: impl Fn(u32, u32) -> bool,
    allow_currency_only_success: bool,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Option<VendorExtendedCostBlock> {
    if extended_cost == 0 {
        return None;
    }

    if quantity % buy_count.max(1) != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantBuyQuantity,
        ));
    }

    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Some(VendorExtendedCostBlock::Silent);
    };
    let stacks = quantity / buy_count.max(1);

    for (item_id, item_count) in extended_cost_entry
        .item_id
        .iter()
        .copied()
        .zip(extended_cost_entry.item_count.iter().copied())
    {
        if item_id == 0 {
            continue;
        }

        let Ok(item_id) = u32::try_from(item_id) else {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        };
        let amount = u32::from(item_count).wrapping_mul(stacks);
        if !has_item_count(item_id, amount) {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        }
    }

    for (i, currency_id) in extended_cost_entry.currency_id.iter().copied().enumerate() {
        if currency_id == 0 {
            continue;
        }

        let currency_id = u32::from(currency_id);
        if !vendor_currency_type_is_known(currency_store, currency_id) {
            return Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem));
        }

        if item_extended_cost_currency_requires_season_earned(extended_cost_entry.flags, i)
            || !has_currency(
                currency_id,
                extended_cost_entry.currency_count[i].wrapping_mul(stacks),
            )
        {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        }
    }

    if extended_cost_entry.required_arena_rating != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantEquipRank,
        ));
    }

    if extended_cost_entry.min_faction_id != 0 {
        return Some(VendorExtendedCostBlock::Buy(BuyResult::ReputationRequire));
    }

    if extended_cost_entry.requires_guild() || extended_cost_entry.required_achievement != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins,
        ));
    }

    if allow_currency_only_success {
        None
    } else {
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins,
        ))
    }
}

pub(in crate::handlers::character) fn vendor_buy_extended_cost_item_costs(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Vec<(u32, u32)> {
    if extended_cost == 0 {
        return Vec::new();
    }
    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Vec::new();
    };
    let stacks = quantity / buy_count.max(1);
    extended_cost_entry
        .item_id
        .iter()
        .copied()
        .zip(extended_cost_entry.item_count.iter().copied())
        .filter(|(item_id, _)| *item_id > 0)
        .map(|(item_id, count)| {
            (
                u32::try_from(item_id).unwrap_or(0),
                u32::from(count).wrapping_mul(stacks),
            )
        })
        .collect()
}

pub(in crate::handlers::character) fn vendor_buy_extended_cost_currency_costs(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Vec<(u32, u32)> {
    if extended_cost == 0 {
        return Vec::new();
    }
    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Vec::new();
    };
    let stacks = quantity / buy_count.max(1);
    extended_cost_entry
        .currency_id
        .iter()
        .copied()
        .zip(extended_cost_entry.currency_count.iter().copied())
        .filter(|(currency_id, _)| *currency_id != 0)
        .map(|(currency_id, count)| (u32::from(currency_id), count.wrapping_mul(stacks)))
        .collect()
}

fn item_extended_cost_currency_requires_season_earned(
    flags: ItemExtendedCostFlags,
    currency_index: usize,
) -> bool {
    match currency_index {
        0 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
        1 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
        2 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
        3 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
        4 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
        _ => false,
    }
}

pub(in crate::handlers::character) fn vendor_buy_direct_store_block_result(
    bag: u8,
    slot: u8,
    _quantity: u32,
) -> Option<InventoryResult> {
    if (bag == NULL_BAG && slot == NULL_SLOT) || is_inventory_pos(bag, slot) {
        return None;
    }

    if is_equipment_pos(bag, slot) {
        return Some(InventoryResult::NotEquippable);
    }

    Some(InventoryResult::WrongSlot)
}

pub(in crate::handlers::character) fn vendor_buy_stock_refill_count(
    current_count: u32,
    elapsed_secs: u64,
    incr_time: u32,
    buy_count: u32,
    max_count: u32,
) -> (u32, bool) {
    if max_count == 0 || current_count >= max_count || incr_time == 0 {
        // C++ assumes nonzero incrtime for finite stock; keep invalid DB rows from dividing by zero.
        return (current_count.min(max_count), current_count >= max_count);
    }

    let increments = elapsed_secs / u64::from(incr_time);
    if increments == 0 {
        return (current_count, false);
    }

    let restored = increments.saturating_mul(u64::from(buy_count.max(1)));
    let new_count = u64::from(current_count).saturating_add(restored);
    if new_count >= u64::from(max_count) {
        (max_count, true)
    } else {
        (new_count as u32, false)
    }
}

pub(in crate::handlers::character) fn vendor_list_should_skip_sold_out(
    max_count: i32,
    current_count: u32,
    is_game_master: bool,
) -> bool {
    max_count > 0 && current_count == 0 && !is_game_master
}

pub(in crate::handlers::character) fn vendor_list_item_refundable(
    item_flags: Option<ItemFlags>,
    max_stack_size: Option<u32>,
    extended_cost: i32,
) -> bool {
    extended_cost > 0
        && max_stack_size == Some(1)
        && item_flags.is_some_and(|flags| flags.contains(ItemFlags::ITEM_PURCHASE_RECORD))
}

pub(in crate::handlers::character) fn vendor_list_should_skip_allowed_class(
    allowable_class: Option<i16>,
    bonding: Option<u8>,
    player_class: u8,
    is_game_master: bool,
) -> bool {
    if is_game_master || bonding != Some(ItemBondingType::OnAcquire as u8) {
        return false;
    }

    let Some(allowable_class) = allowable_class else {
        return false;
    };
    (i32::from(allowable_class) & player_class_mask(player_class) as i32) == 0
}

pub(in crate::handlers::character) fn vendor_list_should_skip_faction_flags(
    flags2: Option<u32>,
    team: Team,
    is_game_master: bool,
) -> bool {
    if is_game_master {
        return false;
    }

    let Some(flags2) = flags2 else {
        return false;
    };
    ((flags2 & ItemFlags2::FactionHorde as u32) != 0 && team == Team::Alliance)
        || ((flags2 & ItemFlags2::FactionAlliance as u32) != 0 && team == Team::Horde)
}

pub(in crate::handlers::character) fn vendor_buy_template_block_result(
    allowable_class: Option<i16>,
    bonding: Option<u8>,
    flags2: Option<u32>,
    player_class: u8,
    player_race: u8,
    is_game_master: bool,
) -> Option<VendorBuyTemplateBlock> {
    if vendor_list_should_skip_allowed_class(allowable_class, bonding, player_class, is_game_master)
    {
        return Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem));
    }

    if vendor_list_should_skip_faction_flags(
        flags2,
        player_team_for_race_cpp(player_race),
        is_game_master,
    ) {
        return Some(VendorBuyTemplateBlock::Silent);
    }

    None
}

pub(in crate::handlers::character) fn vendor_buy_direct_inventory_destination(
    player_guid: ObjectGuid,
    buy: &BuyItem,
) -> Option<(u8, u8)> {
    let slot = buy.slot as u8;
    if slot as usize > MAX_BAG_SIZE && slot != NULL_SLOT {
        return None;
    }

    let bag = if buy.container_guid == player_guid {
        INVENTORY_SLOT_BAG_0
    } else {
        NULL_BAG
    };

    Some((bag, slot))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) enum LoadedItemRefundDecision {
    None,
    Valid {
        paid_money: u64,
        paid_extended_cost: u16,
    },
    Clear {
        new_flags: u32,
    },
}

pub(in crate::handlers::character) fn loaded_item_refund_decision(
    item_flags: u32,
    played_time: u32,
    paid_money: Option<u64>,
    paid_extended_cost: Option<u16>,
) -> LoadedItemRefundDecision {
    let flags = ItemFieldFlags::from_bits_retain(item_flags);
    if !flags.contains(ItemFieldFlags::REFUNDABLE) {
        return LoadedItemRefundDecision::None;
    }

    let new_flags = (flags & !ItemFieldFlags::REFUNDABLE).bits();
    if played_time > 2 * 60 * 60 {
        return LoadedItemRefundDecision::Clear { new_flags };
    }

    match (paid_money, paid_extended_cost) {
        (Some(paid_money), Some(paid_extended_cost)) => LoadedItemRefundDecision::Valid {
            paid_money,
            paid_extended_cost,
        },
        _ => LoadedItemRefundDecision::Clear { new_flags },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) enum SellItemAmountAction {
    Invalid,
    FullStack { amount: u32 },
    PartialStack { amount: u32, remaining: u32 },
}

pub(in crate::handlers::character) fn sell_item_amount_action(
    current_count: u32,
    requested_amount: i32,
) -> SellItemAmountAction {
    let amount = if requested_amount == 0 {
        current_count
    } else {
        let Ok(amount) = u32::try_from(requested_amount) else {
            return SellItemAmountAction::Invalid;
        };
        amount
    };

    if amount == 0 || amount > current_count {
        return SellItemAmountAction::Invalid;
    }

    if amount < current_count {
        SellItemAmountAction::PartialStack {
            amount,
            remaining: current_count - amount,
        }
    } else {
        SellItemAmountAction::FullStack { amount }
    }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
