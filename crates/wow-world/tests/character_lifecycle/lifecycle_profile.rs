use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use std::sync::Arc;

use wow_world::session::WorldSession;
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_packet::packets::character::{
    DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
};
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new_character_lifecycle_fixture(
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.character_set_equipment_set_guid_generator_for_test(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}







fn declined_names_packet(player: ObjectGuid, names: [&str; 5]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&player);
    for name in names {
        pkt.write_bits(name.len() as u32, 7);
    }
    for name in names {
        pkt.write_string(name);
    }
    pkt
}

fn read_declined_names_result(encoded: Vec<u8>) -> (i32, ObjectGuid) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::SetPlayerDeclinedNamesResult)
    );
    packet.skip_opcode();
    let result = packet.read_int32().unwrap();
    let player = packet.read_guid().unwrap();
    assert_eq!(packet.remaining(), 0);
    (result, player)
}

#[tokio::test]
async fn set_player_declined_names_without_runtime_sends_error_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 42);

    session
        .handle_set_player_declined_names(declined_names_packet(
            player,
            ["Gen", "Dat", "Acc", "Inst", "Prep"],
        ))
        .await;

    assert_eq!(
        read_declined_names_result(send_rx.try_recv().unwrap()),
        (DECLINED_NAMES_RESULT_ERROR_LIKE_CPP, player)
    );
}

#[tokio::test]
async fn set_player_declined_names_short_packet_does_not_send_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_set_player_declined_names(WorldPacket::from_bytes(&[0x2a, 0x00]))
        .await;

    assert!(send_rx.try_recv().is_err());
}
