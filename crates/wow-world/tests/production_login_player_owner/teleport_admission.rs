//! Player::TeleportTo validates all destination coordinates before side effects
//! (Player.cpp:1237-1244; GridDefines.h:231-248), unlike the lower Map cell check.
use super::*;

#[tokio::test]
async fn production_rejected_worldport_recovers_once_then_saves_retained_source() {
    let (mut session, port, _, receiver) = hydrate(true, true, true).await;
    port.manager
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(ObjectGuid::create_player(1, 42))
        .unwrap()
        .gameplay_state_mut()
        .homebind
        .as_mut()
        .unwrap()
        .position = Position::new(100.0, 200.0, 300.0, 0.0);
    let maps = |ids: Vec<u32>| {
        Arc::new(MapStore::from_entries(ids.into_iter().map(|id| MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        })))
    };
    session.set_map_store(maps(vec![0, 1]));
    session.set_state(SessionState::LoggedIn);
    session
        .teleport_to(1, Position::new(7.0, 8.0, 9.0, 0.5))
        .await;
    while receiver.try_recv().is_ok() {}
    // Admission can change in transit. The ACK must not treat a missing
    // destination as successful entry, run initialization or consume transfer.
    session.set_map_store(maps(vec![0]));
    let catalogs = CreatureSpawnCatalogsLikeCpp {
        difficulty: Arc::new(CreatureDifficultyStoreLikeCpp::default()),
        base_stats: Arc::new(CreatureBaseStatsStoreLikeCpp::default()),
        health_rates: CreatureClassificationHealthRatesLikeCpp::default(),
        addons: Arc::new(CreatureAddonStoreLikeCpp::default()),
        equipment: Arc::new(CreatureEquipmentStoreLikeCpp::default()),
        power_types: Arc::new(PowerTypeStore::from_entries([])),
    };
    session
        .handle_world_port_response_with_catalogs_like_cpp(
            &catalogs,
            &trait_tree::TraitNodeEntryStore::from_entries([]),
            wow_packet::WorldPacket::new_empty(),
        )
        .await;
    assert_eq!(session.state(), SessionState::Transfer);
    assert_eq!(
        receiver
            .try_iter()
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            wow_constants::ServerOpcodes::CancelCombat as u16,
            wow_constants::ServerOpcodes::TransferPending as u16,
            wow_constants::ServerOpcodes::SuspendToken as u16,
        ]
    );
    assert!(port.manager.try_lock().unwrap().find_map(1, 0).is_none());
    session
        .handle_world_port_response_with_catalogs_like_cpp(
            &catalogs,
            &trait_tree::TraitNodeEntryStore::from_entries([]),
            wow_packet::WorldPacket::new_empty(),
        )
        .await;
    assert!(
        receiver.is_empty(),
        "an old ACK cannot complete the new recovery before NewWorld"
    );
    assert_eq!(session.state(), SessionState::Transfer);
    session
        .handle_suspend_token_response(wow_packet::WorldPacket::new_empty())
        .await;
    let bytes = receiver
        .try_recv()
        .expect("failed admission retains pending transfer");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::NewWorld as u16
    );
    assert!(receiver.is_empty());
    session.set_map_store(maps(vec![]));
    for _ in 0..2 {
        session
            .handle_world_port_response_with_catalogs_like_cpp(
                &catalogs,
                &trait_tree::TraitNodeEntryStore::from_entries([]),
                wow_packet::WorldPacket::new_empty(),
            )
            .await;
        assert_eq!(session.state(), SessionState::Disconnecting);
        assert!(
            receiver.is_empty(),
            "terminal rejection cannot start another handshake"
        );
    }
    session
        .handle_suspend_token_response(wow_packet::WorldPacket::new_empty())
        .await;
    assert!(receiver.is_empty());
    save::assert_terminal_source_save(&mut session, &port).await;
}

#[tokio::test]
async fn production_detached_return_to_source_requires_world_entry_handshake() {
    let (mut session, port, _, receiver) = hydrate(true, true, true).await;
    session.set_map_store(Arc::new(MapStore::from_entries([0, 1].map(|id| {
        MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }))));
    session.set_state(SessionState::LoggedIn);
    session
        .teleport_to(1, Position::new(7.0, 8.0, 9.0, 0.5))
        .await;
    while receiver.try_recv().is_ok() {}

    // Rejected recovery inputs must not emit a second transfer or lose the
    // pending far transfer while the Player remains detached.
    session
        .teleport_to(0, Position::new(f32::NAN, 2.0, 3.0, 0.5))
        .await;
    assert!(receiver.is_empty());
    assert_eq!(session.state(), SessionState::Transfer);

    // The failed destination has not been bound: homebind can share the old
    // map ID even though Player is detached. A near ACK cannot add it back.
    session
        .teleport_to(0, Position::new(1.0, 2.0, 3.0, 0.5))
        .await;
    let opcodes: Vec<_> = receiver
        .try_iter()
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect();
    assert_eq!(
        opcodes,
        vec![
            wow_constants::ServerOpcodes::CancelCombat as u16,
            wow_constants::ServerOpcodes::TransferPending as u16,
            wow_constants::ServerOpcodes::SuspendToken as u16,
        ]
    );
    assert_eq!(session.state(), SessionState::Transfer);
    assert_eq!(
        port.manager
            .try_lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .player_count(),
        0
    );
    session
        .handle_suspend_token_response(wow_packet::WorldPacket::new_empty())
        .await;
    let bytes = receiver
        .try_recv()
        .expect("retained Player answers recovery suspend ACK");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::NewWorld as u16
    );
    assert!(receiver.is_empty());
}

#[tokio::test]
async fn production_far_transfer_retains_owner_until_suspend_ack() {
    let (mut session, port, _, receiver) = hydrate(true, true, true).await;
    session.set_map_store(Arc::new(MapStore::from_entries([0, 1].map(|id| {
        MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }))));
    session.set_state(SessionState::LoggedIn);
    session
        .teleport_to(1, Position::new(7.0, 8.0, 9.0, 0.5))
        .await;
    assert_eq!(session.state(), SessionState::Transfer);
    assert!(
        port.manager
            .try_lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(ObjectGuid::create_player(1, 42))
            .is_none()
    );
    while receiver.try_recv().is_ok() {}
    // This public production path reads far_pending through the retained owner.
    // Dropping the detached Player would suppress NewWorld and strand the client.
    session
        .handle_suspend_token_response(wow_packet::WorldPacket::new_empty())
        .await;
    let bytes = receiver
        .try_recv()
        .expect("retained Player answers suspend ACK");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::NewWorld as u16
    );
    assert!(receiver.is_empty());
}

#[tokio::test]
async fn production_teleport_accepts_cpp_coordinate_limits_and_finite_orientation() {
    let limit = Position::MAP_HALFSIZE_LIKE_CPP - 0.5;
    for sign in [-1.0, 1.0] {
        let (mut session, port, _, receiver) = hydrate(true, true, true).await;
        let destination = Position::new(sign * limit, -sign * limit, sign * limit, sign * 100.0);
        session.teleport_to(0, destination).await;
        assert!(
            !receiver.is_empty(),
            "valid boundary must start the near transfer"
        );
        let manager = port.manager.try_lock().unwrap();
        let player = manager
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(ObjectGuid::create_player(1, 42))
            .unwrap();
        assert!(player.teleport_state_like_cpp().near_pending);
        assert_eq!(
            player.teleport_state_like_cpp().near_destination,
            Some((0, destination))
        );
        assert_ne!(player.unit().world().position(), destination);
    }
}

#[tokio::test]
async fn production_invalid_teleport_preserves_owner_and_existing_transfer() {
    let (mut session, port, _, receiver) = hydrate(true, true, true).await;
    session.set_map_store(Arc::new(MapStore::from_entries([0, 1].map(|id| {
        MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }))));
    session.set_state(SessionState::LoggedIn);
    let guid = ObjectGuid::create_player(1, 42);
    let original = {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        let teleport = player.teleport_state_mut_like_cpp();
        teleport.near_pending = true;
        teleport.near_destination = Some((0, Position::new(4.0, 5.0, 6.0, 0.5)));
        (
            player.unit().world().position(),
            player.teleport_state_like_cpp().clone(),
        )
    };
    assert!(receiver.is_empty());
    for missing_map in [42, 65_536, u32::MAX] {
        session
            .teleport_to(missing_map, Position::new(1.0, 2.0, 3.0, 0.0))
            .await;
        assert!(
            receiver.is_empty(),
            "map lookup must use the complete u32 ID"
        );
        assert_eq!(session.state(), SessionState::LoggedIn);
    }
    for map_id in [0, 1] {
        for (x, y, z, orientation) in [
            (1.0, 2.0, f32::INFINITY, 0.0),
            (1.0, 2.0, 3.0, f32::NAN),
            (1.0, 2.0, 100_000.0, 0.0),
            (f32::NAN, 2.0, 3.0, 0.0),
            (1.0, -100_000.0, 3.0, 0.0),
        ] {
            session
                .teleport_to(
                    map_id,
                    Position {
                        x,
                        y,
                        z,
                        orientation,
                    },
                )
                .await;
            assert_eq!(session.state(), SessionState::LoggedIn);
            assert!(
                receiver.is_empty(),
                "invalid destination must not emit transfer packets"
            );
            let manager = port.manager.try_lock().unwrap();
            let player = manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .expect("invalid destination must not detach Player");
            assert_eq!(player.unit().world().position(), original.0);
            assert_eq!(player.teleport_state_like_cpp(), &original.1);
        }
    }
}
