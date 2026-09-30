use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use std::sync::Arc;

use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_entities::{
    PlayerHomebindLikeCpp as RepresentedHomebindLikeCpp,
    PlayerTaxiFlightNodeLikeCpp as RepresentedTaxiFlightNodeLikeCpp,
};
use wow_packet::WorldPacket;
use wow_world::session::WorldSession;
use wow_world::test_fixtures::{
    install_canonical_player_owner_for_test, player_is_alive_for_test,
    set_loaded_player_identity_like_cpp, set_player_position_for_test,
};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new_character_lifecycle_fixture(
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
    session.character_set_equipment_set_guid_generator_for_test(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn make_hearth_and_resurrect_session(area_flags: u32) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_map_store(world_maps([571]));
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    set_player_position_for_test(&mut session, Position::new(1.0, 2.0, 3.0, 0.5));
    session.character_set_player_zone_area_for_test(10, 77);
    session.set_player_alive_like_cpp(false);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 77,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: area_flags,
        },
    ])));
    let _ = session.character_set_represented_homebind_for_test(RepresentedHomebindLikeCpp {
        map_id: 571,
        area_id: 77,
        position: Position::new(10.0, 20.0, 30.0, 1.5),
    });
    (session, send_rx)
}

#[tokio::test]
async fn hearth_and_resurrect_rejects_area_without_cpp_flag() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(0);

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(!player_is_alive_for_test(&session));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn hearth_and_resurrect_rejects_player_in_flight_like_cpp() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(
        wow_data::AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP,
    );
    session.character_set_taxi_flight_state_for_test(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 571,
            position: Position::new(1.0, 2.0, 3.0, 0.0),
            teleport_flag: false,
        },
        None,
    );

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(!player_is_alive_for_test(&session));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn hearth_and_resurrect_allowed_area_resurrects_and_teleports_home_like_cpp() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(
        wow_data::AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP,
    );

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(player_is_alive_for_test(&session));
    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            wow_constants::ServerOpcodes::CancelCombat as u16,
            wow_constants::ServerOpcodes::MoveTeleport as u16,
        ]
    );
}
