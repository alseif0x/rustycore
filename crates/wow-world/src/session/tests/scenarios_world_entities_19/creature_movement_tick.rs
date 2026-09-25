//! Legacy creature movement tick scenarios.

use super::*;

#[test]
fn legacy_creature_movement_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let guid = test_creature_guid(90_007);
    register_test_creature(&mut session, manager.clone(), guid, 25);

    let outcome = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        None,
        &MMapRuntimeConfigLikeCpp::default(),
        None,
        &HashMap::new(),
        10,
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert_eq!(outcome.creatures_seen, 0);
    assert_eq!(outcome.movement_packets, 0);
    assert!(outcome.plan.events.is_empty());
    let guard = manager.read().unwrap();
    assert_eq!(
        guard
            .find_creature(0, 0, guid)
            .expect("creature remains present")
            .state(),
        wow_entities::CreatureAiState::Idle
    );
}
#[test]
fn legacy_creature_movement_tick_once_moves_once_syncs_canonical_and_plans_fanout_like_cpp() {
    use crate::map_manager::{RecipientRule, RuntimeTickOwner, VISIBILITY_RADIUS};
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let guid = test_creature_guid(90_008);
    register_test_creature(&mut session, manager.clone(), guid, 25);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x9008);
            creature.backdate_runtime_clock_for_test(Duration::from_millis(10));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mmap_config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let outcome = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &mmap_config,
        None,
        &HashMap::new(),
        10,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.movement_packets, 1);
    assert_eq!(outcome.canonical_syncs, 1);
    assert_eq!(outcome.plan.events.len(), 1);
    let event = &outcome.plan.events[0];
    assert_eq!(event.source_guid, guid);
    match &event.recipients {
        RecipientRule::NearbyVisible {
            source_guid,
            map_id,
            instance_id,
            range,
            required_3d,
            ..
        } => {
            assert_eq!(*source_guid, guid);
            assert_eq!(*map_id, 0);
            assert_eq!(*instance_id, 0);
            assert_eq!(*range, VISIBILITY_RADIUS);
            assert!(!required_3d);
        }
        other => panic!("expected NearbyVisible, got {other:?}"),
    }

    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, guid).expect("legacy creature");
        assert_eq!(
            creature.state(),
            wow_entities::CreatureAiState::WalkingRandom
        );
        assert_eq!(
            creature.runtime_motion_master_ticks_like_cpp(),
            1,
            "one global map frame must call MotionMaster::Update once per creature, independent of fanout recipients"
        );
    }
    let legacy_authority = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .creature
        .loot_authority_like_cpp()
        .clone();
    {
        let guard = canonical.lock().unwrap();
        let typed = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(guid, Clone::clone)
            .expect("canonical creature sync must keep typed record fresh");
        assert_eq!(
            typed.ai_state(),
            wow_entities::CreatureAiState::WalkingRandom
        );
        assert!(legacy_authority.shares_storage_like_cpp(typed.loot_authority_like_cpp()));
    }
}
#[test]
fn legacy_creature_movement_tick_once_uses_creature_visibility_override_like_cpp() {
    use crate::map_manager::{RecipientRule, RuntimeTickOwner};
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let guid = test_creature_guid(90_009);
    register_test_creature(&mut session, manager.clone(), guid, 25);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x5757);
            creature
                .creature
                .unit_mut()
                .world_mut()
                .set_visibility_distance_override_like_cpp(
                    wow_entities::VisibilityDistanceTypeLikeCpp::Gigantic,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mmap_config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let outcome = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        None,
        &mmap_config,
        None,
        &HashMap::new(),
        10,
    );

    assert_eq!(outcome.movement_packets, 1);
    let event = &outcome.plan.events[0];
    match &event.recipients {
        RecipientRule::NearbyVisible { range, .. } => assert_eq!(
            *range,
            wow_entities::VisibilityDistanceTypeLikeCpp::Gigantic.distance_like_cpp(),
            "C++ SendMessageToSet uses source GetVisibilityRange(), including creature addon visibility overrides"
        ),
        other => panic!("expected NearbyVisible, got {other:?}"),
    }
}
