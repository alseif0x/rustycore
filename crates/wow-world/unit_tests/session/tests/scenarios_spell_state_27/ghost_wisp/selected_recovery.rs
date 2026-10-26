use super::*;
use crate::session::mailbox::{
    PlayerRecoveryAdmission, PlayerRecoveryStatus, RunPlayerRecoveryRequest,
    RunWorldPhasePassLikeCppRequest, SessionPhasePassOutcomeLikeCpp,
    SessionPhasePermitLikeCpp, SessionPhasePermitStateLikeCpp, SessionPhaseRequestLikeCpp,
};
use crate::session::PlayerRegistry;
use super::observers::aura_packets;
use super::selected_lifetime::{install_race, selected_slot};

fn register_recovery_session(session: &mut crate::session::WorldSession) {
    let registry = Arc::new(PlayerRegistry::new());
    session.set_player_registry(Arc::clone(&registry));
    session.register_in_player_registry();
    assert!(registry
        .session_phase_address_like_cpp(session.player_guid().unwrap())
        .is_some());
}

fn start_recovery_admission(
    session: &crate::session::WorldSession,
) -> PlayerRecoveryAdmission {
    let (map_tick_epoch, diff_ms) = {
        let manager = session.core.canonical_map_manager.as_ref().unwrap();
        let mut manager = manager.lock().unwrap();
        let plan = manager
            .begin_tick_like_cpp(1_000)
            .into_started()
            .expect("canonical map tick starts");
        (plan.epoch_like_cpp(), plan.effective_diff_ms())
    };
    let map_admission = session
        .current_map_phase_admission_for_test_like_cpp(81, 17, diff_ms)
        .expect("current Player receives a recovery admission");
    PlayerRecoveryAdmission {
        coordinator_id: map_admission.coordinator_id,
        tick_epoch: map_admission.tick_epoch,
        map_tick_epoch,
        registration: map_admission.registration,
        handle: map_admission.handle,
        map_key: map_admission.map_key,
        map_incarnation: map_admission.map_incarnation,
        residence_revision: map_admission.residence_revision,
        diff_ms,
        unix_time_ns: 50_000_000_000,
    }
}

async fn run_phase(
    session: &mut crate::session::WorldSession,
    request: SessionPhaseRequestLikeCpp,
) {
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    assert!(session
        .run_requested_session_phase_like_cpp(request, &catalogs)
        .await
        .is_none());
}

fn remaining(session: &crate::session::WorldSession, slot: u8) -> i32 {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(slot)
                .expect("Ghost remains applied")
                .duration_remaining
        })
        .expect("canonical Player remains current")
}

#[tokio::test]
async fn selected_recovery_advances_once_after_the_coordinated_world_pass() {
    let mut fixture = fixture();
    fixture.session.core.state = crate::session::SessionState::LoggedIn;
    install_race(&mut fixture.session, 0);
    cast_selected(&mut fixture, GHOST, None).await;
    let packets = outgoing(&fixture);
    assert!(!packets.is_empty(), "the fresh Ghost application is published");
    let slot = selected_slot(&fixture, GHOST);
    let stale = Instant::now() - Duration::from_secs(3_600);
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let aura = auras.runtime_application_mut_like_cpp(slot).unwrap();
            aura.duration_total = 60_000;
            aura.duration_remaining = 5_000;
            aura.applied_at = stale;
            auras.visible_auras_to_update.insert(slot);
        })
        .expect("canonical Player remains current");
    register_recovery_session(&mut fixture.session);
    let admission = start_recovery_admission(&fixture.session);
    fixture.session.mark_map_phase_coordinated_like_cpp();

    let (response_tx, response_rx) = flume::bounded(1);
    let world_permit = SessionPhasePermitLikeCpp::new_like_cpp();
    run_phase(
        &mut fixture.session,
        SessionPhaseRequestLikeCpp::World(RunWorldPhasePassLikeCppRequest {
            coordinator_id: admission.coordinator_id,
            tick_epoch: admission.tick_epoch,
            diff_ms: admission.diff_ms,
            permit: world_permit,
            response_tx,
        }),
    )
    .await;
    assert_eq!(
        response_rx.try_recv().unwrap().outcome,
        SessionPhasePassOutcomeLikeCpp::Ran
    );
    assert_eq!(remaining(&fixture.session, slot), 5_000);
    assert!(fixture
        .canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(admission.handle, |player| {
            player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .contains(&slot)
        })
        .unwrap());
    assert!(aura_packets(outgoing(&fixture)).is_empty());

    let (response_tx, response_rx) = flume::bounded(1);
    let recovery_permit = SessionPhasePermitLikeCpp::new_like_cpp();
    run_phase(
        &mut fixture.session,
        SessionPhaseRequestLikeCpp::PlayerRecovery(RunPlayerRecoveryRequest {
            admission,
            permit: Arc::clone(&recovery_permit),
            response_tx,
        }),
    )
    .await;
    let reply = response_rx.try_recv().expect("recovery reports its result");
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));
    assert_eq!(reply.map_tick_epoch, admission.map_tick_epoch);
    assert_eq!(
        recovery_permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
    assert_eq!(remaining(&fixture.session, slot), 5_000 - admission.diff_ms as i32);
    fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            assert_eq!(auras.runtime_application_like_cpp(slot).unwrap().applied_at, stale);
            assert!(!auras.visible_auras_to_update.contains(&slot));
        })
        .unwrap();
    assert_eq!(
        aura_packets(outgoing(&fixture)).len(),
        1,
        "Recovery publishes one selected delta directly to this WorldSession"
    );
}

#[tokio::test]
async fn positive_countdown_without_pending_aura_work_emits_no_frame() {
    let mut fixture = fixture();
    fixture.session.core.state = crate::session::SessionState::LoggedIn;
    install_race(&mut fixture.session, 0);
    cast_selected(&mut fixture, GHOST, None).await;
    outgoing(&fixture);
    let slot = selected_slot(&fixture, GHOST);
    let stale = Instant::now() - Duration::from_secs(3_600);
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let aura = auras.runtime_application_mut_like_cpp(slot).unwrap();
            aura.duration_total = 60_000;
            aura.duration_remaining = 5_000;
            aura.applied_at = stale;
            auras.visible_auras_to_update.remove(&slot);
        })
        .expect("canonical Player remains current");
    register_recovery_session(&mut fixture.session);
    let admission = start_recovery_admission(&fixture.session);
    fixture.session.mark_map_phase_coordinated_like_cpp();

    let (response_tx, response_rx) = flume::bounded(1);
    run_phase(
        &mut fixture.session,
        SessionPhaseRequestLikeCpp::World(RunWorldPhasePassLikeCppRequest {
            coordinator_id: admission.coordinator_id,
            tick_epoch: admission.tick_epoch,
            diff_ms: admission.diff_ms,
            permit: SessionPhasePermitLikeCpp::new_like_cpp(),
            response_tx,
        }),
    )
    .await;
    assert_eq!(
        response_rx.try_recv().unwrap().outcome,
        SessionPhasePassOutcomeLikeCpp::Ran
    );
    assert!(aura_packets(outgoing(&fixture)).is_empty());

    let (response_tx, response_rx) = flume::bounded(1);
    run_phase(
        &mut fixture.session,
        SessionPhaseRequestLikeCpp::PlayerRecovery(RunPlayerRecoveryRequest {
            admission,
            permit: SessionPhasePermitLikeCpp::new_like_cpp(),
            response_tx,
        }),
    )
    .await;
    assert_eq!(
        response_rx.try_recv().unwrap().status,
        Some(PlayerRecoveryStatus::Completed)
    );
    assert_eq!(
        remaining(&fixture.session, slot),
        5_000 - admission.diff_ms as i32
    );
    assert!(fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| {
            !player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .contains(&slot)
        })
        .unwrap());
    assert!(
        aura_packets(outgoing(&fixture)).is_empty(),
        "a positive Unit countdown does not create a client delta"
    );
}
