//! Owned cooperative return, consumed joins and the real unsettled ledger.
use super::*;
use crate::runtime::map_tick::object_work::ObjectWorkBeginFailure;
use crate::session_supervision::{ProducerKind, TickPhase};
use std::sync::{Arc, Mutex};
use std::time::Duration;
mod before_objects;
mod fixtures;
use fixtures::*;

fn rejected_plan(failure: &CanonicalObjectResumeFailure) -> &wow_map::MapTickPlanLikeCpp {
    match failure {
        CanonicalObjectResumeFailure::BeginRejected {
            failure: ObjectWorkBeginFailure::BeforePrefix { plan },
        } => plan,
        _ => panic!("the actual rejected plan must stay owned"),
    }
}

fn returned_plan(exit: &CanonicalMapProducerExit) -> &wow_map::MapTickPlanLikeCpp {
    match exit {
        CanonicalMapProducerExit::RetainedObjects { failure, .. } => rejected_plan(failure),
        _ => panic!("the actual rejection must stay owned"),
    }
}

#[tokio::test]
async fn stop_false_keeps_original_allocations_and_same_objects_admission() {
    let (mut owner, failure, updated, destroyed) = rejected_begin();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry
        .try_admit_tick(origin, rejected_plan(&failure).epoch_like_cpp(), false)
        .unwrap();
    admission.enter_phase(TickPhase::Objects);
    let mut held = Some((failure, admission));
    let stop = AtomicBool::new(false);
    let mut interval = tokio::time::interval(Duration::from_millis(1));
    for _ in 0..2 {
        interval.tick().await;
        assert!(take_retained_objects_on_stop(&mut held, &stop).is_none());
        match &held.as_ref().unwrap().0 {
            CanonicalObjectResumeFailure::BeginRejected {
                failure: ObjectWorkBeginFailure::BeforePrefix { plan },
            } => {
                assert_eq!(plan.updated_maps_like_cpp().as_ptr(), updated);
                assert_eq!(plan.destroyed_maps_like_cpp().as_ptr(), destroyed);
                assert!(owner.can_resume_tick(plan));
            }
            _ => panic!("stop false must leave the original failure untouched"),
        }
        // This uses the actual ticket's original ledger record and epoch.
        held.as_ref().unwrap().1.enter_phase(TickPhase::Objects);
        assert!(registry.try_admit_tick(origin, 2, false).is_none());
        assert!(owner.begin_tick_like_cpp(999_999).is_busy());
    }
    assert!(
        owner
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    let request = registry.close_tick_admission();
    assert!(
        registry
            .wait_for_quiescence(request, Duration::from_millis(1))
            .await
            .is_err()
    );
    assert!(!registry.session_drain_authorized());
}

#[tokio::test]
async fn stop_true_moves_original_plan_and_ticket_without_settlement_or_cleanup() {
    let (mut owner, failure, updated, destroyed) = rejected_begin();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry
        .try_admit_tick(origin, rejected_plan(&failure).epoch_like_cpp(), false)
        .unwrap();
    admission.enter_phase(TickPhase::Objects);
    let mut held = Some((failure, admission));
    let stop = AtomicBool::new(false);
    stop.store(true, Ordering::Release);
    let exit = take_retained_objects_on_stop(&mut held, &stop).unwrap();
    assert!(held.is_none());
    assert_eq!(
        returned_plan(&exit).updated_maps_like_cpp().as_ptr(),
        updated
    );
    assert_eq!(
        returned_plan(&exit).destroyed_maps_like_cpp().as_ptr(),
        destroyed
    );
    assert_eq!(returned_plan(&exit).updated_maps_like_cpp().len(), 2);
    assert_eq!(returned_plan(&exit).destroyed_maps_like_cpp().len(), 1);
    assert!(owner.can_resume_tick(returned_plan(&exit)));
    match &exit {
        CanonicalMapProducerExit::RetainedObjects { admission, .. } => {
            admission.enter_phase(TickPhase::Objects)
        }
        _ => panic!("the original admission must be returned with its failure"),
    }
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    assert!(take_retained_objects_on_stop(&mut held, &stop).is_none());
    drop(exit);
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    let request = registry.close_tick_admission();
    assert!(
        registry
            .wait_for_terminal_settlement(request, Duration::from_millis(1))
            .await
            .is_err()
    );
    assert!(!registry.session_drain_authorized());
}

#[tokio::test]
async fn already_consumed_select_retains_original_exit_and_never_repolls_handle() {
    let (owner, failure, updated, destroyed) = rejected_begin();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry
        .try_admit_tick(origin, rejected_plan(&failure).epoch_like_cpp(), false)
        .unwrap();
    admission.enter_phase(TickPhase::Objects);
    let mut handle = tokio::spawn(async move {
        let mut held = Some((failure, admission));
        take_retained_objects_on_stop(&mut held, &AtomicBool::new(true)).unwrap()
    });
    let returned = tokio::select! { result = &mut handle => result.unwrap() };
    let mut exit = Some(returned);
    assert!(!stop_canonical_map_producer(&mut handle, true, &mut exit).await);
    assert_eq!(
        returned_plan(exit.as_ref().unwrap())
            .updated_maps_like_cpp()
            .as_ptr(),
        updated
    );
    assert_eq!(
        returned_plan(exit.as_ref().unwrap())
            .destroyed_maps_like_cpp()
            .as_ptr(),
        destroyed
    );
    assert!(owner.can_resume_tick(returned_plan(exit.as_ref().unwrap())));
    // Even if a caller passes false, an already-owned return is not polled twice.
    assert!(!stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    assert_eq!(
        returned_plan(exit.as_ref().unwrap())
            .updated_maps_like_cpp()
            .as_ptr(),
        updated
    );
    match exit.as_ref().unwrap() {
        CanonicalMapProducerExit::RetainedObjects { admission, .. } => {
            admission.enter_phase(TickPhase::Objects)
        }
        _ => panic!("select must retain the same actual admission"),
    }
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    let request = registry.close_tick_admission();
    assert!(
        registry
            .wait_for_quiescence(request, Duration::from_millis(1))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn joining_retained_return_preserves_owners_and_reports_incomplete() {
    let (owner, failure, updated, destroyed) = rejected_begin();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry
        .try_admit_tick(origin, rejected_plan(&failure).epoch_like_cpp(), false)
        .unwrap();
    admission.enter_phase(TickPhase::Objects);
    let mut handle = tokio::spawn(async move {
        let mut held = Some((failure, admission));
        take_retained_objects_on_stop(&mut held, &AtomicBool::new(true)).unwrap()
    });
    let mut exit = None;
    assert!(!stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    assert_eq!(
        returned_plan(exit.as_ref().unwrap())
            .updated_maps_like_cpp()
            .as_ptr(),
        updated
    );
    assert_eq!(
        returned_plan(exit.as_ref().unwrap())
            .destroyed_maps_like_cpp()
            .as_ptr(),
        destroyed
    );
    assert!(owner.can_resume_tick(returned_plan(exit.as_ref().unwrap())));
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
}

#[tokio::test]
async fn normal_real_empty_producer_returns_typed_exit_with_real_receipt_still_required() {
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let stop = Arc::new(AtomicBool::new(false));
    let mut canonical = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 1);
    canonical.create_world_map(1, 0);
    canonical.updater.activate(1);
    let manager = Arc::new(Mutex::new(canonical));
    let mut handle = spawn_empty_producer(
        Arc::clone(&manager),
        Arc::clone(&registry),
        Arc::clone(&stop),
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if manager.lock().unwrap().updater.wait_calls() >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("actual ordinary producer must continue across empty outputs");
    let completed = manager.lock().unwrap().updater.wait_calls();
    stop.store(true, Ordering::Release);
    let mut exit = None;
    assert!(stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    assert!(matches!(
        exit,
        Some(CanonicalMapProducerExit::WithoutObjectFailure)
    ));
    assert_eq!(manager.lock().unwrap().updater.wait_calls(), completed);
    assert_eq!(
        manager.lock().unwrap().tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .len(),
        completed as usize
    );
    assert!(!registry.session_drain_authorized());
    let request = registry.close_tick_admission();
    let receipt = registry
        .wait_for_quiescence(request, Duration::from_secs(2))
        .await
        .unwrap();
    assert!(registry.enable_session_drain(receipt).is_ok());
    assert!(registry.session_drain_authorized());
}

#[tokio::test]
async fn aborted_owner_join_is_unproven_and_does_not_release_actual_admission() {
    let (mut owner, failure, _, _) = rejected_begin();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry
        .try_admit_tick(origin, rejected_plan(&failure).epoch_like_cpp(), false)
        .unwrap();
    admission.enter_phase(TickPhase::Objects);
    let mut handle: JoinHandle<CanonicalMapProducerExit> = tokio::spawn(async move {
        let held = Some((failure, admission));
        std::future::pending::<()>().await;
        let mut held = held;
        take_retained_objects_on_stop(&mut held, &AtomicBool::new(true)).unwrap()
    });
    handle.abort();
    let mut exit = None;
    assert!(!stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    assert!(exit.is_none());
    assert!(!stop_canonical_map_producer(&mut handle, true, &mut exit).await);
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    let request = registry.close_tick_admission();
    assert!(
        registry
            .wait_for_terminal_settlement(request, Duration::from_millis(1))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn timeout_requests_abort_without_return_or_settlement_receipt() {
    let (mut owner, failure, _, _) = rejected_begin();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let admission = registry
        .try_admit_tick(origin, rejected_plan(&failure).epoch_like_cpp(), false)
        .unwrap();
    admission.enter_phase(TickPhase::Objects);
    let mut handle: JoinHandle<CanonicalMapProducerExit> = tokio::spawn(async move {
        let held = Some((failure, admission));
        std::future::pending::<()>().await;
        let mut held = held;
        take_retained_objects_on_stop(&mut held, &AtomicBool::new(true)).unwrap()
    });
    let mut exit = None;
    assert!(!stop_canonical_map_producer(&mut handle, false, &mut exit).await);
    assert!(exit.is_none());
    match handle.await {
        Err(error) => assert!(error.is_cancelled()),
        Ok(_) => panic!("timeout abort must not fabricate a returned exit"),
    }
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    let request = registry.close_tick_admission();
    assert!(
        registry
            .wait_for_quiescence(request, Duration::from_millis(1))
            .await
            .is_err()
    );
}
