use super::*;
use wow_constants::{EnchantmentSlot, ItemFieldFlags};
use wow_entities::{Item, ItemValuesSnapshot};

#[test]
fn mutable_sql_row_uses_frozen_item_facts_and_explicit_projection_inputs() {
    let mut item = Item::new(17);
    item.set_count(9);
    item.set_expiration(81_000);
    item.set_spell_charges(0, 7);
    item.set_spell_charges(1, -3);
    item.set_spell_charges(2, 99);
    item.set_durability(41);
    item.set_create_played_time(55);
    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(0xf123_4567));

    let slots = [
        EnchantmentSlot::EnhancementPermanent,
        EnchantmentSlot::EnhancementTemporary,
        EnchantmentSlot::EnhancementSocket,
        EnchantmentSlot::EnhancementSocket2,
        EnchantmentSlot::EnhancementSocket3,
        EnchantmentSlot::EnhancementSocketBonus,
        EnchantmentSlot::EnhancementSocketPrismatic,
        EnchantmentSlot::EnhancementUse,
        EnchantmentSlot::Property0,
        EnchantmentSlot::Property1,
        EnchantmentSlot::Property2,
        EnchantmentSlot::Property3,
        EnchantmentSlot::Property4,
    ];
    for (index, slot) in slots.into_iter().enumerate() {
        let id = match index {
            0 | 2 | 12 => 401,
            1 | 5 => 0,
            other => 500 + other as i32,
        };
        let duration = if id == 0 { 777 } else { 1_000 + index as u32 };
        item.set_enchantment(slot, id, duration, -(index as i16));
    }
    let captured = ItemValuesSnapshot::capture(&item);

    item.set_expiration(12);
    item.set_spell_charges(0, -99);
    item.set_durability(3);
    item.set_create_played_time(4);
    item.replace_all_item_flags(ItemFieldFlags::empty());
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 999, 0, 0);

    let captured_enchantments = &captured.read_view().data().enchantments;
    assert_eq!(captured_enchantments.len(), slots.len());
    for (index, enchantment) in captured_enchantments.iter().enumerate() {
        let expected_id = match index {
            0 | 2 | 12 => 401,
            1 | 5 => 0,
            other => 500 + other as i32,
        };
        let expected_duration = if expected_id == 0 {
            777
        } else {
            1_000 + index as u32
        };
        assert_eq!(enchantment.id, expected_id, "enchantment slot {index}");
        assert_eq!(
            enchantment.duration, expected_duration,
            "enchantment duration slot {index}"
        );
        assert_eq!(
            enchantment.charges,
            -(index as i16),
            "enchantment charges slot {index}"
        );
    }

    let enchantment_text = "401 6000 -4 0 0 0 0 0 0 0 0 0 0 ".to_owned();
    let persisted = item_storage_mutable_persistence_like_cpp(
        7_777,
        captured.read_view(),
        4,
        0x8765_4321,
        enchantment_text.clone(),
        2,
    );

    assert_eq!(persisted.item_guid, 7_777);
    assert_eq!(persisted.count, 4);
    assert_eq!(persisted.expiration, 81_000);
    assert_eq!(persisted.charges, "7 -3 ");
    assert_eq!(persisted.flags, 0x8765_4321);
    assert_eq!(persisted.enchantments, enchantment_text);
    assert_eq!(persisted.durability, 41);
    assert_eq!(persisted.played_time, 55);
}
