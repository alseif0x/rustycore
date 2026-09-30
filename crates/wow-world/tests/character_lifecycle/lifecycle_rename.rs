use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use wow_constants::ServerOpcodes;
use wow_constants::character::CHAR_NAME_NO_NAME_LIKE_CPP;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;
use wow_packet::packets::character::CharacterRenameRequest;
use wow_world::session::{SessionState, WorldSession};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    (
        WorldSession::new_character_lifecycle_fixture(
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
async fn character_rename_invalid_name_sends_cpp_result_without_guid() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: String::new(),
        })
        .await;

    let sent = send_rx.try_recv().expect("rename result");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), CHAR_NAME_NO_NAME_LIKE_CPP);
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_bits(6).unwrap(), 0);
    assert_eq!(pkt.remaining(), 0);
}

#[tokio::test]
async fn character_rename_non_owned_guid_kicks_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid: ObjectGuid::create_player(1, 42),
            new_name: "Newname".to_string(),
        })
        .await;

    assert_eq!(session.state(), SessionState::Disconnecting);
    assert!(send_rx.try_recv().is_err());
}
