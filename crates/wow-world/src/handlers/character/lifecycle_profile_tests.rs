use std::sync::Arc;

use crate::session::WorldSession;
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_packet::packets::character::{
    BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP, BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
    DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
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

fn confirm_barbers_choice_packet(customizations: &[(u32, u32)]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(customizations.len() as u32);
    for (option_id, choice_id) in customizations {
        pkt.write_uint32(*option_id);
        pkt.write_uint32(*choice_id);
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
async fn alter_appearance_on_represented_barber_chair_records_request_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 22);
    let chair_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    assert!(session.use_represented_gameobject_barber_chair_like_cpp(
        gameobject_guid,
        player_guid,
        chair_position,
        wow_entities::BarberChairUseSource {
            chair_height: 2,
            sit_anim_kit: 0,
            customization_scope: 7,
        },
    ));
    let _enable_barber_shop = send_rx.try_recv().unwrap();

    session
        .handle_alter_appearance(alter_appearance_packet(1, 7, 11, &[(20, 200), (10, 100)]))
        .await;

    assert_eq!(
        read_barber_shop_result(send_rx.try_recv().unwrap()),
        BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP
    );
    assert_eq!(
        session.represented_alter_appearance_requests_like_cpp(),
        &[RepresentedAlterAppearanceLikeCpp {
            new_sex: 1,
            customizations: vec![
                ChrCustomizationChoice {
                    option_id: 10,
                    choice_id: 100,
                },
                ChrCustomizationChoice {
                    option_id: 20,
                    choice_id: 200,
                },
            ],
            customized_race: 7,
            customized_chr_model_id: 11,
            cost: 0,
        }]
    );
}

#[tokio::test]
async fn confirm_barbers_choice_records_request_without_success_packet_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_confirm_barbers_choice(confirm_barbers_choice_packet(&[(20, 200), (10, 100)]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_confirm_barbers_choice_requests_like_cpp(),
        &[RepresentedConfirmBarbersChoiceLikeCpp {
            customizations: vec![
                ChrCustomizationChoice {
                    option_id: 20,
                    choice_id: 200,
                },
                ChrCustomizationChoice {
                    option_id: 10,
                    choice_id: 100,
                },
            ],
            cost: 0,
        }]
    );
}
