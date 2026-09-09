//! Spell handler stored item scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[test]
fn open_item_wrapped_without_has_loot_uses_gift_row_like_cpp() {
    let item_guid = ObjectGuid::create_item(1, 900);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let gift_creator = ObjectGuid::create_player(1, 77);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner_guid),
        max_durability: 30,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.force_state(ItemUpdateState::Unchanged);
    item.set_gift_creator(gift_creator);
    item.replace_all_item_flags(ItemFieldFlags::WRAPPED);
    item.set_durability(25);

    let durability =
        apply_wrapped_gift_transform_like_cpp(&mut item, 200, ItemFieldFlags::SOULBOUND.bits(), 20);

    assert_eq!(durability, 25);
    assert_eq!(item.object().entry(), 200);
    assert_eq!(item.data().gift_creator, ObjectGuid::EMPTY);
    assert_eq!(item.item_flags_bits(), ItemFieldFlags::SOULBOUND.bits());
    assert_eq!(item.data().max_durability, 20);
    assert_eq!(item.data().durability, 25);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(!item.is_wrapped());
}
#[test]
fn open_item_wrapped_gift_with_zero_durability_stays_zero_like_cpp() {
    let item_guid = ObjectGuid::create_item(1, 901);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner_guid),
        max_durability: 30,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.force_state(ItemUpdateState::Unchanged);
    item.replace_all_item_flags(ItemFieldFlags::WRAPPED);
    item.set_durability(0);

    let durability =
        apply_wrapped_gift_transform_like_cpp(&mut item, 200, ItemFieldFlags::SOULBOUND.bits(), 20);

    assert_eq!(durability, 0);
    assert_eq!(item.data().max_durability, 20);
    assert_eq!(item.data().durability, 0);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(!item.is_wrapped());
}
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
