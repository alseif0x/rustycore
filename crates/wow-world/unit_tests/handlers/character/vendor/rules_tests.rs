//! Unit tests for pure vendor admission, listing, pricing, and purchase rules.

use super::{
    SellItemAmountAction, VendorBuyTemplateBlock, VendorExtendedCostBlock, sell_item_amount_action,
    vendor_buy_coinage_update_like_cpp, vendor_buy_currency_packet_quantity_to_cpp_count,
    vendor_buy_currency_quantity_block_result, vendor_buy_direct_inventory_destination,
    vendor_buy_direct_store_block_result, vendor_buy_extended_cost_block_result,
    vendor_buy_extended_cost_currency_costs, vendor_buy_extended_cost_item_costs,
    vendor_buy_muid_to_cpp_slot, vendor_buy_packet_quantity_to_cpp_count,
    vendor_buy_player_condition_block_result_like_cpp, vendor_buy_quantity_and_price,
    vendor_buy_required_reputation_block_result, vendor_buy_stock_refill_count,
    vendor_buy_template_block_result, vendor_conditions_block_result, vendor_list_item_refundable,
    vendor_list_reaches_cpp_item_limit, vendor_list_should_skip_allowed_class,
    vendor_list_should_skip_currency_row, vendor_list_should_skip_faction_flags,
    vendor_list_should_skip_sold_out, vendor_player_condition_failed_id_like_cpp,
    vendor_stored_new_item_flags_like_cpp,
};
use wow_constants::BuyResult;
use wow_constants::{
    InventoryResult, ItemBondingType, ItemExtendedCostFlags, ItemFieldFlags, ItemFlags, ItemFlags2,
    Team,
};
use wow_core::ObjectGuid;
use wow_data::{
    CurrencyTypesStore, ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
};
use wow_entities::{INVENTORY_SLOT_BAG_0, MAX_BAG_SIZE, NULL_BAG, NULL_SLOT};
use wow_packet::packets::misc::BuyItem;

use super::super::super::player_team_for_race_cpp;

#[test]
fn vendor_buy_price_uses_cpp_buy_count_unit_price() {
    assert_eq!(vendor_buy_quantity_and_price(500, 5, 1), (1, 100));
    assert_eq!(vendor_buy_quantity_and_price(500, 5, 3), (3, 300));
    assert_eq!(vendor_buy_quantity_and_price(500, 0, 2), (2, 1000));
    assert_eq!(vendor_buy_quantity_and_price(0, 5, 3), (3, 0));
    assert_eq!(vendor_buy_quantity_and_price(1, 5, 1), (1, 1));
}

#[test]
fn vendor_buy_price_clamps_count_to_cpp_max_money_amount() {
    let unit_price = (wow_entities::MAX_MONEY_AMOUNT / 2) + 1;

    assert_eq!(
        vendor_buy_quantity_and_price(unit_price, 1, 3),
        (1, unit_price)
    );
}

#[test]
fn vendor_buy_zero_gold_price_does_not_dirty_coinage_like_cpp() {
    assert_eq!(vendor_buy_coinage_update_like_cpp(0, 12_345), None);
    assert_eq!(vendor_buy_coinage_update_like_cpp(1, 12_344), Some(12_344));
}

#[test]
fn vendor_buy_packet_quantity_uses_cpp_uint8_count_conversion() {
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(0), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(1), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(256), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(-1), 255);
}

#[test]
fn vendor_buy_currency_preflight_matches_cpp_quantity_guards() {
    assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(0), 1);
    assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(5), 5);
    assert_eq!(
        vendor_buy_currency_quantity_block_result(5, 3),
        Some(InventoryResult::CantBuyQuantity)
    );
    assert_eq!(vendor_buy_currency_quantity_block_result(5, 10), None);
    assert_eq!(
        vendor_buy_currency_quantity_block_result(0, 10),
        Some(InventoryResult::CantBuyQuantity)
    );
}

#[test]
fn vendor_buy_muid_uses_cpp_one_based_uint32_slot_conversion() {
    assert_eq!(vendor_buy_muid_to_cpp_slot(0), None);
    assert_eq!(vendor_buy_muid_to_cpp_slot(1), Some(0));
    assert_eq!(vendor_buy_muid_to_cpp_slot(2), Some(1));
    assert_eq!(vendor_buy_muid_to_cpp_slot(-1), Some(u32::MAX - 1));
}

#[test]
fn vendor_list_currency_rows_match_cpp_basic_guards() {
    let store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
        id: 395,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }]);
    assert!(vendor_list_should_skip_currency_row(Some(&store), 395, 0));
    assert!(!vendor_list_should_skip_currency_row(Some(&store), 395, 10));
    assert!(vendor_list_should_skip_currency_row(
        Some(&store),
        999_999,
        10
    ));
    assert!(vendor_list_should_skip_currency_row(None, 395, 10));
}

#[test]
fn vendor_player_condition_id_evaluates_player_condition_store_like_cpp() {
    let store = PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 0,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ]);
    let context = PlayerConditionContextLikeCpp {
        class_mask: 1,
        ..Default::default()
    };

    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(0, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(42, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(43, Some(&store), Some(context)),
        43
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(999, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(42, Some(&store), Some(context)),
        None
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(43, Some(&store), Some(context)),
        Some(InventoryResult::ItemLocked)
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(42, None, Some(context)),
        Some(InventoryResult::ItemLocked)
    );
}

#[test]
fn vendor_condition_presence_fails_closed_until_condition_mgr_exists() {
    assert_eq!(vendor_conditions_block_result(false), None);
    assert_eq!(
        vendor_conditions_block_result(true),
        Some(BuyResult::CantFindItem)
    );
}

#[test]
fn vendor_buy_extended_cost_fails_closed_like_cpp_preflight() {
    let currency_store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
        id: 395,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }]);
    let extended_cost_store =
        ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
            id: 12,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            item_count: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            currency_id: [395, 0, 0, 0, 0],
            currency_count: [10, 0, 0, 0, 0],
        }]);

    assert_eq!(
        vendor_buy_extended_cost_block_result(
            None,
            None,
            |_, _| false,
            |_, _| false,
            false,
            0,
            5,
            3
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| false,
            |_, _| false,
            false,
            12,
            5,
            3
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantBuyQuantity
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| currency_id == 395 && amount >= 20,
            false,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| currency_id == 395 && amount >= 20,
            true,
            12,
            5,
            10
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_currency_costs(Some(&extended_cost_store), 12, 5, 10),
        vec![(395, 20)]
    );
    let item_turnin_store =
        ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
            id: 13,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [700, 0, 0, 0, 0],
            item_count: [3, 0, 0, 0, 0],
            currency_id: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
            currency_count: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
        }]);
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&item_turnin_store),
            Some(&currency_store),
            |item_id, amount| item_id == 700 && amount == 6,
            |_, _| true,
            true,
            13,
            5,
            10
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&item_turnin_store),
            Some(&currency_store),
            |_, _| false,
            |_, _| true,
            true,
            13,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_item_costs(Some(&item_turnin_store), 13, 5, 10),
        vec![(700, 6)]
    );
    let checked_currency_amount = std::cell::Cell::new(false);
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| {
                checked_currency_amount.set(true);
                assert_eq!(currency_id, 395);
                assert_eq!(amount, 20);
                false
            },
            true,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert!(checked_currency_amount.get());
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            None,
            |_, _| true,
            |_, _| true,
            true,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |_, _| true,
            true,
            99,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Silent)
    );
}

#[test]
fn vendor_buy_direct_store_preflight_matches_cpp_store_branch() {
    assert_eq!(
        vendor_buy_direct_store_block_result(NULL_BAG, NULL_SLOT, 1),
        None
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 35, 1),
        None
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(NULL_BAG, 35, 1),
        Some(InventoryResult::WrongSlot)
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 0, 1),
        Some(InventoryResult::NotEquippable)
    );
}

#[test]
fn vendor_buy_stock_refill_matches_cpp_increment_and_full_reset() {
    assert_eq!(vendor_buy_stock_refill_count(2, 20, 10, 5, 20), (12, false));
    assert_eq!(vendor_buy_stock_refill_count(18, 10, 10, 5, 20), (20, true));
    assert_eq!(vendor_buy_stock_refill_count(2, 9, 10, 5, 20), (2, false));
}

#[test]
fn vendor_stored_new_item_keeps_cpp_new_and_bonding_flags() {
    assert_eq!(
        vendor_stored_new_item_flags_like_cpp(None, INVENTORY_SLOT_BAG_0, 23),
        ItemFieldFlags::NEW_ITEM.bits()
    );

    let mut template = wow_entities::ItemStorageTemplate::regular_item(700, 1);
    template.bonding = ItemBondingType::OnAcquire;
    assert_eq!(
        vendor_stored_new_item_flags_like_cpp(Some(&template), INVENTORY_SLOT_BAG_0, 23),
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );
}

#[test]
fn vendor_list_item_limit_matches_cpp_cap() {
    assert!(!vendor_list_reaches_cpp_item_limit(149));
    assert!(vendor_list_reaches_cpp_item_limit(150));
    assert!(vendor_list_reaches_cpp_item_limit(151));
}

#[test]
fn sell_item_amount_action_matches_cpp_amount_branch() {
    assert_eq!(
        sell_item_amount_action(5, 0),
        SellItemAmountAction::FullStack { amount: 5 }
    );
    assert_eq!(
        sell_item_amount_action(5, 5),
        SellItemAmountAction::FullStack { amount: 5 }
    );
    assert_eq!(
        sell_item_amount_action(5, 2),
        SellItemAmountAction::PartialStack {
            amount: 2,
            remaining: 3
        }
    );
    assert_eq!(sell_item_amount_action(5, 6), SellItemAmountAction::Invalid);
    assert_eq!(
        sell_item_amount_action(5, -1),
        SellItemAmountAction::Invalid
    );
}

#[test]
fn vendor_list_sold_out_filter_matches_cpp_gm_branch() {
    assert!(vendor_list_should_skip_sold_out(5, 0, false));
    assert!(!vendor_list_should_skip_sold_out(5, 0, true));
    assert!(!vendor_list_should_skip_sold_out(5, 1, false));
    assert!(!vendor_list_should_skip_sold_out(0, 0, false));
}

#[test]
fn vendor_list_refundable_flag_matches_cpp_template_guard() {
    assert!(vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(1),
        42
    ));
    assert!(!vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(2),
        42
    ));
    assert!(!vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(1),
        0
    ));
    assert!(!vendor_list_item_refundable(None, Some(1), 42));
}

#[test]
fn vendor_list_allowed_class_filter_matches_cpp_bind_on_acquire_branch() {
    let warrior_mask = 1i16 << (1 - 1);
    let mage_mask = 1i16 << (8 - 1);

    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        1,
        false,
    ));
    assert!(vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnEquip as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        true,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask | mage_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(-1),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
}

#[test]
fn vendor_list_faction_filter_matches_cpp_team_branch() {
    assert_eq!(player_team_for_race_cpp(1), Team::Alliance);
    assert_eq!(player_team_for_race_cpp(2), Team::Horde);
    assert_eq!(player_team_for_race_cpp(11), Team::Alliance);
    assert_eq!(player_team_for_race_cpp(10), Team::Horde);

    assert!(vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionHorde as u32),
        Team::Alliance,
        false,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionHorde as u32),
        Team::Horde,
        false,
    ));
    assert!(vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionAlliance as u32),
        Team::Horde,
        false,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionAlliance as u32),
        Team::Horde,
        true,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        None,
        Team::Alliance,
        false
    ));
}

#[test]
fn vendor_buy_template_gates_match_cpp_error_shapes() {
    let warrior_mask = 1i16 << (1 - 1);

    assert_eq!(
        vendor_buy_template_block_result(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            None,
            8,
            1,
            false,
        ),
        Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem))
    );
    assert_eq!(
        vendor_buy_template_block_result(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            None,
            8,
            1,
            true,
        ),
        None
    );
    assert_eq!(
        vendor_buy_template_block_result(
            None,
            None,
            Some(ItemFlags2::FactionHorde as u32),
            1,
            1,
            false,
        ),
        Some(VendorBuyTemplateBlock::Silent)
    );
    assert_eq!(
        vendor_buy_template_block_result(
            None,
            None,
            Some(ItemFlags2::FactionHorde as u32),
            1,
            2,
            false,
        ),
        None
    );
}

#[test]
fn vendor_buy_destination_uses_cpp_uint8_slot_conversion() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: 256,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        Some((INVENTORY_SLOT_BAG_0, 0))
    );
}

#[test]
fn vendor_buy_destination_maps_player_container_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: 35,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        Some((INVENTORY_SLOT_BAG_0, 35))
    );
}

#[test]
fn vendor_buy_destination_rejects_cpp_slot_over_max_bag_size() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: (MAX_BAG_SIZE + 1) as i32,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        None
    );
}

#[test]
fn vendor_required_reputation_fails_closed_until_reputation_mgr_exists() {
    assert_eq!(
        vendor_buy_required_reputation_block_result(None, None, -1),
        None
    );
    assert_eq!(
        vendor_buy_required_reputation_block_result(Some(72), Some(5), -1),
        Some(BuyResult::ReputationRequire)
    );
    assert_eq!(
        vendor_buy_required_reputation_block_result(Some(72), Some(5), 5),
        None
    );
}
