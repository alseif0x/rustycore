use super::*;

type NoRespawnLoadRecordLikeCpp =
    fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn finish_without_load_record(
    manager: &mut MapManager,
    tick: &mut MapObjectTickContinuation,
    token: ObjectMapUpdateToken,
    creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
) -> Result<ObjectMapFinishOutcome, ObjectMapTickError> {
    manager.finish_object_map::<NoRespawnLoadRecordLikeCpp>(
        tick,
        token,
        None,
        None,
        creature_update_owner,
    )
}

#[test]
fn foreign_plan_origin_is_rejected_by_resume_gate_begin_and_abandon() {
    let mut plan_owner = MapManager::new(MIN_GRID_DELAY_MS, 200);
    let mut plan_recipient = MapManager::new(MIN_GRID_DELAY_MS, 200);
    plan_owner.create_world_map(1, 0);
    plan_recipient.create_world_map(1, 0);
    let foreign_plan = plan_owner
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let recipient_plan = plan_recipient
        .begin_tick_like_cpp(300)
        .into_started()
        .expect("timer passed");

    assert_eq!(
        foreign_plan.epoch_like_cpp(),
        recipient_plan.epoch_like_cpp()
    );
    assert_eq!(
        foreign_plan.updated_maps_like_cpp(),
        recipient_plan.updated_maps_like_cpp()
    );
    assert_eq!(foreign_plan.effective_diff_ms(), 200);
    assert_eq!(recipient_plan.effective_diff_ms(), 300);
    assert!(!plan_recipient.can_resume_tick(&foreign_plan));
    assert!(matches!(
        plan_recipient.begin_object_tick(foreign_plan),
        Err(ObjectMapTickError::OriginMismatch { plan_epoch: 1 })
    ));
    assert_eq!(
        plan_recipient.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(1)
    );
    assert_eq!(plan_recipient.timer.current(), 300);
    assert!(plan_recipient.begin_tick_like_cpp(1).is_busy());

    let mut abandon_owner = MapManager::new(MIN_GRID_DELAY_MS, 200);
    let mut abandon_recipient = MapManager::new(MIN_GRID_DELAY_MS, 200);
    abandon_owner.create_world_map(1, 0);
    abandon_recipient.create_world_map(1, 0);
    let foreign_plan = abandon_owner
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let recipient_plan = abandon_recipient
        .begin_tick_like_cpp(300)
        .into_started()
        .expect("timer passed");
    assert!(!abandon_recipient.can_resume_tick(&foreign_plan));
    assert_eq!(
        abandon_recipient.abandon_tick_like_cpp(foreign_plan),
        MapTickResumeLikeCpp::Rejected {
            state: MapTickCoordinationStateLikeCpp::AwaitingSessions(
                recipient_plan.epoch_like_cpp(),
            ),
            plan_epoch: recipient_plan.epoch_like_cpp(),
        }
    );
    assert_eq!(
        abandon_recipient.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(1)
    );
    assert_eq!(abandon_recipient.timer.current(), 300);
    assert!(abandon_recipient.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn foreign_continuation_and_token_cannot_cross_managers() {
    let mut first = MapManager::new(MIN_GRID_DELAY_MS, 200);
    let mut second = MapManager::new(MIN_GRID_DELAY_MS, 200);
    first.create_world_map(1, 0);
    second.create_world_map(1, 0);
    insert_dynamic_object_for_update(&mut first, 9980201, 10_000, true);
    insert_dynamic_object_for_update(&mut second, 9980201, 10_000, true);
    insert_creature_for_update(&mut first, 9980202, true);
    insert_creature_for_update(&mut second, 9980202, true);

    let first_plan = first
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let second_plan = second
        .begin_tick_like_cpp(300)
        .into_started()
        .expect("timer passed");
    assert_eq!(first_plan.epoch_like_cpp(), second_plan.epoch_like_cpp());
    assert_eq!(
        first_plan.updated_maps_like_cpp(),
        second_plan.updated_maps_like_cpp()
    );
    assert_eq!(first_plan.effective_diff_ms(), 200);
    assert_eq!(second_plan.effective_diff_ms(), 300);

    let mut first_tick = first.begin_object_tick(first_plan).unwrap();
    let mut second_tick = second.begin_object_tick(second_plan).unwrap();
    assert!(matches!(
        second.prepare_next_object_map(
            &mut first_tick,
            MapObjectUpdateSelectionLikeCpp::NearbyCells,
        ),
        Err(ObjectMapTickError::OriginMismatch { plan_epoch: 1 })
    ));
    assert_eq!(
        second
            .find_map(1, 0)
            .unwrap()
            .last_dynamic_objects_update_summary()
            .visited,
        0
    );

    let first_token = first
        .prepare_next_object_map(
            &mut first_tick,
            MapObjectUpdateSelectionLikeCpp::NearbyCells,
        )
        .unwrap()
        .unwrap();
    let second_token = second
        .prepare_next_object_map(
            &mut second_tick,
            MapObjectUpdateSelectionLikeCpp::NearbyCells,
        )
        .unwrap()
        .unwrap();
    assert_eq!(first_token.key(), second_token.key());
    assert_eq!(first_token.incarnation(), second_token.incarnation());
    assert_eq!(first_token.effective_diff_ms(), 200);
    assert_eq!(second_token.effective_diff_ms(), 300);

    assert_eq!(
        finish_without_load_record(
            &mut second,
            &mut second_tick,
            first_token,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        ),
        Err(ObjectMapTickError::OriginMismatch { plan_epoch: 1 })
    );
    let second_map = second.find_map(1, 0).unwrap();
    assert_eq!(second_map.last_creatures_update_summary().visited, 0);
    assert!(
        !second_map
            .last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
    assert!(second_map.delayed_update_calls().is_empty());
    assert_eq!(
        second
            .find_map(1, 0)
            .unwrap()
            .last_dynamic_objects_update_summary()
            .visited,
        1
    );

    assert_eq!(
        finish_without_load_record(
            &mut second,
            &mut second_tick,
            second_token,
            MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
        ),
        Ok(ObjectMapFinishOutcome::Completed)
    );
    second.finalize_object_tick(second_tick).unwrap();
    assert_eq!(
        first.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(1)
    );
    assert!(first.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn staged_map_inflight_blocks_second_prepare_and_manager_timer_advance() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let plan = manager
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let epoch = plan.epoch_like_cpp();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager
        .prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores)
        .unwrap()
        .unwrap();

    assert_eq!(token.key(), MapKey::new(1, 0));
    assert_eq!(token.effective_diff_ms(), 200);
    assert!(matches!(
        manager.prepare_next_object_map(
            &mut tick,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
        ),
        Err(ObjectMapTickError::MapInFlight { participant })
            if participant.key == MapKey::new(1, 0)
                && participant.incarnation == token.incarnation()
    ));

    assert!(manager.begin_tick_like_cpp(17).is_busy());
    assert_eq!(manager.timer.current(), 200);
    assert_eq!(
        finish_without_load_record(
            &mut manager,
            &mut tick,
            token,
            MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
        ),
        Ok(ObjectMapFinishOutcome::Completed)
    );
    assert!(matches!(
        manager
            .prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores,),
        Ok(None)
    ));
    manager.finalize_object_tick(tick).unwrap();

    let plan = manager
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed after finalization");
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager
        .prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores)
        .unwrap()
        .unwrap();
    assert!(matches!(
        manager.finalize_object_tick(tick),
        Err(ObjectMapTickError::MapInFlight { .. })
    ));
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch + 1)
    );
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    drop(token);
}

#[test]
fn staged_tick_rejects_finalize_before_unstarted_participants_are_processed() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let plan = manager
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let epoch = plan.epoch_like_cpp();
    let tick = manager.begin_object_tick(plan).unwrap();

    assert_eq!(
        manager.finalize_object_tick(tick),
        Err(ObjectMapTickError::Incomplete {
            processed_participants: 0,
            total_participants: 2,
        })
    );
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)
    );
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn stale_map_token_does_not_run_creature_or_tail_on_same_key_replacement() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    insert_creature_for_update(&mut manager, 9980101, true);
    manager.updater.activate(1);

    let plan = manager
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager
        .prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores)
        .unwrap()
        .unwrap();
    let admitted_incarnation = token.incarnation();

    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    insert_creature_for_update(&mut manager, 9980102, true);
    let replacement_incarnation = manager.map_incarnation_like_cpp(MapKey::new(1, 0)).unwrap();
    assert_ne!(replacement_incarnation, admitted_incarnation);

    assert_eq!(
        finish_without_load_record(
            &mut manager,
            &mut tick,
            token,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        ),
        Ok(ObjectMapFinishOutcome::StaleParticipant {
            key: MapKey::new(1, 0),
            admitted_incarnation,
            current_incarnation: Some(replacement_incarnation),
        })
    );
    let replacement = manager.find_map(1, 0).unwrap();
    assert_eq!(replacement.last_creatures_update_summary().visited, 0);
    assert!(
        !replacement
            .last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
    assert_eq!(manager.updater.pending_requests, 0);
    assert_eq!(manager.updater.scheduled_updates, 1);

    manager.finalize_object_tick(tick).unwrap();
    let replacement = manager.find_map(1, 0).unwrap();
    assert_eq!(replacement.delayed_update_calls(), [200]);
    assert_eq!(manager.updater.wait_calls(), 1);
}

#[test]
fn staged_maps_keep_order_and_diff_and_finalize_one_wait_then_all_delayed_updates() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(2, 0);
    manager.create_world_map(1, 0);
    manager.updater.activate(1);

    let plan = manager
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let mut tick = manager.begin_object_tick(plan).unwrap();

    let first = manager
        .prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells)
        .unwrap()
        .unwrap();
    assert_eq!(first.key(), MapKey::new(1, 0));
    assert_eq!(
        first.incarnation(),
        manager.map_incarnation_like_cpp(first.key()).unwrap()
    );
    assert_eq!(first.effective_diff_ms(), 200);
    assert_eq!(
        finish_without_load_record(
            &mut manager,
            &mut tick,
            first,
            MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
        ),
        Ok(ObjectMapFinishOutcome::Completed)
    );

    let second = manager
        .prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells)
        .unwrap()
        .unwrap();
    assert_eq!(second.key(), MapKey::new(2, 0));
    assert_eq!(second.effective_diff_ms(), 200);
    assert_eq!(
        finish_without_load_record(
            &mut manager,
            &mut tick,
            second,
            MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
        ),
        Ok(ObjectMapFinishOutcome::Completed)
    );
    assert!(matches!(
        manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells),
        Ok(None)
    ));

    manager.finalize_object_tick(tick).unwrap();
    assert_eq!(manager.updater.scheduled_updates(), 2);
    assert_eq!(manager.updater.pending_requests, 0);
    assert_eq!(manager.updater.wait_calls(), 1);
    for key in [MapKey::new(1, 0), MapKey::new(2, 0)] {
        let map = manager.find_map(key.map_id, key.instance_id).unwrap();
        assert_eq!(map.delayed_update_calls(), [200]);
        assert_eq!(
            map.last_map_update_tail_summary_like_cpp()
                .script_hook
                .diff_ms,
            200
        );
    }
}
