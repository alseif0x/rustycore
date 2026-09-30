//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn battle_pet_cage_battle_pet_creates_cage_item_removes_and_deletes_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x184);
    let expected_item = RepresentedBattlePetCageItemLikeCpp {
        item_id: BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP,
        species_id: 11,
        breed_data: 44 | (3 << 24),
        level: 17,
        display_id: 33,
    };

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 100,
            max_health: 100,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(pet_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::Caged(expected_item)
    );

    assert_eq!(
        session.represented_battle_pet_cage_items_like_cpp(),
        &[expected_item]
    );
    assert_eq!(
        session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("removed pet row remains represented")
            .save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Removed
    );
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetDeleted as u16
    );
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.remaining(), 0);
}
#[test]
fn is_toy_item_uses_toy_db2_item_id_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_toy_store(Arc::new(ToyStore::from_entries([ToyEntry {
        id: 9,
        source_text: "known".to_string(),
        item_id: 30_000,
        flags: 0,
        source_type_enum: 0,
    }])));

    assert!(session.is_toy_item_like_cpp(30_000));
    assert!(!session.is_toy_item_like_cpp(30_001));
}
#[test]
fn load_account_item_appearances_rebuilds_blocks_and_favorites_like_cpp() {
    let (mut session, _, _) = make_session();
    session.represented_item_appearances_like_cpp.insert(999);
    session
        .represented_favorite_item_appearances_like_cpp
        .insert(998, FavoriteAppearanceStateLikeCpp::New);

    session.load_represented_account_item_appearances_like_cpp(
        [(2, 1_u32), (0, (1_u32 << 1) | (1_u32 << 31)), (1, 0_u32)],
        [65, 96],
    );

    assert_eq!(
        session.represented_item_appearances_like_cpp,
        HashSet::from([1, 31, 64])
    );
    assert_eq!(
        session.account_transmog_active_player_rows_like_cpp(),
        vec![(1_u32 << 1) | (1_u32 << 31), 0, 1]
    );
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(65),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(96),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(998),
        None
    );
}
#[test]
fn account_item_appearance_save_plan_matches_collection_mgr_state_transitions_like_cpp() {
    let (mut session, _, _) = make_session();
    session.represented_item_appearances_like_cpp = HashSet::from([1, 31, 64]);
    session
        .represented_favorite_item_appearances_like_cpp
        .insert(65, FavoriteAppearanceStateLikeCpp::New);
    session
        .represented_favorite_item_appearances_like_cpp
        .insert(96, FavoriteAppearanceStateLikeCpp::Removed);
    session
        .represented_favorite_item_appearances_like_cpp
        .insert(97, FavoriteAppearanceStateLikeCpp::Unchanged);

    let plan = session
        .account_item_appearance_save_plan_like_cpp()
        .expect("resolved collection owner");

    assert_eq!(
        plan.appearance_blocks,
        vec![(0, (1_u32 << 1) | (1_u32 << 31)), (2, 1_u32)]
    );
    assert_eq!(plan.favorite_inserts, vec![65]);
    assert_eq!(plan.favorite_deletes, vec![96]);
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(65),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(96),
        None
    );
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(97),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
}
#[test]
fn load_account_transmog_illusions_includes_static_defaults_like_cpp() {
    let (mut session, _, _) = make_session();
    session.represented_transmog_illusions_like_cpp.insert(999);

    session
        .load_represented_account_transmog_illusions_like_cpp([(0, 1_u32 << 1), (2, 1_u32 << 3)]);

    assert!(session.has_transmog_illusion_like_cpp(1));
    assert!(session.has_transmog_illusion_like_cpp(67));
    for illusion_id in DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP {
        assert!(session.has_transmog_illusion_like_cpp(illusion_id));
    }
    assert!(!session.has_transmog_illusion_like_cpp(999));
}
#[test]
fn account_transmog_illusion_save_plan_writes_non_empty_blocks_like_cpp() {
    let (mut session, _, _) = make_session();
    session.load_represented_account_transmog_illusions_like_cpp([(2, 1_u32 << 3)]);

    let plan = session
        .account_transmog_illusion_save_plan_like_cpp()
        .expect("resolved collection owner");

    assert_eq!(
        plan.illusion_blocks,
        vec![
            (
                0,
                (1_u32 << 3) | (1_u32 << 13) | (1_u32 << 22) | (1_u32 << 23),
            ),
            (1, (1_u32 << 2) | (1_u32 << 11) | (1_u32 << 12)),
            (2, 1_u32 << 3),
        ]
    );
    assert!(!plan.is_empty());
}
#[test]
fn temporary_item_appearance_tracks_conditional_transmog_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 76);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid_1 = ObjectGuid::create_item(1, 901);
    let item_guid_2 = ObjectGuid::create_item(1, 902);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ConditionalTransmogTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_temporary_item_appearance_like_cpp(65, item_guid_1)
        .expect("first temporary provider should add conditional transmog");
    let active = update
        .active_player_data
        .expect("conditional transmog should mark active player data");
    assert_eq!(active.values.conditional_transmog, vec![65]);
    assert_eq!(
        active.values.conditional_transmog_update_mask,
        Some(vec![1])
    );
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));

    assert!(
        session
            .add_temporary_item_appearance_like_cpp(65, item_guid_2)
            .is_none()
    );
    assert_eq!(
        session.items_providing_temporary_appearance_like_cpp(65),
        HashSet::from([item_guid_1, item_guid_2])
    );

    assert!(
        session
            .remove_temporary_item_appearance_like_cpp(65, item_guid_1)
            .is_none()
    );
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));

    let update = session
        .remove_temporary_item_appearance_like_cpp(65, item_guid_2)
        .expect("last temporary provider should remove conditional transmog");
    let active = update
        .active_player_data
        .expect("conditional transmog removal should mark active player data");
    assert_eq!(active.values.conditional_transmog, Vec::<i32>::new());
    assert_eq!(
        active.values.conditional_transmog_update_mask,
        Some(vec![1])
    );
    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));

    session
        .add_temporary_item_appearance_like_cpp(65, item_guid_1)
        .expect("temporary appearance can be re-added");
    let update = session
        .add_item_appearance_like_cpp(65)
        .expect("permanent appearance should remove matching temporary appearance");
    let active = update
        .active_player_data
        .expect("permanent appearance should mark active player data");
    assert_eq!(active.values.conditional_transmog, Vec::<i32>::new());
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, false));
    assert!(
        session
            .items_providing_temporary_appearance_like_cpp(65)
            .is_empty()
    );
}
#[test]
fn is_transmog_set_completed_ignores_temporary_and_missing_entries_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 70,
            item_modified_appearance_id: 96,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 70,
            item_modified_appearance_id: 999,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 4,
            transmog_set_id: 71,
            item_modified_appearance_id: 300,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 96,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 300,
                item_id: 779,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9002,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 777,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Head as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 778,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 779,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));

    assert!(!session.is_transmog_set_completed_like_cpp(99));
    assert!(session.is_transmog_set_completed_like_cpp(71));

    session
        .represented_temporary_item_appearances_like_cpp
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 901)]));
    session.represented_item_appearances_like_cpp.insert(96);
    assert!(!session.is_transmog_set_completed_like_cpp(70));

    session.represented_item_appearances_like_cpp.insert(65);
    assert!(session.is_transmog_set_completed_like_cpp(70));
}
#[test]
fn is_transmog_set_completed_keeps_first_completed_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 80,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 80,
            item_modified_appearance_id: 96,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 81,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 4,
            transmog_set_id: 81,
            item_modified_appearance_id: 96,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 96,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 777,
            class_id: 2,
            subclass_id: 7,
            material: 0,
            inventory_type: InventoryType::Weapon as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 778,
            class_id: 2,
            subclass_id: 7,
            material: 0,
            inventory_type: InventoryType::WeaponOffhand as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));

    session
        .represented_temporary_item_appearances_like_cpp
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 901)]));
    session.represented_item_appearances_like_cpp.insert(96);
    assert!(session.is_transmog_set_completed_like_cpp(80));

    session.represented_item_appearances_like_cpp.remove(&96);
    session.represented_item_appearances_like_cpp.insert(65);
    session
        .represented_temporary_item_appearances_like_cpp
        .insert(96, HashSet::from([ObjectGuid::create_item(1, 902)]));
    assert!(session.is_transmog_set_completed_like_cpp(81));
}
#[test]
fn add_item_appearance_records_transmog_criteria_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 78);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogCriteriaTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries_and_sets(
        [
            TransmogSetItemEntry {
                id: 1,
                transmog_set_id: 70,
                item_modified_appearance_id: 65,
                flags: 0,
            },
            TransmogSetItemEntry {
                id: 2,
                transmog_set_id: 70,
                item_modified_appearance_id: 96,
                flags: 0,
            },
        ],
        [TransmogSetEntry {
            id: 70,
            name: "criterion set".to_string(),
            class_mask: 0,
            tracking_quest_id: 0,
            flags: 0,
            transmog_set_group_id: 7000,
            item_name_description_id: 0,
            parent_transmog_set_id: 0,
            expansion_id: 0,
            ui_order: 0,
        }],
    )));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 96,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 777,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Head as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 778,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    session
        .represented_temporary_item_appearances_like_cpp
        .insert(96, HashSet::from([ObjectGuid::create_item(1, 902)]));

    session
        .add_item_appearance_like_cpp(65)
        .expect("first permanent piece should update transmog state");
    assert_eq!(
        session.represented_transmog_criteria_events,
        vec![RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
            equipment_slot: EQUIPMENT_SLOT_HEAD as u32,
            item_modified_appearance_id: 65,
        }]
    );

    session.represented_transmog_criteria_events.clear();
    session
        .add_item_appearance_like_cpp(96)
        .expect("second permanent piece should complete the set");
    assert_eq!(
        session.represented_transmog_criteria_events,
        vec![
            RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
                equipment_slot: EQUIPMENT_SLOT_CHEST as u32,
                item_modified_appearance_id: 96,
            },
            RepresentedTransmogCriteriaEvent::CollectTransmogSetFromGroup {
                transmog_set_group_id: 7000,
            },
        ]
    );
}
#[test]
fn add_transmog_set_skips_missing_and_marks_appearances_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 72);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogSetTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 0,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 70,
            item_modified_appearance_id: 999,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 70,
            item_modified_appearance_id: 65,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 0,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_transmog_set_like_cpp(70)
        .expect("represented current player should receive transmog set update");
    let active = update
        .active_player_data
        .expect("transmog set should mark active player data");

    assert!(session.represented_has_item_appearance_like_cpp(0));
    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert!(!session.represented_has_item_appearance_like_cpp(999));
    assert_eq!(active.values.transmog, vec![1, 0, 1 << 1]);
    assert_eq!(active.values.transmog_update_mask, Some(vec![0b111]));
    assert!(
        active
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        active
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_TRANSMOG_BIT)
    );
    assert!(session.add_transmog_set_like_cpp(99).is_none());
}
#[test]
fn item_template_flags_use_item_sparse_flags_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [],
        [
            (100, [ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32, 0, 0, 0]),
            (101, [0, 0, 0, 0]),
        ],
    )));

    assert!(
        session
            .item_template_flags(100)
            .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
    );
    assert!(session.is_item_bound_account_wide(100));
    assert!(!session.is_item_bound_account_wide(101));
    assert_eq!(session.item_template_flags(102), None);
}
#[test]
fn item_durability_repair_cost_uses_cpp_db2_cost_and_quality() {
    let (mut session, _, _) = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 100,
            class_id: ItemClass::Armor as u8,
            subclass_id: ItemSubClassArmor::Shield as u8,
            material: 0,
            inventory_type: InventoryType::Shield as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 101,
            class_id: ItemClass::Armor as u8,
            subclass_id: 4,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_random_property_templates([
        (
            100,
            ItemRandomPropertyTemplateEntry {
                item_level: 57,
                quality: ItemQuality::Rare as i8,
                inventory_type: InventoryType::Shield as i8,
            },
        ),
        (
            101,
            ItemRandomPropertyTemplateEntry {
                item_level: 57,
                quality: ItemQuality::Rare as i8,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
    ])));
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 57,
            weapon_sub_class_cost: [0; 21],
            armor_sub_class_cost: std::array::from_fn(|i| {
                if i == ItemSubClassArmor::Shield as usize {
                    13
                } else if i == 4 {
                    5
                } else {
                    0
                }
            }),
        },
    ])));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry {
            id: (ItemQuality::Rare as u32 + 1) * 2,
            data: 1.25,
        },
    ])));

    assert_eq!(
        session.item_durability_repair_cost_like_cpp(100, 40, 50, 0.8, 2.0),
        260
    );
    assert_eq!(
        session.item_durability_repair_cost_like_cpp(101, 10, 13, 1.0, 1.0),
        19
    );
    assert_eq!(
        session.item_durability_repair_cost_like_cpp(100, 50, 50, 1.0, 1.0),
        0
    );
}
