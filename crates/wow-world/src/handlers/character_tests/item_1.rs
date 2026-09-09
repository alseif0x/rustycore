//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn continue_login_inventory_reads_cross_the_typed_lifecycle_port() {
    let source = include_str!("../character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");

    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory"));
    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory"));
    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage"));
    for statement in [
        "CharStatements::SEL_CHAR_EQUIPMENT",
        "CharStatements::SEL_CHAR_BAG_CONTENTS",
        "CharStatements::SEL_CHAR_VOID_STORAGE",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}
#[test]
fn continue_login_item_repairs_cross_the_typed_lifecycle_port() {
    let source = include_str!("../character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");

    assert_eq!(
        handler
            .matches("persist_login_item_repairs_like_cpp")
            .count(),
        2
    );
    assert!(handler.contains("PlayerLoginItemRepairActionLikeCpp::ClearRefundable"));
    assert!(handler.contains("PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad"));
    assert!(!handler.contains("SqlTransaction::new()"));
    for statement in [
        "CharStatements::DEL_ITEM_REFUND_INSTANCE",
        "CharStatements::UPD_ITEM_INSTANCE_FLAGS",
        "CharStatements::UPD_ITEM_INSTANCE_ON_LOAD",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}
#[tokio::test]
async fn account_item_appearance_load_preserves_independent_query_failure_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([AccountCollectionLoadOutcomeLikeCpp::Loaded(
        AccountCollectionLoadedLikeCpp::ItemAppearances {
            appearance_blocks: AccountCollectionRowsLikeCpp::Failed {
                reason: "appearance read failed".to_owned(),
            },
            favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![91]),
        },
    )]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port);

    session.load_account_item_appearances_like_cpp().await;

    assert!(
        session
            .account_transmog_active_player_rows_like_cpp()
            .is_empty()
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(91, true));
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
#[tokio::test]
async fn save_equipment_set_new_equipment_normalizes_and_sends_id_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 55);
    session.insert_inventory_item_like_cpp(
        0,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut pieces = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = item_guid;
    let appearances = [77; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            0,
            pieces,
            appearances,
            [12, 34],
            Some(2),
            "Tank",
            "INV_Helmet_01",
        ))
        .await;

    let (generated_guid, set_type, set_id) = read_equipment_set_id(send_rx.try_recv().unwrap());
    assert_eq!((generated_guid, set_type, set_id), (1, 0, 7));
    let saved = session
        .represented_equipment_set_like_cpp(generated_guid)
        .unwrap();
    assert_eq!(saved.guid, generated_guid);
    assert_eq!(saved.set_id, 7);
    assert_eq!(saved.set_name, "Tank");
    assert_eq!(saved.set_icon, "INV_Helmet_01");
    assert_eq!(saved.pieces[0], item_guid);
    assert_eq!(saved.appearances[0], 0);
    assert_eq!(saved.appearances[1], 0);
    assert_eq!(saved.enchants, [0, 0]);
    assert_eq!(saved.assigned_spec_index, 2);
    assert_eq!(
        saved.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New
    );
    assert_ne!(saved.ignore_mask & (1 << 1), 0);
}
#[tokio::test]
async fn save_equipment_set_requires_process_wide_guid_allocator() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(1);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "No allocator",
            "INV_Misc_QuestionMark",
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.represented_equipment_set_like_cpp(1).is_none());
}
#[tokio::test]
async fn concurrent_sessions_share_equipment_and_transmog_set_guid_namespace_like_cpp() {
    let (mut equipment_session, equipment_rx) = make_session_with_send_capacity(1);
    let (mut transmog_session, transmog_rx) = make_session_with_send_capacity(1);
    let generator = Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(400));
    equipment_session.set_equipment_set_guid_generator_like_cpp(Arc::clone(&generator));
    transmog_session.set_equipment_set_guid_generator_like_cpp(Arc::clone(&generator));
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    tokio::join!(
        equipment_session.handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Equipment",
            "INV_Sword_01",
        )),
        transmog_session.handle_save_equipment_set(save_equipment_set_packet(
            1,
            0,
            8,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Transmog",
            "INV_Chest_Cloth_01",
        )),
    );

    let equipment = read_equipment_set_id(equipment_rx.try_recv().unwrap());
    let transmog = read_equipment_set_id(transmog_rx.try_recv().unwrap());
    let mut guids = [equipment.0, transmog.0];
    guids.sort_unstable();
    assert_eq!(guids, [400, 401]);
    assert_eq!(equipment.1, 0);
    assert_eq!(transmog.1, 1);
    assert_eq!(generator.next_after_max_used(), 402);
}
#[tokio::test]
async fn save_equipment_set_existing_marks_changed_without_id_packet_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            100,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Dps",
            "INV_Sword_01",
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    let saved = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(saved.set_name, "Dps");
    assert_eq!(saved.assigned_spec_index, -1);
    assert_eq!(
        saved.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Changed
    );
}
#[tokio::test]
async fn save_equipment_set_negative_type_follows_cpp_non_equipment_branch() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            -1,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Odd",
            "INV_Odd",
        ))
        .await;

    let (generated_guid, set_type, set_id) = read_equipment_set_id(send_rx.try_recv().unwrap());
    assert_eq!((generated_guid, set_type, set_id), (1, -1, 7));
    let saved = session
        .represented_equipment_set_like_cpp(generated_guid)
        .unwrap();
    assert_eq!(saved.raw_set_type, -1);
    assert_eq!(
        saved.set_type,
        crate::session::RepresentedEquipmentSetTypeLikeCpp::Transmog
    );
}
#[tokio::test]
async fn save_equipment_set_rejects_equipment_guid_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_inventory_item_like_cpp(
        0,
        InventoryItem {
            guid: ObjectGuid::create_item(1, 55),
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut pieces = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = ObjectGuid::create_item(1, 99);

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            0,
            pieces,
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Bad",
            "INV_Bad",
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.represented_equipment_set_like_cpp(1).is_none());
}
#[tokio::test]
async fn assign_equipment_set_spec_updates_matching_equipment_set_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 2))
        .await;

    let equipment_set = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(equipment_set.assigned_spec_index, 2);
    assert_eq!(
        equipment_set.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Changed
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn assign_equipment_set_spec_preserves_new_state_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New,
        ),
    );

    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 3))
        .await;

    let equipment_set = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(equipment_set.assigned_spec_index, 3);
    assert_eq!(
        equipment_set.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New
    );
}
#[tokio::test]
async fn assign_equipment_set_spec_ignores_transmog_missing_and_out_of_range_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::transmog(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );
    session.insert_represented_equipment_set_like_cpp(
        200,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            8,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 4))
        .await;
    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(99, 4))
        .await;
    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(
            crate::session::MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
            4,
        ))
        .await;

    assert_eq!(
        session
            .represented_equipment_set_like_cpp(100)
            .unwrap()
            .assigned_spec_index,
        -1
    );
    assert_eq!(
        session
            .represented_equipment_set_like_cpp(200)
            .unwrap()
            .assigned_spec_index,
        -1
    );
}
#[tokio::test]
async fn delete_equipment_set_marks_existing_set_deleted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(100))
        .await;

    let equipment_set = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(
        equipment_set.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Deleted
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn delete_equipment_set_removes_new_set_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New,
        ),
    );

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(100))
        .await;

    assert!(session.represented_equipment_set_like_cpp(100).is_none());
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn delete_equipment_set_missing_id_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(404))
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn use_equipment_set_moves_direct_inventory_item_and_sends_result_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 55);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[0] = item_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0102_0304_0506_0708, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102_0304_0506_0708, 0)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 0)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
}
#[tokio::test]
async fn use_equipment_set_applies_represented_item_mods_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 66);
    let entry_id = 111;
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(strength_item_stats_store(entry_id, 11)));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: 66,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(item);
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[EQUIPMENT_SLOT_MAINHAND as usize] = item_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0203, items))
        .await;

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        11,
        "C++ HandleUseEquipmentSet reaches SwapItem -> EquipItem -> _ApplyItemMods"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UseEquipmentSetResult
        ]
    );
}
#[tokio::test]
async fn use_equipment_set_empty_slot_unequips_to_backpack_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 56);
    session.insert_inventory_item_like_cpp(
        1,
        InventoryItem {
            guid: item_guid,
            entry_id: 101,
            db_guid: 56,
            inventory_type: Some(InventoryType::Neck as u8),
        },
    );

    session
        .handle_use_equipment_set(use_equipment_set_packet(
            0x0102_0304_0506_0709,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
        ))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102_0304_0506_0709, 0)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 1)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}
