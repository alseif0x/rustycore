use super::*;

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
    session.set_item_store(Arc::new(ItemStore::from_records([represented_test_item_record_like_cpp(
        mainhand_item_id,
        InventoryType::Weapon2Hand,
        ItemClass::Weapon,
        ItemSubClassWeapon::Axe2 as u8,
    )])));
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
