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
