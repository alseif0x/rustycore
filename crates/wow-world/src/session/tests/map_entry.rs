//! Map selection is not Player attachment. The compatibility entrypoint still
//! composes both synchronously; a decision is not a durable/asynchronous permit.
use super::*;

#[test]
fn appearance_read_distinguishes_missing_active_and_detached_owner() {
    let (mut session, _, _) = make_session();
    assert_eq!(session.owned_player_customizations_like_cpp(), None);
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    assert_eq!(session.owned_player_customizations_like_cpp(), Some(vec![]));
    let choices = vec![wow_entities::PlayerCustomizationChoice {
        option_id: 17,
        choice_id: 29,
    }];
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.gameplay_state_mut().customizations = choices.clone();
        })
        .unwrap();
    assert_eq!(
        session.owned_player_customizations_like_cpp(),
        Some(choices.clone())
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        session.owned_player_customizations_like_cpp(),
        Some(choices)
    );
    session
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .retire_player_like_cpp(session.player_handle_like_cpp.unwrap())
        .unwrap();
    assert_eq!(session.owned_player_customizations_like_cpp(), None);
}

#[tokio::test]
async fn recovery_is_bounded_and_terminal_save_requires_coherent_source() {
    use wow_entities::PlayerTransferRecovery;
    let (mut session, _, output) = make_session();
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([0, 1]));
    let source = Position::new(1.0, 2.0, 3.0, 0.5);
    let home = Position::new(10.0, 20.0, 30.0, 0.5);
    session.set_player_position_like_cpp(source);
    assert!(
        session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
            map_id: 0,
            area_id: 0,
            position: home,
        })
    );
    let handle = session.player_handle_like_cpp.unwrap();
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    session.pending_teleport = Some((1, Position::default()));
    assert!(session.set_represented_far_teleport_pending_like_cpp(true));
    session.state = SessionState::Transfer;
    session.recover_rejected_worldport_like_cpp().await;
    assert_eq!(
        session
            .player_teleport_state_snapshot_like_cpp()
            .unwrap()
            .recovery,
        PlayerTransferRecovery::Homebind
    );
    assert_eq!(session.pending_teleport, Some((0, home)));
    drain_server_opcodes(&output);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([]));
    assert!(!session.try_attach_worldport_destination_like_cpp(0, home));
    for _ in 0..2 {
        session.recover_rejected_worldport_like_cpp().await;
        assert_eq!(session.state, SessionState::Disconnecting);
        let state = session.player_teleport_state_snapshot_like_cpp().unwrap();
        assert_eq!(state.recovery, PlayerTransferRecovery::Terminal);
        assert!(state.far_pending, "terminal is not successful entry");
        assert_eq!(session.player_handle_like_cpp, Some(handle));
        assert!(output.is_empty());
    }
    let save = session
        .prepare_player_save_like_cpp(0)
        .expect("coherent source can be saved");
    assert_eq!(save.header.map_id, 0);
    assert_eq!(save.header.position, source);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([0]));
    session.teleport_to(0, home).await;
    assert!(!session.try_attach_worldport_destination_like_cpp(0, home));
    assert!(output.is_empty());
    session
        .with_owned_player_mut_like_cpp(|p| {
            p.unit_mut()
                .world_mut()
                .relocate(Position::new(f32::NAN, 2.0, 3.0, 0.5))
        })
        .unwrap();
    assert!(session.prepare_player_save_like_cpp(0).is_none());
}

#[tokio::test]
async fn recovery_missing_or_invalid_homebind_terminates_without_replacing_source() {
    for invalid in [false, true] {
        let (mut session, _, output) = make_session();
        install_canonical_player_owner_for_test(&mut session, 0, 0);
        session.set_map_store(crate::teleport_test_fixtures::world_maps([0]));
        assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        session.pending_teleport = Some((1, Position::default()));
        assert!(session.set_represented_far_teleport_pending_like_cpp(true));
        if invalid {
            assert!(
                session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
                    map_id: 0,
                    area_id: 0,
                    position: Position::new(f32::NAN, 0.0, 0.0, 0.0),
                })
            );
        }
        session.recover_rejected_worldport_like_cpp().await;
        assert_eq!(session.state, SessionState::Disconnecting);
        assert_eq!(session.pending_teleport, Some((1, Position::default())));
        assert!(output.is_empty());
    }
}

#[test]
fn rejected_entry_preserves_source_coordinates_and_exact_detached_owner() {
    for collision in [false, true] {
        let (mut session, _, _) = make_session();
        let guid = install_canonical_player_owner_for_test(&mut session, 0, 0);
        session.set_map_store(crate::teleport_test_fixtures::world_maps([0, 1]));
        let source = Position::new(1.0, 2.0, 3.0, 0.5);
        session.set_player_position_like_cpp(source);
        let handle = session.player_handle_like_cpp.unwrap();
        let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
        let address = session
            .with_owned_player_like_cpp(|p| p as *const Player as usize)
            .unwrap();
        assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        if collision {
            add_canonical_test_player_on_map(&manager, guid, Position::default(), 1, 0);
        } else {
            session.set_map_store(crate::teleport_test_fixtures::world_maps([0]));
        }
        assert!(
            !session
                .try_attach_worldport_destination_like_cpp(1, Position::new(10.0, 20.0, 30.0, 0.0))
        );
        assert_eq!(session.player_handle_like_cpp, Some(handle));
        assert_eq!(session.player_map_id_like_cpp(), 0);
        let manager = manager.try_lock().expect("failure releases map guards");
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(wow_map::PlayerResidenceLikeCpp::Detached)
        );
        assert_eq!(
            manager.with_player_like_cpp(handle, |p| (
                p as *const Player as usize,
                p.unit().world().map_id(),
                p.unit().world().position()
            )),
            Some((address, 0, source))
        );
    }
}

#[tokio::test]
async fn detached_return_keeps_incarnation_through_immediate_and_delayed_entry() {
    for delayed in [false, true] {
        let (mut session, _, output) = make_session();
        install_canonical_player_owner_for_test(&mut session, 0, 0);
        session.set_map_store(crate::teleport_test_fixtures::world_maps([0, 1]));
        session.set_player_health_like_cpp(100, 100);
        let handle = session.player_handle_like_cpp.unwrap();
        let address = session
            .with_owned_player_like_cpp(|p| p as *const Player as usize)
            .unwrap();
        assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        assert!(session.update_player_teleport_state_like_cpp(|state| {
            state.can_delay = delayed;
            state.far_pending = true;
            state.near_pending = true;
            state.near_destination = Some((0, Position::default()));
        }));
        let destination = Position::new(10.0, 20.0, 30.0, 0.5);
        session
            .teleport_to_with_options(0, destination, TELE_TO_SEAMLESS_LIKE_CPP)
            .await;
        if delayed {
            assert!(output.is_empty());
            assert!(session.process_represented_delayed_teleport_after_update_like_cpp());
        }
        assert_eq!(
            drain_server_opcodes(&output),
            vec![
                ServerOpcodes::CancelCombat,
                ServerOpcodes::TransferPending,
                ServerOpcodes::SuspendToken,
            ]
        );
        assert!(session.represented_far_teleport_pending_like_cpp());
        assert!(
            !session
                .player_teleport_state_snapshot_like_cpp()
                .unwrap()
                .near_pending
        );
        assert_eq!(session.pending_teleport, Some((0, destination)));
        assert_eq!(session.current_canonical_player_map_key_like_cpp(), None);
        assert_eq!(session.player_handle_like_cpp, Some(handle));
        // Exercise attachment separately from packet publication. This is not
        // full ACK/client acceptance; preparing a map must not count as entry.
        assert!(session.try_attach_worldport_destination_like_cpp(0, destination));
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(0, 0))
        );
        assert_eq!(session.player_handle_like_cpp, Some(handle));
        assert_eq!(
            session.with_owned_player_like_cpp(|p| (
                p as *const Player as usize,
                p.unit().world().position()
            )),
            Some((address, destination))
        );
    }
}

#[test]
fn map_entry_preparation_preserves_active_and_detached_player_residence() {
    for detached in [false, true] {
        let (mut session, _, output) = make_session();
        install_canonical_player_owner_for_test(&mut session, 0, 0);
        session.set_map_store(crate::teleport_test_fixtures::world_maps([0, 1]));
        let position = Position::new(7.0, 8.0, 9.0, 0.5);
        session.set_player_position_like_cpp(position);
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let handle = session.player_handle_like_cpp.unwrap();
        let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
        let residence = manager.lock().unwrap().player_residence_like_cpp(handle);
        for created in [true, false] {
            let decision = session.prepare_canonical_map_entry_like_cpp(1).unwrap();
            let key = match decision {
                wow_map::CreateMapDecision::Create { key, .. } => {
                    assert!(created);
                    key
                }
                wow_map::CreateMapDecision::Existing { key, .. } => {
                    assert!(!created);
                    key
                }
                other => panic!("unexpected admission {other:?}"),
            };
            assert_eq!(key, wow_map::MapKey::new(1, 0));
            assert_eq!(session.player_handle_like_cpp, Some(handle));
            let manager = manager
                .try_lock()
                .expect("preparation releases its map guard");
            assert_eq!(manager.player_residence_like_cpp(handle), residence);
            assert_eq!(
                manager.with_player_like_cpp(handle, |p| p.unit().world().position()),
                Some(position)
            );
            assert_eq!(manager.find_map(1, 0).unwrap().player_count(), 0);
        }
        assert!(output.is_empty());
    }
}

#[test]
fn map_entry_missing_catalog_does_not_change_the_current_player() {
    let (mut session, _, output) = make_session();
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([0]));
    let handle = session.player_handle_like_cpp.unwrap();
    assert_eq!(session.prepare_canonical_map_entry_like_cpp(1), None);
    let manager = session
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .try_lock()
        .unwrap();
    assert_eq!(
        manager.player_residence_like_cpp(handle),
        Some(wow_map::PlayerResidenceLikeCpp::Active(
            wow_map::MapKey::new(0, 0)
        ))
    );
    assert!(manager.find_map(1, 0).is_none());
    assert!(output.is_empty());
}
