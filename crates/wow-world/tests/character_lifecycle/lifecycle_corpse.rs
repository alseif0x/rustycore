use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use super::fixtures::make_session;
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_packet::packets::query::{QueryCorpseLocationFromClient, QueryCorpseTransport};

#[tokio::test]
async fn query_corpse_location_without_runtime_corpse_sends_cpp_invalid_shape() {
    let (mut session, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);

    session
        .handle_query_corpse_location(QueryCorpseLocationFromClient { player })
        .await;

    let bytes = send_rx.try_recv().expect("corpse location response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::CorpseLocation as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(&bytes[3..19], &player.to_raw_bytes());
    assert_eq!(bytes.len(), 55);
}

#[tokio::test]
async fn query_corpse_transport_without_runtime_corpse_sends_cpp_default_shape() {
    let (mut session, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let transport = ObjectGuid::create_world_object(HighGuid::Transport, 0, 1, 571, 0, 77, 42);

    session
        .handle_query_corpse_transport(QueryCorpseTransport { player, transport })
        .await;

    let bytes = send_rx.try_recv().expect("corpse transport response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::CorpseTransportQuery as u16
    );
    assert_eq!(&bytes[2..18], &player.to_raw_bytes());
    assert_eq!(&bytes[18..22], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[22..26], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[26..30], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[30..34], &0.0_f32.to_le_bytes());
    assert_eq!(bytes.len(), 34);
}
