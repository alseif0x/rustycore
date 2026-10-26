use super::*;

fn dungeon_maps() -> Arc<wow_data::MapStore> {
    Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 33,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]))
}

async fn repop_at_graveyard(session: &mut WorldSession, graveyards: &wow_data::GraveyardStore) {
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_entities::repop_at_graveyard_like_cpp(
        session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards,
        },
    )
    .await;
}

fn continuation_phase(session: &WorldSession) -> Option<wow_entities::PlayerResurrectionPhase> {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase)
        })
        .flatten()
}

#[tokio::test]
async fn recovery_fences_reclaim_failed_delete_retains_the_nonreplayable_owner() {
    let (mut session, _send_rx, _realm_rx, _port) =
        released_ghost_without_reclaim_delay_like_cpp(4_431).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
    corpse_port.fail_next_delete();
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    assert!(
        session.player_is_alive_like_cpp(),
        "native resurrection precedes deletion"
    );
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert_eq!(corpse_port.delete_requests(), vec![4_431]);
    assert_eq!(current_corpse_guid_like_cpp(&session, player_guid), None);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    assert!(session.core.player_handle_like_cpp.is_some());

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
    assert_eq!(corpse_port.delete_requests(), vec![4_431]);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
}

#[tokio::test]
async fn recovery_fences_reclaim_cancelled_delete_is_retained_without_retry() {
    let (mut session, _send_rx, _realm_rx, _port) =
        released_ghost_without_reclaim_delay_like_cpp(4_432).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
    corpse_port.hold_delete();
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    let mut reclaim = Box::pin(handle_reclaim_corpse(
        &mut session,
        reclaim_corpse_packet(corpse_guid),
    ));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), corpse_port.wait_for_delete()) => {
            assert!(result.is_ok(), "reclaim did not reach the controlled delete port");
        }
        _ = &mut reclaim => panic!("reclaim returned before the controlled delete completed"),
    }
    drop(reclaim);

    assert_eq!(corpse_port.delete_requests(), vec![4_432]);
    assert_eq!(current_corpse_guid_like_cpp(&session, player_guid), None);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
    assert_eq!(corpse_port.delete_requests(), vec![4_432]);
    assert_eq!(save_port.character_save_count(), 0);
}

#[tokio::test]
async fn recovery_fences_reclaim_cancelled_save_stays_saving_without_replay() {
    let (mut session, _send_rx, _realm_rx, _old_port) =
        released_ghost_without_reclaim_delay_like_cpp(4_441).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    save_port.hold_character_save();
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = session.core.canonical_map_manager.as_ref().unwrap().clone();

    let mut reclaim = Box::pin(handle_reclaim_corpse(
        &mut session,
        reclaim_corpse_packet(corpse_guid),
    ));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), save_port.wait_for_character_save()) => {
            assert!(result.is_ok(), "reclaim did not reach the held full-save port");
        }
        _ = &mut reclaim => panic!("reclaim returned before its held save resolved"),
    }
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .map(|continuation| continuation.phase)
            })
            .unwrap(),
        Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
    );
    drop(reclaim);

    assert_eq!(corpse_port.delete_requests(), vec![4_441]);
    assert_eq!(current_corpse_guid_like_cpp(&session, player_guid), None);
    assert_eq!(save_port.character_save_count(), 1);
    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
    assert_eq!(corpse_port.delete_requests(), vec![4_441]);
    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
    );
}

#[tokio::test]
async fn recovery_fences_reclaim_failed_or_unknown_save_never_replays_deletion() {
    for outcome in [
        wow_persistence::PersistenceOutcomeLikeCpp::Failed {
            reason: "recovery save failed".into(),
        },
        wow_persistence::PersistenceOutcomeLikeCpp::Unknown {
            reason: "recovery save result unknown".into(),
        },
    ] {
        let (mut session, _send_rx, _realm_rx, _old_port) =
            released_ghost_without_reclaim_delay_like_cpp(4_433).await;
        let player_guid = session.player_guid().unwrap();
        let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
        session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
        let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
        session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
        let save_port = crate::session::RecordingPlayerLifecyclePort::new(outcome);
        session.set_player_lifecycle_port_like_cpp(save_port.clone());

        handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

        assert_eq!(corpse_port.delete_requests(), vec![4_433]);
        assert_eq!(save_port.character_save_count(), 1);
        assert_eq!(
            continuation_phase(&session),
            Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
        );
        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
        handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
        assert_eq!(corpse_port.delete_requests(), vec![4_433]);
        assert_eq!(save_port.character_save_count(), 1);
        assert_eq!(
            continuation_phase(&session),
            Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
        );
    }
}

#[tokio::test]
async fn recovery_fences_reclaim_deferred_save_resumes_on_registered_worldport_ack_once() {
    let (mut session, _send_rx, _realm_rx, _old_port) =
        released_ghost_without_reclaim_delay_like_cpp(4_439).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let old_position = session.player_position_like_cpp().unwrap();
    assert!(session.set_pending_teleport_like_cpp(Some((571, old_position))));
    assert!(session.set_represented_far_teleport_pending_like_cpp(true));

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(corpse_port.delete_requests(), vec![4_439]);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::PlayerSaveDeferred)
    );
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((500, 1_000)),
        "the native half-resurrection has already run before the deferred save"
    );

    session.set_player_health_like_cpp(321, 1_000);
    crate::session::install_worldport_ack_inputs(&mut session);
    session.set_state(crate::session::SessionState::Transfer);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let response = crate::session::registry::get_handler(ClientOpcodes::WorldPortResponse)
        .expect("registered WorldPortResponse handler");
    (response.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;

    assert_eq!(session.state(), crate::session::SessionState::LoggedIn);
    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(
        save_port.character_save_requests()[0].character.health,
        321,
        "the captured full-save snapshot precedes worldport's final stat refresh and proves the half-resurrection was not replayed"
    );
    assert_eq!(corpse_port.delete_requests(), vec![4_439]);
    let (health, max_health) = session
        .canonical_player_health_snapshot_like_cpp()
        .expect("canonical Player health after registered ACK");
    assert!(health <= max_health, "worldport final stats remain within max health");
    assert_eq!(continuation_phase(&session), None);
    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
    assert_eq!(corpse_port.delete_requests(), vec![4_439]);
    assert_eq!(save_port.character_save_count(), 1);
}

#[tokio::test]
async fn recovery_fences_registered_near_ack_resumes_ready_to_save_without_replay() {
    let (mut session, send_rx, _realm_rx, player_guid) =
        dead_player_session_without_corpse_port(4_440);
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_entities::resurrect_player_like_cpp(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: None,
        },
        0.5,
        false,
    )
    .await
    .expect("the canonical Player reaches the live post-native state");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
                    restore_percent: 0.5,
                },
                wow_entities::PlayerResurrectionPhase::ReadyToSavePlayer,
            ));
        })
        .unwrap();
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    session.set_state(crate::session::SessionState::LoggedIn);
    let _native_packets = send_rx.try_iter().collect::<Vec<_>>();
    let destination = Position::new(10.0, 20.0, 30.0, 0.5);
    wow_world_entities::teleport_to_with_options(
        &mut session,
        571,
        destination,
        wow_world_entities::TELE_TO_NONE_LIKE_CPP,
    )
    .await;
    let teleport_state = session
        .core
        .with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp())
        .expect("canonical Player owns the pending near teleport");
    assert!(teleport_state.near_pending);
    assert_eq!(teleport_state.near_destination, Some((571, destination)));
    let transfer_packets = send_rx.try_iter().collect::<Vec<_>>();
    assert!(transfer_packets.iter().any(|packet| {
        u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::MoveTeleport as u16
    }));

    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let registration = crate::session::registry::get_handler(ClientOpcodes::MoveTeleportAck)
        .expect("registered MoveTeleportAck handler");
    let mut ack = WorldPacket::new_empty();
    ack.write_packed_guid(&player_guid);
    ack.write_int32(0);
    ack.write_int32(0);
    (registration.handler)(&mut session, &catalogs, ack).await;

    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(continuation_phase(&session), None);
    assert!(session.player_is_alive_like_cpp());
    assert!(
        !send_rx.try_iter().any(|packet| {
            u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::MoveTeleport as u16
        }),
        "the ACK resumes the save stage without issuing another teleport"
    );
}

#[tokio::test]
async fn recovery_fences_reclaim_post_native_conversion_without_corpse_finishes() {
    let (mut session, _send_rx, _realm_rx, player_guid) =
        dead_player_session_without_corpse_port(4_434);
    let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_entities::resurrect_player_like_cpp(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: None,
        },
        0.5,
        false,
    )
    .await
    .expect("the real native resurrection establishes the reclaim post-native phase");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
                    restore_percent: 0.5,
                },
                wow_entities::PlayerResurrectionPhase::RecoveryBonesReady,
            ));
        })
        .unwrap();
    assert!(
        wow_world_entities::continue_player_resurrection_after_teleport(
            &mut session,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creatures,
                graveyards: None,
            },
        )
        .await
    );
    assert_eq!(session.player_guid(), Some(player_guid));
    assert!(session.player_is_alive_like_cpp());
    assert!(corpse_port.delete_requests().is_empty());
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(continuation_phase(&session), None);
}

#[tokio::test]
async fn recovery_fences_reclaim_missing_corpse_rejects_before_operation_admission() {
    let (mut session, _send_rx, _realm_rx, _old_port) =
        released_ghost_without_reclaim_delay_like_cpp(4_443).await;
    session.set_state(crate::session::SessionState::LoggedIn);
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            false,
        )
        .await
    );
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());

    let corpse_port = Arc::new(RecordingCorpsePortLikeCpp::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    assert!(session.player_is_alive_like_cpp() == false);
    assert_eq!(corpse_port.delete_requests(), Vec::<u64>::new());
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(continuation_phase(&session), None);
    assert_eq!(session.state(), crate::session::SessionState::LoggedIn);
}

#[tokio::test]
async fn recovery_fences_forced_repop_no_corpse_stops_countdown_without_save_or_delete() {
    let (mut session, _send_rx, _realm_rx, _player_guid, corpse_port) =
        dead_player_session_on_map_like_cpp(4_435, dungeon_maps());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    repop_at_graveyard(&mut session, &wow_data::GraveyardStore::default()).await;

    assert_eq!(corpse_port.delete_requests(), Vec::<u64>::new());
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
    assert_eq!(continuation_phase(&session), None);
}

#[tokio::test]
async fn recovery_fences_forced_repop_delete_failure_stops_before_countdown_or_teleport() {
    let (mut session, send_rx, _realm_rx, _player_guid, corpse_port) =
        dead_player_session_on_map_like_cpp(4_436, dungeon_maps());
    corpse_port.fail_next_delete();
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_eq!(corpse_port.delete_requests(), vec![4_436]);
    assert_eq!(current_corpse_guid_like_cpp(&session, _player_guid), None);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|packet| u16::from_le_bytes([packet[0], packet[1]]))
        .collect::<Vec<_>>();
    assert!(!opcodes.contains(&(ServerOpcodes::MoveTeleport as u16)));
}

#[tokio::test]
async fn recovery_fences_forced_repop_cancelled_delete_keeps_countdown_and_owner_held() {
    let (mut session, _send_rx, _realm_rx, _player_guid, corpse_port) =
        dead_player_session_on_map_like_cpp(4_437, dungeon_maps());
    corpse_port.hold_delete();
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let mut repop = Box::pin(wow_world_entities::handle_repop_request(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: &graveyards,
        },
        repop_request_packet(true),
    ));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), corpse_port.wait_for_delete()) => {
            assert!(result.is_ok(), "forced Repop did not reach corpse deletion");
        }
        _ = &mut repop => panic!("forced Repop returned before its held delete resolved"),
    }
    drop(repop);

    assert_eq!(corpse_port.delete_requests(), vec![4_437]);
    assert_eq!(current_corpse_guid_like_cpp(&session, _player_guid), None);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    repop_at_graveyard(&mut session, &graveyards).await;
    assert_eq!(corpse_port.delete_requests(), vec![4_437]);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
}

#[tokio::test]
async fn recovery_fences_forced_repop_cancelled_save_keeps_the_existing_owner_held() {
    let (mut session, _send_rx, _realm_rx, _player_guid, corpse_port) =
        dead_player_session_on_map_like_cpp(4_442, dungeon_maps());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    save_port.hold_character_save();
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = session.core.canonical_map_manager.as_ref().unwrap().clone();
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let mut repop = Box::pin(wow_world_entities::handle_repop_request(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: &graveyards,
        },
        repop_request_packet(true),
    ));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), save_port.wait_for_character_save()) => {
            assert!(result.is_ok(), "forced Repop did not reach the held save port");
        }
        _ = &mut repop => panic!("forced Repop returned before its held save resolved"),
    }
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .map(|continuation| continuation.phase)
            })
            .unwrap(),
        Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
    );
    drop(repop);

    assert_eq!(corpse_port.delete_requests(), vec![4_442]);
    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    repop_at_graveyard(&mut session, &graveyards).await;
    assert_eq!(corpse_port.delete_requests(), vec![4_442]);
    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert_eq!(
        continuation_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
    );
}

#[tokio::test]
async fn recovery_fences_forced_repop_failed_or_unknown_save_blocks_post_delete_work() {
    for outcome in [
        wow_persistence::PersistenceOutcomeLikeCpp::Failed {
            reason: "forced repop save failed".into(),
        },
        wow_persistence::PersistenceOutcomeLikeCpp::Unknown {
            reason: "forced repop save result unknown".into(),
        },
    ] {
        let (mut session, send_rx, _realm_rx, _player_guid, corpse_port) =
            dead_player_session_on_map_like_cpp(4_438, dungeon_maps());
        let save_port = crate::session::RecordingPlayerLifecyclePort::new(outcome);
        session.set_player_lifecycle_port_like_cpp(save_port.clone());

        handle_repop_request(&mut session, repop_request_packet(true)).await;

        assert_eq!(corpse_port.delete_requests(), vec![4_438]);
        assert_eq!(save_port.character_save_count(), 1);
        assert_eq!(
            session.fixtures.combat.represented_repop_at_graveyard_count,
            0
        );
        assert_eq!(
            continuation_phase(&session),
            Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
        );
        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
        let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|packet| u16::from_le_bytes([packet[0], packet[1]]))
            .collect::<Vec<_>>();
        assert!(!opcodes.contains(&(ServerOpcodes::MoveTeleport as u16)));
        repop_at_graveyard(&mut session, &wow_data::GraveyardStore::default()).await;
        assert_eq!(corpse_port.delete_requests(), vec![4_438]);
        assert_eq!(save_port.character_save_count(), 1);
    }
}
