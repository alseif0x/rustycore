// External application scenarios migrated with their original assertions.

// Persisted transport login requests stay with the character login owner.

use super::*;

use wow_world::test_fixtures::CollectionLoadPortLikeCpp;

#[tokio::test]
async fn persisted_transport_login_requests_world_row_by_guid_and_keeps_absence_unknown_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_login_transports([
        PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(Vec::new()),
        PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
            reason: "world transport read failed".to_owned(),
        },
    ]);
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let mut session = WorldSession::new_character_lifecycle_fixture(
        1, "TestAccount".into(), 0, 2, 9, 54261, vec![0u8; 40], "esES".into(), pkt_rx,
        send_tx,
    );
    session.set_player_lifecycle_port_like_cpp(port.clone());

    let empty = session
        .character_resolve_persisted_transport_login_for_test(77, 571, Position::new(1.0, 2.0, 3.0, 4.0))
        .await;
    let failed = session
        .character_resolve_persisted_transport_login_for_test(88, 571, Position::new(1.0, 2.0, 3.0, 4.0))
        .await;

    assert!(empty.is_none());
    assert!(failed.is_none());
    assert_eq!(
        port.login_transport_requests(),
        vec![
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 77 },
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 88 },
        ]
    );
}
