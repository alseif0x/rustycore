// External application scenarios migrated with their original assertions.

// Typed homebind repair requests remain with the login recovery owner.

use super::*;
use wow_data::{PlayerCreateInfoLikeCpp, PlayerCreatePositionLikeCpp};
use wow_packet::WorldPacket;
use wow_persistence::PlayerHomebindPersistenceRequestLikeCpp;

use wow_world::test_fixtures::CollectionLoadPortLikeCpp;

#[tokio::test]
async fn homebind_repair_writes_typed_delete_and_insert_requests_nonfatally_like_cpp() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(4);
    let mut session = WorldSession::new_character_lifecycle_fixture(
        1, "TestAccount".into(), 0, 2, 9, 54261, vec![0u8; 40], "esES".into(), pkt_rx,
        send_tx,
    );
    // This private port records both requests and returns nonfatal Failed outcomes.
    let port = CollectionLoadPortLikeCpp::new([]);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 77);

    session
        .character_delete_invalid_character_homebind_for_test(guid)
        .await;
    let create_position = PlayerCreatePositionLikeCpp {
        map_id: 0,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        transport_guid: None,
    };
    let repaired = session
        .character_repair_character_homebind_for_test(
            guid,
            1,
            PlayerCreateInfoLikeCpp {
                create_position,
                create_position_npe: None,
            },
            wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,
            true,
        )
        .await
        .expect("nonfatal persistence failure does not discard selected homebind");
    assert_eq!(repaired.map_id, 0);
    assert_eq!(
        port.homebind_requests(),
        vec![
            PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid {
                player_guid: guid.counter() as u64,
            },
            PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
                player_guid: guid.counter() as u64,
                map_id: 0,
                area_id: 0,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: 4.0,
            },
        ]
    );
}
