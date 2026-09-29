use super::*;

fn test_item_enchantments_db_string(entries: &[(usize, i32, u32, i16)]) -> String {
    let mut fields = vec!["0".to_string(); wow_entities::MAX_ENCHANTMENT_SLOT * 3];
    for &(slot, id, duration, charges) in entries {
        let base = slot * 3;
        fields[base] = id.to_string();
        fields[base + 1] = duration.to_string();
        fields[base + 2] = charges.to_string();
    }
    fields.join(" ")
}

#[test]
fn loaded_item_storage_normalizes_template_duration_and_effect_charge_scope_like_cpp() {
    let mut item = wow_entities::Item::default();

    assert!(apply_loaded_item_storage_mutable_fields_like_cpp(
        &mut item,
        0,
        45_000,
        "7 -3 99 100 101 ",
        2,
    ));

    assert_eq!(item.data().expiration, 45_000);
    assert_eq!(item.data().spell_charges[0], 7);
    assert_eq!(item.data().spell_charges[1], -3);
    assert_eq!(item.data().spell_charges[2], 0);
    assert_eq!(
        item_spell_charges_db_string(&[7, -3, 99, 100, 101], 2),
        "7 -3 "
    );
}

#[test]
fn loaded_item_instance_fields_preserve_enchantments_and_random_suffix_like_cpp() {
    let mut item = wow_entities::Item::default();
    let suffixes =
        wow_data::ItemRandomSuffixStore::from_entries([wow_data::ItemRandomSuffixEntry {
            id: 77,
            enchantments: [901, 902, 903, 0, 0],
            allocation_pct: [1000, 2000, 3000, 0, 0],
        }]);
    let enchantments = test_item_enchantments_db_string(&[
        (EnchantmentSlot::EnhancementPermanent as usize, 2673, 0, 0),
        (
            EnchantmentSlot::EnhancementTemporary as usize,
            3826,
            30_000,
            3,
        ),
        (EnchantmentSlot::Property2 as usize, 901, 0, 0),
    ]);
    let enchantments =
        loaded_item_enchantments_like_cpp(&enchantments).expect("valid C++ enchantment string");
    let random_properties = loaded_item_random_properties_like_cpp(-77, 456, None, Some(&suffixes));
    let effective_enchantments = loaded_item_effective_enchantments_like_cpp(
        Some(&enchantments),
        -77,
        None,
        Some(&suffixes),
    );

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, -77);
    assert_eq!(item.data().property_seed, 456);
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        2673
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].duration,
        30_000
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].charges,
        3
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        901
    );
}

#[test]
fn loaded_item_instance_fields_ignore_short_enchantment_string_like_cpp() {
    let mut item = wow_entities::Item::default();
    let enchantments = loaded_item_enchantments_like_cpp("2673 0 0");
    assert!(enchantments.is_none());
    let effective_enchantments =
        loaded_item_effective_enchantments_like_cpp(enchantments.as_ref(), 0, None, None);

    apply_loaded_item_instance_fields_like_cpp(&mut item, &effective_enchantments, None);

    assert!(
        item.data()
            .enchantments
            .iter()
            .all(|enchantment| *enchantment == wow_entities::ItemEnchantment::default())
    );
    assert_eq!(item.data().random_properties_id, 0);
    assert_eq!(item.data().property_seed, 0);
}

#[test]
fn loaded_item_instance_fields_rebuild_random_suffix_slots_like_cpp() {
    let mut item = wow_entities::Item::default();
    let suffixes =
        wow_data::ItemRandomSuffixStore::from_entries([wow_data::ItemRandomSuffixEntry {
            id: 77,
            enchantments: [901, 902, 903, 904, 905],
            allocation_pct: [1000, 2000, 3000, 4000, 5000],
        }]);
    let effective_enchantments =
        loaded_item_effective_enchantments_like_cpp(None, -77, None, Some(&suffixes));
    let random_properties = loaded_item_random_properties_like_cpp(-77, 456, None, Some(&suffixes));

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, -77);
    assert_eq!(item.data().property_seed, 456);
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property0 as usize].id,
        901
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property1 as usize].id,
        902
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        903
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property3 as usize].id,
        0
    );
}

#[test]
fn loaded_item_instance_fields_rebuild_random_property_slots_like_cpp() {
    let mut item = wow_entities::Item::default();
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 1004, 1005],
        }]);
    let effective_enchantments =
        loaded_item_effective_enchantments_like_cpp(None, 77, Some(&properties), None);
    let random_properties = loaded_item_random_properties_like_cpp(77, 0, Some(&properties), None);

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, 77);
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property0 as usize].id,
        0
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        1001
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property3 as usize].id,
        1002
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property4 as usize].id,
        1003
    );
    assert_eq!(item.data().property_seed, 0);
}

#[test]
fn loaded_item_instance_fields_preserve_valid_zero_db_enchantments_like_cpp() {
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let enchantments = test_item_enchantments_db_string(&[]);
    let enchantments =
        loaded_item_enchantments_like_cpp(&enchantments).expect("valid C++ enchantment string");

    let effective_enchantments = loaded_item_effective_enchantments_like_cpp(
        Some(&enchantments),
        77,
        Some(&properties),
        None,
    );

    assert!(effective_enchantments.iter().all(|enchantment| {
        *enchantment == wow_packet::packets::update::ItemEnchantmentValuesUpdate::default()
    }));
}

#[test]
fn loaded_item_db_enchantments_override_random_property_slots_like_cpp() {
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let enchantments =
        test_item_enchantments_db_string(&[(EnchantmentSlot::Property2 as usize, 555, 0, 0)]);
    let enchantments =
        loaded_item_enchantments_like_cpp(&enchantments).expect("valid C++ enchantment string");

    let effective_enchantments = loaded_item_effective_enchantments_like_cpp(
        Some(&enchantments),
        77,
        Some(&properties),
        None,
    );

    assert_eq!(
        effective_enchantments[EnchantmentSlot::Property2 as usize].id,
        555
    );
    assert_eq!(
        effective_enchantments[EnchantmentSlot::Property3 as usize].id,
        0
    );
}

#[test]
fn loaded_item_slots_apply_equipped_enchantments_for_cpp_apply_all_item_mods_range() {
    assert!(loaded_item_slot_applies_equipped_enchantments_like_cpp(
        wow_entities::EQUIPMENT_SLOT_END - 1
    ));
    assert!(loaded_item_slot_applies_equipped_enchantments_like_cpp(
        INVENTORY_SLOT_BAG_START
    ));
    assert!(loaded_item_slot_applies_equipped_enchantments_like_cpp(
        INVENTORY_SLOT_BAG_END - 1
    ));
    assert!(!loaded_item_slot_applies_equipped_enchantments_like_cpp(
        INVENTORY_SLOT_BAG_END
    ));
}

#[test]
fn loaded_positive_random_property_ignores_stale_seed_like_cpp() {
    let mut item = wow_entities::Item::default();
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let effective_enchantments =
        [wow_packet::packets::update::ItemEnchantmentValuesUpdate::default();
            wow_entities::MAX_ENCHANTMENT_SLOT];
    let random_properties =
        loaded_item_random_properties_like_cpp(77, 456, Some(&properties), None);

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, 77);
    assert_eq!(item.data().property_seed, 0);
}

#[test]
fn loaded_missing_random_property_records_are_rejected_like_cpp() {
    let properties = wow_data::ItemRandomPropertiesStore::from_entries([]);
    let suffixes = wow_data::ItemRandomSuffixStore::from_entries([]);

    assert_eq!(
        loaded_item_random_properties_like_cpp(77, 456, Some(&properties), Some(&suffixes)),
        None
    );
    assert_eq!(
        loaded_item_random_properties_like_cpp(-77, 456, Some(&properties), Some(&suffixes)),
        None
    );
}

#[test]
fn bank_storage_mutable_state_round_trips_loaded_expiration_and_charges_like_cpp() {
    let mut item = wow_entities::Item::default();
    assert!(!apply_loaded_item_storage_mutable_fields_like_cpp(
        &mut item,
        90_000,
        90_000,
        "5 -2 0 7 1 ",
        5,
    ));
    item.set_durability(44);
    item.set_create_played_time(55);

    let persisted = item_storage_mutable_persistence_like_cpp(
        7_777,
        &item,
        3,
        0x1234,
        "901 12000 2 ".to_string(),
        5,
    );

    assert_eq!(persisted.item_guid, 7_777);
    assert_eq!(persisted.count, 3);
    assert_eq!(persisted.expiration, 90_000);
    assert_eq!(persisted.charges, "5 -2 0 7 1 ");
    assert_eq!(persisted.flags, 0x1234);
    assert_eq!(persisted.enchantments, "901 12000 2 ");
    assert_eq!(persisted.durability, 44);
    assert_eq!(persisted.played_time, 55);
}
#[test]
fn loaded_socketed_gems_preserve_cpp_item_ids_context_and_bonus_lists() {
    assert_eq!(
        loaded_socketed_gems_like_cpp([
            (700, "11 12".to_string(), 3),
            (0, "13".to_string(), 4),
            (701, "bad 14".to_string(), 5),
        ]),
        vec![
            SocketedGem {
                item_id: 700,
                context: 3,
                bonus_list_ids: vec![11, 12],
            },
            SocketedGem::default(),
            SocketedGem {
                item_id: 701,
                context: 5,
                bonus_list_ids: vec![14],
            },
        ]
    );
}
