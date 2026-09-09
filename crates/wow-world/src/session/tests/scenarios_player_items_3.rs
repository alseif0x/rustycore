//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_condition_total_avg_item_level_rejects_twohand_candidate_with_offhand_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_mainhand_item_id = 30_039_u32;
    let equipped_offhand_item_id = 30_040_u32;
    let rejected_twohand_item_id = 30_041_u32;
    let equipped_mainhand_guid = ObjectGuid::create_item(1, 30_039);
    let equipped_offhand_guid = ObjectGuid::create_item(1, 30_040);
    let rejected_twohand_guid = ObjectGuid::create_item(1, 30_041);
    let player_guid = ObjectGuid::create_player(1, 174);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelTwoHandOffhandRejected".to_string(),
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
        represented_test_item_record_like_cpp(
            equipped_mainhand_item_id,
            InventoryType::Weapon,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe as u8,
        ),
        represented_test_item_record_like_cpp(
            equipped_offhand_item_id,
            InventoryType::Shield,
            ItemClass::Armor,
            ItemSubClassArmor::Shield as u8,
        ),
        represented_test_item_record_like_cpp(
            rejected_twohand_item_id,
            InventoryType::Weapon2Hand,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe2 as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    equipped_mainhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon, 0),
                ),
                (
                    equipped_offhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Shield, 0),
                ),
                (
                    rejected_twohand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
            ],
            [
                (
                    equipped_mainhand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon as i8,
                    },
                ),
                (
                    equipped_offhand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Shield as i8,
                    },
                ),
                (
                    rejected_twohand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 300,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon2Hand as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: equipped_mainhand_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 100,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: equipped_offhand_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 100,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: rejected_twohand_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 300,
            flags: [0; 4],
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        equipped_mainhand_guid,
        equipped_mainhand_item_id,
        InventoryType::Weapon,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        equipped_offhand_guid,
        equipped_offhand_item_id,
        InventoryType::Shield,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        rejected_twohand_guid,
        rejected_twohand_item_id,
        InventoryType::Weapon2Hand,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 12.5,
        "C++ UpdateAverageItemLevelTotal calls CanEquipItem(..., swap=true, not_loading=false); a non-TitanGrip 2H candidate is rejected while the offhand slot is occupied"
    );
    assert_eq!(
        context.avg_equipped_item_level, 12.5,
        "C++ UpdateAverageItemLevelEquipped still counts the currently equipped mainhand and offhand only"
    );
}
#[test]
fn represented_condition_total_avg_item_level_counts_contained_items_for_max_count_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_042_u32;
    let bag_item_id = 30_043_u32;
    let limited_chest_item_id = 30_044_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_042);
    let bag_guid = ObjectGuid::create_item(1, 30_043);
    let contained_chest_guid = ObjectGuid::create_item(1, 30_044);
    let second_contained_chest_guid = ObjectGuid::create_item(1, 30_045);
    let player_guid = ObjectGuid::create_player(1, 175);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelContainedMaxCountRejected".to_string(),
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
            bag_item_id,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
        represented_test_item_record_like_cpp(
            limited_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
    ])));
    let mut limited_sparse = sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0);
    limited_sparse.max_count = 1;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    equipped_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (
                    bag_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Bag, 0),
                ),
                (limited_chest_item_id, limited_sparse),
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
                    limited_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 300,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: equipped_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 100,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: limited_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 300,
            flags: [0; 4],
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        equipped_chest_guid,
        equipped_chest_item_id,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_BAG_START,
        bag_guid,
        bag_item_id,
        InventoryType::Bag,
    );
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut contained_chest = session.make_inventory_item_object(
        contained_chest_guid,
        limited_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        0,
    );
    contained_chest.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(contained_chest);
    let mut second_contained_chest = session.make_inventory_item_object(
        second_contained_chest_guid,
        limited_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        1,
    );
    second_contained_chest.set_container_guid_and_slot(bag_guid, 1);
    session.insert_inventory_item_object(second_contained_chest);

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 6.25,
        "C++ CanEquipItem calls CanTakeMoreSimilarItems, which counts contained items through GetItemCount(..., inBankAlso=true); two MaxCount=1 contained candidates are both rejected"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still ignores contained inventory candidates"
    );
}
#[test]
fn represented_condition_total_avg_item_level_uses_represented_bag_contents_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_020_u32;
    let bag_item_id = 30_021_u32;
    let contained_chest_item_id = 30_022_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_020);
    let bag_guid = ObjectGuid::create_item(1, 30_021);
    let contained_chest_guid = ObjectGuid::create_item(1, 30_022);
    let player_guid = ObjectGuid::create_player(1, 167);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelBagContents".to_string(),
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
            bag_item_id,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
        represented_test_item_record_like_cpp(
            contained_chest_item_id,
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
                    bag_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Bag, 0),
                ),
                (
                    contained_chest_item_id,
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
                    contained_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 220,
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
        INVENTORY_SLOT_BAG_START,
        bag_guid,
        bag_item_id,
        InventoryType::Bag,
    );
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut contained_chest = session.make_inventory_item_object(
        contained_chest_guid,
        contained_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        0,
    );
    contained_chest.set_container_guid_and_slot(bag_guid, INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(contained_chest);

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 13.75,
        "C++ UpdateAverageItemLevelTotal uses ForEachItem(Everywhere), so represented bag-contained candidates can replace equipped lower item-level candidates"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still ignores bag-contained items"
    );
}
#[test]
fn represented_condition_total_avg_item_level_uses_represented_bank_item_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_023_u32;
    let bank_chest_item_id = 30_024_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_023);
    let bank_chest_guid = ObjectGuid::create_item(1, 30_024);
    let player_guid = ObjectGuid::create_player(1, 168);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelBankItem".to_string(),
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
            bank_chest_item_id,
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
                    bank_chest_item_id,
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
                    bank_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 240,
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
        BANK_SLOT_ITEM_START,
        bank_chest_guid,
        bank_chest_item_id,
        InventoryType::Chest,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 15.0,
        "C++ ItemSearchLocation::Everywhere includes bank slots, so represented bank candidates can replace equipped lower item-level candidates"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still ignores bank items"
    );
}
#[test]
fn represented_condition_total_avg_item_level_uses_represented_bank_bag_contents_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_025_u32;
    let bank_bag_item_id = 30_026_u32;
    let contained_chest_item_id = 30_027_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_025);
    let bank_bag_guid = ObjectGuid::create_item(1, 30_026);
    let contained_chest_guid = ObjectGuid::create_item(1, 30_027);
    let player_guid = ObjectGuid::create_player(1, 169);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelBankBagContents".to_string(),
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
            bank_bag_item_id,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
        represented_test_item_record_like_cpp(
            contained_chest_item_id,
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
                    bank_bag_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Bag, 0),
                ),
                (
                    contained_chest_item_id,
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
                    contained_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 260,
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
        BANK_SLOT_BAG_START,
        bank_bag_guid,
        bank_bag_item_id,
        InventoryType::Bag,
    );
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut contained_chest = session.make_inventory_item_object(
        contained_chest_guid,
        contained_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        0,
    );
    contained_chest.set_container_guid_and_slot(bank_bag_guid, BANK_SLOT_BAG_START);
    session.insert_inventory_item_object(contained_chest);

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 16.25,
        "C++ ItemSearchLocation::Everywhere includes bank bag contents, so represented bank-bag candidates can replace equipped lower item-level candidates"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still ignores bank-bag contents"
    );
}
#[test]
fn represented_condition_total_avg_item_level_uses_represented_reagent_bank_contents_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_028_u32;
    let reagent_bag_item_id = 30_029_u32;
    let contained_chest_item_id = 30_030_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_028);
    let reagent_bag_guid = ObjectGuid::create_item(1, 30_029);
    let contained_chest_guid = ObjectGuid::create_item(1, 30_030);
    let player_guid = ObjectGuid::create_player(1, 170);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelReagentBankContents".to_string(),
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
            reagent_bag_item_id,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
        represented_test_item_record_like_cpp(
            contained_chest_item_id,
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
                    reagent_bag_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Bag, 0),
                ),
                (
                    contained_chest_item_id,
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
                    contained_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 280,
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
        REAGENT_BAG_SLOT_START,
        reagent_bag_guid,
        reagent_bag_item_id,
        InventoryType::Bag,
    );
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut contained_chest = session.make_inventory_item_object(
        contained_chest_guid,
        contained_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        0,
    );
    contained_chest.set_container_guid_and_slot(reagent_bag_guid, REAGENT_BAG_SLOT_START);
    session.insert_inventory_item_object(contained_chest);

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 17.5,
        "C++ ItemSearchLocation::Everywhere includes reagent-bank contents, so represented reagent-bank candidates can replace equipped lower item-level candidates"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still ignores reagent-bank contents"
    );
}
#[test]
fn represented_condition_total_avg_item_level_does_not_count_same_ring_twice_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let ring_item_id = 30_019_u32;
    let ring_guid = ObjectGuid::create_item(1, 30_019);
    let player_guid = ObjectGuid::create_player(1, 166);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelRingDuplicate".to_string(),
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
            ring_item_id,
            InventoryType::Finger,
            ItemClass::Armor,
            ItemSubClassArmor::Miscellaneous as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                ring_item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Finger, 0),
            )],
            [(
                ring_item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Finger as i8,
                },
            )],
        ),
    ));
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        ring_guid,
        ring_item_id,
        InventoryType::Finger,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 6.25,
        "C++ ForEachEquipmentSlot duplicate-GUID gate prevents the same ring candidate from filling both finger slots"
    );
}
