//! Disposal is a slot-only operation backed by the original owned request/reply.

use super::*;

fn pending(manager: &mut MapManager, tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken, guid: ObjectGuid, stage: u8) -> ActorMovementPending
{
    let progress = manager.prepare_movement(tick, token, guid, None, stage != 0, |_, _| true).unwrap();
    let ActorMovementProgress::Pending(request) = progress else { panic!("fixture must suspend"); };
    if stage != 2 { return request; }
    let ActorMovementPending::StaticHeight(request) = request else { panic!("static height first"); };
    let (_, continuation) = request.into_parts();
    match manager.resume_movement_static_height(tick, token, continuation, wow_entities::INVALID_HEIGHT) {
        Ok(ActorMovementProgress::Pending(request @ ActorMovementPending::GridHeight(_))) => request,
        _ => panic!("invalid static height must request the lazy grid fallback"),
    }
}

fn prefix(manager: &MapManager, guid: ObjectGuid) -> (u64, u64, u32, Position, CreatureAiState) {
    let actor = stored(manager, guid);
    (actor.runtime_elapsed_ms_like_cpp(), actor.runtime_motion_master_ticks_like_cpp(),
        actor.spline_id(), actor.position(), actor.state())
}

#[test]
fn all_three_unlaunched_requests_dispose_only_the_slot_without_replaying_or_restoring() {
    for stage in 0..3 {
        let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 460_001 + i64::from(stage), 200);
        let request = pending(&mut manager, &tick, &mut token, guid, stage);
        let before = prefix(&manager, guid);
        assert!(token.actor_operation.is_some());
        assert!(manager.discard_movement_request(&tick, &mut token, request).is_ok());
        assert!(token.actor_operation.is_none());
        assert_eq!(prefix(&manager, guid), before);
        assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(1));
        assert!(manager.begin_tick_like_cpp(1).is_busy());
    }
}

#[test]
fn three_reply_families_dispose_observed_inputs_without_launch_or_height_application() {
    for stage in 0..3 {
        let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 460_010 + i64::from(stage), 200);
        let request = pending(&mut manager, &tick, &mut token, guid, stage);
        let before = prefix(&manager, guid);
        match request {
            ActorMovementPending::Path(request) => {
                let (query, continuation) = request.into_parts();
                assert!(manager.discard_movement_path_reply(&tick, &mut token, continuation, Some(detour(&query))).is_ok());
            }
            ActorMovementPending::StaticHeight(request) => {
                let (_, continuation) = request.into_parts();
                assert!(manager.discard_movement_static_height_reply(&tick, &mut token, continuation, 91.0).is_ok());
            }
            ActorMovementPending::GridHeight(request) => {
                let (_, continuation) = request.into_parts();
                assert!(manager.discard_movement_grid_height_reply(&tick, &mut token, continuation, 92.0).is_ok());
            }
        }
        assert!(token.actor_operation.is_none());
        assert_eq!(prefix(&manager, guid), before);
        assert!(stored(&manager, guid).active_move_spline_like_cpp().is_none());
        assert!(manager.begin_tick_like_cpp(1).is_busy());
    }
}

#[test]
fn request_rejections_retain_original_query_continuation_and_pending_slot() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 460_020, 200);
    let (_, foreign_tick, mut foreign_token, _) = fixture(CreatureMovementSource::Home, 460_020, 200);
    let phase = stored(&manager, guid).phase_shift().clone();
    let request = pending(&mut manager, &tick, &mut token, guid, 0);
    let before = prefix(&manager, guid);
    let (error, request) = match manager.discard_movement_request(&foreign_tick, &mut token, request) {
        Err(failure) => failure, Ok(()) => panic!("foreign tick cannot dispose"),
    };
    assert!(matches!(error, ActorMovementError::Tick(ObjectMapTickError::OriginMismatch { .. })));
    let (error, request) = match manager.discard_movement_request(&tick, &mut foreign_token, request) {
        Err(failure) => failure, Ok(()) => panic!("foreign token cannot dispose"),
    };
    assert!(matches!(error, ActorMovementError::OperationMismatch { .. }));
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    let (error, request) = match manager.discard_movement_request(&tick, &mut token, request) {
        Err(failure) => failure, Ok(()) => panic!("wrong epoch cannot dispose"),
    };
    assert!(matches!(error, ActorMovementError::Tick(ObjectMapTickError::WrongEpoch { .. })));
    assert!(token.actor_operation.is_some());
    assert_eq!(prefix(&manager, guid), before);
    let ActorMovementPending::Path(request) = request else { panic!("path request retained"); };
    let (query, continuation) = request.into_parts();
    assert_eq!(query.start, before.3);
    assert_eq!((continuation.map_id(), continuation.instance_id()), (1, 0));
    assert_eq!(continuation.phase_shift(), &phase);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
    assert!(manager.discard_movement_path_reply(&tick, &mut token, continuation, Some(detour(&query))).is_ok());
}

#[test]
fn path_reply_rejections_return_the_same_response_allocation_until_explicit_disposal() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 460_021, 200);
    let (_, foreign_tick, mut foreign_token, _) = fixture(CreatureMovementSource::Home, 460_021, 200);
    let ActorMovementPending::Path(request) = pending(&mut manager, &tick, &mut token, guid, 0) else { panic!("path"); };
    let (query, continuation) = request.into_parts();
    let response = Some(detour(&query));
    let buffer = response.as_ref().unwrap().point_path.points.as_ptr();
    let phase = continuation.phase_shift().clone();
    let failure = match manager.discard_movement_path_reply(&foreign_tick, &mut token, continuation, response) {
        Err(failure) => failure, Ok(()) => panic!("foreign tick"),
    };
    assert!(matches!(failure.error, ActorMovementError::Tick(ObjectMapTickError::OriginMismatch { .. })));
    let failure = match manager.discard_movement_path_reply(&tick, &mut foreign_token, failure.continuation, failure.response) {
        Err(failure) => failure, Ok(()) => panic!("foreign token"),
    };
    assert!(matches!(failure.error, ActorMovementError::OperationMismatch { .. }));
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    let failure = match manager.discard_movement_path_reply(&tick, &mut token, failure.continuation, failure.response) {
        Err(failure) => failure, Ok(()) => panic!("wrong epoch"),
    };
    assert!(matches!(failure.error, ActorMovementError::Tick(ObjectMapTickError::WrongEpoch { .. })));
    assert_eq!(failure.response.as_ref().unwrap().point_path.points.as_ptr(), buffer);
    assert_eq!(failure.continuation.phase_shift(), &phase);
    assert!(token.actor_operation.is_some());
    assert!(foreign_token.actor_operation.is_none());
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
    assert!(manager.discard_movement_path_reply(&tick, &mut token, failure.continuation, failure.response).is_ok());
}

#[test]
fn height_reply_rejection_preserves_scalar_bits_and_original_continuation() {
    for grid in [false, true] {
        let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 460_030 + i64::from(grid), 200);
        let request = pending(&mut manager, &tick, &mut token, guid, if grid { 2 } else { 1 });
        let response = f32::from_bits(0x7fc0_1234);
        manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
        match request {
            ActorMovementPending::StaticHeight(request) => {
                let (_, continuation) = request.into_parts();
                let failure = match manager.discard_movement_static_height_reply(&tick, &mut token, continuation, response) {
                    Err(failure) => failure, Ok(()) => panic!("wrong epoch"),
                };
                assert_eq!(failure.response.to_bits(), response.to_bits());
                manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
                assert!(manager.discard_movement_static_height_reply(&tick, &mut token, failure.continuation, failure.response).is_ok());
            }
            ActorMovementPending::GridHeight(request) => {
                let (_, continuation) = request.into_parts();
                let failure = match manager.discard_movement_grid_height_reply(&tick, &mut token, continuation, response) {
                    Err(failure) => failure, Ok(()) => panic!("wrong epoch"),
                };
                assert_eq!(failure.response.to_bits(), response.to_bits());
                manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
                assert!(manager.discard_movement_grid_height_reply(&tick, &mut token, failure.continuation, failure.response).is_ok());
            }
            _ => panic!("height request retained"),
        }
        assert!(token.actor_operation.is_none());
    }
}

#[test]
fn stale_map_and_aba_disposal_touch_no_replacement_and_leave_map_accounting_pending() {
    for (unlaunched, stale_map) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
        manager.updater.activate(1);
        manager.create_world_map(1, 0);
        let incoming = actor(CreatureMovementSource::Home, 460_040 + i64::from(stale_map));
        let guid = incoming.guid();
        insert_actor(&mut manager, incoming, true);
        let (tick, mut token) = begin(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
        let request = pending(&mut manager, &tick, &mut token, guid, 0);
        if stale_map {
            let old = manager.maps.remove(&MapKey::new(1, 0)).unwrap();
            manager.create_world_map(1, 0);
            drop(old);
        } else {
            drop(manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap());
        }
        insert_actor(&mut manager, actor(CreatureMovementSource::Home, 460_040 + i64::from(stale_map)), false);
        let before = prefix(&manager, guid);
        if unlaunched {
            assert!(manager.discard_movement_request(&tick, &mut token, request).is_ok());
        } else {
            let ActorMovementPending::Path(request) = request else { panic!("path"); };
            let (query, continuation) = request.into_parts();
            assert!(manager.discard_movement_path_reply(&tick, &mut token, continuation, Some(detour(&query))).is_ok());
        }
        assert!(token.actor_operation.is_none());
        assert_eq!(prefix(&manager, guid), before);
        assert_eq!((manager.updater.pending_requests, manager.updater.scheduled_updates(), manager.updater.wait_calls()), (1, 1, 0));
        assert!(manager.begin_tick_like_cpp(1).is_busy());
    }
}

#[test]
fn dropping_owned_request_still_cannot_dispose_or_finish_without_its_evidence() {
    let (mut manager, mut tick, mut token, guid) = fixture(CreatureMovementSource::Home, 460_050, 200);
    let request = pending(&mut manager, &tick, &mut token, guid, 0);
    drop(request);
    assert!(token.actor_operation.is_some());
    assert!(matches!(manager.try_finish_object_map::<LoadRecord>(&mut tick, token, None, None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime), Err((ObjectMapTickError::ActorOperationInFlight { .. }, _))));
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}
