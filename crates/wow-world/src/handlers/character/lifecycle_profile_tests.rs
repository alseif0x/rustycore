use std::sync::Arc;

use crate::session::WorldSession;
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_packet::packets::character::{
    BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP, DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
};
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn alter_appearance_packet(
    new_sex: u8,
    customized_race: i32,
    customized_chr_model_id: i32,
    customizations: &[(i32, i32)],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(customizations.len() as u32);
    pkt.write_uint8(new_sex);
    pkt.write_int32(customized_race);
    pkt.write_int32(customized_chr_model_id);
    for (option_id, choice_id) in customizations {
        pkt.write_int32(*option_id);
        pkt.write_int32(*choice_id);
    }
    pkt
}

fn read_barber_shop_result(encoded: Vec<u8>) -> i32 {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::BarberShopResult)
    );
    packet.skip_opcode();
    let result = packet.read_int32().unwrap();
    assert_eq!(packet.remaining(), 0);
    result
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
async fn alter_appearance_without_barber_chair_sends_not_on_chair_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    session
        .handle_alter_appearance(alter_appearance_packet(1, 1, 0, &[(20, 200)]))
        .await;

    assert_eq!(
        read_barber_shop_result(send_rx.try_recv().unwrap()),
        BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP
    );
    assert!(
        session
            .represented_alter_appearance_requests_like_cpp()
            .is_empty()
    );
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
