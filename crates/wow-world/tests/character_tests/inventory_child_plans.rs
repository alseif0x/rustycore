use super::*;
use super::inventory_children::{make_session_with_send_capacity, install_equippable_item_fixture, insert_equippable_test_item, install_child_equipment_fixture};

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

#[test]
fn child_equip_plan_rejects_displacement_when_parent_started_equipped_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry = 128;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        entry,
        entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry,
        82,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        entry,
        83,
        InventoryType::Weapon,
    );
    mark_inventory_child_for_test(&mut session, child_guid, parent_guid);
    insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry,
        84,
        InventoryType::Weapon,
    );

    assert_eq!(
        inventory_equip_child_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            parent_guid,
        ),
        Err(InventoryResult::CantSwap)
    );
}

#[test]
fn direct_inventory_move_from_equipment_removes_represented_item_mods_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 65);
    let entry_id = 110;
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|i| {
                    if i == 0 {
                        (ItemModType::Strength as i8, 9)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: 65,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = make_inventory_item_object_for_test(
        &session,
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    insert_inventory_item_object_for_test(&mut session, item);
    assert_eq!(
        inventory_move_with_item_mods_for_test(&mut session, 
            INVENTORY_SLOT_ITEM_START,
            EQUIPMENT_SLOT_MAINHAND
        ),
        Some(true)
    );
    assert_eq!(
        represented_item_bonus_state_for_test(&session).stats_base[0],
        9
    );
    assert_eq!(
        inventory_move_with_item_mods_for_test(&mut session, 
            EQUIPMENT_SLOT_MAINHAND,
            INVENTORY_SLOT_ITEM_START
        ),
        Some(true)
    );
    assert_eq!(
        represented_item_bonus_state_for_test(&session).stats_base[0],
        0,
        "C++ RemoveItem calls _ApplyItemMods(..., false) before taking equipped items out of storage"
    );
}

#[test]
fn child_equip_plan_targets_db2_slot_before_parent_moves_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    enable_ownerless_inventory_snapshots_for_test(&mut session);
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
    mark_inventory_child_for_test(&mut session, child_guid, parent_guid);

    assert_eq!(
        inventory_equip_child_for_test(&session, 
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                parent_guid,
            )
            .expect("child equip preflight"),
        Some(InventoryChildPlanForTest::new(child_guid, wow_entities::EQUIPMENT_SLOT_OFFHAND, None))
    );
}

#[test]
fn real_swap_plans_child_for_item_entering_source_equipment_slot_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    enable_ownerless_inventory_snapshots_for_test(&mut session);
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
    mark_inventory_child_for_test(&mut session, child_guid, destination_parent_guid);

    let plans = inventory_real_swap_children_for_test(&session, 
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
        vec![InventoryChildPlanForTest::new(child_guid, wow_entities::EQUIPMENT_SLOT_OFFHAND, None)]
    );
}

#[test]
fn child_equip_plan_preflights_displaced_equipment_storage_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    enable_ownerless_inventory_snapshots_for_test(&mut session);
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
    mark_inventory_child_for_test(&mut session, child_guid, parent_guid);
    insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        displaced_entry,
        81,
        InventoryType::Weapon,
    );

    let plan = inventory_equip_child_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            parent_guid,
        )
        .expect("child equip preflight")
        .expect("linked child");
    assert_eq!(
        plan.displaced_storage(),
        Some((
            INVENTORY_SLOT_BAG_0,
            wow_entities::NULL_SLOT,
            InventoryStorageTargetForTest::inventory(),
        ))
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            wow_entities::EQUIPMENT_SLOT_OFFHAND,
        )
        .map(|item| item.entry_id),
        Some(displaced_entry),
        "CanEquipChildItem must not mutate the destination during preflight"
    );
}

#[tokio::test]
async fn auto_equip_item_slot_commit_failure_does_not_apply_item_mods_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    enable_ownerless_inventory_snapshots_for_test(&mut session);
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
    set_failing_player_inventory_persistence_port_for_test(&mut session);

    auto_equip_item_slot_for_test(&mut session, wow_packet::packets::item::AutoEquipItemSlot {
            inv_update: wow_packet::packets::item::InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        represented_item_bonus_state_for_test(&session).stats_base[0],
        0,
        "item mods must remain unchanged until the inventory transaction commits"
    );
    assert!(
        inventory_set_spell_events_empty_for_test(&session),
        "set-bonus events must not be exposed after a failed commit"
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(item_guid)
    );
    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::InventoryChangeFailure],
        "failed persistence emits only the inventory failure"
    );
}
