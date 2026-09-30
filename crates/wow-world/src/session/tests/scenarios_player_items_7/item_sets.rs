use super::*;

#[test]
fn destroyed_inventory_item_set_remove_matches_cpp_even_for_broken_equipped_item() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 912);
    let hands_guid = ObjectGuid::create_item(1, 913);
    let backpack_guid = ObjectGuid::create_item(1, 914);
    session.set_player_guid(Some(player_guid));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 704,
        name: "Destroy Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 106,
            1 => 107,
            2 => 108,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 20,
            chr_spec_id: 0,
            spell_id: 9020,
            threshold: 2,
            item_set_id: 704,
        },
    ])));

    let mut broken_chest = session.make_inventory_item_object(
        chest_guid,
        106,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    broken_chest.set_max_durability(10);
    broken_chest.set_durability(0);
    session.insert_inventory_item_object(broken_chest);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: chest_guid,
            entry_id: 106,
            db_guid: chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        107,
        InventoryType::Hands,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        backpack_guid,
        108,
        InventoryType::Chest,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 704,
            spell_entry_id: 20,
            spell_id: 9020,
            threshold: 2,
            apply: true,
        }]
    );

    assert!(!session.record_direct_inventory_item_set_remove_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        backpack_guid,
    ));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp().len(),
        1
    );

    assert!(session.record_direct_inventory_item_set_remove_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
    ));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp()[1],
        RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 704,
            spell_entry_id: 20,
            spell_id: 9020,
            threshold: 2,
            apply: false,
        },
        "C++ DestroyItem removes item-set bonuses for equipped/equipped-bag slots, and item-set bonuses still count broken items"
    );
}
#[test]
fn represented_item_set_guards_skill_legacy_flag_spec_and_broken_items_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 908);
    let hands_guid = ObjectGuid::create_item(1, 909);
    session.set_player_guid(Some(player_guid));
    session.set_represented_primary_specialization_id_like_cpp(66);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([
        ItemSetEntry {
            id: 701,
            name: "Skill Set".to_string(),
            set_flags: 0,
            required_skill: 333,
            required_skill_rank: 80,
            item_id: std::array::from_fn(|i| if i == 0 { 102 } else { 0 }),
        },
        ItemSetEntry {
            id: 702,
            name: "Inactive Set".to_string(),
            set_flags: ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP,
            required_skill: 0,
            required_skill_rank: 0,
            item_id: std::array::from_fn(|i| if i == 0 { 103 } else { 0 }),
        },
        ItemSetEntry {
            id: 703,
            name: "Spec Set".to_string(),
            set_flags: 0,
            required_skill: 0,
            required_skill_rank: 0,
            item_id: std::array::from_fn(|i| match i {
                0 => 104,
                1 => 105,
                _ => 0,
            }),
        },
    ])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 10,
            chr_spec_id: 0,
            spell_id: 9010,
            threshold: 1,
            item_set_id: 701,
        },
        ItemSetSpellEntry {
            id: 11,
            chr_spec_id: 0,
            spell_id: 9011,
            threshold: 1,
            item_set_id: 702,
        },
        ItemSetSpellEntry {
            id: 12,
            chr_spec_id: 65,
            spell_id: 9012,
            threshold: 2,
            item_set_id: 703,
        },
        ItemSetSpellEntry {
            id: 13,
            chr_spec_id: 66,
            spell_id: 9013,
            threshold: 2,
            item_set_id: 703,
        },
    ])));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        102,
        InventoryType::Chest,
    );
    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    session.set_player_skill_values_like_cpp(HashMap::from([(333, 80)]));
    assert!(session.record_represented_items_set_item_like_cpp(chest_guid, true));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        103,
        InventoryType::Hands,
    );
    assert!(!session.record_represented_items_set_item_like_cpp(hands_guid, true));

    let first_spec_guid = ObjectGuid::create_item(1, 910);
    let second_spec_guid = ObjectGuid::create_item(1, 911);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        first_spec_guid,
        104,
        InventoryType::Chest,
    );
    let mut broken_second = session.make_inventory_item_object(
        second_spec_guid,
        105,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_HANDS,
    );
    broken_second.set_max_durability(10);
    broken_second.set_durability(0);
    session.insert_inventory_item_object(broken_second);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_HANDS,
        InventoryItem {
            guid: second_spec_guid,
            entry_id: 105,
            db_guid: second_spec_guid.counter() as u64,
            inventory_type: Some(InventoryType::Hands as u8),
        },
    );

    assert!(!session.record_represented_items_set_item_like_cpp(first_spec_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(second_spec_guid, true));
    assert!(
        !session
            .represented_item_set_spell_events_like_cpp()
            .iter()
            .any(|event| event.spell_id == 9012),
        "C++ AddItemsSetItem does not cast set spells for a non-primary ChrSpecID"
    );
    assert!(
        session
            .represented_item_set_spell_events_like_cpp()
            .iter()
            .any(|event| event.spell_id == 9013 && event.apply),
        "C++ item set bonuses are not dependent on item broken state"
    );
}
