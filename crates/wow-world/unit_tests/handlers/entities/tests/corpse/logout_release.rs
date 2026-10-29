//! Native death release belongs to LogoutPlayer, before its single final save.

use super::*;
use std::future::Future;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::Poll;
use wow_world_lifecycle::{
    FinalizationDisposition as Disposition, FinalizationMode as Mode,
    FinalizationOutcome as Outcome, FinalizationStep as Step,
};

fn pending_death(
    counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    Arc<RecordingCorpsePortLikeCpp>,
    Arc<crate::session::RecordingPlayerLifecyclePort>,
) {
    let (mut session, instance, realm, guid, corpse) =
        dead_player_session_with_graveyard_like_cpp(counter);
    session.core.state = crate::session::SessionState::LoggedIn;
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            player
                .resurrection_state_mut_like_cpp()
                .death_timer_remaining_ms = 360_000;
        })
        .unwrap();
    let save = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save.clone());
    (session, instance, realm, guid, corpse, save)
}

async fn finalize_with_writer(
    session: &mut WorldSession,
    mode: Mode,
    output: &flume::Receiver<Vec<u8>>,
    graveyards: &wow_data::GraveyardStore,
) -> (wow_world_lifecycle::FinalizationReport, Vec<Vec<u8>>, usize) {
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let fence = wow_network::SocketWriteFenceLikeCpp::default();
    session
        .core
        .transport
        .connection
        .set_send_write_fence_like_cpp(fence.clone());
    let mut operation = Box::pin(
        wow_world_application::finalize_session_with_resurrection_catalogs(
            session,
            mode,
            generators.item.as_ref(),
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creatures,
                graveyards: Some(graveyards),
            },
        ),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut packets = Vec::new();
    let mut markers = 0;
    loop {
        let result = std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx))).await;
        for bytes in output.try_iter() {
            if fence.acknowledge_marker_like_cpp(&bytes) {
                markers += 1;
            } else {
                packets.push(bytes);
            }
        }
        if let Poll::Ready(report) = result {
            return (report, packets, markers);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native logout did not finish"
        );
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn all_logout_modes_release_before_one_graveyard_save_and_completion_publication() {
    for (index, mode) in [
        Mode::TimedLogout,
        Mode::CharacterSelection,
        Mode::Disconnect,
    ]
    .into_iter()
    .enumerate()
    {
        let (mut session, instance, realm, guid, corpse_port, save_port) =
            pending_death(9_400 + index as i64);
        let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
        let graveyards = wow_data::GraveyardStore::default();
        let (report, packets, markers) =
            finalize_with_writer(&mut session, mode, &instance, &graveyards).await;
        assert_eq!(report.disposition, Disposition::Complete);
        assert_eq!(report.outcome(Step::DeathRelease), Outcome::Applied);
        assert_eq!(report.outcome(Step::CharacterSave), Outcome::Applied);
        assert_eq!(corpse_port.saves.lock().unwrap().len(), 1);
        let requests = save_port.character_save_requests();
        assert_eq!(
            requests.len(),
            1,
            "BuildPlayerRepop must not save the Player separately"
        );
        let saved = &requests[0].character;
        assert_eq!(saved.health, 1);
        assert_ne!(saved.player_flags & 0x10, 0, "PLAYER_FLAGS_GHOST");
        assert_eq!(
            (
                saved.position.map_id,
                saved.position.x,
                saved.position.y,
                saved.position.z
            ),
            (571, 50.0, 60.0, 70.0)
        );
        let mut manager = manager.lock().unwrap();
        let map = manager.find_map_mut(571, 0).unwrap().map_mut();
        let corpse = map
            .corpse_guid_by_player_like_cpp(guid)
            .expect("one native logout corpse");
        assert!(
            map.get_typed_corpse(corpse)
                .unwrap()
                .world()
                .object()
                .is_in_world()
        );
        assert!(
            map.get_typed_player(guid).is_none(),
            "only completed native work authorizes retirement"
        );
        drop(manager);
        let realm_packets = realm.try_iter().collect::<Vec<_>>();
        if mode == Mode::Disconnect {
            assert!(
                packets.is_empty() && realm_packets.is_empty(),
                "closed self socket is not a native-effect receipt"
            );
            assert_eq!(markers, 0);
        } else {
            assert_eq!(
                markers, 1,
                "instance writer completes before the realm logout ACK"
            );
            assert!(
                packets
                    .iter()
                    .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                        == ServerOpcodes::UpdateObject as u16)
            );
            assert!(
                !packets
                    .iter()
                    .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                        == ServerOpcodes::MoveTeleport as u16),
                "logout does not wait for client teleport ACK"
            );
            assert_eq!(
                realm_packets.last(),
                Some(&wow_packet::packets::misc::LogoutComplete.to_bytes())
            );
            assert_eq!(
                realm_packets.first(),
                Some(&wow_packet::packets::combat::CancelCombat.to_bytes())
            );
            let pre = realm_packets
                .iter()
                .position(|bytes| {
                    u16::from_le_bytes([bytes[0], bytes[1]]) == ServerOpcodes::PreRessurect as u16
                })
                .unwrap();
            assert!(pre < realm_packets.len() - 1);
        }
    }
}

#[tokio::test]
async fn death_release_requires_catalogs_only_when_the_native_timer_is_running() {
    for timer in [0, 360_000] {
        let (mut session, _instance, _realm, guid, corpse, save) =
            pending_death(9_410 + i64::from(timer));
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .death_timer_remaining_ms = timer;
                if timer == 0 {
                    player
                        .unit_mut()
                        .set_death_state(wow_constants::DeathState::Alive);
                    player.unit_mut().set_health(100);
                }
            })
            .unwrap();
        let generators = session.id_generators_for_test_like_cpp();
        let report = wow_world_application::finalize_session_with_generator_like_cpp(
            &mut session,
            Mode::Disconnect,
            generators.item.as_ref(),
        )
        .await;
        if timer == 0 {
            assert_eq!(report.outcome(Step::DeathRelease), Outcome::NoWork);
            assert_eq!(report.disposition, Disposition::Complete);
            assert_eq!(save.character_save_count(), 1);
        } else {
            assert_eq!(report.outcome(Step::DeathRelease), Outcome::Unavailable);
            assert_eq!(report.disposition, Disposition::RetainAndEscalate);
            assert_eq!(report.outcome(Step::CharacterSave), Outcome::NotAttempted);
            assert_eq!(report.outcome(Step::Retirement), Outcome::NotAttempted);
            assert_eq!(save.character_save_count(), 0);
            assert_eq!(session.player_guid(), Some(guid));
        }
        assert!(corpse.saves.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn not_yet_due_logout_timer_does_not_release_or_start_finalization() {
    let (mut session, _instance, _realm, guid, corpse, save) = pending_death(9_411);
    session
        .lifecycle
        .set_logout_start_time_like_cpp(wow_entities::game_time_secs_like_cpp() + 3_600);
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    wow_world_application::run_logout_timer_with_catalogs(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: Some(&graveyards),
        },
    )
    .await;
    assert!(session.finalization_report_like_cpp().is_none());
    assert_eq!(session.player_guid(), Some(guid));
    assert!(corpse.saves.lock().unwrap().is_empty());
    assert_eq!(save.character_save_count(), 0);
}

#[tokio::test]
async fn disconnected_full_self_queues_still_apply_ghost_wisp_and_publish_the_peer_corpse() {
    let (mut session, instance, realm, guid, corpse, save) = pending_death(9_413);
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([])));
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let mut spells = wow_entities::PlayerSpellRuntimeState::default();
            spells.add_known_spell_like_cpp(wow_world_entities::SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP);
            player.replace_spell_runtime_like_cpp(spells);
        })
        .unwrap();
    let (peer, peer_output, _) = super::bones_birth::install_peer(&mut session, 9_414);
    peer.core.client_visible_guids_like_cpp.insert(guid);
    while session.core.send_tx().try_send(vec![0xFF]).is_ok() {}
    while session.core.realm_route_tx().try_send(vec![0xFE]).is_ok() {}
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let report = wow_world_application::finalize_session_with_resurrection_catalogs(
        &mut session,
        Mode::Disconnect,
        generators.item.as_ref(),
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: Some(&graveyards),
        },
    )
    .await;
    assert_eq!(report.disposition, Disposition::Complete);
    assert_eq!(report.outcome(Step::DeathRelease), Outcome::Applied);
    assert_eq!(save.character_save_count(), 1);
    let requests = save.character_save_requests();
    let auras = requests[0]
        .ghost_wisp
        .as_ref()
        .expect("canonical Ghost/Wisp save families");
    for spell in [8326, 20584] {
        assert!(
            auras
                .iter()
                .any(|family| family.spell_id == spell && !family.auras.is_empty())
        );
    }
    let expected = {
        let manager = manager.lock().unwrap();
        let map = manager.find_map(571, 0).unwrap().map();
        let corpse_guid = map.corpse_guid_by_player_like_cpp(guid).unwrap();
        let data = wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(
            map.get_typed_corpse(corpse_guid).unwrap(),
        );
        wow_packet::packets::update::UpdateObject::create_world_objects(
            vec![wow_packet::packets::update::UpdateObject::create_corpse_birth_block(data)],
            571,
        )
        .to_bytes()
    };
    assert_eq!(peer_output.try_iter().collect::<Vec<_>>(), vec![expected]);
    assert_eq!(corpse.saves.lock().unwrap().len(), 1);
    assert!(instance.try_iter().all(|bytes| bytes == [0xFF]));
    assert!(
        realm.try_iter().all(|bytes| bytes == [0xFE]),
        "no self ACK is required or emitted after disconnect"
    );
}

#[tokio::test]
async fn logout_completes_graveyard_far_transfer_natively_without_waiting_for_a_client_ack() {
    let (mut session, _instance, _realm, guid, corpse, save) = pending_death(9_415);
    let maps = crate::teleport_test_fixtures::world_maps([571, 0]);
    session.set_map_store(Arc::clone(&maps));
    let (safe_locs, _) = wow_data::WorldSafeLocStore::from_rows_like_cpp(
        [wow_data::WorldSafeLocRow {
            id: wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP,
            map_id: 0,
            x: 50.0,
            y: 60.0,
            z: 70.0,
            facing_degrees: 0.0,
        }],
        &maps,
    );
    session.set_world_safe_loc_store_like_cpp(Arc::new(safe_locs));
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let report = wow_world_application::finalize_session_with_resurrection_catalogs(
        &mut session,
        Mode::Disconnect,
        generators.item.as_ref(),
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: Some(&graveyards),
        },
    )
    .await;
    assert_eq!(report.disposition, Disposition::Complete);
    assert_eq!(report.outcome(Step::DeathRelease), Outcome::Applied);
    assert_eq!(save.character_save_count(), 1);
    let saved = &save.character_save_requests()[0].character;
    assert_eq!(
        (
            saved.position.map_id,
            saved.position.x,
            saved.position.y,
            saved.position.z
        ),
        (0, 50.0, 60.0, 70.0)
    );
    assert_eq!(corpse.saves.lock().unwrap().len(), 1);
    assert_eq!(
        corpse.saves.lock().unwrap()[0].owner_guid,
        guid.counter() as u64
    );
}

#[tokio::test]
async fn already_released_ghost_has_no_death_release_work_and_needs_no_catalogs() {
    let (mut session, instance, realm, guid, corpse, save) = pending_death(9_416);
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    assert!(
        wow_world_entities::build_player_repop_like_cpp(
            &mut session,
            wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creatures,
                graveyards: &graveyards
            }
        )
        .await
    );
    wow_world_entities::repop_at_graveyard_like_cpp(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: &graveyards,
        },
    )
    .await;
    let corpse_guid = current_corpse_guid_like_cpp(&session, guid).unwrap();
    let _ = instance.try_iter().count();
    let _ = realm.try_iter().count();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let report = wow_world_application::finalize_session_with_generator_like_cpp(
        &mut session,
        Mode::Disconnect,
        generators.item.as_ref(),
    )
    .await;
    assert_eq!(report.disposition, Disposition::Complete);
    assert_eq!(report.outcome(Step::DeathRelease), Outcome::NoWork);
    assert_eq!(corpse.saves.lock().unwrap().len(), 1);
    assert_eq!(save.character_save_count(), 1);
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .corpse_guid_by_player_like_cpp(guid),
        Some(corpse_guid)
    );
    assert!(instance.is_empty() && realm.is_empty());
}

#[tokio::test]
async fn running_death_timer_requires_the_graveyard_capability_inside_supplied_catalogs() {
    let (mut session, _instance, _realm, guid, corpse, save) = pending_death(9_417);
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let report = wow_world_application::finalize_session_with_resurrection_catalogs(
        &mut session,
        Mode::Disconnect,
        generators.item.as_ref(),
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: None,
        },
    )
    .await;
    assert_eq!(report.outcome(Step::DeathRelease), Outcome::Unavailable);
    assert_eq!(report.disposition, Disposition::RetainAndEscalate);
    assert_eq!(report.outcome(Step::CharacterSave), Outcome::NotAttempted);
    assert_eq!(report.outcome(Step::Retirement), Outcome::NotAttempted);
    assert_eq!(session.player_guid(), Some(guid));
    assert!(corpse.saves.lock().unwrap().is_empty());
    assert_eq!(save.character_save_count(), 0);
}

#[tokio::test]
async fn corpse_save_failure_unknown_or_cancellation_holds_logout_without_replay() {
    for (index, result) in [
        None,
        Some("committed"),
        Some("failed"),
        Some("unknown_commit"),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut session, _instance, realm, guid, _corpse, save) =
            pending_death(9_420 + index as i64);
        let (tx, rx) = flume::bounded(1);
        let gated = Arc::new(super::release::GatedCorpseSavePort {
            receiver: Mutex::new(Some(rx)),
            started: AtomicBool::new(false),
            saves: AtomicUsize::new(0),
        });
        session.set_map_corpse_persistence_port_like_cpp(gated.clone());
        let handle = session.core.player_handle_like_cpp.unwrap();
        let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
        let generators = session.id_generators_for_test_like_cpp();
        let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
        let graveyards = wow_data::GraveyardStore::default();
        let catalogs = wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creatures,
            graveyards: Some(&graveyards),
        };
        let mut operation = Box::pin(
            wow_world_application::finalize_session_with_resurrection_catalogs(
                &mut session,
                Mode::Disconnect,
                generators.item.as_ref(),
                catalogs,
            ),
        );
        assert!(
            std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx).is_pending())).await
        );
        assert!(gated.started.load(Ordering::SeqCst));
        assert_eq!(save.character_save_count(), 0);
        assert!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| {
                    assert!(player.trade_session_logging_out_like_cpp());
                    assert_eq!(
                        player
                            .resurrection_state_like_cpp()
                            .delayed_after_teleport
                            .unwrap()
                            .phase,
                        wow_entities::PlayerResurrectionPhase::CorpseSaveInFlight
                    );
                })
                .is_some()
        );
        if let Some(reason) = result {
            let outcome = if reason == "committed" {
                wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed
            } else {
                wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed {
                    reason: reason.into(),
                }
            };
            tx.try_send(outcome).unwrap();
            let report = operation.await;
            if reason == "committed" {
                assert_eq!(report.disposition, Disposition::Complete);
                assert_eq!(report.outcome(Step::DeathRelease), Outcome::Applied);
                assert_eq!(save.character_save_count(), 1);
                assert_eq!(gated.saves.load(Ordering::SeqCst), 1);
                assert_eq!(
                    wow_world_application::finalize_session_with_resurrection_catalogs(
                        &mut session,
                        Mode::Disconnect,
                        generators.item.as_ref(),
                        catalogs
                    )
                    .await,
                    report
                );
                assert_eq!(save.character_save_count(), 1);
                continue;
            }
            assert_eq!(report.outcome(Step::DeathRelease), Outcome::Deferred);
        } else {
            drop(operation);
        }
        assert_eq!(save.character_save_count(), 0);
        let report = wow_world_application::finalize_session_with_resurrection_catalogs(
            &mut session,
            Mode::Disconnect,
            generators.item.as_ref(),
            catalogs,
        )
        .await;
        assert_eq!(report.disposition, Disposition::RetainAndEscalate);
        assert_eq!(report.outcome(Step::Retirement), Outcome::NotAttempted);
        assert_eq!(report.outcome(Step::CharacterSave), Outcome::NotAttempted);
        assert_eq!(gated.saves.load(Ordering::SeqCst), 1);
        assert_eq!(session.player_guid(), Some(guid));
        assert_eq!(session.core.player_handle_like_cpp, Some(handle));
        assert!(
            !realm
                .try_iter()
                .any(|bytes| bytes == wow_packet::packets::misc::LogoutComplete.to_bytes())
        );
    }
}
