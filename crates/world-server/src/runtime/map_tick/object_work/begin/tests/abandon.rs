//! Explicit before-prefix abandonment; after-prefix values remain owned.
use super::*;
use wow_core::ObjectGuid;

fn admitted_with_both_vectors(diff: u32) -> (MapManager, wow_map::MapTickPlanLikeCpp) {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    manager.create_map_entry(
        33,
        7,
        1,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    manager.find_map_mut(33, 7).unwrap().set_can_unload(true);
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(diff).into_started().unwrap();
    (manager, plan)
}

#[test]
fn actual_before_prefix_failure_rejects_wrong_owner_then_abandons_same_plan_at_original_owner() {
    let catalogs = Catalogs::empty();
    let (mut owner, plan) = admitted_with_both_vectors(200);
    let (mut foreign, foreign_plan) = admitted_with_both_vectors(300);
    let updated = plan.updated_maps_like_cpp().as_ptr();
    let destroyed = plan.destroyed_maps_like_cpp().as_ptr();
    let epoch = plan.epoch_like_cpp();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let failure = match catalogs.begin(&mut foreign, plan, &mut scheduler) {
        Err(failure @ ObjectWorkBeginFailure::BeforePrefix { .. }) => failure,
        _ => panic!("real foreign preflight must return the same plan before prefix"),
    };
    let failure = match failure.try_abandon_before_prefix(&mut foreign) {
        Err(failure @ ObjectWorkBeginFailure::BeforePrefix { .. }) => failure,
        _ => panic!("foreign abandon must retain the complete original failure"),
    };
    match &failure {
        ObjectWorkBeginFailure::BeforePrefix { plan } => {
            assert_eq!(plan.updated_maps_like_cpp().as_ptr(), updated);
            assert_eq!(plan.destroyed_maps_like_cpp().as_ptr(), destroyed);
            assert_eq!(plan.updated_maps_like_cpp().len(), 2);
            assert_eq!(plan.destroyed_maps_like_cpp().len(), 1);
            assert_eq!(plan.epoch_like_cpp(), epoch);
            assert_eq!(plan.effective_diff_ms(), 200);
            assert!(owner.can_resume_tick(plan));
            assert!(!foreign.can_resume_tick(plan));
        }
        _ => panic!("before-prefix variant must be retained"),
    }
    assert_eq!(scheduler.timer_ms(), 500);
    assert_eq!(
        owner.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );
    assert_eq!(
        foreign.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );
    assert!(failure.try_abandon_before_prefix(&mut owner).is_ok());
    assert_eq!(
        owner.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(
        foreign.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );
    assert!(foreign.can_resume_tick(&foreign_plan));
    assert_eq!(scheduler.timer_ms(), 500);
    assert_eq!(owner.updater.wait_calls(), 0);
    assert_eq!(foreign.updater.wait_calls(), 0);
    assert!(owner.find_map(33, 7).is_some());
    assert!(
        owner
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    assert!(
        foreign
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
}

#[test]
fn after_prefix_boundary_keeps_error_plan_and_summary_at_both_managers_without_abandoning() {
    let catalogs = Catalogs::empty();
    let (mut owner, plan) = admitted_with_both_vectors(200);
    let (mut foreign, foreign_plan) = admitted_with_both_vectors(300);
    let epoch = plan.epoch_like_cpp();
    let updated = plan.updated_maps_like_cpp().as_ptr();
    let destroyed = plan.destroyed_maps_like_cpp().as_ptr();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(200);
    let mut respawn_summary = canonical_map_tick_respawn_phase_like_cpp(
        &mut owner,
        plan.updated_maps_like_cpp(),
        None,
        plan.effective_diff_ms(),
        &mut scheduler,
        &catalogs.metadata,
        &catalogs.conditions,
        &catalogs.maps,
        &catalogs.caches,
    );
    // A payload sentinel only, not a claim of a produced combat effect. This
    // boundary packaging test proves an owned nonempty buffer is never cloned
    // or discarded. Normal exclusive try_begin cannot change manager origin.
    respawn_summary
        .expired_pvp_combat_refs
        .push((1, 0, ObjectGuid::EMPTY, ObjectGuid::EMPTY));
    let summary_buffer = respawn_summary.expired_pvp_combat_refs.as_ptr();
    let (error, plan) = match foreign.try_begin_object_tick(plan) {
        Err(rejected) => rejected,
        Ok(_) => panic!("real foreign BEGIN must return its original error and plan"),
    };
    let failure = ObjectWorkBeginFailure::AfterPrefix {
        error,
        plan,
        respawn_summary,
    };
    let failure = match failure.try_abandon_before_prefix(&mut foreign) {
        Err(failure) => failure,
        Ok(()) => panic!("AfterPrefix cannot abandon even at a foreign manager"),
    };
    let failure = match failure.try_abandon_before_prefix(&mut owner) {
        Err(failure) => failure,
        Ok(()) => panic!("AfterPrefix cannot abandon its original manager either"),
    };
    match failure {
        ObjectWorkBeginFailure::AfterPrefix {
            error: ObjectMapTickError::OriginMismatch { plan_epoch },
            plan,
            respawn_summary,
        } => {
            assert_eq!(plan_epoch, epoch);
            assert_eq!(plan.epoch_like_cpp(), epoch);
            assert_eq!(plan.effective_diff_ms(), 200);
            assert_eq!(plan.updated_maps_like_cpp().as_ptr(), updated);
            assert_eq!(plan.destroyed_maps_like_cpp().as_ptr(), destroyed);
            assert_eq!(plan.updated_maps_like_cpp().len(), 2);
            assert_eq!(plan.destroyed_maps_like_cpp().len(), 1);
            assert_eq!(
                respawn_summary.expired_pvp_combat_refs.as_ptr(),
                summary_buffer
            );
            assert_eq!(
                respawn_summary.expired_pvp_combat_refs,
                vec![(1, 0, ObjectGuid::EMPTY, ObjectGuid::EMPTY)]
            );
            assert_eq!(respawn_summary.maps_evaluated, 2);
            assert_eq!(respawn_summary.outcomes, 0);
            assert!(owner.can_resume_tick(&plan));
        }
        _ => panic!("AfterPrefix must retain the actual error and complete payload"),
    }
    assert_eq!(
        owner.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );
    assert_eq!(
        foreign.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );
    assert!(foreign.can_resume_tick(&foreign_plan));
    assert_eq!(scheduler.timer_ms(), 200);
    assert_eq!(owner.updater.wait_calls(), 0);
    assert_eq!(foreign.updater.wait_calls(), 0);
    assert!(owner.find_map(33, 7).is_some());
    assert!(
        owner
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
}
