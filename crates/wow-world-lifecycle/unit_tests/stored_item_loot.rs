use wow_constants::BagFamilyMask;
use wow_core::ObjectGuid;
use wow_packet::packets::loot::{LootEntry, LootEntryFlags};

use super::{
    stored_item_row_can_load_like_cpp_representable, stored_loot_item_should_persist_like_cpp,
};

#[test]
fn open_item_stored_loot_preserves_random_properties_and_context_like_cpp() {
    assert!(stored_item_row_can_load_like_cpp_representable(
        25, 2, 7, false, false, -77, 456, 2, true
    ));
    assert!(stored_item_row_can_load_like_cpp_representable(
        25, 2, 7, false, true, -77, 456, 2, true
    ));

    let entry = LootEntry {
        loot_list_id: 7,
        item_id: 25,
        quantity: 2,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    let response_item = wow_packet::packets::loot::LootItemData {
        item_type: 0,
        ui_type: entry.free_for_all_ui_type_like_cpp(),
        can_trade_to_tap_list: false,
        loot: wow_packet::packets::item::ItemInstance {
            item_id: entry.item_id as i32,
            ..wow_packet::packets::item::ItemInstance::default()
        },
        loot_list_id: entry.loot_list_id,
        quantity: entry.quantity,
        loot_item_type: 0,
    };
    assert_eq!(entry.random_properties_id, -77);
    assert_eq!(entry.random_properties_seed, 456);
    assert_eq!(entry.item_context, 2);
    assert_eq!(response_item.loot.item_id, 25);
    assert!(response_item.loot.item_bonus.is_none());

    assert!(!stored_item_row_can_load_like_cpp_representable(
        25, 2, 256, false, false, 0, 0, 0, true
    ));
    assert!(!stored_item_row_can_load_like_cpp_representable(
        25, 2, 7, true, false, 0, 0, 0, true
    ));
}

#[test]
fn stored_loot_item_persistence_skips_missing_template_and_currency_tokens_like_cpp() {
    // template missing -> no persist
    assert!(!stored_loot_item_should_persist_like_cpp(
        false,
        BagFamilyMask::NONE
    ));

    // normal template -> persist
    assert!(stored_loot_item_should_persist_like_cpp(
        true,
        BagFamilyMask::NONE
    ));

    // currency token -> no persist (C++ ItemTemplate::IsCurrencyToken)
    assert!(!stored_loot_item_should_persist_like_cpp(
        true,
        BagFamilyMask::CURRENCY_TOKENS
    ));

    // currency token combined with other families still no persist
    assert!(!stored_loot_item_should_persist_like_cpp(
        true,
        BagFamilyMask::CURRENCY_TOKENS | BagFamilyMask::HERBS
    ));
}
