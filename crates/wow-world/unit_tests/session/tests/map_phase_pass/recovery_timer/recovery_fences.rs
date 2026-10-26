use super::*;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{Notify, Semaphore};
use wow_world_lifecycle::{
    PlayerSaveDiagnosticsLikeCpp, PlayerSaveHostLikeCpp, PlayerSaveParticipantsLikeCpp,
};

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
