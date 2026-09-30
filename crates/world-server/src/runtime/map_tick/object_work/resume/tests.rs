//! Owned stage rejection, explicit retry, and the real admission ledger.
use super::*;
use crate::session_supervision::{ProducerKind, TickDisposition, TickPhase};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use wow_core::ObjectGuid;
use wow_map::{MapManager, MapTickCoordinationStateLikeCpp, ObjectMapTickError};
mod fixtures;
use fixtures::*;

#[test]
fn prepare_rejection_retains_original_summary_until_actual_token_is_finished() {
    let catalogs = Catalogs::empty();
    let (mut manager, mut work) = setup(&[1]);
    let allocation = seed_summary(&mut work);
    let token = work.try_prepare_next(&mut manager).unwrap().unwrap();
    let incarnation = token.incarnation();
    let failure = match resume_objects(
        &mut manager,
        work,
        &catalogs.metadata,
        &catalogs.maps,
        &catalogs.caches,
    ) {
        Err(failure @ CanonicalObjectResumeFailure::PrepareRejected { .. }) => failure,
        _ => panic!("actual outstanding token must reject prepare"),
    };
    let failure = match catalogs.retry(failure, &mut manager) {
        Err(failure @ CanonicalObjectResumeFailure::PrepareRejected { .. }) => failure,
        _ => panic!("retry cannot dispose the outstanding token"),
    };
    let work = match failure {
        CanonicalObjectResumeFailure::PrepareRejected {
            error: ObjectMapTickError::MapInFlight { participant },
            work,
        } => {
            assert_eq!(participant.incarnation, incarnation);
            assert_eq!(
                work.respawn_summary.expired_pvp_combat_refs.as_ptr(),
                allocation
            );
            assert_eq!(work.respawn_summary.respawn_db_delete_failed, 17);
            work
        }
        _ => panic!("original in-flight work must remain owned"),
    };
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    let work = match finish_prepared_map(
        &mut manager,
        work,
        token,
        &catalogs.metadata,
        &catalogs.caches,
    ) {
        Ok(work) => work,
        Err(_) => panic!("the original token must settle"),
    };
    let summary = delivered(resume_objects(
        &mut manager,
        work,
        &catalogs.metadata,
        &catalogs.maps,
        &catalogs.caches,
    ));
    assert_eq!(summary.expired_pvp_combat_refs.as_ptr(), allocation);
    assert_eq!(summary.respawn_db_delete_failed, 17);
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(
        manager.find_map(1, 0).unwrap().delayed_update_calls(),
        [200]
    );
}

#[test]
fn foreign_finish_returns_original_token_and_work_for_exact_finish_retry() {
    let catalogs = Catalogs::empty();
    let (mut owner, mut work) = setup(&[1]);
    let (mut foreign, _foreign_work) = setup(&[1]);
    let allocation = seed_summary(&mut work);
    let token = work.try_prepare_next(&mut owner).unwrap().unwrap();
    let identity = (token.key(), token.incarnation(), token.effective_diff_ms());
    let failure = match finish_prepared_map(
        &mut foreign,
        work,
        token,
        &catalogs.metadata,
        &catalogs.caches,
    ) {
        Err(failure @ CanonicalObjectResumeFailure::FinishRejected { .. }) => failure,
        _ => panic!("foreign manager must reject finish with the original token"),
    };
    match &failure {
        CanonicalObjectResumeFailure::FinishRejected {
            error: ObjectMapTickError::OriginMismatch { .. },
            work,
            token,
        } => {
            assert_eq!(
                (token.key(), token.incarnation(), token.effective_diff_ms()),
                identity
            );
            assert_eq!(
                work.respawn_summary.expired_pvp_combat_refs.as_ptr(),
                allocation
            );
        }
        _ => panic!("finish error priority must remain origin mismatch"),
    }
    assert_eq!(owner.updater.wait_calls(), 0);
    assert_eq!(foreign.updater.wait_calls(), 0);
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    let summary = delivered(catalogs.retry(failure, &mut owner));
    assert_eq!(summary.expired_pvp_combat_refs.as_ptr(), allocation);
    assert_eq!(summary.respawn_db_delete_failed, 17);
    assert_eq!(owner.updater.wait_calls(), 1);
    assert_eq!(owner.find_map(1, 0).unwrap().delayed_update_calls(), [200]);
    assert!(
        foreign
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
}

#[test]
fn foreign_finalize_retry_runs_only_original_finalization_and_consuming_tail() {
    let catalogs = Catalogs::empty();
    let (mut owner, mut work) = setup(&[1]);
    let (mut foreign, _foreign_work) = setup(&[]);
    let allocation = seed_summary(&mut work);
    let token = work.try_prepare_next(&mut owner).unwrap().unwrap();
    work = match finish_prepared_map(
        &mut owner,
        work,
        token,
        &catalogs.metadata,
        &catalogs.caches,
    ) {
        Ok(work) => work,
        Err(_) => panic!("real original map must finish"),
    };
    assert!(work.try_prepare_next(&mut owner).unwrap().is_none());
    let failure = match finalize_work(&mut foreign, work, &catalogs.maps) {
        Err(failure @ CanonicalObjectResumeFailure::FinalizeRejected { .. }) => failure,
        _ => panic!("foreign finalization must retain original work"),
    };
    match &failure {
        CanonicalObjectResumeFailure::FinalizeRejected {
            error: ObjectMapTickError::OriginMismatch { .. },
            work,
        } => {
            assert_eq!(
                work.respawn_summary.expired_pvp_combat_refs.as_ptr(),
                allocation
            );
        }
        _ => panic!("foreign finalization must preserve its error"),
    }
    assert_eq!(owner.updater.wait_calls(), 0);
    let summary = delivered(catalogs.retry(failure, &mut owner));
    assert_eq!(summary.expired_pvp_combat_refs.as_ptr(), allocation);
    assert_eq!(summary.maps_evaluated, 0);
    assert_eq!(owner.updater.wait_calls(), 1);
    assert_eq!(owner.find_map(1, 0).unwrap().delayed_update_calls(), [200]);
    assert_eq!(
        owner.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(foreign.updater.wait_calls(), 0);
}

#[test]
fn incomplete_finalize_retry_does_not_prepare_or_replay_prefix() {
    let catalogs = Catalogs::empty();
    let (mut manager, mut work) = setup(&[1]);
    let allocation = seed_summary(&mut work);
    let failure = match finalize_work(&mut manager, work, &catalogs.maps) {
        Err(failure) => failure,
        _ => panic!("incomplete participants must reject finalization"),
    };
    match catalogs.retry(failure, &mut manager) {
        Err(CanonicalObjectResumeFailure::FinalizeRejected {
            error:
                ObjectMapTickError::Incomplete {
                    processed_participants: 0,
                    total_participants: 1,
                },
            work,
        }) => {
            assert_eq!(
                work.respawn_summary.expired_pvp_combat_refs.as_ptr(),
                allocation
            );
            assert_eq!(work.respawn_summary.maps_evaluated, 0);
        }
        _ => panic!("explicit retry must remain at finalization"),
    }
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn begin_rejection_returns_original_plan_and_retry_leaves_it_untouched() {
    let catalogs = Catalogs::empty();
    let mut owner = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    let mut foreign = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    owner.create_world_map(1, 0);
    let plan = owner.begin_tick_like_cpp(200).into_started().unwrap();
    let original_epoch = plan.epoch_like_cpp();
    let original_participants = plan.updated_maps_like_cpp().as_ptr();
    let original_destroyed = plan.destroyed_maps_like_cpp().as_ptr();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let failure = match super::super::super::try_canonical_map_tick_resume(
        &mut foreign,
        None,
        plan,
        &mut scheduler,
        &catalogs.metadata,
        &catalogs.conditions,
        &catalogs.maps,
        &catalogs.caches,
    ) {
        Err(failure @ CanonicalObjectResumeFailure::BeginRejected { .. }) => failure,
        _ => panic!("foreign preflight must return the original plan"),
    };
    assert_eq!(scheduler.timer_ms(), 500);
    match catalogs.retry(failure, &mut owner) {
        Err(CanonicalObjectResumeFailure::BeginRejected {
            failure: ObjectWorkBeginFailure::BeforePrefix { plan },
        }) => {
            assert_eq!(plan.epoch_like_cpp(), original_epoch);
            assert_eq!(plan.updated_maps_like_cpp().as_ptr(), original_participants);
            assert_eq!(plan.destroyed_maps_like_cpp().as_ptr(), original_destroyed);
            assert!(owner.can_resume_tick(&plan));
        }
        _ => panic!("retry must return the same untouched begin failure"),
    }
    assert_eq!(scheduler.timer_ms(), 500);
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    let mut another = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    let plan = another.begin_tick_like_cpp(200).into_started().unwrap();
    assert!(
        super::super::super::canonical_map_tick_resume_like_cpp(
            &mut foreign,
            None,
            plan,
            &mut scheduler,
            &catalogs.metadata,
            &catalogs.conditions,
            &catalogs.maps,
            &catalogs.caches
        )
        .is_none()
    );
    assert_eq!(scheduler.timer_ms(), 500);
    assert!(another.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn ordinary_empty_result_settles_actual_objects_admission_and_allows_next_tick() {
    let catalogs = Catalogs::empty();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let ticket = registry.try_admit_tick(origin, 1, false).unwrap();
    ticket.enter_phase(TickPhase::Map);
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    ticket.enter_phase(TickPhase::Objects);
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(10_000);
    assert!(matches!(
        super::super::super::try_canonical_map_tick_resume(
            &mut manager,
            None,
            plan,
            &mut scheduler,
            &catalogs.metadata,
            &catalogs.conditions,
            &catalogs.maps,
            &catalogs.caches
        ),
        Ok(None)
    ));
    assert_eq!(scheduler.timer_ms(), 9_800);
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(
        manager.find_map(1, 0).unwrap().delayed_update_calls(),
        [200]
    );
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    ticket.enter_phase(TickPhase::PostTail);
    assert!(ticket.complete(TickDisposition::FullyFinished));
    let next = registry.try_admit_tick(origin, 2, false).unwrap();
    assert!(next.complete(TickDisposition::AbandonedAfterAccounting));
    assert!(manager.begin_tick_like_cpp(200).into_started().is_some());
    assert_eq!(manager.updater.wait_calls(), 1);
}

#[tokio::test]
async fn actual_empty_producer_continues_post_tail_and_returns_settlement_receipt() {
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let stop = Arc::new(AtomicBool::new(false));
    let mut canonical = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 1);
    canonical.create_world_map(1, 0);
    canonical.updater.activate(1);
    let manager = Arc::new(Mutex::new(canonical));
    let producer = spawn_empty_producer(
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
    .expect("actual producer must finalize an empty ordinary result");
    // The current-thread task reaches its interval wait only after PostTail and
    // FullyFinished. No game-event await is due in these concrete catalogs.
    let completed_ticks = manager.lock().unwrap().updater.wait_calls();
    stop.store(true, Ordering::Release);
    tokio::time::timeout(Duration::from_secs(2), producer)
        .await
        .unwrap()
        .unwrap();
    assert!(completed_ticks >= 2);
    assert_eq!(
        manager.lock().unwrap().updater.wait_calls(),
        completed_ticks
    );
    assert_eq!(
        manager.lock().unwrap().tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .len(),
        completed_ticks as usize
    );
    let request = registry.close_tick_admission();
    let receipt = registry
        .wait_for_quiescence(request, Duration::from_secs(2))
        .await
        .unwrap();
    assert!(registry.enable_session_drain(receipt).is_ok());
    assert!(registry.session_drain_authorized());
}

#[tokio::test]
async fn holding_actual_failed_work_blocks_admission_and_drop_is_not_a_receipt() {
    let catalogs = Catalogs::empty();
    let registry = Arc::new(crate::ActiveWorldSessionRegistryLikeCpp::new());
    let origin = registry.register_producer(ProducerKind::Canonical);
    let ticket = registry.try_admit_tick(origin, 1, false).unwrap();
    ticket.enter_phase(TickPhase::Objects);
    let (mut manager, mut work) = setup(&[1]);
    let allocation = seed_summary(&mut work);
    let token = work.try_prepare_next(&mut manager).unwrap().unwrap();
    let failure = match resume_objects(
        &mut manager,
        work,
        &catalogs.metadata,
        &catalogs.maps,
        &catalogs.caches,
    ) {
        Err(failure) => failure,
        _ => panic!("outstanding real token must reject prepare"),
    };
    let held = Some((failure, ticket));
    let mut interval = tokio::time::interval(Duration::from_millis(1));
    let mut advanced = 0;
    for _ in 0..2 {
        interval.tick().await;
        if held.is_some() {
            continue;
        }
        advanced += 1;
    }
    assert_eq!(advanced, 0);
    match &held.as_ref().unwrap().0 {
        CanonicalObjectResumeFailure::PrepareRejected { work, .. } => {
            assert_eq!(
                work.respawn_summary.expired_pvp_combat_refs.as_ptr(),
                allocation
            );
        }
        _ => panic!("holding must preserve actual work"),
    }
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    drop(held);
    drop(token);
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    let request = registry.close_tick_admission();
    assert!(
        registry
            .wait_for_quiescence(request, Duration::from_millis(1))
            .await
            .is_err()
    );
    assert!(!registry.session_drain_authorized());
}
