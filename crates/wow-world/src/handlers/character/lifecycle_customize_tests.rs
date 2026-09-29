use super::CHAR_CREATE_ERROR_LIKE_CPP;
use crate::session::WorldSession;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;
use wow_packet::packets::character::CharCustomize;
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    (
        WorldSession::new(
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
        ),
        send_rx,
    )
}

#[tokio::test]
async fn char_customize_without_character_db_sends_cpp_failure() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);

    session
        .handle_char_customize(CharCustomize {
            guid,
            sex_id: 1,
            customizations: vec![],
            name: "Newname".to_string(),
        })
        .await;

    let sent = send_rx.try_recv().expect("customize failure");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharCustomizeFailure)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), CHAR_CREATE_ERROR_LIKE_CPP);
    assert_eq!(pkt.read_guid().unwrap(), guid);
    assert_eq!(pkt.remaining(), 0);
}

#[tokio::test]
async fn char_customize_non_owned_guid_kicks_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_char_customize(CharCustomize {
            guid: ObjectGuid::create_player(1, 42),
            sex_id: 1,
            customizations: vec![],
            name: "Newname".to_string(),
        })
        .await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(send_rx.try_recv().is_err());
}
