use super::*;
use crate::session::mailbox::{
    PlayerRecoveryAdmission, PlayerRecoveryReply, PlayerRecoveryStatus, RunPlayerRecoveryRequest,
    RunWorldPhasePassLikeCppRequest, SessionPhaseRequestLikeCpp,
};
use crate::session::tests::lifecycle_persistence::RecordingPortLikeCpp;
use wow_persistence::PersistenceOutcomeLikeCpp;

fn logged_in_recovery_session() -> (
    WorldSession,
    PlayerRecoveryAdmission,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, packet_tx, send_rx) = make_session();
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    register_session_for_phases(&mut session);
    session.core.state = SessionState::LoggedIn;
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .resurrection_state_mut_like_cpp()
                .death_timer_remaining_ms = 300_000;
        })
        .expect("canonical Player is installed");

    let (map_tick_epoch, diff_ms) = {
        let manager = session.core.canonical_map_manager.as_ref().unwrap();
        let mut manager = manager.lock().unwrap();
        let plan = manager
            .begin_tick_like_cpp(1_000)
            .into_started()
            .expect("the test starts an admitted map tick");
        (plan.epoch_like_cpp(), plan.effective_diff_ms())
    };
    let map_admission = session
        .current_map_phase_admission_for_test_like_cpp(81, 17, diff_ms)
        .expect("the active Player has a map admission");
    let admission = PlayerRecoveryAdmission {
        coordinator_id: map_admission.coordinator_id,
        tick_epoch: map_admission.tick_epoch,
        map_tick_epoch,
        registration: map_admission.registration,
        handle: map_admission.handle,
        map_key: map_admission.map_key,
        map_incarnation: map_admission.map_incarnation,
        residence_revision: map_admission.residence_revision,
        diff_ms,
    };
    (session, admission, packet_tx, send_rx)
}

fn death_timer(session: &WorldSession) -> u32 {
    session
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .death_timer_remaining_ms
        })
        .expect("canonical Player remains installed")
}

async fn run_world_request(session: &mut WorldSession, diff_ms: u32) {
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .session_phase_sender_like_cpp()
        .try_send(SessionPhaseRequestLikeCpp::World(
            RunWorldPhasePassLikeCppRequest {
                coordinator_id: 81,
                tick_epoch: 17,
                diff_ms,
                permit,
                response_tx,
            },
        ))
        .unwrap();
    let request = session
        .session_phase_receiver_like_cpp()
        .try_recv()
        .unwrap();
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let pending_finalization = session
        .run_requested_session_phase_like_cpp(request, &catalogs)
        .await;
    assert!(
        pending_finalization.is_none(),
        "the fixture keeps both transport peers alive, so this pass must not be finalized as a disconnect"
    );
    assert_eq!(
        response_rx.try_recv().unwrap().outcome,
        SessionPhasePassOutcomeLikeCpp::Ran
    );
}

async fn run_recovery_request(
    session: &mut WorldSession,
    admission: PlayerRecoveryAdmission,
    permit: std::sync::Arc<SessionPhasePermitLikeCpp>,
) -> PlayerRecoveryReply {
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .session_phase_sender_like_cpp()
        .try_send(SessionPhaseRequestLikeCpp::PlayerRecovery(
            RunPlayerRecoveryRequest {
                admission,
                permit,
                response_tx,
            },
        ))
        .unwrap();
    let request = session
        .session_phase_receiver_like_cpp()
        .try_recv()
        .unwrap();
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let pending_finalization = session
        .run_requested_session_phase_like_cpp(request, &catalogs)
        .await;
    assert!(pending_finalization.is_none());
    response_rx.try_recv().unwrap()
}

#[tokio::test]
async fn coordinated_world_tick_defers_player_timers_to_one_recovery_rail_pass() {
    let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    run_world_request(&mut session, admission.diff_ms).await;
    assert_eq!(death_timer(&session), 300_000);

    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let reply = run_recovery_request(&mut session, admission, std::sync::Arc::clone(&permit)).await;
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
    assert_eq!(reply.coordinator_id, admission.coordinator_id);
    assert_eq!(reply.tick_epoch, admission.tick_epoch);
    assert_eq!(reply.map_tick_epoch, admission.map_tick_epoch);
    assert_eq!(reply.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));
    assert_eq!(death_timer(&session), 300_000 - admission.diff_ms);

    let replay = SessionPhasePermitLikeCpp::new_like_cpp();
    let reply = run_recovery_request(&mut session, admission, replay).await;
    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(reply.status, None);
    assert_eq!(death_timer(&session), 300_000 - admission.diff_ms);
}

#[tokio::test]
async fn stale_or_revoked_recovery_request_has_no_timer_effect() {
    let (mut stale_session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    let stale = PlayerRecoveryAdmission {
        map_tick_epoch: admission.map_tick_epoch + 1,
        ..admission
    };
    let stale_reply = run_recovery_request(
        &mut stale_session,
        stale,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(
        stale_reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(death_timer(&stale_session), 300_000);

    let (mut revoked_session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    assert_eq!(
        permit.revoke_before_start_like_cpp(),
        crate::session::mailbox::SessionPhaseRevokeLikeCpp::Revoked
    );
    let reply = run_recovery_request(&mut revoked_session, admission, permit).await;
    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart
    );
    assert_eq!(reply.status, None);
    assert_eq!(death_timer(&revoked_session), 300_000);
}

#[tokio::test]
async fn shutdown_refuses_recovery_before_claim_without_timer_effect() {
    let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session.refuse_requested_session_phase_like_cpp(SessionPhaseRequestLikeCpp::PlayerRecovery(
        RunPlayerRecoveryRequest {
            admission,
            permit: std::sync::Arc::clone(&permit),
            response_tx,
        },
    ));
    let reply = response_rx.try_recv().unwrap();
    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RefusedBeforeStart
    );
    assert_eq!(death_timer(&session), 300_000);
}

fn install_recovery_save_snapshot_inputs(session: &mut WorldSession) {
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_position_like_cpp(position);
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([
            wow_data::character_progression::ChrClassesEntry {
                id: 5,
                starting_level: 1,
                ..Default::default()
            },
        ]),
    ));
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 20,
            spirit: 30,
            base_mana: 1000,
        },
    )])));
    assert!(session.complete_represented_trait_config_authority_load_like_cpp([], true));
}

#[tokio::test]
async fn recovery_waits_for_periodic_save_before_later_player_work() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "controlled uncertain save".to_owned(),
        },
    ] {
        let unknown = matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. });
        let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
        install_recovery_save_snapshot_inputs(&mut session);
        session.set_player_save_interval_ms_like_cpp(100);
        let port = RecordingPortLikeCpp::new(outcome);
        port.hold_character_save();
        session.set_player_lifecycle_port_like_cpp(port.clone());

        let manager = session
            .core
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager is installed")
            .clone();
        let handle = session
            .core
            .player_handle_like_cpp
            .expect("canonical Player handle is installed");
        let permit = SessionPhasePermitLikeCpp::new_like_cpp();
        let (response_tx, response_rx) = flume::bounded(1);
        session
            .session_phase_sender_like_cpp()
            .try_send(SessionPhaseRequestLikeCpp::PlayerRecovery(
                RunPlayerRecoveryRequest {
                    admission,
                    permit: Arc::clone(&permit),
                    response_tx,
                },
            ))
            .unwrap();
        let request = session
            .session_phase_receiver_like_cpp()
            .try_recv()
            .unwrap();
        let catalogs = session.session_handler_catalogs_for_test_like_cpp();
        let mut recovery =
            Box::pin(session.run_requested_session_phase_like_cpp(request, &catalogs));
        tokio::select! {
            result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                port.wait_for_character_save(),
            ) => assert!(result.is_ok(), "the canonical SaveTimer did not reach the controlled lifecycle port"),
            _ = &mut recovery => panic!("recovery completed before its controlled save was released"),
        }

        assert_eq!(port.character_save_count(), 1);
        assert_eq!(
            permit.state_like_cpp(),
            SessionPhasePermitStateLikeCpp::Running
        );
        let timer_during_save = manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .resurrection_state_like_cpp()
                    .death_timer_remaining_ms
            })
            .unwrap();
        assert_eq!(timer_during_save, 300_000);

        if unknown {
            // The save passed the transfer preflight before entering the port.
            // Add a canonical delayed-teleport intent during that await to prove
            // the unknown-save fence prevents consuming it afterward.
            let destination = Position::new(8.0, 9.0, 10.0, 0.5);
            manager
                .lock()
                .unwrap()
                .with_player_mut_like_cpp(handle, |player| {
                    let state = player.teleport_state_mut_like_cpp();
                    state.far_destination = Some((571, destination));
                    state.has_delayed = true;
                    state.delayed = Some((571, destination, 0));
                })
                .expect("the save await releases canonical Player access");
        }

        port.release_character_save();
        let pending_finalization = recovery.as_mut().await;
        drop(recovery);
        assert!(pending_finalization.is_none());
        let reply = response_rx.try_recv().unwrap();

        assert_eq!(reply.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
        if unknown {
            assert_eq!(reply.status, Some(PlayerRecoveryStatus::Unavailable));
            assert_eq!(
                permit.state_like_cpp(),
                SessionPhasePermitStateLikeCpp::CompletedUnavailable
            );
            assert_eq!(session.state(), SessionState::Disconnecting);
            assert_eq!(death_timer(&session), 300_000);
            assert!(
                session.represented_has_delayed_teleport_like_cpp(),
                "the unavailable save must stop before consuming delayed teleport"
            );
        } else {
            assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));
            assert_eq!(
                permit.state_like_cpp(),
                SessionPhasePermitStateLikeCpp::Completed
            );
            assert_eq!(session.state(), SessionState::LoggedIn);
            assert_eq!(death_timer(&session), 300_000 - admission.diff_ms);
        }
    }
}
