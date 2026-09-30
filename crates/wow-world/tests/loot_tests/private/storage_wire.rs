//! Parent-approved migrated originals with complete production wire oracles.
//! Supersedes the historical cfg(test) vectors that omitted Create/Values;
//! full original PRE bodies remain in .codex-loot-62-pre.
//! Direct: requests/item_storage.rs creates -> removes -> pushes -> player values.
//! Batch: requests/item_storage/disenchant.rs creates -> all pushes -> removes -> values.
//! No opcode filtering: the entire single observer queue is compared.
use super::storage_support::*;
use wow_packet::packets::update::UpdateType;

fn drain_wire_packets(rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(packet) = rx.try_recv() {
        packets.push(packet);
    }
    packets
}

fn wire_opcodes(packets: &[Vec<u8>]) -> Vec<u16> {
    packets
        .iter()
        .map(|bytes| WorldPacket::from_bytes(bytes).read_uint16().unwrap())
        .collect()
}

fn inventory_update_guid(bytes: &[u8], update_type: UpdateType) -> ObjectGuid {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::UpdateObject as u16
    );
    assert_eq!(packet.read_uint32().unwrap(), 1);
    assert_eq!(packet.read_uint16().unwrap(), 0);
    assert!(!packet.read_bit().unwrap());
    let block_size = packet.read_uint32().unwrap() as usize;
    assert_eq!(block_size, packet.remaining());
    assert_eq!(packet.read_uint8().unwrap(), update_type as u8);
    packet.read_packed_guid().unwrap()
}

#[tokio::test]
async fn durable_direct_item_claim_notifies_removed_before_item_push_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    // Use one observer channel for both physical routes so this test can
    // assert their relative C++ wire order. Route separation is covered
    // independently by the existing ItemPushResult route scenario.
    let shared_send = first.send_tx().clone();
    install_loot_wire_channel_for_test(&mut first, shared_send);
    install_storage_port(
        &mut first,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );

    handle_loot_item_for_test(
        &mut first,
        loot_item_packet(represented_loot_object_guid_like_cpp(owner), 0),
    )
    .await;

    assert_eq!(applied_loot_item_quantity_for_test(&first, 25), 1);
    let packets = drain_wire_packets(&first_rx);
    assert_eq!(
        wire_opcodes(&packets),
        vec![
            wow_constants::ServerOpcodes::UpdateObject as u16,
            wow_constants::ServerOpcodes::LootRemoved as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::UpdateObject as u16,
        ],
        "C++ Player::StoreLootItem notifies removal before SendNewItem"
    );
    assert!(inventory_update_guid(&packets[0], UpdateType::CreateObject).is_item());
    assert_eq!(
        inventory_update_guid(&packets[3], UpdateType::Values),
        first_guid
    );
}

#[tokio::test]
async fn local_disenchant_batch_commits_all_materials_and_original_claim_like_cpp() {
    let (mut session, rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&rx);
    let shared_send = session.send_tx().clone();
    install_loot_wire_channel_for_test(&mut session, shared_send);
    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut session, owner)
            .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(player_guid, 700);
    install_limited_test_item_template(&mut session, 700, 0);
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        70_000,
    )));
    install_storage_port(
        &mut session,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );

    assert!(
        store_loot_materials_for_test(
            &mut session,
            &materials,
            0,
            Some(&claim),
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            player_guid,
            false
        )
        .await
    );
    assert_eq!(applied_loot_item_quantity_for_test(&session, 700), 2);
    let packets = drain_wire_packets(&rx);
    assert_eq!(
        wire_opcodes(&packets),
        vec![
            wow_constants::ServerOpcodes::UpdateObject as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::LootRemoved as u16,
            wow_constants::ServerOpcodes::UpdateObject as u16,
        ],
        "C++ Loot::AutoStore sends every material before the original roll slot is removed"
    );
    assert!(inventory_update_guid(&packets[0], UpdateType::CreateObject).is_item());
    assert_eq!(
        inventory_update_guid(&packets[4], UpdateType::Values),
        player_guid
    );
    assert!(claim.is_committed_like_cpp());
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_err()
    );
}
