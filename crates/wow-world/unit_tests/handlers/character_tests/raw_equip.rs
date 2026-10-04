// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

async fn recorded_raw_equip(outcome: PersistenceOutcomeLikeCpp, applied: bool) {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let owner = ObjectGuid::create_player(1, 30_230);
    session.set_player_guid(Some(owner));
    install_equippable_item_fixture(&mut session, 30_230, InventoryType::Weapon, Some(3));
    let guid = insert_equippable_test_item(&mut session, INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START, 30_230, 30_230, InventoryType::Weapon);
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_count(2);
        item.set_durability(73);
        item.set_create_played_time(123);
        item.set_spell_charges(0, -2);
    });
    let before = session.resolved_inventory_item_object_like_cpp(guid).unwrap();
    let (port, requests) = PlayerInventoryPersistencePortFixtureLikeCpp::new_like_cpp(outcome);
    session.set_player_inventory_persistence_port_like_cpp(port);
    while send_rx.try_recv().is_ok() {}

    session.execute_inventory_equip_to_empty_raw_like_cpp(INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START, wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)).await;

    let recorded = requests.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    let wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::Equip(request) = &recorded[0] else {
        panic!("raw equip must submit its Equip request");
    };
    assert_eq!(request.mutable_item.item_guid, 30_230);
    assert_eq!(request.mutable_item.count, before.count());
    assert_eq!(request.mutable_item.count, 2);
    assert_eq!(request.mutable_item.expiration, before.data().expiration);
    assert_eq!(request.mutable_item.flags, before.item_flags_bits());
    assert_eq!(request.mutable_item.charges, "");
    assert_eq!(request.mutable_item.durability, 73);
    assert_eq!(request.mutable_item.played_time, 123);
    assert_eq!(request.mutable_item.enchantments, "0 0 0 ".repeat(before.data().enchantments.len()));
    assert_eq!(request.delete_source_link_owner_guid, owner.counter() as u64);
    assert_eq!(request.delete_source_link_item_guid, 30_230);
    assert_eq!(request.destination_link.owner_guid, owner.counter() as u64);
    assert_eq!(request.destination_link.bag_guid, 0);
    assert_eq!(request.destination_link.slot, EQUIPMENT_SLOT_MAINHAND);
    assert_eq!(request.destination_link.item_guid, 30_230);
    drop(recorded);

    let item = session.resolved_inventory_item_object_like_cpp(guid).unwrap();
    let mut packets = Vec::new();
    let mut packet_bytes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        packets.push(WorldPacket::from_bytes(&bytes).server_opcode());
        packet_bytes.push(bytes);
    }
    if applied {
        assert!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).is_none());
        assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND).unwrap().guid, guid);
        assert_eq!(item.slot(), EQUIPMENT_SLOT_MAINHAND);
        assert!(item.has_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED));
        assert!(packets.contains(&Some(ServerOpcodes::UpdateObject)));
        assert!(!packets.contains(&Some(ServerOpcodes::InventoryChangeFailure)));
        // Position values precede the item relocation and bonus-stat fallback.
        assert_eq!(packets, vec![Some(ServerOpcodes::UpdateObject); 3]);
        session.publish_inventory_position_changes_like_cpp(&[
            (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            (INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        ]);
        assert_eq!(send_rx.try_recv().unwrap(), packet_bytes[0]);
        assert!(send_rx.try_recv().is_err());
        session.send_item_relocation_values_update_like_cpp(guid, true, &[]);
        assert_eq!(send_rx.try_recv().unwrap(), packet_bytes[1]);
        assert!(session.send_represented_item_bonus_player_stat_update_like_cpp());
        assert_eq!(send_rx.try_recv().unwrap(), packet_bytes[2]);
        assert!(send_rx.try_recv().is_err());
    } else {
        assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, guid);
        assert!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND).is_none());
        assert_eq!(item.slot(), before.slot());
        assert_eq!(item.count(), before.count());
        assert_eq!(item.item_flags_bits(), before.item_flags_bits());
        assert_eq!(item.data().enchantments, before.data().enchantments);
        assert_eq!(packets, vec![Some(ServerOpcodes::InventoryChangeFailure)]);
    }
}

#[tokio::test]
async fn raw_equip_applied_records_request_then_moves_runtime_and_publishes() {
    recorded_raw_equip(PersistenceOutcomeLikeCpp::Applied { rows: 1 }, true).await;
}

#[tokio::test]
async fn raw_equip_failed_records_request_keeps_runtime_and_only_reports_error() {
    recorded_raw_equip(PersistenceOutcomeLikeCpp::Failed { reason: "recorded rollback".into() }, false).await;
}

#[tokio::test]
async fn raw_equip_unknown_records_request_keeps_runtime_and_only_reports_error() {
    recorded_raw_equip(PersistenceOutcomeLikeCpp::Unknown { reason: "recorded unknown commit".into() }, false).await;
}
