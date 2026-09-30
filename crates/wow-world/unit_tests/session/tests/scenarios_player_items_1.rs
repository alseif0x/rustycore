//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn auto_unequip_offhand_records_titan_grip_penalty_check_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mainhand_item_id = 30_012_u32;
    let offhand_item_id = 30_013_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_012);
    let offhand_guid = ObjectGuid::create_item(1, 30_013);
    let player_guid = ObjectGuid::create_player(1, 162);
    let penalty_spell_id = 49_152_u32;
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TitanGripPenaltyRemoveItem".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(true, penalty_spell_id);
    });
    session
        .apply_aura(penalty_spell_id as i32, player_guid, 0, 0)
        .expect("represented Titan Grip penalty aura");
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                mainhand_item_id,
                InventoryType::Weapon2Hand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe2 as u8,
            ),
            (
                offhand_item_id,
                InventoryType::Weapon2Hand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe2 as u8,
            ),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::Weapon2Hand,
    );

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(true));

    assert_eq!(
        session.represented_titan_grip_penalty_actions_like_cpp(),
        &[TitanGripPenaltyAction::Remove(penalty_spell_id)],
        "C++ RemoveItem calls CheckTitanGripPenalty after clearing the offhand slot; once only the main-hand 2H remains, the Titan Grip penalty aura is removed"
    );
}
#[test]
fn auto_unequip_offhand_records_average_equipped_item_level_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mainhand_item_id = 30_014_u32;
    let offhand_item_id = 30_015_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_014);
    let offhand_guid = ObjectGuid::create_item(1, 30_015);
    let player_guid = ObjectGuid::create_player(1, 163);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelRemoveItem".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(false, 0);
    });
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: mainhand_item_id,
            class_id: ItemClass::Weapon as u8,
            subclass_id: ItemSubClassWeapon::Axe2 as u8,
            material: 0,
            inventory_type: InventoryType::Weapon2Hand as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: offhand_item_id,
            class_id: ItemClass::Weapon as u8,
            subclass_id: ItemSubClassWeapon::Axe2 as u8,
            material: 0,
            inventory_type: InventoryType::Weapon2Hand as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    mainhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
                (
                    offhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
            ],
            [
                (
                    mainhand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 200,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon2Hand as i8,
                    },
                ),
                (
                    offhand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon2Hand as i8,
                    },
                ),
            ],
        ),
    ));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::Weapon2Hand,
    );

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(true));

    assert_eq!(
        session.represented_avg_equipped_item_level_updates_like_cpp(),
        &[25.0],
        "C++ UpdateAverageItemLevelEquipped divides equipped item-level sum by 16 and counts a remaining main-hand 2H twice without Titan Grip"
    );
    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");
    assert_eq!(
        context.avg_equipped_item_level, 25.0,
        "C++ PlayerCondition uses PlayerData::AvgItemLevel[1], so the represented context must use the same equipped-average formula"
    );
}
#[test]
fn represented_condition_total_avg_item_level_uses_cpp_slot_formula_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mainhand_item_id = 30_016_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_016);
    let player_guid = ObjectGuid::create_player(1, 164);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelTotal".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(false, 0);
    });
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: mainhand_item_id,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Axe2 as u8,
        material: 0,
        inventory_type: InventoryType::Weapon2Hand as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                mainhand_item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
            )],
            [(
                mainhand_item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 200,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Weapon2Hand as i8,
                },
            )],
        ),
    ));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 25.0,
        "C++ UpdateAverageItemLevelTotal divides the best-slot item-level sum by 16 and counts a main-hand 2H twice without Titan Grip"
    );
    assert_eq!(
        context.avg_equipped_item_level, 25.0,
        "The represented equipped boundary has the same result when only equipped items are known"
    );
}
#[test]
fn represented_condition_total_avg_item_level_uses_best_represented_slot_candidate_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_017_u32;
    let bag_chest_item_id = 30_018_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_017);
    let bag_chest_guid = ObjectGuid::create_item(1, 30_018);
    let player_guid = ObjectGuid::create_player(1, 165);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelBestCandidate".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            equipped_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            bag_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    equipped_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (
                    bag_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
            ],
            [
                (
                    equipped_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    bag_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 200,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
            ],
        ),
    ));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        equipped_chest_guid,
        equipped_chest_item_id,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        bag_chest_guid,
        bag_chest_item_id,
        InventoryType::Chest,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 12.5,
        "C++ UpdateAverageItemLevelTotal keeps the highest represented equipable candidate per equipment slot and divides by 16"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped only uses currently equipped items"
    );
}
#[test]
fn represented_condition_avg_item_level_uses_runtime_item_level_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_046_u32;
    let bag_chest_item_id = 30_047_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_046);
    let bag_chest_guid = ObjectGuid::create_item(1, 30_047);
    let player_guid = ObjectGuid::create_player(1, 176);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelRuntimeLevel".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            equipped_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            bag_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    equipped_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (
                    bag_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
            ],
            [
                (
                    equipped_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    bag_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
            ],
        ),
    ));
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut equipped_chest = session.make_inventory_item_object(
        equipped_chest_guid,
        equipped_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    equipped_chest.set_debug_item_level(180);
    session.insert_inventory_item_object(equipped_chest);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: equipped_chest_guid,
            entry_id: equipped_chest_item_id,
            db_guid: equipped_chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    let mut bag_chest = session.make_inventory_item_object(
        bag_chest_guid,
        bag_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    bag_chest.set_debug_item_level(260);
    session.insert_inventory_item_object(bag_chest);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: bag_chest_guid,
            entry_id: bag_chest_item_id,
            db_guid: bag_chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 16.25,
        "C++ UpdateAverageItemLevelTotal calls Item::GetItemLevel(owner), so a runtime item level can replace the static template item level for best-slot candidates"
    );
    assert_eq!(
        context.avg_equipped_item_level, 11.25,
        "C++ UpdateAverageItemLevelEquipped also calls Item::GetItemLevel(owner) for equipped items"
    );
}
#[test]
fn represented_condition_avg_item_level_applies_item_bonus_level_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let chest_item_id = 30_048_u32;
    let chest_guid = ObjectGuid::create_item(1, 30_048);
    let player_guid = ObjectGuid::create_player(1, 177);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelBonusLevel".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                chest_item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                chest_item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_item_bonus_db2_store(Arc::new(ItemBonusDb2Store::from_entries([
        ItemBonusDb2Entry {
            id: 1,
            value: [55, 0, 0, 0],
            parent_item_bonus_list_id: 77,
            bonus_type: ItemBonusType::ItemLevel as u8,
            order_index: 0,
        },
        ItemBonusDb2Entry {
            id: 2,
            value: [900, 0, 0, 0],
            parent_item_bonus_list_id: 78,
            bonus_type: ItemBonusType::Quality as u8,
            order_index: 0,
        },
    ])));
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut chest = session.make_inventory_item_object(
        chest_guid,
        chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    chest.set_item_bonus_key(ItemBonusKey {
        item_id: chest_item_id as i32,
        bonus_list_ids: vec![77, 78],
    });
    session.insert_inventory_item_object(chest);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: chest_guid,
            entry_id: chest_item_id,
            db_guid: chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 9.6875,
        "C++ BonusData::ItemLevelBonus is added before PlayerData::AvgItemLevel[0] is computed"
    );
    assert_eq!(
        context.avg_equipped_item_level, 9.6875,
        "C++ UpdateAverageItemLevelEquipped also consumes Item::GetItemLevel(owner) with item bonus levels"
    );
}
#[test]
fn represented_item_level_uses_player_level_curve_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_158_u32;
    session.set_player_level_like_cpp(45);
    install_represented_item_level_curve_fixture_like_cpp(&mut session, item_id, 0, 9_001);

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(145),
        "C++ Item::GetItemLevel replaces template item level with DB2Manager::GetCurveValueAt(PlayerLevelToItemLevelCurveId, owner level) before bonus/caps"
    );
}
#[test]
fn represented_item_level_curve_clamps_owner_level_by_content_tuning_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_159_u32;
    session.set_player_level_like_cpp(80);
    install_represented_item_level_curve_fixture_like_cpp(&mut session, item_id, 55, 9_002);
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 55,
            min_level: 10,
            max_level: 40,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(140),
        "C++ clamps owner level through GetContentTuningData(contentTuningId, true) before evaluating the item-level curve"
    );
}
#[test]
fn represented_item_level_curve_fixed_level_overrides_content_tuning_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_160_u32;
    let item_guid = ObjectGuid::create_item(1, 30_160);
    session.set_player_level_like_cpp(80);
    install_represented_item_level_curve_fixture_like_cpp(&mut session, item_id, 56, 9_003);
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 56,
            min_level: 10,
            max_level: 40,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut item = session.make_inventory_item_object(
        item_guid,
        item_id,
        owner,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_modifier(ItemModifier::TimewalkerLevel, 50);

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, Some(&item)),
        Some(150),
        "C++ fixedLevel/ITEM_MODIFIER_TIMEWALKER_LEVEL bypasses ContentTuning clamp for Item::GetItemLevel"
    );
}
#[test]
fn represented_item_level_curve_missing_data_does_not_fall_back_to_template_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_161_u32;
    session.set_player_level_like_cpp(45);
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_with_scaling_like_cpp(InventoryType::Chest, 0, 0, 9_004),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(WorldSession::MIN_ITEM_LEVEL_LIKE_CPP),
        "C++ GetCurveValueAt returns 0 for missing curve data; Item::GetItemLevel then clamps to MIN_ITEM_LEVEL instead of falling back to proto ItemLevel"
    );
}
#[test]
fn represented_item_level_applies_min_cap_to_equipable_item_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_049_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level: 150,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(150),
        "C++ Item::GetItemLevel(owner) raises equipable items below MinItemLevel"
    );
}
#[test]
fn represented_item_level_respects_min_cap_cutoff_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_050_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level_cutoff: 120,
        min_item_level: 150,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100),
        "C++ only applies MinItemLevel when itemLevelBeforeUpgrades reaches MinItemLevelCutoff"
    );

    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level_cutoff: 90,
        min_item_level: 150,
        ..Default::default()
    });
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(150),
        "C++ cutoff uses itemLevelBeforeUpgrades, not the capped item level"
    );
}
#[test]
fn represented_item_level_applies_max_cap_to_equipable_item_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_051_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 200,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        max_item_level: 120,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(120),
        "C++ Item::GetItemLevel(owner) caps equipable items above MaxItemLevel"
    );
}
#[test]
fn represented_item_level_ignores_max_cap_with_pvp_cap_flag_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_052_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(
                    InventoryType::Chest,
                    ItemFlags3::IgnoreItemLevelCapInPvp as u32,
                ),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 200,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        max_item_level: 120,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(200),
        "C++ clears maxItemLevel when ITEM_FLAG3_IGNORE_ITEM_LEVEL_CAP_IN_PVP is present"
    );
}
#[test]
fn represented_item_level_area_scaling_activates_on_battleground_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_154_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 25);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(489, wow_data::map::MAP_BATTLEGROUND, 0),
    ])));
    session.set_player_map_position_like_cpp(489, Position::ZERO);

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(125)
    );
}
