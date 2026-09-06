//! Explicit logout must not undo save quarantine by publishing character-select readiness.
//! Controlled persistence, partial production login; not a DB/client logout capture.
use super::*;

#[tokio::test]
async fn production_explicit_logout_completes_pending_transfer_before_save_and_retirement() {
    pending_far_disconnect(DisconnectDestination::Requested, true).await;
}

#[tokio::test]
async fn production_explicit_logout_terminal_recovery_leaves_one_source_save_to_disconnect() {
    pending_far_disconnect(DisconnectDestination::Rejected, true).await;
}

async fn exercise(outcome: PersistenceOutcomeLikeCpp, cancel: bool) {
    let (mut session, port, _output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    session.set_state(SessionState::LoggedIn);
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        player.teleport_state_mut_like_cpp().far_pending = true;
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        player.teleport_state_mut_like_cpp().far_pending = false;
    }
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(!cancel),
        outcome: outcome.clone(),
    });
    *port.save_probe.lock().unwrap() = Some(probe.clone());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let request = wow_packet::packets::misc::LogoutRequest { idle_logout: false };
    if cancel {
        use std::future::Future;
        let mut logout =
            Box::pin(session.handle_logout_request_with_generator_like_cpp(&generator, request));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                logout.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        assert_eq!(probe.requests.lock().unwrap().len(), 1);
        drop(logout);
        session.kick("controlled explicit logout cancellation after save submission");
    } else {
        session
            .handle_logout_request_with_generator_like_cpp(&generator, request)
            .await;
    }
    let quarantined = cancel || matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. });
    assert_eq!(
        session.state(),
        if quarantined {
            SessionState::Disconnecting
        } else {
            SessionState::Authed
        }
    );
    let packets: Vec<_> = receiver
        .try_iter()
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect();
    assert!(packets.contains(&(wow_constants::ServerOpcodes::LogoutResponse as u16)));
    assert_eq!(
        packets.contains(&(wow_constants::ServerOpcodes::LogoutComplete as u16)),
        !quarantined
    );
    assert_eq!(probe.requests.lock().unwrap().len(), 1);
    if quarantined {
        assert_eq!(session.player_guid(), Some(guid));
        let manager = port.manager.lock().unwrap();
        let player = manager
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .unwrap();
        assert!(player.has_deferred_player_save_like_cpp());
        drop(manager);
        session
            .save_disconnect_player_to_db_with_generator_like_cpp(&generator)
            .await;
        assert_eq!(session.state(), SessionState::Disconnecting);
        assert_eq!(
            probe.requests.lock().unwrap().len(),
            1,
            "unknown save is not replayed by disconnect"
        );
    } else {
        assert!(session.player_guid().is_none());
        let manager = port.manager.lock().unwrap();
        assert!(
            manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
    }
}

#[tokio::test]
async fn production_explicit_logout_does_not_restore_authed_after_unknown_commit() {
    exercise(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "lost COMMIT reply".into(),
        },
        false,
    )
    .await;
}

#[tokio::test]
async fn production_cancelled_explicit_logout_keeps_owner_and_does_not_replay_save() {
    exercise(PersistenceOutcomeLikeCpp::Applied { rows: 1 }, true).await;
}

#[tokio::test]
async fn production_explicit_logout_preserves_applied_and_known_rollback_behavior() {
    exercise(PersistenceOutcomeLikeCpp::Applied { rows: 1 }, false).await;
    exercise(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "known rollback".into(),
        },
        false,
    )
    .await;
}

#[tokio::test]
async fn production_explicit_logout_rejects_unavailable_source_save_without_retiring_owner() {
    let (mut session, port, _output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    session.set_state(SessionState::LoggedIn);
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(probe.clone());
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        player.teleport_state_mut_like_cpp().recovery =
            wow_entities::PlayerTransferRecovery::Terminal;
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::new(f32::NAN, 0.0, 0.0, 0.0));
    }
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    session
        .handle_logout_request_with_generator_like_cpp(
            &generator,
            wow_packet::packets::misc::LogoutRequest { idle_logout: false },
        )
        .await;
    assert_eq!(session.state(), SessionState::Disconnecting);
    assert_eq!(session.player_guid(), Some(guid));
    assert!(probe.requests.lock().unwrap().is_empty());
    assert!(
        port.manager
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .is_some()
    );
    assert!(
        receiver
            .try_iter()
            .all(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                != wow_constants::ServerOpcodes::LogoutComplete as u16)
    );
}
