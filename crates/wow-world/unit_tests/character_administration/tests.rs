//! Production-driver acceptance for the Lifecycle-owned rename operation.

use wow_world_lifecycle::{
    character_administration_rename_candidate_fixture_like_cpp as candidate,
    character_administration_rename_fixture_like_cpp as fixture,
};

#[tokio::test]
async fn production_session_driver_executes_ready_rename_callbacks() {
    use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};
    use wow_core::ObjectGuid;
    use wow_packet::packets::character::CharacterRenameRequest;

    let (port, complete, _) = fixture();
    let (_packet_tx, packet_rx) = flume::bounded(1);
    let (send_tx, send_rx) = flume::bounded(2);
    let mut session = WorldSession::new(
        1,
        "RenameDriver".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "esES".into(),
        packet_rx,
        send_tx,
    );
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    let catalogs = SessionHandlerCatalogsLikeCpp::default();
    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: "Newname".into(),
        })
        .await;

    // Drive the actual Session pass, not the standalone callback adapter. A
    // pending query must not keep this future pending or submit the transaction.
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        session.process_pending_with_catalogs_like_cpp(&catalogs),
    )
    .await
    .expect("pending DB read does not block the driver");
    assert!(port.commit_snapshot().is_empty());
    assert!(send_rx.try_recv().is_err());
    complete.send(candidate()).unwrap();

    let bytes = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            session
                .process_pending_with_catalogs_like_cpp(&catalogs)
                .await;
            if let Ok(bytes) = send_rx.try_recv() {
                break bytes;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("real driver publishes the completed rename");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::CharacterRenameResult)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert_eq!(port.commit_count(), 1);
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
}
