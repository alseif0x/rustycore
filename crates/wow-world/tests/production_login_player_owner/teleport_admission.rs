//! Player::TeleportTo validates all destination coordinates before side effects
//! (Player.cpp:1237-1244; GridDefines.h:231-248), unlike the lower Map cell check.
use super::*;

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
