use super::*;

fn pending(manager: &mut MapManager, tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken, guid: ObjectGuid) -> (CreaturePathQueryLikeCpp, ActorPathContinuation)
{
    path_request(manager.prepare_movement(tick, token, guid, None, false, |_, _| true).unwrap())
}

fn prefix(manager: &MapManager, guid: ObjectGuid) -> (u64, u64, u32, Position) {
    let actor = stored(manager, guid);
    (actor.runtime_elapsed_ms_like_cpp(), actor.runtime_motion_master_ticks_like_cpp(), actor.spline_id(), actor.position())
}

#[test]
fn rejected_origin_and_epoch_prepare_do_not_run_clock_or_policy() {
    let (mut first, tick, mut token, guid) = fixture(CreatureMovementSource::Random, 450_020, 200);
    let (mut second, foreign_tick, mut foreign_token, _) = fixture(CreatureMovementSource::Random, 450_020, 300);
    let before = prefix(&first, guid);
    assert!(matches!(first.prepare_movement(&foreign_tick, &mut token, guid, None, false,
        |_, _| panic!("foreign origin cannot run policy")),
        Err(ActorMovementError::Tick(ObjectMapTickError::OriginMismatch { .. }))));
    assert!(matches!(first.prepare_movement(&tick, &mut foreign_token, guid, None, false,
        |_, _| panic!("foreign token cannot run policy")),
        Err(ActorMovementError::Tick(ObjectMapTickError::OriginMismatch { .. }))));
    first.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    assert!(matches!(first.prepare_movement(&tick, &mut token, guid, None, false,
        |_, _| panic!("wrong epoch cannot run policy")),
        Err(ActorMovementError::Tick(ObjectMapTickError::WrongEpoch { .. }))));
    assert_eq!(prefix(&first, guid), before);
    assert!(token.actor_operation.is_none());
    assert_eq!(prefix(&second, guid).0, 0);
    assert!(second.begin_tick_like_cpp(1).is_busy());
    let mut untouched = actor(CreatureMovementSource::Random, 450_020);
    for _ in 0..4 {
        let actual = first.find_map_mut(1, 0).unwrap().map_mut().creature_actor_mut(guid).unwrap()
            .pick_random_destination_from_current_position_like_cpp(3.0);
        assert_eq!(actual, untouched.pick_random_destination_from_current_position_like_cpp(3.0),
            "rejected prepares must not draw from the movement RNG");
    }
}

#[test]
fn selected_workset_rejects_outside_actor_and_record_before_prefix() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let incoming = actor(CreatureMovementSource::Home, 450_021);
    let guid = incoming.guid();
    insert_actor(&mut manager, incoming, false);
    let (tick, mut token) = begin(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    assert!(matches!(manager.prepare_movement(&tick, &mut token, guid, None, false,
        |_, _| panic!("unselected actor cannot run policy")),
        Err(ActorMovementError::OutsideSelection { .. })));
    assert_eq!(prefix(&manager, guid).0, 0);
    assert!(token.actor_operation.is_none());

    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_022, 200);
    let removed = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap();
    let incoming = actor(CreatureMovementSource::Home, 450_022);
    manager.find_map_mut(1, 0).unwrap().map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(incoming.creature).unwrap()).unwrap();
    assert!(matches!(manager.prepare_movement(&tick, &mut token, guid, None, false,
        |_, _| panic!("a Record is not a runtime Actor")),
        Err(ActorMovementError::OutsideSelection { .. })));
    assert!(token.actor_operation.is_none());
    assert_eq!(manager.find_map(1, 0).unwrap().map().get_typed_creature(guid).unwrap().current_health(), 25);
    drop(removed);
}

#[test]
fn foreign_and_wrong_epoch_resume_return_the_same_response_buffer_and_continuation() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_023, 200);
    let (mut other, other_tick, mut other_token, _) = fixture(CreatureMovementSource::Home, 450_023, 300);
    let (query, continuation) = pending(&mut manager, &tick, &mut token, guid);
    let phase = continuation.phase_shift().clone();
    let before = prefix(&manager, guid);
    let response = Some(detour(&query));
    let buffer = response.as_ref().unwrap().point_path.points.as_ptr();
    let failure = match other.resume_movement_path(&other_tick, &mut other_token, continuation, response) {
        Err(failure) => failure,
        Ok(_) => panic!("foreign manager cannot resume"),
    };
    assert!(matches!(failure.error, ActorMovementError::OperationMismatch { .. }));
    assert_eq!(failure.response.as_ref().unwrap().point_path.points.as_ptr(), buffer);
    assert_eq!(failure.continuation.phase_shift(), &phase);
    assert!(token.actor_operation.is_some());
    assert!(other_token.actor_operation.is_none());
    assert_eq!(prefix(&other, guid).0, 0);

    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    let failure = match manager.resume_movement_path(&tick, &mut token, failure.continuation, failure.response) {
        Err(failure) => failure,
        Ok(_) => panic!("wrong epoch cannot resume"),
    };
    assert!(matches!(failure.error, ActorMovementError::Tick(ObjectMapTickError::WrongEpoch { .. })));
    assert_eq!(failure.response.as_ref().unwrap().point_path.points.as_ptr(), buffer);
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_some());
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
    assert!(matches!(manager.resume_movement_path(&tick, &mut token, failure.continuation, failure.response),
        Ok(ActorMovementProgress::Complete(_))));
    assert_eq!(prefix(&manager, guid).0, before.0);
    assert_eq!(prefix(&manager, guid).1, before.1);
    assert!(token.actor_operation.is_none());
}

#[test]
fn same_guid_readmission_rejects_old_pending_without_mutating_replacement() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_024, 200);
    let (query, continuation) = pending(&mut manager, &tick, &mut token, guid);
    let old = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap();
    insert_actor(&mut manager, actor(CreatureMovementSource::Home, 450_024), false);
    let before = prefix(&manager, guid);
    let response = Some(detour(&query));
    let buffer = response.as_ref().unwrap().point_path.points.as_ptr();
    let failure = match manager.resume_movement_path(&tick, &mut token, continuation, response) {
        Err(failure) => failure,
        Ok(_) => panic!("same GUID has a different actor witness"),
    };
    assert!(matches!(failure.error, ActorMovementError::WitnessMismatch { .. }));
    assert_eq!(failure.response.as_ref().unwrap().point_path.points.as_ptr(), buffer);
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_some());
    drop(failure);
    drop(old);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn full_snapshot_keeps_witness_and_motor_while_original_pending_resumes() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_025, 200);
    let witness = manager.find_map(1, 0).unwrap().map().creature_actor_witness(guid).unwrap();
    let (query, continuation) = pending(&mut manager, &tick, &mut token, guid);
    let before = prefix(&manager, guid);
    let pointer = stored(&manager, guid) as *const WorldCreature;
    let mut replacement = actor(CreatureMovementSource::Home, 450_025).creature;
    replacement.unit_mut().set_health(17);
    manager.find_map_mut(1, 0).unwrap().map_mut()
        .replace_creature_snapshot(MapObjectRecord::new_creature(replacement).unwrap()).unwrap();
    assert!(witness.same_actor(&manager.find_map(1, 0).unwrap().map().creature_actor_witness(guid).unwrap()));
    assert_eq!(stored(&manager, guid) as *const WorldCreature, pointer);
    assert_eq!(prefix(&manager, guid), before);
    assert!(matches!(manager.resume_movement_path(&tick, &mut token, continuation, Some(detour(&query))),
        Ok(ActorMovementProgress::Complete(_))));
    assert_eq!(stored(&manager, guid).creature.current_health(), 17);
    assert_eq!(prefix(&manager, guid).0, before.0);
    assert!(token.actor_operation.is_none());
}

#[test]
fn stale_map_pending_retains_operation_accounting_and_never_runs_replacement() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_026, 200);
    let (query, continuation) = pending(&mut manager, &tick, &mut token, guid);
    let original = manager.maps.remove(&MapKey::new(1, 0)).unwrap();
    manager.create_world_map(1, 0);
    insert_actor(&mut manager, actor(CreatureMovementSource::Home, 450_026), false);
    let before = prefix(&manager, guid);
    let failure = match manager.resume_movement_path(&tick, &mut token, continuation, Some(detour(&query))) {
        Err(failure) => failure,
        Ok(_) => panic!("map incarnation changed"),
    };
    assert!(matches!(failure.error, ActorMovementError::StaleParticipant { .. }));
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_some());
    drop(failure);
    drop(original);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn pending_blocks_second_prepare_tail_and_drop_does_not_settle_tick() {
    let (mut manager, mut tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_027, 200);
    let (_, continuation) = pending(&mut manager, &tick, &mut token, guid);
    let before = prefix(&manager, guid);
    assert!(matches!(manager.prepare_movement(&tick, &mut token, guid, None, false,
        |_, _| panic!("second prepare cannot rerun policy")),
        Err(ActorMovementError::Tick(ObjectMapTickError::ActorOperationInFlight { .. }))));
    assert_eq!(prefix(&manager, guid), before);
    let token = match manager.try_finish_object_map::<LoadRecord>(&mut tick, token, None, None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime) {
        Err((ObjectMapTickError::ActorOperationInFlight { .. }, token)) => token,
        _ => panic!("tail must return the owned token while an actor is pending"),
    };
    assert!(token.actor_operation.is_some());
    drop(continuation);
    assert!(token.actor_operation.is_some());
    assert!(matches!(manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores),
        Err(ObjectMapTickError::MapInFlight { .. })));
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    assert_eq!(prefix(&manager, guid), before);
    drop(token);
    assert!(matches!(manager.finalize_object_tick(tick), Err(ObjectMapTickError::MapInFlight { .. })));
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn rejected_static_and_grid_height_resumes_return_exact_scalar_responses() {
    for grid in [false, true] {
        let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_028 + i64::from(grid), 200);
        let progress = manager.prepare_movement(&tick, &mut token, guid, None, true, |_, _| true).unwrap();
        let ActorMovementProgress::Pending(ActorMovementPending::StaticHeight(request)) = progress else { panic!("home destination height first"); };
        let (_, continuation) = request.into_parts();
        let response = f32::from_bits(0x7fc0_1234);
        if grid {
            let progress = manager.resume_movement_static_height(&tick, &mut token, continuation, wow_entities::INVALID_HEIGHT)
                .unwrap_or_else(|failure| panic!("{:?}", failure.error));
            let ActorMovementProgress::Pending(ActorMovementPending::GridHeight(request)) = progress else { panic!("fallback height"); };
            let (_, continuation) = request.into_parts();
            manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
            let failure = match manager.resume_movement_grid_height(&tick, &mut token, continuation, response) {
                Err(failure) => failure, Ok(_) => panic!("wrong epoch"),
            };
            assert_eq!(failure.response.to_bits(), response.to_bits());
            assert!(matches!(failure.error, ActorMovementError::Tick(ObjectMapTickError::WrongEpoch { .. })));
        } else {
            manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
            let failure = match manager.resume_movement_static_height(&tick, &mut token, continuation, response) {
                Err(failure) => failure, Ok(_) => panic!("wrong epoch"),
            };
            assert_eq!(failure.response.to_bits(), response.to_bits());
            assert!(matches!(failure.error, ActorMovementError::Tick(ObjectMapTickError::WrongEpoch { .. })));
        }
        assert!(token.actor_operation.is_some());
        assert_eq!(prefix(&manager, guid).0, 200);
        assert_eq!(prefix(&manager, guid).1, 1);
    }
}

#[test]
fn pending_actor_keeps_updater_pending_until_map_finishes_and_waits_only_at_tick_tail() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.updater.activate(1);
    manager.create_world_map(1, 0);
    let incoming = actor(CreatureMovementSource::Home, 450_030);
    let guid = incoming.guid();
    insert_actor(&mut manager, incoming, true);
    let (mut tick, mut token) = begin(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let (query, continuation) = pending(&mut manager, &tick, &mut token, guid);
    assert_eq!((manager.updater.pending_requests, manager.updater.scheduled_updates(), manager.updater.wait_calls()), (1, 1, 0));
    token = match manager.try_finish_object_map::<LoadRecord>(&mut tick, token, None, None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime) {
        Err((ObjectMapTickError::ActorOperationInFlight { .. }, token)) => token,
        _ => panic!("pending operation cannot finish the map"),
    };
    assert_eq!((manager.updater.pending_requests, manager.updater.scheduled_updates(), manager.updater.wait_calls()), (1, 1, 0));
    assert!(matches!(manager.resume_movement_path(&tick, &mut token, continuation, Some(detour(&query))),
        Ok(ActorMovementProgress::Complete(_))));
    assert_eq!(manager.updater.pending_requests, 1, "Actor completion does not settle map accounting");
    manager.try_finish_object_map::<LoadRecord>(&mut tick, token, None, None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime).unwrap_or_else(|(error, _)| panic!("{error:?}"));
    assert_eq!((manager.updater.pending_requests, manager.updater.scheduled_updates(), manager.updater.wait_calls()), (0, 1, 0));
    assert!(manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().is_none());
    manager.finalize_object_tick(tick).unwrap();
    assert_eq!(manager.updater.wait_calls(), 1);
}

#[test]
fn replacing_pending_actor_with_record_rejects_resume_without_changing_record() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_031, 200);
    let (query, continuation) = pending(&mut manager, &tick, &mut token, guid);
    let old = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap();
    let mut record = actor(CreatureMovementSource::Home, 450_031).creature;
    record.unit_mut().set_health(13);
    manager.find_map_mut(1, 0).unwrap().map_mut().insert_map_object_record(MapObjectRecord::new_creature(record).unwrap()).unwrap();
    let failure = match manager.resume_movement_path(&tick, &mut token, continuation, Some(detour(&query))) {
        Err(failure) => failure, Ok(_) => panic!("Record cannot consume an Actor continuation"),
    };
    assert!(matches!(failure.error, ActorMovementError::ActorUnavailable { .. }));
    assert_eq!(manager.find_map(1, 0).unwrap().map().get_typed_creature(guid).unwrap().current_health(), 13);
    assert!(token.actor_operation.is_some());
    drop(failure);
    drop(old);
}
