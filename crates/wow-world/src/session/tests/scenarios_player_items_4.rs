//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_player_pvp_item_level_mode_follows_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_567);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PvpItemLevelOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    assert!(session.set_represented_using_pvp_item_levels_like_cpp(true));
    assert_eq!(
        session.resolved_using_pvp_item_levels_like_cpp(),
        Some(true)
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_using_pvp_item_levels_like_cpp(),
        Some(true)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_using_pvp_item_levels_like_cpp(), None);
    assert!(!session.set_represented_using_pvp_item_levels_like_cpp(true));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().using_pvp_item_levels
            }),
        Some(false)
    );
}
#[test]
fn canonical_player_item_modifier_runtime_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_569);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ItemModifierOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(
        session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
            min_item_level: 100,
            ..Default::default()
        })
    );
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 17 }
    ));
    assert!(
        session
            .mutate_player_item_modifier_runtime_like_cpp(|runtime| {
                runtime.item_set_effects.insert(
                    700,
                    RepresentedItemSetEffectLikeCpp {
                        item_set_id: 700,
                        equipped_items: std::collections::HashSet::from([ObjectGuid::create_item(
                            1, 88,
                        )]),
                        set_bonuses: BTreeSet::from([35]),
                    },
                );
            })
            .is_some()
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .shield_block_value,
        17
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(700)
            .expect("canonical item-set effect")
            .set_bonuses,
        BTreeSet::from([35])
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 29 }
    ));
    assert!(
        session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
            max_item_level: 200,
            ..Default::default()
        })
    );
    let detached = session
        .player_item_modifier_runtime_snapshot_like_cpp()
        .expect("detached canonical item-modifier owner");
    assert_eq!(detached.bonuses.shield_block_value, 29);
    assert_eq!(detached.item_level_caps.max_item_level, 200);

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(
        session
            .player_item_modifier_runtime_snapshot_like_cpp()
            .is_none()
    );
    assert!(!session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 99 }
    ));
    assert!(
        !session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
            max_item_level: 999,
            ..Default::default()
        })
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().item_modifiers.clone()
            }),
        Some(wow_entities::PlayerItemModifierRuntimeStateLikeCpp::default())
    );
}
#[test]
fn canonical_player_inventory_capacity_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_561);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InventoryCapacityOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_player_inventory_slot_count_like_cpp(24));
    assert!(session.set_player_bank_bag_slot_count_like_cpp(3));
    assert!(session.set_represented_bank_bag_slot_flag_like_cpp(2, 0x40));
    assert_eq!(
        session.resolved_player_inventory_slot_count_like_cpp(),
        Some(24)
    );
    assert_eq!(
        session.resolved_player_bank_bag_slot_count_like_cpp(),
        Some(3)
    );
    assert_eq!(
        session.represented_bank_bag_slot_flag_like_cpp(2),
        Some(0x40)
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_player_inventory_slot_count_like_cpp(),
        Some(24)
    );
    assert_eq!(
        session.resolved_player_bank_bag_slot_count_like_cpp(),
        Some(3)
    );
    assert_eq!(
        session.represented_bank_bag_slot_flag_like_cpp(2),
        Some(0x40)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_inventory_slot_count(30);
    replacement.set_bank_bag_slot_count(7);
    assert!(replacement.set_bank_bag_slot_flag_value_like_cpp(2, 0x80));
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(
        session.resolved_player_inventory_slot_count_like_cpp(),
        None
    );
    assert_eq!(session.resolved_player_bank_bag_slot_count_like_cpp(), None);
    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), None);
    assert!(!session.set_player_inventory_slot_count_like_cpp(16));
    assert!(!session.set_player_bank_bag_slot_count_like_cpp(1));
    assert!(!session.set_represented_bank_bag_slot_flag_like_cpp(2, 0));
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.inventory_slot_count(),
                player.bank_bag_slot_count(),
                player.bank_bag_slot_flag_value_like_cpp(2),
            )),
        Some((30, 7, Some(0x80)))
    );
}
#[test]
fn canonical_player_inventory_runtime_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_562);
    let item_guid = ObjectGuid::create_item(1, 8_001);
    let replacement_item_guid = ObjectGuid::create_item(1, 8_002);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InventoryRuntimeOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    let inventory_item = InventoryItem {
        guid: item_guid,
        entry_id: 12_345,
        db_guid: 8_001,
        inventory_type: Some(1),
    };
    let mut item_object = Item::default();
    item_object.object_mut().create(item_guid);
    item_object.object_mut().set_entry(inventory_item.entry_id);
    item_object.set_count(5);
    assert_eq!(
        session.insert_inventory_item_like_cpp(INVENTORY_SLOT_ITEM_START, inventory_item.clone()),
        None
    );
    assert_eq!(session.insert_inventory_item_object(item_object), None);
    session.insert_buyback_item_like_cpp(BUYBACK_SLOT_START, inventory_item.clone());
    session.set_buyback_slot_metadata_like_cpp(BUYBACK_SLOT_START, 77, 88);

    assert_eq!(
        session
            .resolved_inventory_item_like_cpp(INVENTORY_SLOT_ITEM_START)
            .as_ref()
            .map(|item| (item.guid, item.entry_id)),
        Some((item_guid, 12_345))
    );
    assert_eq!(
        session
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.count()),
        Some(5)
    );
    assert_eq!(
        session
            .resolved_buyback_price_like_cpp()
            .map(|prices| prices[0]),
        Some(77)
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.count()),
        Some(5)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_inventory_item = InventoryItem {
        guid: replacement_item_guid,
        entry_id: 54_321,
        db_guid: 8_002,
        inventory_type: Some(2),
    };
    let mut replacement_item_object = Item::default();
    replacement_item_object
        .object_mut()
        .create(replacement_item_guid);
    replacement_item_object
        .object_mut()
        .set_entry(replacement_inventory_item.entry_id);
    replacement_item_object.set_count(9);
    replacement
        .inventory_runtime_mut_like_cpp()
        .inventory_items_mut()
        .insert(INVENTORY_SLOT_ITEM_START, replacement_inventory_item);
    replacement
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .insert(replacement_item_guid, replacement_item_object);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_inventory_items_like_cpp(), None);
    assert_eq!(session.resolved_inventory_item_objects_like_cpp(), None);
    assert_eq!(
        session.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.inventory_items_mut().clear();
        }),
        None
    );
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                let inventory = player.inventory_runtime_like_cpp();
                (
                    inventory
                        .inventory_items()
                        .get(&INVENTORY_SLOT_ITEM_START)
                        .map(|item| item.guid),
                    inventory
                        .item_objects()
                        .get(&replacement_item_guid)
                        .map(|item| item.count()),
                )
            }),
        Some((Some(replacement_item_guid), Some(9)))
    );
}
#[test]
fn canonical_access_requirement_item_or_item2_matches_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 85);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessItem".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    install_access_notification_stores_like_cpp(&mut session);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.item = 700;
    requirement.item2 = 701;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("missing key item notification"),
        PrintNotification {
            notify_text: "You must be at least level 0 and have The Workshop Key to enter."
                .to_string(),
        }
        .to_bytes()
    );
    assert_eq!(
        send_rx.try_recv().expect("missing key item abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );

    let item_guid = ObjectGuid::create_item(1, 701);
    session.insert_inventory_item_like_cpp(
        23,
        InventoryItem {
            guid: item_guid,
            entry_id: 701,
            db_guid: 701,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        23,
    );
    session.insert_inventory_item_object(item);

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_access_requirement_missing_item_sends_notification_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessItemNotify".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    install_access_notification_stores_like_cpp(&mut session);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.level_min = 80;
    requirement.item = 701;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("missing item notification"),
        PrintNotification {
            notify_text: "You must be at least level 0 and have The Scarlet Key to enter."
                .to_string(),
        }
        .to_bytes()
    );
    assert_eq!(
        send_rx.try_recv().expect("missing item abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn loaded_player_visible_items_for_create_includes_loaded_enchant_visual_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 90_526);
    let item_guid = ObjectGuid::create_item(1, 90_527);
    session.set_player_guid(Some(player_guid));
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 908,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            item_visual: 44,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: 90_527,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 908, 0, 0);
    session.insert_inventory_item_object(item);

    let visible_items = session
        .loaded_player_visible_items_for_create_like_cpp()
        .expect("fixture canonical inventory owner");

    assert_eq!(
        visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        (700, 0, 44)
    );
}
#[test]
fn equipment_set_save_marks_written_sets_unchanged_and_removes_deleted_like_cpp() {
    let (mut session, _, _) = make_session();
    session.mark_represented_equipment_sets_loaded_like_cpp();

    let mut changed = RepresentedEquipmentSetLikeCpp::equipment(
        7,
        0,
        RepresentedEquipmentSetUpdateStateLikeCpp::Changed,
    );
    changed.guid = 700;
    session.insert_represented_equipment_set_like_cpp(700, changed);

    let mut deleted = RepresentedEquipmentSetLikeCpp::transmog(
        8,
        -1,
        RepresentedEquipmentSetUpdateStateLikeCpp::Deleted,
    );
    deleted.guid = 800;
    session.insert_represented_equipment_set_like_cpp(800, deleted);

    session.mark_equipment_sets_saved_like_cpp();

    assert_eq!(
        session
            .represented_equipment_set_like_cpp(700)
            .unwrap()
            .state,
        RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged
    );
    assert!(session.represented_equipment_set_like_cpp(800).is_none());
}
#[test]
fn canonical_player_saved_equipment_and_void_storage_follow_handle_generation_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 59_199);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    install_stackable_test_item_template(&mut session, 19_019, 1);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "StorageOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    session.clear_represented_equipment_sets_like_cpp();
    assert!(session.load_represented_equipment_set_row_like_cpp(
        700,
        7,
        "Tank".to_string(),
        "INV_Shield".to_string(),
        0,
        1,
        [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    ));
    session.mark_represented_equipment_sets_loaded_like_cpp();
    session.clear_represented_void_storage_like_cpp();
    assert!(session.load_represented_void_storage_row_like_cpp(
        3,
        RepresentedVoidStorageItemLikeCpp {
            item_id: 900,
            item_entry: 19_019,
            creator_guid: player_guid,
            fixed_scaling_level: 80,
            random_properties_id: 0,
            random_properties_seed: 0,
            context: 0,
        },
    ));
    session.mark_represented_void_storage_loaded_like_cpp();
    assert_eq!(
        session
            .represented_load_equipment_set_packet_like_cpp()
            .expect("active canonical Player")
            .sets[0]
            .set_name,
        "Tank"
    );
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(159)
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session
            .represented_void_storage_item_by_id_like_cpp(900)
            .map(|(slot, item)| (slot, item.item_entry)),
        Some((3, 19_019))
    );

    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().equipment_sets.insert(
        701,
        RepresentedEquipmentSetLikeCpp::equipment(
            8,
            0,
            RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );
    replacement.gameplay_state_mut().equipment_sets_loaded = true;
    replacement.gameplay_state_mut().void_storage_items =
        vec![None; wow_entities::PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP];
    replacement.gameplay_state_mut().void_storage_items[4] =
        Some(RepresentedVoidStorageItemLikeCpp {
            item_id: 901,
            item_entry: 19_019,
            creator_guid: player_guid,
            fixed_scaling_level: 70,
            random_properties_id: 0,
            random_properties_seed: 0,
            context: 0,
        });
    replacement.gameplay_state_mut().void_storage_loaded = true;
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(
        session
            .represented_load_equipment_set_packet_like_cpp()
            .is_none()
    );
    assert!(
        session
            .represented_void_storage_free_slots_like_cpp()
            .is_none()
    );
    assert!(!session.delete_represented_equipment_set_like_cpp(701));
    assert!(
        session
            .delete_represented_void_storage_item_like_cpp(4)
            .is_none()
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.gameplay_state().equipment_sets.contains_key(&701),
                player.gameplay_state().void_storage_items[4]
                    .as_ref()
                    .map(|item| item.item_id),
            ),),
        Some((true, Some(901)))
    );
}
#[test]
fn player_currency_item_refund_ignores_caps_and_total_counters_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_race = 1;
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        wow_data::CurrencyTypesEntry {
            max_qty: 100,
            max_earnable_per_week: 50,
            flags: wow_constants::CurrencyTypesFlags::TRACK_QUANTITY,
            flags_b: wow_constants::CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
            ..currency_entry(395)
        },
    ])));
    session.player_currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 95,
            weekly_quantity: 49,
            tracked_quantity: 11,
            increased_cap_quantity: 0,
            earned_quantity: 12,
            flags: 0,
        },
    );

    let delta = session.add_currency_item_refund(395, 20).unwrap().unwrap();
    assert_eq!(delta.currency_id, 395);
    assert_eq!(delta.amount, 20);
    assert_eq!(delta.quantity, 115);
    assert_eq!(delta.weekly_quantity, Some(49));
    assert_eq!(delta.max_quantity, Some(100));
    assert_eq!(delta.total_earned, Some(12));

    let currency = session.player_currencies.get(&395).unwrap();
    assert_eq!(currency.quantity, 115);
    assert_eq!(currency.weekly_quantity, 49);
    assert_eq!(currency.tracked_quantity, 11);
    assert_eq!(currency.earned_quantity, 12);
    assert_eq!(currency.state, PlayerCurrencyState::Changed);
}
#[test]
fn send_equip_error_preserves_cpp_item_level_and_limit_fields() {
    let (session, _, send_rx) = make_session();
    let item1 = ObjectGuid::new(0, 0x0102);
    let item2 = ObjectGuid::new(0, 0x0506);
    let expected = InventoryChangeFailure::new(InventoryResult::CantEquipLevelI, item1, item2)
        .with_level(42)
        .to_bytes();

    session.send_equip_error(
        InventoryResult::CantEquipLevelI,
        Some(item1),
        Some(item2),
        42,
        0,
    );
    assert_eq!(send_rx.try_recv().unwrap(), expected);

    let expected =
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryEquippedExceededIs)
            .with_limit_category(777)
            .to_bytes();
    session.send_equip_error(
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs,
        None,
        None,
        0,
        777,
    );
    assert_eq!(send_rx.try_recv().unwrap(), expected);
}
#[test]
fn apply_enchantment_random_suffix_ref_uses_cpp_abs_lookup() {
    let (mut session, _, _) = make_session();
    session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
        ItemRandomSuffixEntry {
            id: 77,
            enchantments: [901, 900, 902, 0, 0],
            allocation_pct: [1_000, 2_000, 3_000, 0, 0],
        },
    ])));

    let suffix = session
        .apply_enchantment_random_suffix_ref(-77)
        .expect("random suffix should resolve by abs(RandomPropertiesID)");

    assert_eq!(suffix.id, 77);
    assert_eq!(suffix.enchantments, [901, 900, 902, 0, 0]);
    assert_eq!(suffix.allocation_pct, [1_000, 2_000, 3_000, 0, 0]);
    assert!(session.apply_enchantment_random_suffix_ref(0).is_none());
    assert!(session.apply_enchantment_random_suffix_ref(-78).is_none());
}
