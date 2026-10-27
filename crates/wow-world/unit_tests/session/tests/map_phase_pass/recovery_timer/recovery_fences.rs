use super::*;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{Notify, Semaphore};
use wow_world_lifecycle::{
    PlayerSaveDiagnosticsLikeCpp, PlayerSaveHostLikeCpp, PlayerSaveParticipantsLikeCpp,
};

#[derive(Default)]
struct SelfResCorpsePersistence {
    deletes: std::sync::Mutex<Vec<u64>>,
    delete_gate: std::sync::Mutex<Option<(Arc<Notify>, Arc<Semaphore>)>>,
}

impl SelfResCorpsePersistence {
    fn install_delete_gate(&self) -> (Arc<Notify>, Arc<Semaphore>) {
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Semaphore::new(0));
        *self.delete_gate.lock().unwrap() =
            Some((Arc::clone(&entered), Arc::clone(&release)));
        (entered, release)
    }
}

impl wow_persistence::MapCorpsePersistencePortLikeCpp for SelfResCorpsePersistence {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::MapCorpseLoadRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::MapCorpseLoadOutcomeLikeCpp,
    > {
        Box::pin(async {
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Failed {
                reason: "self-resurrection test does not load corpses".into(),
            }
        })
    }

    fn save_corpse_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::CorpseSaveRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp,
    > {
        Box::pin(async { wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed })
    }

    fn delete_corpse_like_cpp<'a>(
        &'a self,
        owner_guid: u64,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp,
    > {
        let gate = self.delete_gate.lock().unwrap().clone();
        Box::pin(async move {
            self.deletes.lock().unwrap().push(owner_guid);
            if let Some((entered, release)) = gate {
                entered.notify_one();
                release
                    .acquire()
                    .await
                    .expect("the controlled corpse delete gate remains open")
                    .forget();
            }
            wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed
        })
    }
}

struct ObjectiveDrainGate<'a> {
    session: &'a mut WorldSession,
    entered: Arc<Notify>,
    release: Arc<Semaphore>,
}

impl PlayerSaveHostLikeCpp for ObjectiveDrainGate<'_> {
    fn player_save_participants_like_cpp(&mut self) -> PlayerSaveParticipantsLikeCpp<'_> {
        PlayerSaveHostLikeCpp::player_save_participants_like_cpp(self.session)
    }

    fn apply_pending_durable_item_loot_completions_for_player_save_like_cpp<'a>(
        &'a mut self,
        generator: &'a wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        PlayerSaveHostLikeCpp::apply_pending_durable_item_loot_completions_for_player_save_like_cpp(
            self.session,
            generator,
        )
    }

    fn reconcile_durable_loot_money_before_player_save_like_cpp(
        &mut self,
    ) -> impl Future<Output = bool> + Send {
        PlayerSaveHostLikeCpp::reconcile_durable_loot_money_before_player_save_like_cpp(
            self.session,
        )
    }

    fn quarantine_player_save_like_cpp(&mut self, reason: &str) {
        PlayerSaveHostLikeCpp::quarantine_player_save_like_cpp(self.session, reason)
    }

    fn drain_represented_quest_objective_progress_for_player_save_like_cpp<'a>(
        &'a mut self,
        generator: &'a wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        let session = &mut *self.session;
        let entered = Arc::clone(&self.entered);
        let release = Arc::clone(&self.release);
        async move {
            entered.notify_one();
            release
                .acquire()
                .await
                .expect("the controlled objective-drain gate remains open")
                .forget();
            PlayerSaveHostLikeCpp::drain_represented_quest_objective_progress_for_player_save_like_cpp(
                session,
                generator,
            )
            .await;
        }
    }

    fn sync_player_save_registry_state_like_cpp(&mut self) {
        PlayerSaveHostLikeCpp::sync_player_save_registry_state_like_cpp(self.session)
    }

    fn player_save_diagnostics_like_cpp(&self) -> PlayerSaveDiagnosticsLikeCpp {
        PlayerSaveHostLikeCpp::player_save_diagnostics_like_cpp(self.session)
    }
}

#[tokio::test]
async fn recovery_fences_real_save_ack_survives_objective_drain_cancellation_and_resumes_once() {
    let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    let port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = session.core.canonical_map_manager.as_ref().unwrap().clone();
    manager
        .lock()
        .unwrap()
        .with_player_mut_like_cpp(handle, |player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
                    restore_percent: 0.5,
                },
                wow_entities::PlayerResurrectionPhase::ReadyToSavePlayer,
            ));
        })
        .unwrap();

    let entered = Arc::new(Notify::new());
    let release = Arc::new(Semaphore::new(0));
    let entered_wait = Arc::clone(&entered);
    let mut host = ObjectiveDrainGate {
        session: &mut session,
        entered,
        release,
    };
    let generators = host.session.id_generators_for_test_like_cpp();
    let mut save = Box::pin(
        wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
            &mut host,
            generators.item.as_ref(),
        ),
    );
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), entered_wait.notified()) => {
            assert!(result.is_ok(), "the real save did not reach its post-ACK objective drain");
        }
        outcome = &mut save => panic!("the real save returned before its gated post-ACK drain: {outcome:?}"),
    }

    let phase_during_drain = manager
        .lock()
        .unwrap()
        .with_player_like_cpp(handle, |player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase)
        })
        .unwrap();
    assert_eq!(
        phase_during_drain,
        Some(wow_entities::PlayerResurrectionPhase::PlayerSaveApplied),
        "only the real incarnation-bound committed receipt earns the applied phase"
    );
    assert_eq!(port.character_save_count(), 1);
    assert_eq!(port.character_save_requests().len(), 1);

    drop(save);
    drop(host);

    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let reply = run_recovery_request(&mut session, admission, permit).await;
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));
    assert_eq!(port.character_save_count(), 1, "resume must not save again");
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport),
        Some(None),
        "the admitted recovery pass finishes and clears the same continuation"
    );
}

#[tokio::test]
async fn recovery_fences_near_transfer_does_not_defer_a_confirmed_recovery_save() {
    let (mut session, _admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    let port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    session.update_player_teleport_state_like_cpp(|state| state.near_pending = true);
    manager_set_recovery_save_ready(&session, handle);

    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        crate::session::PlayerSaveOutcomeLikeCpp::Applied
    );
    assert_eq!(port.character_save_count(), 1);
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .map(|continuation| continuation.phase)),
        Some(Some(
            wow_entities::PlayerResurrectionPhase::PlayerSaveApplied
        ))
    );
}

#[tokio::test]
async fn native_self_resurrection_converts_real_corpse_then_full_saves_player() {
    let (mut session, _admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(
        &mut session,
        true,
    ));
    let player_guid = session
        .with_owned_player_like_cpp(|player| player.guid())
        .expect("Player GUID");
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(0);
            player.unit_mut().set_power_index(wow_constants::PowerType::Mana, Some(0));
            player.unit_mut().set_power_index(wow_constants::PowerType::Rage, Some(1));
            player.unit_mut().set_power_index(wow_constants::PowerType::Energy, Some(3));
            player.unit_mut().set_power_index(wow_constants::PowerType::Focus, Some(4));
            player.unit_mut().set_max_power(wow_constants::PowerType::Mana, 200);
            player.unit_mut().set_power(wow_constants::PowerType::Mana, 25);
            player.unit_mut().set_max_power(wow_constants::PowerType::Rage, 100);
            player.unit_mut().set_power(wow_constants::PowerType::Rage, 50);
            player.unit_mut().set_max_power(wow_constants::PowerType::Energy, 100);
            player.unit_mut().set_power(wow_constants::PowerType::Energy, 30);
            player.unit_mut().set_max_power(wow_constants::PowerType::Focus, 100);
            player.unit_mut().set_power(wow_constants::PowerType::Focus, 40);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
            player.add_self_res_spell(21_169);
        })
        .expect("canonical Player remains the recovery owner");

    let corpse_guid = ObjectGuid::create_world_object(
        HighGuid::Corpse,
        0,
        1,
        571,
        0,
        0,
        88_331,
    );
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().set_map(571, 0).expect("corpse map");
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_owner_guid(player_guid);
    session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).expect("corpse record"),
        )
        .expect("corpse inserted on the Player's active map");

    let corpse_port = std::sync::Arc::new(SelfResCorpsePersistence::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
        rows: 1,
    });
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    wow_world_entities::self_resurrect_player_like_cpp(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: None,
        },
        35,
        77,
    )
    .await
    .expect("native SelfRes completes after the committed corpse delete and Player save");

    assert_eq!(
        *corpse_port.deletes.lock().unwrap(),
        vec![player_guid.counter() as u64]
    );
    assert_eq!(save_port.character_save_count(), 1);
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 35);
    let save_requests = save_port.character_save_requests();
    assert_eq!(save_requests.len(), 1);
    assert_eq!(
        save_requests[0].character.health,
        35,
        "the full Player save captures the restored native SelfRes health"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport),
        Some(None),
        "full-save ACK completes and clears this exact SelfRes continuation"
    );
}

#[tokio::test]
async fn native_self_resurrection_cancelled_during_real_corpse_delete_stays_nonreplayable() {
    let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(
        &mut session,
        true,
    ));
    let player_guid = session
        .with_owned_player_like_cpp(|player| player.guid())
        .expect("Player GUID");
    let player_handle = session.core.player_handle_like_cpp.expect("Player handle");
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(0);
            player.unit_mut().set_power_index(wow_constants::PowerType::Mana, Some(0));
            player.unit_mut().set_power_index(wow_constants::PowerType::Rage, Some(1));
            player.unit_mut().set_power_index(wow_constants::PowerType::Energy, Some(3));
            player.unit_mut().set_power_index(wow_constants::PowerType::Focus, Some(4));
            player.unit_mut().set_max_power(wow_constants::PowerType::Mana, 200);
            player.unit_mut().set_power(wow_constants::PowerType::Mana, 25);
            player.unit_mut().set_max_power(wow_constants::PowerType::Rage, 100);
            player.unit_mut().set_power(wow_constants::PowerType::Rage, 50);
            player.unit_mut().set_max_power(wow_constants::PowerType::Energy, 100);
            player.unit_mut().set_power(wow_constants::PowerType::Energy, 30);
            player.unit_mut().set_max_power(wow_constants::PowerType::Focus, 100);
            player.unit_mut().set_power(wow_constants::PowerType::Focus, 40);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
            player.add_self_res_spell(21_169);
        })
        .expect("canonical Player remains the recovery owner");

    let corpse_guid = ObjectGuid::create_world_object(
        HighGuid::Corpse,
        0,
        1,
        571,
        0,
        0,
        88_332,
    );
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().set_map(571, 0).expect("corpse map");
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_owner_guid(player_guid);
    session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).expect("corpse record"),
        )
        .expect("corpse inserted on the Player's active map");

    let corpse_port = Arc::new(SelfResCorpsePersistence::default());
    let (delete_entered, _delete_release) = corpse_port.install_delete_gate();
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
        rows: 1,
    });
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    let mut resurrection = Box::pin(wow_world_entities::self_resurrect_player_like_cpp(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: None,
        },
        35,
        77,
    ));
    tokio::select! {
        result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            delete_entered.notified(),
        ) => {
            assert!(result.is_ok(), "native SelfRes did not reach its controlled corpse-delete await");
        }
        outcome = resurrection.as_mut() => {
            panic!("native SelfRes returned before the controlled real corpse delete: {outcome:?}");
        }
    }
    drop(resurrection);

    let continuation = session
        .with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().delayed_after_teleport
        })
        .flatten()
        .expect("the cancelled delete retains the exact Player continuation");
    assert_eq!(
        continuation.operation,
        wow_entities::PlayerResurrectionOperation::SelfRes {
            health: 35,
            mana: 77,
        }
    );
    assert_eq!(
        continuation.phase,
        wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight
    );
    let player_after_cancel = session
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().death_state(),
                player.unit().data().health,
            )
        })
        .expect("cancelled native SelfRes keeps its canonical Player");
    assert_eq!(
        player_after_cancel,
        (wow_constants::DeathState::Alive, 35),
        "restored values precede the pending corpse delete"
    );
    assert_eq!(
        *corpse_port.deletes.lock().unwrap(),
        vec![player_guid.counter() as u64]
    );
    assert_eq!(save_port.character_save_count(), 0);

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Unavailable));
    assert_eq!(session.core.player_handle_like_cpp, Some(player_handle));
    assert_eq!(
        session
            .with_owned_player_like_cpp(|player| {
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .map(|continuation| {
                        (continuation.operation, continuation.phase)
                    })
            })
            .flatten(),
        Some((
            wow_entities::PlayerResurrectionOperation::SelfRes {
                health: 35,
                mana: 77,
            },
            wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
        ))
    );
    assert_eq!(
        *corpse_port.deletes.lock().unwrap(),
        vec![player_guid.counter() as u64],
        "recovery cannot replay an unconfirmed corpse delete"
    );
    assert_eq!(
        save_port.character_save_count(),
        0,
        "an unconfirmed corpse delete cannot trigger the Player save"
    );
}

#[tokio::test]
async fn recovery_fences_cancelled_pre_ack_save_stays_held_without_snapshot_retry() {
    let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    let port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    port.hold_character_save();
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = session.core.canonical_map_manager.as_ref().unwrap().clone();
    manager_set_recovery_save_ready(&session, handle);

    let mut save = Box::pin(session.save_current_player_to_db_like_cpp());
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), port.wait_for_character_save()) => {
            assert!(result.is_ok(), "the real character save did not reach its controlled port");
        }
        outcome = &mut save => panic!("the held save returned before the port was released: {outcome:?}"),
    }
    assert_eq!(port.character_save_count(), 1);
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
    drop(save);

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Unavailable));
    assert_eq!(
        port.character_save_count(),
        1,
        "cancellation before ACK cannot retry"
    );
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
}

fn manager_set_recovery_save_ready(session: &WorldSession, handle: wow_map::PlayerHandle) {
    session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .with_player_mut_like_cpp(handle, |player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
                    restore_percent: 0.5,
                },
                wow_entities::PlayerResurrectionPhase::ReadyToSavePlayer,
            ));
        })
        .unwrap();
}
