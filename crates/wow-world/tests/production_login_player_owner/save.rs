//! Production wow-world (without cfg(test)): admitted save, real map access during
//! delayed I/O, late incarnation completion and saturated delivery. The port is
//! controlled, not MariaDB; partial login hydration is not full login/relogin QA.
use super::*;
use std::sync::Mutex;
use wow_packet::ServerPacket;
#[path = "save/logout.rs"]
mod logout;

fn character_save_outcome(report: wow_world::FinalizationReport) -> PlayerSaveOutcomeLikeCpp {
    match report.outcome(wow_world::FinalizationStep::CharacterSave) {
        wow_world::FinalizationOutcome::Applied => PlayerSaveOutcomeLikeCpp::Applied,
        wow_world::FinalizationOutcome::DefinitelyRolledBack => PlayerSaveOutcomeLikeCpp::Failed,
        wow_world::FinalizationOutcome::Unknown | wow_world::FinalizationOutcome::InFlight => {
            PlayerSaveOutcomeLikeCpp::Quarantined
        }
        wow_world::FinalizationOutcome::Deferred => PlayerSaveOutcomeLikeCpp::Deferred,
        other => panic!("unexpected Character obligation: {other:?}"),
    }
}

pub(super) async fn assert_terminal_source_save(session: &mut WorldSession, port: &LoginPort) {
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    port.retain_after_save.store(true, Ordering::SeqCst);
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    session
        .finalize_session_with_generator_like_cpp(
            wow_world::FinalizationMode::Disconnect,
            &generator,
        )
        .await;
    let requests = probe.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    // The login fixture installs its canonical Player at the origin, not the rejected
    // destination (7,8,9) or homebind (100,200,300).
    assert_eq!(requests[0].character.position.map_id, 0);
    assert_eq!(requests[0].character.position.x, 0.0);
    assert_eq!(requests[0].character.position.y, 0.0);
    assert_eq!(requests[0].character.position.z, 0.0);
}

pub(super) struct SaveProbe {
    pub(super) requests: Mutex<Vec<PlayerCharacterSaveRequestLikeCpp>>,
    pub(super) released: AtomicBool,
    pub(super) outcome: PersistenceOutcomeLikeCpp,
}

pub(super) fn save<'a>(
    port: &'a LoginPort,
    request: PlayerCharacterSaveRequestLikeCpp,
) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
    let probe = port
        .save_probe
        .lock()
        .unwrap()
        .clone()
        .expect("unexpected save");
    let committed = request.committed_groups_like_cpp();
    probe.requests.lock().unwrap().push(request);
    Box::pin(async move {
        // Tests poll explicitly before and after release; no timer/sleep ordering.
        std::future::poll_fn(|_| {
            if probe.released.load(Ordering::SeqCst) {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        PlayerCharacterSaveResultLikeCpp {
            outcome: probe.outcome.clone(),
            committed,
        }
    })
}

fn spell(id: i32) -> wow_entities::PlayerKnownSpellRecord {
    wow_entities::PlayerKnownSpellRecord {
        spell_id: id,
        state: wow_entities::PlayerSpellLoadState::New,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    }
}

async fn exercise(replace: bool, cancel: bool, outcome: PersistenceOutcomeLikeCpp) {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    let manager = Arc::clone(&port.manager);
    let guid = ObjectGuid::create_player(1, 42);
    {
        let mut owner = manager.lock().unwrap();
        let p = owner
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        let spells = &mut p.gameplay_state_mut().spells;
        spells.replace_rows_like_cpp(std::collections::BTreeMap::from([(10, spell(10))]), true);
        // Change only the canonical owner after login hydrated Session's level.
        p.unit_mut().set_level(73);
        p.teleport_state_mut_like_cpp().far_pending = true;
        assert_eq!(p.defer_save_if_transfer_pending_like_cpp(), Some(true));
        p.teleport_state_mut_like_cpp().far_pending = false;
        owner.create_world_map(1, 0);
    }
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(false),
        outcome: outcome.clone(),
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    port.retain_after_save.store(true, Ordering::SeqCst);
    for _ in 0..8 {
        output.try_send(vec![0]).unwrap();
    }
    assert!(output.is_full());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut future = Box::pin(session.finalize_session_with_generator_like_cpp(
        wow_world::FinalizationMode::Disconnect,
        &generator,
    ));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx).is_pending()))
            .await
    );
    let request = probe.requests.lock().unwrap()[0].clone();
    assert_eq!(request.player_guid, 42);
    assert_eq!(request.character.level, 73);
    assert!(matches!(
        request.spells,
        Some(PlayerSpellSaveGroupLikeCpp::Complete { .. })
    ));
    let replacement = {
        let mut owner = manager
            .try_lock()
            .expect("pending DB must not retain the owner");
        assert!(
            !owner.destroy_map(0, 0),
            "a pending save must not lose its still-active Player through map destruction"
        );
        owner.update(10); // Both real map updates can progress while the save is pending.
        if replace {
            let mut p = Box::new(wow_entities::Player::new(Some(1), false));
            p.unit_mut().world_mut().object_mut().create(guid);
            p.gameplay_state_mut()
                .spells
                .replace_rows_like_cpp(std::collections::BTreeMap::from([(10, spell(10))]), true);
            p.teleport_state_mut_like_cpp().far_pending = true;
            assert_eq!(p.defer_save_if_transfer_pending_like_cpp(), Some(true));
            p.teleport_state_mut_like_cpp().far_pending = false;
            Some(owner.install_detached_player_like_cpp(p).unwrap())
        } else {
            owner
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .get_typed_player_mut(guid)
                .unwrap()
                .gameplay_state_mut()
                .spells
                .insert_row_like_cpp(20, spell(20));
            None
        }
    };
    if !cancel {
        probe.released.store(true, Ordering::SeqCst);
        assert_eq!(
            character_save_outcome(future.await),
            match outcome {
                PersistenceOutcomeLikeCpp::Applied { .. } => PlayerSaveOutcomeLikeCpp::Applied,
                PersistenceOutcomeLikeCpp::Failed { .. } => PlayerSaveOutcomeLikeCpp::Failed,
                PersistenceOutcomeLikeCpp::Unknown { .. } => PlayerSaveOutcomeLikeCpp::Quarantined,
            }
        );
    } else {
        drop(future);
    }
    let owner = manager
        .try_lock()
        .expect("completion releases its owner before publication");
    let inspect = |p: &wow_entities::Player| {
        let spells = &p.gameplay_state().spells;
        let clean =
            !replace && !cancel && matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. });
        assert_eq!(p.has_deferred_player_save_like_cpp(), !clean);
        assert_eq!(
            spells.rows_like_cpp()[&10].state,
            if clean {
                wow_entities::PlayerSpellLoadState::Unchanged
            } else {
                wow_entities::PlayerSpellLoadState::New
            }
        );
        if !replace {
            assert_eq!(
                spells.rows_like_cpp()[&20].state,
                wow_entities::PlayerSpellLoadState::New
            );
        }
    };
    if let Some(handle) = replacement {
        owner.with_player_like_cpp(handle, inspect).unwrap();
    } else {
        inspect(
            owner
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .unwrap(),
        );
    }
    drop(owner);
    assert_eq!(
        receiver.len(),
        8,
        "save/ACK is not packet delivery under backpressure"
    );
    assert_eq!(probe.requests.lock().unwrap().len(), 1);
    if cancel || matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. }) {
        let report = session
            .finalize_session_with_generator_like_cpp(
                wow_world::FinalizationMode::Disconnect,
                &generator,
            )
            .await;
        assert_eq!(
            character_save_outcome(report),
            PlayerSaveOutcomeLikeCpp::Quarantined
        );
        assert_eq!(
            probe.requests.lock().unwrap().len(),
            1,
            "unknown/cancelled transaction cannot be retried over an uncertain durable base"
        );
    }
}

#[tokio::test]
async fn production_save_retains_later_spell_and_allows_two_maps_with_full_output() {
    exercise(false, false, PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}

#[tokio::test]
async fn production_disconnect_finishes_retained_worldport_before_save_with_full_output() {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        player.set_inventory_slot_count(wow_entities::INVENTORY_DEFAULT_SIZE);
        player
            .inventory_runtime_mut_like_cpp()
            .set_equipment_inventory_authority_complete_like_cpp(true);
        let position = player.unit().world().position();
        player.teleport_state_mut_like_cpp().post_add =
            Some(wow_entities::PlayerWorldportPostAddLikeCpp {
                map_id: 0,
                position,
                phase: wow_entities::PlayerWorldportPostAddPhaseLikeCpp::ZoneApplied,
            });
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        player.gameplay_state_mut().using_pvp_item_levels = true;
        player.unit_mut().set_max_health(1000);
        player.unit_mut().set_health(500);
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(wow_entities::PlayerResurrectionContinuation {
            operation: wow_entities::PlayerResurrectionOperation::RequestData(
                wow_entities::PlayerResurrectionRequestLikeCpp {
                    resurrecter: guid,
                    map_id: 0,
                    position,
                    health: 123,
                    mana: 0,
                    aura: 0,
                },
            ),
            phase: wow_entities::PlayerResurrectionPhase::AwaitingTeleport,
        });
        // A represented equipped item makes the normal scaling path publish stats.
        // Finalization must complete native work even when that output is saturated.
        let item_guid = ObjectGuid::create_item(1, 99);
        let mut item = wow_entities::Item::new(0);
        item.object_mut().create(item_guid);
        item.object_mut().set_entry(6948);
        player
            .inventory_runtime_mut_like_cpp()
            .item_objects_mut()
            .insert(item_guid, item);
    }
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    port.retain_after_save.store(true, Ordering::SeqCst);
    for _ in 0..8 {
        output.try_send(vec![0]).unwrap();
    }
    assert!(output.is_full());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let creature_spawns = wow_world_entities::CreatureSpawnCatalogsLikeCpp::default();
    wow_world_application::finalize_session_with_resurrection_catalogs(
        &mut session,
        wow_world::FinalizationMode::Disconnect,
        &generator,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: &generator,
            creature_spawns: &creature_spawns,
            graveyards: None,
        },
    )
    .await;
    assert_eq!(
        receiver.len(),
        8,
        "native finalization does not publish packets"
    );
    let requests = probe.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].character.health, 123);
    let manager = port.manager.lock().unwrap();
    let player = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(player.teleport_state_like_cpp().post_add.is_none());
    assert!(!player.has_deferred_player_save_like_cpp());
    assert!(
        player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .is_none()
    );
    assert!(!player.gameplay_state().using_pvp_item_levels);
}

#[tokio::test]
async fn production_disconnect_completes_pending_far_transfer_before_save() {
    pending_far_disconnect(DisconnectDestination::Requested, false, true).await;
}

#[tokio::test]
async fn production_disconnect_recovers_pending_far_transfer_to_homebind_without_packets() {
    pending_far_disconnect(DisconnectDestination::Homebind, false, true).await;
}

#[tokio::test]
async fn production_disconnect_rejected_far_and_homebind_preserves_terminal_source_save() {
    pending_far_disconnect(DisconnectDestination::Rejected, false, false).await;
    pending_far_disconnect(DisconnectDestination::Rejected, false, true).await;
}

#[derive(Clone, Copy, PartialEq)]
enum DisconnectDestination {
    Requested,
    Homebind,
    Rejected,
}

async fn pending_far_disconnect(
    outcome: DisconnectDestination,
    explicit_logout: bool,
    retain_resurrection: bool,
) {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    let destination = Position::new(7.0, 8.0, 9.0, 0.5);
    let home = Position::new(100.0, 200.0, 300.0, 0.0);
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
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        if explicit_logout {
            // C++ instant logout (resting, `MiscHandler.cpp:244`): this case
            // exercises `LogoutPlayer(true)`, not the 20 s countdown (#54).
            player.set_player_flag(0x20);
        }
        player.unit_mut().set_max_health(1000);
        player.unit_mut().set_health(500);
        if outcome != DisconnectDestination::Rejected {
            player.set_inventory_slot_count(wow_entities::INVENTORY_DEFAULT_SIZE);
            player
                .inventory_runtime_mut_like_cpp()
                .set_equipment_inventory_authority_complete_like_cpp(true);
        }
        player
            .gameplay_state_mut()
            .homebind
            .as_mut()
            .unwrap()
            .position = home;
        player.teleport_state_mut_like_cpp().far_pending = true;
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        player.teleport_state_mut_like_cpp().far_pending = false;
        if retain_resurrection {
            // This fixture stops at PetStable before class power hydration.
            assert_eq!(
                player
                    .unit()
                    .get_power_index(wow_constants::PowerType::Mana),
                Some(0)
            );
            assert_eq!(
                player
                    .unit()
                    .get_power_index(wow_constants::PowerType::Rage),
                None
            );
            player
                .resurrection_state_mut_like_cpp()
                .delayed_after_teleport = Some(wow_entities::PlayerResurrectionContinuation {
                operation: wow_entities::PlayerResurrectionOperation::RequestData(
                    wow_entities::PlayerResurrectionRequestLikeCpp {
                        resurrecter: guid,
                        map_id: 1,
                        position: destination,
                        health: 123,
                        mana: 0,
                        aura: 0,
                    },
                ),
                phase: wow_entities::PlayerResurrectionPhase::AwaitingTeleport,
            });
        }
    }
    session.set_state(SessionState::LoggedIn);
    session.teleport_to(1, destination).await;
    if outcome != DisconnectDestination::Requested {
        session.set_map_store(Arc::new(MapStore::from_entries(
            (outcome == DisconnectDestination::Homebind).then_some(MapEntry {
                id: 0,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            }),
        )));
    }
    while receiver.try_recv().is_ok() {}
    if !explicit_logout {
        for _ in 0..8 {
            output.try_send(vec![0]).unwrap();
        }
        assert!(output.is_full());
        session.kick("controlled disconnect before suspend/worldport ACK");
    }
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    port.retain_after_save.store(true, Ordering::SeqCst);
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    port.retain_after_save
        .store(!explicit_logout, Ordering::SeqCst);
    let creature_spawns = wow_world_entities::CreatureSpawnCatalogsLikeCpp::default();
    let resurrection_catalogs = wow_world_entities::ResurrectionOperationCatalogs {
        item_guid_generator: &generator,
        creature_spawns: &creature_spawns,
        graveyards: None,
    };
    let report = if explicit_logout {
        // Direct production adapter call proves LogoutPlayer ordering, not that
        // the LoggedIn-only network registration admits a packet during Transfer.
        wow_world_character::handle_logout_request_with_catalogs(
            &mut session,
            resurrection_catalogs,
            wow_packet::packets::misc::LogoutRequest { idle_logout: false },
        )
        .await;
        session
            .finalization_report_like_cpp()
            .expect("admitted explicit logout")
    } else {
        wow_world_application::finalize_session_with_resurrection_catalogs(
            &mut session,
            wow_world::FinalizationMode::Disconnect,
            &generator,
            resurrection_catalogs,
        )
        .await
    };
    if outcome == DisconnectDestination::Rejected && retain_resurrection {
        assert_eq!(
            report.outcome(wow_world::FinalizationStep::NativeTransfer),
            wow_world::FinalizationOutcome::Unavailable
        );
        for step in [
            wow_world::FinalizationStep::CharacterSave,
            wow_world::FinalizationStep::Retirement,
            wow_world::FinalizationStep::Release,
        ] {
            assert_eq!(
                report.outcome(step),
                wow_world::FinalizationOutcome::NotAttempted
            );
        }
        assert_eq!(
            report.disposition,
            wow_world::FinalizationDisposition::RetainAndEscalate
        );
        assert_eq!(session.state(), SessionState::Disconnecting);
        assert_eq!(session.player_guid(), Some(guid));
        assert!(probe.requests.lock().unwrap().is_empty());
        if explicit_logout {
            assert_eq!(
                receiver
                    .try_iter()
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                    .collect::<Vec<_>>(),
                vec![wow_constants::ServerOpcodes::LogoutResponse as u16]
            );
        } else {
            assert_eq!(receiver.try_iter().collect::<Vec<_>>(), vec![vec![0]; 8]);
        }
        let manager = port.manager.lock().unwrap();
        let retained = report.player.expect("retained source incarnation");
        manager
            .with_player_like_cpp(retained, |player| {
                assert_eq!(player.guid(), guid);
                assert_eq!(player.unit().world().map_id(), 0);
                assert_eq!(player.unit().world().position(), Position::ZERO);
                assert_eq!(player.unit().data().health, 500);
                assert_eq!(
                    player.teleport_state_like_cpp().recovery,
                    wow_entities::PlayerTransferRecovery::Terminal
                );
                assert!(player.teleport_state_like_cpp().far_pending);
                let continuation = player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .expect("uncompleted RequestData remains owned");
                assert_eq!(
                    continuation.phase,
                    wow_entities::PlayerResurrectionPhase::AwaitingTeleport
                );
                assert!(matches!(
                    continuation.operation,
                    wow_entities::PlayerResurrectionOperation::RequestData(request)
                        if request.resurrecter == guid
                            && request.map_id == 1
                            && request.position == destination
                            && request.health == 123
                ));
            })
            .expect("the exact source Player remains retained, including when detached");
        assert!(manager.find_map(1, 0).is_none());
        return;
    }
    if explicit_logout {
        if outcome == DisconnectDestination::Rejected {
            assert_eq!(session.state(), SessionState::Disconnecting);
            assert_eq!(session.player_guid(), Some(guid));
            assert!(probe.requests.lock().unwrap().is_empty());
            assert!(
                receiver
                    .try_iter()
                    .all(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                        != wow_constants::ServerOpcodes::LogoutComplete as u16)
            );
            wow_world_application::finalize_session_with_resurrection_catalogs(
                &mut session,
                wow_world::FinalizationMode::Disconnect,
                &generator,
                resurrection_catalogs,
            )
            .await;
            let requests = probe.requests.lock().unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].character.position.map_id, 0);
            assert_eq!(requests[0].character.position.x, 0.0);
            assert_eq!(requests[0].character.health, 500);
            return;
        }
        assert_eq!(session.state(), SessionState::Authed);
        assert!(session.player_guid().is_none());
        let requests = probe.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].character.position.map_id, 1);
        assert_eq!(requests[0].character.position.x, destination.x);
        assert_eq!(requests[0].character.health, 123);
        assert!(
            port.manager
                .lock()
                .unwrap()
                .find_map(1, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
        let packets = receiver.try_iter().collect::<Vec<_>>();
        assert_eq!(
            packets
                .iter()
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                .collect::<Vec<_>>(),
            vec![
                wow_constants::ServerOpcodes::LogoutResponse as u16,
                wow_constants::ServerOpcodes::DeathReleaseLoc as u16,
                wow_constants::ServerOpcodes::MoveSetLandWalk as u16,
                wow_constants::ServerOpcodes::MoveUnroot as u16,
                wow_constants::ServerOpcodes::PowerUpdate as u16,
                wow_constants::ServerOpcodes::CancelCombat as u16,
                wow_constants::ServerOpcodes::CancelCombat as u16,
                wow_constants::ServerOpcodes::LogoutComplete as u16
            ]
        );
        // C++ Player.cpp:24909-24932 calls SetPower(MANA, 0) for this request.
        assert_eq!(
            packets[4],
            wow_packet::packets::combat::PowerUpdate {
                guid,
                powers: vec![(0, wow_constants::PowerType::Mana as u8)],
            }
            .to_bytes()
        );
        return;
    }
    assert_eq!(
        receiver.len(),
        8,
        "disconnect admission/recovery cannot publish into full output"
    );
    let requests = probe.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let manager = port.manager.lock().unwrap();
    if outcome == DisconnectDestination::Rejected {
        assert_eq!(requests[0].character.position.map_id, 0);
        assert_eq!(requests[0].character.position.x, 0.0);
        assert_eq!(requests[0].character.position.y, 0.0);
        assert_eq!(requests[0].character.position.z, 0.0);
        assert_eq!(requests[0].character.health, 500);
        assert!(manager.find_map(1, 0).is_none());
        assert_eq!(session.state(), SessionState::Disconnecting);
        return;
    }
    let (expected_map, expected_position) = if outcome == DisconnectDestination::Requested {
        (1, destination)
    } else {
        (0, home)
    };
    let player = manager
        .find_map(expected_map, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .expect(
            "disconnect must finish native entry before save, not only save proposed coordinates",
        );
    assert_eq!(player.unit().world().position(), expected_position);
    assert!(!player.teleport_state_like_cpp().far_pending);
    assert!(player.teleport_state_like_cpp().far_destination.is_none());
    assert!(player.teleport_state_like_cpp().post_add.is_none());
    assert!(!player.has_deferred_player_save_like_cpp());
    assert_eq!(requests[0].character.health, 123);
    assert_eq!(requests[0].character.position.map_id, expected_map as u16);
    assert_eq!(requests[0].character.position.x, expected_position.x);
    assert_eq!(session.state(), SessionState::Disconnecting);
}

async fn pending_near_disconnect(outcome: PersistenceOutcomeLikeCpp) {
    let (mut session, port, _output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    let current = Position::new(1.0, 2.0, 3.0, 0.5);
    let destination = Position::new(9.0, 8.0, 7.0, 1.5);
    let corpse_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Corpse, 0, 1, 0, 0, 0, 42);
    {
        let mut manager = port.manager.lock().unwrap();
        let map = manager.find_map_mut(0, 0).unwrap().map_mut();
        let player = map.get_typed_player_mut(guid).unwrap();
        player.unit_mut().world_mut().relocate(current);
        player.unit_mut().set_max_health(1000);
        player.unit_mut().set_health(1);
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Dead);
        player.resurrection_state_mut_like_cpp().corpse_location = Some((0, current));

        let mut corpse =
            wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
        corpse.world_mut().object_mut().create(corpse_guid);
        corpse.world_mut().set_map(0, 0).unwrap();
        corpse.world_mut().relocate(current);
        corpse.world_mut().object_mut().add_to_world();
        corpse.set_owner_guid(guid);
        map.add_corpse_to_map_like_cpp(corpse).unwrap();
    }
    session.set_state(SessionState::LoggedIn);
    while receiver.try_recv().is_ok() {}
    session.teleport_to(0, destination).await;
    let transfer_opcodes = receiver
        .try_iter()
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    assert!(
        transfer_opcodes.contains(&(wow_constants::ServerOpcodes::MoveTeleport as u16)),
        "fixture establishes a real near transfer and leaves its ACK pending"
    );
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        assert!(player.teleport_state_like_cpp().near_pending);
        assert_eq!(
            player.teleport_state_like_cpp().near_destination,
            Some((0, destination))
        );
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(wow_entities::PlayerResurrectionContinuation {
            operation: wow_entities::PlayerResurrectionOperation::RequestData(
                wow_entities::PlayerResurrectionRequestLikeCpp {
                    resurrecter: guid,
                    map_id: 0,
                    position: destination,
                    health: 123,
                    mana: 0,
                    aura: 0,
                },
            ),
            phase: wow_entities::PlayerResurrectionPhase::AwaitingTeleport,
        });
    }
    session.kick("controlled disconnect during near resurrection transfer");

    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: outcome.clone(),
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    port.retain_after_save.store(true, Ordering::SeqCst);
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let creature_spawns = wow_world_entities::CreatureSpawnCatalogsLikeCpp::default();
    let catalogs = wow_world_application::ResurrectionOperationCatalogs {
        item_guid_generator: &generator,
        creature_spawns: &creature_spawns,
        graveyards: None,
    };
    let report = wow_world_application::finalize_session_with_resurrection_catalogs(
        &mut session,
        wow_world::FinalizationMode::Disconnect,
        &generator,
        catalogs,
    )
    .await;

    assert_eq!(
        report.outcome(wow_world::FinalizationStep::NativeTransfer),
        wow_world::FinalizationOutcome::Applied,
        "near AwaitingTeleport is cancelled without fabricating an ACK"
    );
    let save_outcome = if matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. }) {
        wow_world::FinalizationOutcome::Applied
    } else {
        wow_world::FinalizationOutcome::Unknown
    };
    assert_eq!(
        report.outcome(wow_world::FinalizationStep::CharacterSave),
        save_outcome
    );
    let requests = probe.requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        1,
        "one ordinary disconnect save is submitted"
    );
    assert_eq!(requests[0].character.health, 1);
    assert_eq!(requests[0].character.position.map_id, 0);
    assert_eq!(requests[0].character.position.x, destination.x);
    assert_eq!(requests[0].character.position.y, destination.y);
    assert_eq!(requests[0].character.position.z, destination.z);
    drop(requests);

    let manager = port.manager.lock().unwrap();
    let map = manager.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.corpse_guid_by_player_like_cpp(guid),
        Some(corpse_guid),
        "near disconnect does not convert or remove the corpse"
    );
    let player = map
        .get_typed_player(guid)
        .expect("save-fenced Player remains");
    assert_eq!(player.unit().data().health, 1);
    assert_eq!(player.unit().death_state(), wow_constants::DeathState::Dead);
    assert_eq!(
        player.resurrection_state_like_cpp().corpse_location,
        Some((0, current))
    );
    assert!(
        player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .is_none()
    );
    assert!(player.teleport_state_like_cpp().near_pending);
    assert_eq!(
        player.teleport_state_like_cpp().near_destination,
        Some((0, destination))
    );

    let opcodes = receiver
        .try_iter()
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    assert!(!opcodes.iter().any(|opcode| {
        [
            wow_constants::ServerOpcodes::DeathReleaseLoc as u16,
            wow_constants::ServerOpcodes::MoveSetLandWalk as u16,
            wow_constants::ServerOpcodes::MoveUnroot as u16,
            wow_constants::ServerOpcodes::MoveTeleport as u16,
        ]
        .contains(opcode)
    }));
    drop(manager);

    if matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. }) {
        assert_eq!(
            wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
                &mut session,
                &generator,
            )
            .await,
            PlayerSaveOutcomeLikeCpp::Quarantined
        );
        assert_eq!(
            probe.requests.lock().unwrap().len(),
            1,
            "unknown save is not replayed"
        );
    }
}

#[tokio::test]
async fn production_disconnect_near_resurrection_cancels_unstarted_and_saves_ghost_destination() {
    pending_near_disconnect(PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}

#[tokio::test]
async fn production_disconnect_near_resurrection_unknown_save_keeps_fence_without_replay() {
    pending_near_disconnect(PersistenceOutcomeLikeCpp::Unknown {
        reason: "controlled uncertain disconnect save".into(),
    })
    .await;
}
#[tokio::test]
async fn production_save_old_completion_does_not_clean_replacement_incarnation() {
    exercise(true, false, PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}
#[tokio::test]
async fn production_save_rollback_unknown_and_cancellation_keep_native_dirty_state() {
    exercise(
        false,
        false,
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rollback".into(),
        },
    )
    .await;
    exercise(
        false,
        false,
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "lost reply".into(),
        },
    )
    .await;
    exercise(false, true, PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}
