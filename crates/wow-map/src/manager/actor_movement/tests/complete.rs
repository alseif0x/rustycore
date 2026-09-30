use super::*;

#[test]
fn dead_actor_completes_without_policy_query_or_replaying_clock() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_032, 200);
    manager.find_map_mut(1, 0).unwrap().map_mut().creature_actor_mut(guid).unwrap().creature.mark_ai_dead(0);
    let ActorMovementProgress::Complete(completion) = manager.prepare_movement(&tick, &mut token, guid,
        None, true, |_, _| panic!("dead actor must skip policy")).unwrap() else { panic!("dead actor must skip I/O"); };
    assert!(completion.movement.is_none());
    assert!(completion.home_health_update.is_none());
    assert_eq!(stored(&manager, guid).runtime_elapsed_ms_like_cpp(), 200);
    assert_eq!(stored(&manager, guid).runtime_motion_master_ticks_like_cpp(), 1);
    assert!(token.actor_operation.is_none());
}

#[test]
fn missing_chase_target_returns_owned_stop_and_releases_only_actor_slot() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Chase, 450_033, 200);
    manager.find_map_mut(1, 0).unwrap().map_mut().creature_actor_mut(guid).unwrap()
        .begin_move_spline_like_cpp(Position::xyz(40.0, 10.0, 0.0)).unwrap();
    let ActorMovementProgress::Complete(completion) = manager.prepare_movement(&tick, &mut token, guid,
        None, false, |_, _| panic!("missing target must skip policy")).unwrap() else { panic!("missing target must skip I/O"); };
    assert!(matches!(completion.movement, Some(CreatureMovementStep::Stop(_))));
    assert!(token.actor_operation.is_none());
    assert_eq!(stored(&manager, guid).runtime_elapsed_ms_like_cpp(), 200);
    assert_eq!(stored(&manager, guid).runtime_motion_master_ticks_like_cpp(), 1);
    assert!(manager.begin_tick_like_cpp(1).is_busy(), "Actor completion does not finalize the map/tick");
}
