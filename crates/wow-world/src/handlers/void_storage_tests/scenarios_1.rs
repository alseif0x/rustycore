//! Void-storage regressions, part 1 of 2.
//!
//! Moved out of the void_storage_tests.rs root under #683; every test is unchanged.

use super::*;

#[test]
fn void_item_packet_uses_cpp_void_instance_fields_only() {
    let (session, _, _) = make_void_storage_session();
    let item = represented_void_item(77, 19019);
    let packet = session.represented_void_storage_item_packet_like_cpp(3, &item);

    assert_eq!(packet.item.item_id, 19019);
    assert_eq!(packet.item.random_properties_id, 0);
    assert_eq!(packet.item.random_properties_seed, 0);
    assert!(packet.item.item_bonus.is_none());
    assert_eq!(
        packet.item.modifications.values,
        vec![wow_packet::packets::item::ItemMod::new(
            80,
            ItemModifier::TimewalkerLevel as u8,
        )]
    );
}

#[test]
fn login_load_rejects_invalid_rows_and_identity_collisions() {
    let (mut session, _, _) = make_void_storage_session();
    session.clear_represented_void_storage_like_cpp();
    install_void_test_item_template(&mut session, 19019);
    let item = represented_void_item(77, 19019);
    assert!(session.load_represented_void_storage_row_like_cpp(3, item.clone()));
    assert!(
        !session.load_represented_void_storage_row_like_cpp(3, represented_void_item(78, 19019),)
    );
    assert!(!session.load_represented_void_storage_row_like_cpp(4, item.clone(),));
    assert!(
        !session.load_represented_void_storage_row_like_cpp(4, represented_void_item(0, 19019),)
    );
    assert!(
        !session
            .load_represented_void_storage_row_like_cpp(u8::MAX, represented_void_item(79, 19019),)
    );
    assert!(
        !session.load_represented_void_storage_row_like_cpp(4, represented_void_item(80, 99999),)
    );
    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(3),
        Some(item)
    );
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(159)
    );
}

#[test]
fn login_load_adds_default_void_item_appearance_like_cpp() {
    let (mut session, _, canonical) = make_void_storage_session();
    let entry = 19019;
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    let mut canonical_player = wow_entities::Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical_player.set_race_class_gender(1, 1, Gender::Male);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
        )
        .unwrap();
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Sword as u8,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: entry,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Uncommon as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                entry,
                ItemSparseTemplateEntry {
                    flags: [0; 4],
                    bag_family: 0,
                    start_quest_id: 0,
                    stackable: 1,
                    max_count: 0,
                    lock_id: 0,
                    required_reputation_rank: 0,
                    sell_price: 0,
                    buy_price: 0,
                    vendor_stack_count: 1,
                    price_variance: 1.0,
                    price_random_value: 1.0,
                    max_durability: 0,
                    other_faction_item_id: 0,
                    content_tuning_id: 0,
                    player_level_to_item_level_curve_id: 0,
                    limit_category: 0,
                    instance_bound: 0,
                    zone_bound: [0; 2],
                    required_reputation_faction: 0,
                    allowable_class: 0,
                    required_expansion: 0,
                    bonding: ItemBondingType::None as u8,
                    container_slots: 0,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
            [(
                entry,
                ItemRandomPropertyTemplateEntry {
                    item_level: 1,
                    quality: ItemQuality::Uncommon as i8,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
        ),
    ));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: entry as i32,
            item_appearance_modifier_id: 0,
            item_appearance_id: 1000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    assert!(session.can_add_item_appearance_represented_like_cpp(65));

    session.clear_represented_void_storage_like_cpp();
    assert!(
        session.load_represented_void_storage_row_like_cpp(0, represented_void_item(77, entry),)
    );
    assert!(session.represented_item_appearances_like_cpp.contains(&65));
}

#[test]
fn locked_login_discards_residual_void_rows_and_initializes_empty_storage_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    install_void_test_item_template(&mut session, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(represented_void_item(77, 19019)),
        Some(0)
    );

    session.set_loaded_player_flags_like_cpp(0);
    assert!(!session.prepare_represented_void_storage_login_load_like_cpp());
    assert_eq!(
        session.represented_void_storage_loaded_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(160)
    );

    session.apply_committed_void_storage_unlock_like_cpp();
    assert!(session.void_storage_is_unlocked_like_cpp());
    assert_eq!(
        session.represented_void_storage_loaded_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(160)
    );
}

#[tokio::test]
async fn unlock_submits_one_semantic_write_before_runtime_publication_like_cpp() {
    let (mut session, _, canonical) = make_void_storage_session();
    session.set_loaded_player_flags_like_cpp(0);
    session.set_player_gold_like_cpp(2_000_000);
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    session.handle_void_storage_unlock(packet).await;

    assert!(session.void_storage_is_unlocked_like_cpp());
    assert_eq!(port.unlocks.lock().unwrap().len(), 1);
    let request = port.unlocks.lock().unwrap()[0].clone();
    assert_eq!(request.player_guid, 42);
    assert_eq!(request.money_before, 2_000_000);
    assert_eq!(request.money_after, 1_000_000);
    assert_ne!(
        request.player_flags_after & PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP,
        0
    );
}

#[test]
fn void_storage_mutation_paths_have_no_concrete_persistence_after_port_cut() {
    let source = include_str!("../void_storage.rs");
    for (start, end) in [
        (
            "pub async fn handle_void_storage_unlock",
            "pub async fn handle_void_storage_query",
        ),
        (
            "pub async fn handle_void_storage_transfer",
            "pub async fn handle_void_storage_swap_item",
        ),
        (
            "pub async fn handle_void_storage_swap_item",
            "#[path = \"void_storage_tests/mod.rs\"]",
        ),
    ] {
        let body = source
            .split_once(start)
            .and_then(|(_, tail)| tail.split_once(end).map(|(body, _)| body))
            .expect("audited void-storage handler body");
        for forbidden in ["CharStatements", "SqlTransaction", ".prepare(", "char_db"] {
            assert!(
                !body.contains(forbidden),
                "{start} regained concrete persistence syntax: {forbidden}"
            );
        }
    }
}

#[test]
fn new_void_withdrawal_create_carries_committed_item_state_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    install_void_test_item_template(&mut session, 19019);
    let owner = ObjectGuid::create_player(1, 42);
    let bag = ObjectGuid::create_item(1, 500);
    let item_guid = ObjectGuid::create_item(1, 501);
    let mut item = session.make_inventory_item_object(
        item_guid,
        19019,
        owner,
        2,
        37,
        ItemContext::Timewalking,
        4,
    );
    item.set_contained_in(bag);
    item.set_container_guid_and_slot(bag, 19);
    item.set_property_seed(29);
    item.set_random_properties_id(-13);
    item.set_creator(ObjectGuid::create_player(1, 7));
    item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    item.set_binding(true);
    item.set_enchantment(EnchantmentSlot::Property0, 201, 60_000, 2);

    let create_dynamic_flags = ItemFieldFlags::NEW_ITEM.bits();
    let create = void_withdrawal_item_create_data_like_cpp(&item, create_dynamic_flags, 0);
    assert_eq!(create.item_guid, item_guid);
    assert_eq!(create.entry_id, 19019);
    assert_eq!(create.owner_guid, owner);
    assert_eq!(create.contained_in, bag);
    assert_eq!(create.stack_count, 2);
    assert_eq!(create.dynamic_flags, ItemFieldFlags::NEW_ITEM.bits());
    assert_eq!(create.durability, 37);
    assert_eq!(create.random_properties_seed, 0);
    assert_eq!(create.random_properties_id, 0);
    assert_eq!(create.context, ItemContext::Timewalking as u8);
    assert_eq!(create.container_slots, 0);
    assert_eq!(
        create.enchantments[EnchantmentSlot::Property0 as usize].id,
        0
    );
    assert_eq!(
        create.enchantments[EnchantmentSlot::Property0 as usize].duration,
        0
    );
    assert_eq!(
        create.enchantments[EnchantmentSlot::Property0 as usize].charges,
        0
    );

    let post_store_update =
        crate::session_rules::void_withdrawal_post_store_item_values_update_like_cpp(
            &item,
            create_dynamic_flags,
        )
        .expect("post-store item update");
    let item_data = post_store_update
        .item_data
        .as_ref()
        .expect("post-store update owns ItemData");
    assert!(item_data.mask.is_set(wow_entities::ITEM_DATA_PARENT_BIT));
    assert!(item_data.mask.is_set(wow_entities::ITEM_DATA_CREATOR_BIT));
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS_BIT)
    );
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_PROPERTY_SEED_BIT)
    );
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_RANDOM_PROPERTIES_ID_BIT)
    );
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT)
    );
    assert!(item_data.mask.is_set(
        wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT + EnchantmentSlot::Property0 as usize
    ));
    assert_eq!(item_data.values.creator, ObjectGuid::create_player(1, 7));
    assert_eq!(
        item_data.values.dynamic_flags,
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );
    assert_eq!(item_data.values.property_seed, 29);
    assert_eq!(item_data.values.random_properties_id, -13);
    assert_eq!(
        item_data.values.enchantments[EnchantmentSlot::Property0 as usize].id,
        201
    );

    let packet = crate::entity_update_bridge::item_values_update_to_update_object(
        item_guid,
        571,
        &post_store_update,
    )
    .expect("creator VALUES packet");
    let bytes = packet.to_bytes();
    let mut packed_creator = WorldPacket::new_empty();
    packed_creator.write_packed_guid(&ObjectGuid::create_player(1, 7));
    let packed_creator = packed_creator.into_data();
    assert!(
        bytes
            .windows(packed_creator.len())
            .any(|window| window == packed_creator),
        "the creator GUID must reach the serialized VALUES update"
    );
    assert!(bytes.windows(4).any(|window| {
        window
            == (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND)
                .bits()
                .to_le_bytes()
    }));
}

#[test]
fn withdrawn_bag_create_preserves_template_container_slots_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let bag_entry = 21841;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, 19019);
    let bag = session.make_inventory_item_object(
        ObjectGuid::create_item(1, 501),
        bag_entry,
        ObjectGuid::create_player(1, 42),
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    let container_slots = session
        .item_storage_template(bag_entry)
        .map_or(0, |template| u32::from(template.container_slots));

    let create = void_withdrawal_item_create_data_like_cpp(
        &bag,
        ItemFieldFlags::NEW_ITEM.bits(),
        container_slots,
    );

    assert_eq!(create.container_slots, 8);
    assert!(create.container_item_guids.iter().all(ObjectGuid::is_empty));
}

#[test]
fn committed_withdrawn_bag_registers_canonical_storage_before_child_like_cpp() {
    let (mut session, _, canonical) = make_void_storage_session();
    let bag_entry = 21841;
    let child_entry = 19019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, child_entry);
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut canonical_player = wow_entities::Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical_player.set_race_class_gender(1, 1, Gender::Male);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
        )
        .unwrap();
    let bag_guid = ObjectGuid::create_item(1, 501);
    let child_guid = ObjectGuid::create_item(1, 502);
    let bag_object = session.make_inventory_item_object(
        bag_guid,
        bag_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    assert!(session.apply_committed_new_inventory_item_at_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: bag_entry,
            db_guid: 501,
            inventory_type: Some(InventoryType::Bag as u8),
        },
        bag_object,
    ));

    let child_object = session.make_inventory_item_object(
        child_guid,
        child_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    assert!(session.apply_committed_new_inventory_item_at_like_cpp(
        INVENTORY_SLOT_BAG_START,
        0,
        InventoryItem {
            guid: child_guid,
            entry_id: child_entry,
            db_guid: 502,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
        child_object,
    ));

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 0)
            })
            .flatten(),
        Some(child_guid)
    );
}

#[test]
fn mixed_transfer_publishes_deposit_destroy_before_withdrawal_create_like_cpp() {
    let (mut session, send_rx, _) = make_void_storage_session();
    install_void_test_item_template(&mut session, 19019);
    let owner = ObjectGuid::create_player(1, 42);
    let deposited_guid = ObjectGuid::create_item(1, 500);
    let withdrawn_guid = ObjectGuid::create_item(1, 501);
    let mut withdrawn_item = session.make_inventory_item_object(
        withdrawn_guid,
        19019,
        owner,
        1,
        37,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    withdrawn_item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    session.insert_inventory_item_object(withdrawn_item.clone());
    let mut post_store_item = withdrawn_item.clone();
    WorldSession::clear_item_publication_changes_like_cpp(&mut post_store_item);

    let create_dynamic_flags = ItemFieldFlags::NEW_ITEM.bits();
    let expected_destroy = UpdateObject::destroy_objects(vec![deposited_guid], 571).to_bytes();
    let expected_create = UpdateObject::create_stored_items(
        vec![void_withdrawal_item_create_data_like_cpp(
            &withdrawn_item,
            create_dynamic_flags,
            0,
        )],
        571,
    )
    .to_bytes();

    session.publish_void_storage_item_lifecycle_like_cpp(
        571,
        vec![(
            (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            vec![deposited_guid],
        )],
        vec![WorldSession::new_void_withdrawal_item_publication_like_cpp(
            &withdrawn_item,
            &post_store_item,
            create_dynamic_flags,
            0,
            571,
        )],
    );

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(packets.first(), Some(&expected_destroy));
    let create_index = packets
        .iter()
        .position(|packet| packet == &expected_create)
        .expect("withdrawal CREATE_OBJECT packet");
    assert!(
        create_index > 0,
        "C++ publishes every deposit destroy before withdrawal creates"
    );
}

#[test]
fn planned_stack_merge_publishes_store_then_post_store_values_like_cpp() {
    let (session, send_rx, _) = make_void_storage_session();
    let owner = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 501);
    let mut create_item = session.make_inventory_item_object(
        item_guid,
        19019,
        owner,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    create_item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    let create_dynamic_flags = ItemFieldFlags::NEW_ITEM.bits();

    let mut first_post_store_item = create_item.clone();
    WorldSession::clear_item_publication_changes_like_cpp(&mut first_post_store_item);
    first_post_store_item.set_creator(ObjectGuid::create_player(1, 7));
    first_post_store_item.set_binding(true);

    let mut merged_post_store_item = first_post_store_item.clone();
    WorldSession::clear_item_publication_changes_like_cpp(&mut merged_post_store_item);
    merged_post_store_item.set_creator(ObjectGuid::create_player(1, 8));

    let expected_create = UpdateObject::create_stored_items(
        vec![void_withdrawal_item_create_data_like_cpp(
            &create_item,
            create_dynamic_flags,
            0,
        )],
        571,
    )
    .to_bytes();
    let expected_store_merge = UpdateObject::item_stack_count_update(item_guid, 571, 2).to_bytes();
    let expected_post_store_merge =
        crate::entity_update_bridge::item_values_update_to_update_object(
            item_guid,
            571,
            &merged_post_store_item.values_update(),
        )
        .expect("planned-stack post-store VALUES update")
        .to_bytes();

    let mut publications = vec![WorldSession::new_void_withdrawal_item_publication_like_cpp(
        &create_item,
        &first_post_store_item,
        create_dynamic_flags,
        0,
        571,
    )];
    publications.extend(
        WorldSession::merged_void_withdrawal_item_publications_like_cpp(
            item_guid,
            2,
            None,
            &merged_post_store_item,
            571,
        ),
    );
    session.publish_void_storage_item_lifecycle_like_cpp(571, Vec::new(), publications);

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let create_index = packets
        .iter()
        .position(|packet| packet == &expected_create)
        .expect("count-one CREATE_OBJECT packet");
    let store_merge_index = packets
        .iter()
        .position(|packet| packet == &expected_store_merge)
        .expect("count-two StoreItem VALUES packet");
    let post_store_merge_index = packets
        .iter()
        .position(|packet| packet == &expected_post_store_merge)
        .expect("post-StoreNewItem creator VALUES packet");
    assert!(
        create_index < store_merge_index && store_merge_index < post_store_merge_index,
        "C++ publishes CREATE, then the merge count, then post-store field changes"
    );
}

#[test]
fn nested_withdrawal_resolves_planned_bag_database_and_item_guids_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let runtime_bag_guid = ObjectGuid::create_item(1, 600);
    session.insert_inventory_item_like_cpp(
        bag_slot,
        InventoryItem {
            guid: runtime_bag_guid,
            entry_id: 21841,
            db_guid: 600,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let planned = std::collections::HashMap::from([(bag_slot, 700)]);
    let planned_bag_guid = ObjectGuid::create_item(1, 700);
    let planned_item_guids = std::collections::HashMap::from([(bag_slot, planned_bag_guid)]);

    assert_eq!(
        session.void_storage_withdrawal_container_db_guid_like_cpp(INVENTORY_SLOT_BAG_0, &planned,),
        Some(0)
    );
    assert_eq!(
        session.void_storage_withdrawal_container_db_guid_like_cpp(bag_slot, &planned,),
        Some(700),
        "the planned bag must beat the stale runtime bag being replaced in the transaction"
    );
    assert_eq!(
        session.void_storage_withdrawal_container_db_guid_like_cpp(
            wow_entities::INVENTORY_SLOT_BAG_START + 1,
            &planned,
        ),
        None
    );
    assert_eq!(
        session.void_storage_withdrawal_container_item_guid_like_cpp(
            INVENTORY_SLOT_BAG_0,
            &planned_item_guids,
        ),
        session.player_guid()
    );
    assert_eq!(
        session
            .void_storage_withdrawal_container_item_guid_like_cpp(bag_slot, &planned_item_guids,),
        Some(planned_bag_guid),
        "the planned bag object must beat the stale runtime bag in the same slot"
    );
    assert_ne!(planned_bag_guid, runtime_bag_guid);
}

#[test]
fn withdrawal_restores_and_persists_effective_random_property_enchantments_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    session.set_item_random_properties_store(Arc::new(ItemRandomPropertiesStore::from_entries([
        ItemRandomPropertiesEntry {
            id: 17,
            enchantments: [101, 102, 103, 104, 105],
        },
    ])));
    session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
        ItemRandomSuffixEntry {
            id: 13,
            enchantments: [201, 202, 203, 204, 205],
            allocation_pct: [0; 5],
        },
    ])));

    let positive = session.effective_void_storage_random_properties_like_cpp(17, 29);
    assert_eq!(positive.id, 17);
    assert_eq!(positive.seed, 0);
    assert_eq!(
        positive.enchantment_ids[EnchantmentSlot::Property2 as usize],
        101
    );
    assert_eq!(
        positive.enchantment_ids[EnchantmentSlot::Property3 as usize],
        102
    );
    assert_eq!(
        positive.enchantment_ids[EnchantmentSlot::Property4 as usize],
        103
    );

    let suffix = session.effective_void_storage_random_properties_like_cpp(-13, 29);
    assert_eq!(suffix.id, -13);
    assert_eq!(suffix.seed, 29);
    assert_eq!(
        suffix.enchantment_ids[EnchantmentSlot::Property0 as usize],
        201
    );
    assert_eq!(
        suffix.enchantment_ids[EnchantmentSlot::Property1 as usize],
        202
    );
    assert_eq!(
        suffix.enchantment_ids[EnchantmentSlot::Property2 as usize],
        203
    );
    assert_eq!(
        session.effective_void_storage_random_properties_like_cpp(-999, 29),
        EffectiveVoidStorageRandomPropertiesLikeCpp::default()
    );

    let mut runtime_item = wow_entities::Item::default();
    runtime_item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 999, 60_000, 2);
    WorldSession::apply_effective_void_storage_random_properties_like_cpp(
        &mut runtime_item,
        &suffix,
    );
    assert_eq!(runtime_item.data().random_properties_id, -13);
    assert_eq!(runtime_item.data().property_seed, 29);
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::Property0 as usize].id,
        201
    );
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::Property1 as usize].id,
        202
    );
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        203
    );
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        999,
        "SetItemRandomProperties must not clear unrelated destination enchantments"
    );
}

#[test]
fn empty_inventory_positions_use_active_backpack_slot_count_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();

    session.set_player_inventory_slot_count_like_cpp(INVENTORY_DEFAULT_SIZE);
    let default_positions = session
        .represented_empty_inventory_positions_like_cpp()
        .expect("test inventory owner resolves");
    assert_eq!(default_positions.len(), usize::from(INVENTORY_DEFAULT_SIZE));
    assert_eq!(
        default_positions.last(),
        Some(&(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15))
    );

    session.set_player_inventory_slot_count_like_cpp(24);
    let expanded_positions = session
        .represented_empty_inventory_positions_like_cpp()
        .expect("test inventory owner resolves");
    assert_eq!(expanded_positions.len(), 24);
    assert_eq!(
        expanded_positions.last(),
        Some(&(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 23))
    );
}

#[test]
fn swap_destination_slot_truncates_to_uint8_before_range_check_like_cpp() {
    assert_eq!(
        WorldSession::void_storage_swap_destination_slot_like_cpp(256),
        0
    );
    assert_eq!(
        WorldSession::void_storage_swap_destination_slot_like_cpp(415),
        159
    );
    assert_eq!(
        WorldSession::void_storage_swap_destination_slot_like_cpp(416),
        160,
        "truncation occurs before the handler's 160-slot range check"
    );
}
