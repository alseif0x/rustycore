//! Petition packets.
//!
//! Separated from trade.rs under #709.

use super::*;

#[tokio::test]
async fn sign_petition_records_guid_and_choice_like_cpp_without_runtime_mgr() {
    let (mut session, send_rx) = make_session();
    let petition_guid = ObjectGuid::create_item(1, 91_777);

    session
        .handle_sign_petition(sign_petition_packet(petition_guid, 1))
        .await;

    assert_eq!(
        session.represented_sign_petitions_like_cpp(),
        &[crate::session::RepresentedSignPetitionLikeCpp {
            petition_guid,
            choice: 1,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn decline_petition_records_guid_like_cpp_without_client_notification() {
    let (mut session, send_rx) = make_session();
    let petition_guid = ObjectGuid::create_item(1, 91_778);

    session
        .handle_decline_petition(decline_petition_packet(petition_guid))
        .await;

    assert_eq!(
        session.represented_decline_petitions_like_cpp(),
        &[crate::session::RepresentedDeclinePetitionLikeCpp { petition_guid }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn query_petition_without_runtime_mgr_sends_not_found_like_cpp() {
    let (mut session, send_rx) = make_session();
    let item_guid = ObjectGuid::create_item(1, 91_779);

    session
        .handle_query_petition(query_petition_packet(123, item_guid))
        .await;

    assert_eq!(
        session.represented_query_petitions_like_cpp(),
        &[crate::session::RepresentedQueryPetitionLikeCpp {
            petition_id: 123,
            item_guid,
        }]
    );

    let bytes = send_rx.try_recv().expect("query petition response");
    let mut body = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        body.server_opcode(),
        Some(ServerOpcodes::QueryPetitionResponse)
    );
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::QueryPetitionResponse as u16
    );
    assert_eq!(body.read_uint32().unwrap(), item_guid.counter() as u32);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}
