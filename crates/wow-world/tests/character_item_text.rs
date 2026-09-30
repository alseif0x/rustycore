use wow_constants::ItemContext;
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_packet::{WorldPacket, packets::query::ItemTextQuery};
use wow_world::session::WorldSession;
use wow_world::test_fixtures::{
    insert_inventory_item_object_for_test, make_inventory_item_object_for_test,
};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let session = WorldSession::new(
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
    (session, send_rx)
}

#[tokio::test]
async fn item_text_query_missing_item_sends_cpp_invalid_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 1);

    session
        .handle_item_text_query(ItemTextQuery { id: item_guid })
        .await;

    let bytes = send_rx.try_recv().expect("item text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryItemTextResponse as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x00);
    assert_eq!(&bytes[5..21], &item_guid.to_raw_bytes());
    assert_eq!(bytes.len(), 21);
}

#[tokio::test]
async fn item_text_query_inventory_item_sends_text_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let owner_guid = ObjectGuid::create_player(1, 700);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 2);
    let mut item = make_inventory_item_object_for_test(
        &session,
        item_guid,
        8000,
        owner_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    item.set_text("abc");
    insert_inventory_item_object_for_test(&mut session, item);

    session
        .handle_item_text_query(ItemTextQuery { id: item_guid })
        .await;

    let bytes = send_rx.try_recv().expect("item text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryItemTextResponse as u16
    );
    assert_eq!(bytes[2], 0x80);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x18);
    assert_eq!(&bytes[5..8], b"abc");
    assert_eq!(&bytes[8..24], &item_guid.to_raw_bytes());
    assert_eq!(bytes.len(), 24);
}
