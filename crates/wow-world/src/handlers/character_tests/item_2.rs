//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn use_equipment_set_ignored_guid_preserves_slot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 57);
    session.insert_inventory_item_like_cpp(
        2,
        InventoryItem {
            guid: item_guid,
            entry_id: 102,
            db_guid: 57,
            inventory_type: Some(InventoryType::Shoulders as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[2] = ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64);

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0102, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102, 0)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 2)
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
async fn use_equipment_set_skips_non_weapon_slots_in_combat_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.in_combat = true;
    let head_guid = ObjectGuid::create_item(1, 58);
    let mainhand_guid = ObjectGuid::create_item(1, 59);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: head_guid,
            entry_id: 103,
            db_guid: 58,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START + 1,
        InventoryItem {
            guid: mainhand_guid,
            entry_id: 104,
            db_guid: 59,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[0] = head_guid;
    items[EQUIPMENT_SLOT_MAINHAND as usize] = mainhand_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0103, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0103, 0)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 0)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        head_guid
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .unwrap()
            .guid,
        mainhand_guid
    );
}
#[test]
fn direct_inventory_swap_persistence_plan_replaces_both_occupied_positions_like_cpp() {
    let src = InventoryItem {
        guid: ObjectGuid::create_item(1, 58),
        entry_id: 103,
        db_guid: 58,
        inventory_type: None,
    };
    let dst = InventoryItem {
        guid: ObjectGuid::create_item(1, 59),
        entry_id: 104,
        db_guid: 59,
        inventory_type: None,
    };

    assert_eq!(
        plan_direct_inventory_swap_persistence_like_cpp(35, 36, Some(&src), Some(&dst)),
        vec![
            DirectInventoryPositionUpdateLikeCpp {
                slot: 36,
                item_db_guid: 58,
            },
            DirectInventoryPositionUpdateLikeCpp {
                slot: 35,
                item_db_guid: 59,
            },
        ],
        "C++ _SaveInventory replaces each changed item's final position in one transaction"
    );
}
#[test]
fn direct_inventory_swap_persistence_plan_moves_into_empty_position_like_cpp() {
    let src = InventoryItem {
        guid: ObjectGuid::create_item(1, 60),
        entry_id: 105,
        db_guid: 60,
        inventory_type: None,
    };

    assert_eq!(
        plan_direct_inventory_swap_persistence_like_cpp(35, 36, Some(&src), None),
        vec![DirectInventoryPositionUpdateLikeCpp {
            slot: 36,
            item_db_guid: 60,
        }]
    );
}
#[tokio::test]
async fn swap_inv_item_empty_source_returns_without_moving_destination_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let destination_guid = ObjectGuid::create_item(1, 62);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START + 1,
        InventoryItem {
            guid: destination_guid,
            entry_id: 107,
            db_guid: 62,
            inventory_type: None,
        },
    );

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1,)
            .map(|item| item.guid),
        Some(destination_guid)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::SwapItem silently returns when the source is empty"
    );
}
#[tokio::test]
async fn swap_inv_item_commit_failure_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_bank_move_item_fixture(&mut session, 106, 1);
    let item_guid = insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 106, 61, 1);
    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(item_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::InventoryChangeFailure]
    );
}
#[tokio::test]
async fn auto_equip_item_slot_without_persistence_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_equippable_item_fixture(&mut session, 105, InventoryType::Weapon, None);
    let item_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        105,
        60,
        InventoryType::Weapon,
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    let error = send_rx
        .try_recv()
        .expect("missing persistence should report an inventory failure");
    assert_eq!(
        u16::from_le_bytes([error[0], error[1]]),
        ServerOpcodes::InventoryChangeFailure as u16,
    );
}
#[tokio::test]
async fn auto_equip_item_slot_commit_failure_does_not_apply_item_mods_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 109;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, Some(7));
    session.set_item_set_store(Arc::new(wow_data::ItemSetStore::from_entries([
        wow_data::ItemSetEntry {
            id: 706,
            name: "Auto Equip Set".to_string(),
            set_flags: 0,
            required_skill: 0,
            required_skill_rank: 0,
            item_id: std::array::from_fn(|i| if i == 0 { entry_id } else { 0 }),
        },
    ])));
    session.set_item_set_spell_store(Arc::new(wow_data::ItemSetSpellStore::from_entries([
        wow_data::ItemSetSpellEntry {
            id: 22,
            chr_spec_id: 0,
            spell_id: 9022,
            threshold: 1,
            item_set_id: 706,
        },
    ])));
    let item_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        64,
        InventoryType::Weapon,
    );
    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        0,
        "item mods must remain unchanged until the inventory transaction commits"
    );
    assert!(
        session
            .represented_item_set_spell_events_like_cpp()
            .is_empty(),
        "set-bonus events must not be exposed after a failed commit"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(item_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::InventoryChangeFailure],
        "failed persistence emits only the inventory failure"
    );
}
#[test]
fn direct_inventory_move_from_equipment_removes_represented_item_mods_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 65);
    let entry_id = 110;
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(strength_item_stats_store(entry_id, 9)));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: 65,
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
    assert_eq!(
        session.move_represented_direct_inventory_item_with_item_mods_like_cpp(
            INVENTORY_SLOT_ITEM_START,
            EQUIPMENT_SLOT_MAINHAND
        ),
        Some(true)
    );
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        9
    );
    assert_eq!(
        session.move_represented_direct_inventory_item_with_item_mods_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            INVENTORY_SLOT_ITEM_START
        ),
        Some(true)
    );
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        0,
        "C++ RemoveItem calls _ApplyItemMods(..., false) before taking equipped items out of storage"
    );
}
#[tokio::test]
async fn auto_equip_item_slot_rejects_bad_inv_count_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 61);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 106,
            db_guid: 61,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate { items: Vec::new() },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
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
#[tokio::test]
async fn auto_equip_item_slot_rejects_source_position_mismatch_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 62);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 107,
            db_guid: 62,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
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
#[tokio::test]
async fn auto_equip_item_slot_rejects_non_equipment_destination_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 63);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 108,
            db_guid: 63,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)
            .is_none()
    );
}
#[tokio::test]
async fn swap_inv_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            src_slot: 250,
            dst_slot: 251,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before slot validation or equip errors"
    );
}
#[tokio::test]
async fn swap_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_swap_item(SwapItem {
            inv_update: InvUpdate { items: Vec::new() },
            container_slot_a: INVENTORY_SLOT_BAG_START,
            container_slot_b: INVENTORY_SLOT_BAG_START,
            slot_a: 0,
            slot_b: 1,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before container validation"
    );
}
#[tokio::test]
async fn swap_item_validates_source_and_destination_positions_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(2);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_swap_item(SwapItem {
            inv_update: InvUpdate {
                items: vec![(200, 0), (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            container_slot_a: 200,
            container_slot_b: INVENTORY_SLOT_BAG_0,
            slot_a: 0,
            slot_b: INVENTORY_SLOT_ITEM_START,
        })
        .await;
    assert_eq!(
        inventory_failure_result(&realm_rx.try_recv().expect("invalid source realm error")),
        InventoryResult::ItemNotFound as i32
    );
    assert!(instance_rx.try_recv().is_err());

    session
        .handle_swap_item(SwapItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START), (200, 0)],
            },
            container_slot_a: INVENTORY_SLOT_BAG_0,
            container_slot_b: 200,
            slot_a: INVENTORY_SLOT_ITEM_START,
            slot_b: 0,
        })
        .await;
    assert_eq!(
        inventory_failure_result(
            &realm_rx
                .try_recv()
                .expect("invalid destination realm error")
        ),
        InventoryResult::WrongSlot as i32
    );
    assert!(instance_rx.try_recv().is_err());
}
#[tokio::test]
async fn swap_inv_item_rejects_bank_positions_without_bank_access_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_bank_move_item_fixture(&mut session, 120, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 120, 70, 1);

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: wow_entities::BANK_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ silently rejects a bank swap when WorldSession::CanUseBank fails"
    );
}
#[test]
fn equip_destination_uses_visualize_binding_rule_like_cpp() {
    let mut equipped = wow_entities::Item::default();
    equipped.set_bonding(ItemBondingType::OnEquip);
    let unequipped = equipped.clone();
    bind_inventory_item_for_destination_like_cpp(
        &mut equipped,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
    );
    assert!(
        equipped.is_soul_bound(),
        "C++ Player::VisualizeItem binds BIND_ON_EQUIP before EquipItem persistence"
    );
    assert!(
        item_dynamic_flags_changed_like_cpp(&unequipped, &equipped),
        "the equip path must publish ITEM_DATA_DYNAMIC_FLAGS after applying binding"
    );

    let mut backpack = wow_entities::Item::default();
    backpack.set_bonding(ItemBondingType::OnEquip);
    bind_inventory_item_for_destination_like_cpp(
        &mut backpack,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
    );
    assert!(
        !backpack.is_soul_bound(),
        "ordinary C++ _StoreItem destinations keep BIND_ON_EQUIP unbound"
    );

    let mut equipped_bag = wow_entities::Item::default();
    equipped_bag.set_bonding(ItemBondingType::OnEquip);
    bind_inventory_item_for_destination_like_cpp(
        &mut equipped_bag,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
    );
    assert!(equipped_bag.is_soul_bound());
}
#[test]
fn autostore_bank_target_depends_on_source_domain_like_cpp() {
    assert_eq!(
        autostore_bank_target_like_cpp(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,),
        InventoryStorageTargetLikeCpp::Inventory
    );
    assert_eq!(
        autostore_bank_target_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START,),
        InventoryStorageTargetLikeCpp::Bank
    );
}
#[test]
fn bag_exchange_child_updates_runtime_container_and_wire_field_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let old_bag_guid = ObjectGuid::create_item(1, 80);
    let new_bag_guid = ObjectGuid::create_item(1, 81);
    let mut child = wow_entities::Item::new(900);
    child.set_container_guid_and_slot(old_bag_guid, INVENTORY_SLOT_BAG_START);
    child.set_contained_in(old_bag_guid);
    child.set_slot(7);

    relocate_bag_exchange_child_like_cpp(&mut child, new_bag_guid, 2);

    assert_eq!(child.container_guid(), new_bag_guid);
    assert_eq!(child.data().contained_in, new_bag_guid);
    assert_eq!(child.slot(), 2);
    assert_ne!(child.data().contained_in, player_guid);
}
#[test]
fn child_equip_plan_targets_db2_slot_before_parent_moves_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_entry = 123;
    let child_entry = parent_entry;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, parent_entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        parent_entry,
        child_entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        parent_entry,
        77,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        child_entry,
        78,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });

    assert_eq!(
        session
            .plan_inventory_equip_child_like_cpp(
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                parent_guid,
            )
            .expect("child equip preflight"),
        Some(InventoryEquipChildPlanLikeCpp {
            child_guid,
            destination_slot: wow_entities::EQUIPMENT_SLOT_OFFHAND,
            displaced_storage: None,
        })
    );
}
#[test]
fn real_swap_plans_child_for_item_entering_source_equipment_slot_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry = 124;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        entry,
        entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry,
        85,
        InventoryType::Weapon,
    );
    let destination_parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry,
        86,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        entry,
        87,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(destination_parent_guid);
    });

    let plans = session
        .plan_inventory_real_swap_children_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            source_guid,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            destination_parent_guid,
        )
        .expect("reverse-direction child preflight");

    assert_eq!(
        plans,
        vec![InventoryEquipChildPlanLikeCpp {
            child_guid,
            destination_slot: wow_entities::EQUIPMENT_SLOT_OFFHAND,
            displaced_storage: None,
        }]
    );
}
#[test]
fn child_equip_plan_preflights_displaced_equipment_storage_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_entry = 125;
    let child_entry = parent_entry;
    let displaced_entry = parent_entry;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, parent_entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        parent_entry,
        child_entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        parent_entry,
        79,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        child_entry,
        80,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });
    insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        displaced_entry,
        81,
        InventoryType::Weapon,
    );

    let plan = session
        .plan_inventory_equip_child_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            parent_guid,
        )
        .expect("child equip preflight")
        .expect("linked child");
    assert_eq!(
        plan.displaced_storage,
        Some((
            INVENTORY_SLOT_BAG_0,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
        ))
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND,)
            .map(|item| item.entry_id),
        Some(displaced_entry),
        "CanEquipChildItem must not mutate the destination during preflight"
    );
}
