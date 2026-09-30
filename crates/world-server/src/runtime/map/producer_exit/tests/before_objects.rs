//! Foreign/poison transport boundaries; not reachable normal-producer fault proof.
use super::*;
use wow_world::session::mailbox::{SessionPhaseClaimLikeCpp, SessionPhasePermitLikeCpp,
    SessionPhasePermitStateLikeCpp};

mod fixtures;
use fixtures::*;

#[tokio::test]
async fn unresolved_foreign_abandon_returns_original_plan_ticket_and_claimed_permit_immediately() {
    let (mut owner, plan) = admitted(200);
    let expected = identity(&plan);
    let (foreign, foreign_plan) = admitted(300);
    let foreign = Mutex::new(foreign);
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry.try_admit_tick(origin, plan.plan.epoch_like_cpp(), false).unwrap();
    admission.enter_phase(TickPhase::Map);
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    assert_eq!(permit.claim_like_cpp(), SessionPhaseClaimLikeCpp::Claimed);
    let permits = vec![Arc::clone(&permit)];
    let permit_buffer = permits.as_ptr();
    let exit = rejected_return(&foreign, plan, admission, permits, false);
    assert_original(&exit, &expected);
    assert!(foreign.try_lock().is_ok());
    match &exit {
        CanonicalMapProducerExit::RetainedBeforeObjects {
            cause: BeforeObjectsExitCause::UnresolvedMapPassRejected(status), plan, unresolved_permits, ..
        } => {
            assert_eq!(*status, wow_map::MapTickResumeLikeCpp::Rejected {
                state: wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(foreign_plan.plan.epoch_like_cpp()),
                plan_epoch: expected.epoch });
            assert_eq!(unresolved_permits.as_ptr(), permit_buffer);
            assert_eq!(unresolved_permits.len(), 1);
            assert!(Arc::ptr_eq(&unresolved_permits[0], &permit));
            assert_eq!(permit.state_like_cpp(), SessionPhasePermitStateLikeCpp::Running);
            assert!(owner.can_resume_tick(&plan.plan));
        }
        _ => panic!("the actual nonquiescent branch cause must remain"),
    }
    assert!(foreign.lock().unwrap().can_resume_tick(&foreign_plan.plan));
    assert!(registry.try_admit_tick(origin, expected.epoch + 1, false).is_none());
    assert!(owner.begin_tick_like_cpp(999).is_busy());
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.is_err());
    assert!(permit.complete_like_cpp());
    // A separately completed pass cannot settle this rejected map plan/ticket.
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.is_err());
    assert!(!registry.session_drain_authorized());
    assert_original(&exit, &expected);
}

#[tokio::test]
async fn shutdown_foreign_abandon_keeps_empty_permits_and_same_unaccounted_map_ticket() {
    let (owner, plan) = admitted(200);
    let expected = identity(&plan);
    let (foreign, foreign_plan) = admitted(300);
    let foreign = Mutex::new(foreign);
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry.try_admit_tick(origin, plan.plan.epoch_like_cpp(), false).unwrap();
    admission.enter_phase(TickPhase::Map);
    registry.begin_shutdown_like_cpp();
    assert!(registry.is_shutting_down_like_cpp());
    let permits = Vec::new();
    let permit_buffer = permits.as_ptr();
    let exit = rejected_return(&foreign, plan, admission, permits, true);
    assert_original(&exit, &expected);
    assert!(foreign.try_lock().is_ok());
    match &exit {
        CanonicalMapProducerExit::RetainedBeforeObjects {
            cause: BeforeObjectsExitCause::ShutdownAbandonRejected(status), plan, unresolved_permits, ..
        } => {
            assert_eq!(*status, wow_map::MapTickResumeLikeCpp::Rejected {
                state: wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(foreign_plan.plan.epoch_like_cpp()),
                plan_epoch: expected.epoch });
            assert!(unresolved_permits.is_empty());
            assert_eq!(unresolved_permits.as_ptr(), permit_buffer);
            assert!(owner.can_resume_tick(&plan.plan));
        }
        _ => panic!("shutdown rejection must retain its actual cause"),
    }
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_terminal_settlement(request, Duration::from_millis(1)).await.is_err());
    assert!(!registry.final_respawn_tick_authorized());
    drop(exit);
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_terminal_settlement(request, Duration::from_millis(1)).await.is_err());
}

#[tokio::test]
async fn unresolved_real_poison_returns_whole_plan_and_interrupted_permit_without_manager_recovery() {
    let (owner, plan) = admitted(200);
    let expected = identity(&plan);
    assert!(owner.can_resume_tick(&plan.plan));
    let manager = Mutex::new(owner);
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry.try_admit_tick(origin, plan.plan.epoch_like_cpp(), false).unwrap();
    admission.enter_phase(TickPhase::Map);
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    assert_eq!(permit.claim_like_cpp(), SessionPhaseClaimLikeCpp::Claimed);
    assert!(permit.interrupt_after_start_like_cpp());
    let permits = vec![Arc::clone(&permit)];
    let permit_buffer = permits.as_ptr();
    let exit = poisoned_return(&manager, plan, admission, permits, false);
    assert_original(&exit, &expected);
    assert!(manager.is_poisoned());
    assert!(matches!(manager.try_lock(), Err(std::sync::TryLockError::Poisoned(_))));
    match &exit {
        CanonicalMapProducerExit::RetainedBeforeObjects {
            cause: BeforeObjectsExitCause::UnresolvedMapPassManagerPoisoned, unresolved_permits, ..
        } => {
            assert_eq!(unresolved_permits.as_ptr(), permit_buffer);
            assert!(Arc::ptr_eq(&unresolved_permits[0], &permit));
            assert_eq!(permit.state_like_cpp(), SessionPhasePermitStateLikeCpp::InterruptedAfterStart);
        }
        _ => panic!("poison must not fabricate an abandon status"),
    }
    assert!(registry.try_admit_tick(origin, expected.epoch + 1, false).is_none());
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.is_err());
    assert!(!registry.session_drain_authorized());
}

#[tokio::test]
async fn shutdown_real_poison_retains_empty_vector_and_ticket_without_fake_status() {
    let (owner, plan) = admitted(200);
    let expected = identity(&plan);
    assert!(owner.can_resume_tick(&plan.plan));
    let manager = Mutex::new(owner);
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry.try_admit_tick(origin, plan.plan.epoch_like_cpp(), false).unwrap();
    admission.enter_phase(TickPhase::Map);
    registry.begin_shutdown_like_cpp();
    let exit = poisoned_return(&manager, plan, admission, Vec::new(), true);
    assert_original(&exit, &expected);
    assert!(manager.is_poisoned());
    assert!(matches!(manager.try_lock(), Err(std::sync::TryLockError::Poisoned(_))));
    assert!(matches!(&exit, CanonicalMapProducerExit::RetainedBeforeObjects {
        cause: BeforeObjectsExitCause::ShutdownAbandonManagerPoisoned, unresolved_permits, ..
    } if unresolved_permits.is_empty()));
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_terminal_settlement(request, Duration::from_millis(1)).await.is_err());
    assert!(!registry.final_respawn_tick_authorized());
}

#[tokio::test]
async fn joining_immediate_before_objects_return_retains_original_owners_and_reports_incomplete() {
    let (owner, plan) = admitted(200);
    let expected = identity(&plan);
    let (foreign, _foreign_plan) = admitted(300);
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry.try_admit_tick(origin, plan.plan.epoch_like_cpp(), false).unwrap();
    admission.enter_phase(TickPhase::Map);
    let returned = rejected_return(&Mutex::new(foreign), plan, admission, Vec::new(), true);
    // Only transport an already-produced real API rejection; no producer hook.
    let mut handle = tokio::spawn(async move { returned });
    let mut exit = None;
    assert!(!stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    assert_original(exit.as_ref().unwrap(), &expected);
    match exit.as_ref().unwrap() {
        CanonicalMapProducerExit::RetainedBeforeObjects { plan, .. } => {
            assert!(owner.can_resume_tick(&plan.plan));
        }
        _ => panic!("original terminal exit must remain owned after join"),
    }
    assert!(registry.try_admit_tick(origin, expected.epoch + 1, false).is_none());
    assert!(!stop_canonical_map_producer(&mut handle, true, &mut exit).await);
    assert_original(exit.as_ref().unwrap(), &expected);
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.is_err());
}

#[tokio::test]
async fn consumed_select_keeps_before_objects_exit_and_never_repolls_or_completes_ticket() {
    let (owner, plan) = admitted(200);
    let expected = identity(&plan);
    let (foreign, _foreign_plan) = admitted(300);
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry.try_admit_tick(origin, plan.plan.epoch_like_cpp(), false).unwrap();
    admission.enter_phase(TickPhase::Map);
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    assert_eq!(permit.claim_like_cpp(), SessionPhaseClaimLikeCpp::Claimed);
    let returned = rejected_return(&Mutex::new(foreign), plan, admission, vec![Arc::clone(&permit)], false);
    let mut handle = tokio::spawn(async move { returned });
    let returned = tokio::select! { result = &mut handle => result.unwrap() };
    let mut exit = Some(returned);
    assert!(!stop_canonical_map_producer(&mut handle, true, &mut exit).await);
    assert_original(exit.as_ref().unwrap(), &expected);
    // The already-owned exit prevents a second poll even with false here.
    assert!(!stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    match exit.as_ref().unwrap() {
        CanonicalMapProducerExit::RetainedBeforeObjects { plan, unresolved_permits, .. } => {
            assert!(owner.can_resume_tick(&plan.plan));
            assert!(Arc::ptr_eq(&unresolved_permits[0], &permit));
            assert_eq!(permit.state_like_cpp(), SessionPhasePermitStateLikeCpp::Running);
        }
        _ => panic!("consumed select must keep the actual before-Objects exit"),
    }
    assert!(registry.try_admit_tick(origin, expected.epoch + 1, false).is_none());
    drop(exit);
    let request = registry.close_tick_admission();
    assert!(registry.wait_for_terminal_settlement(request, Duration::from_millis(1)).await.is_err());
}
