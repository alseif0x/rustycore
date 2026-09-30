use super::*;

fn assert_same_step(actual: Option<CreatureMovementStep>, expected: Option<CreatureMovementStep>) {
    match (actual, expected) {
        (None, None) => {}
        (Some(CreatureMovementStep::Stop(actual)), Some(CreatureMovementStep::Stop(expected))) => {
            assert_eq!(actual, expected)
        }
        (
            Some(CreatureMovementStep::Launch {
                source,
                from,
                spline,
            }),
            Some(CreatureMovementStep::Launch {
                source: expected_source,
                from: expected_from,
                spline: expected_spline,
            }),
        ) => {
            assert_eq!(source, expected_source);
            assert_eq!(from, expected_from);
            assert_eq!(spline, expected_spline);
        }
        (actual, expected) => panic!("different movement: {actual:?}, {expected:?}"),
    }
}

#[test]
fn stored_actor_four_families_match_sync_and_continue_the_same_rng_stream() {
    for (index, source) in [
        CreatureMovementSource::Home,
        CreatureMovementSource::Random,
        CreatureMovementSource::Waypoint,
        CreatureMovementSource::Chase,
    ]
    .into_iter()
    .enumerate()
    {
        let counter = 450_001 + index as i64;
        let diff = if source == CreatureMovementSource::Waypoint {
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        } else {
            200
        };
        let (mut manager, mut tick, mut token, guid) = fixture(source, counter, diff);
        let mut synchronous = actor(source, counter);
        let phase = synchronous.phase_shift().clone();
        let mut calls = 0;
        let progress = manager
            .prepare_movement(
                &tick,
                &mut token,
                guid,
                Some(target()),
                false,
                |map, ignore| {
                    calls += 1;
                    assert_eq!(map, 1);
                    assert!(!ignore);
                    true
                },
            )
            .unwrap();
        assert_eq!(calls, 1);
        let (query, continuation) = path_request(progress);
        assert_eq!((continuation.map_id(), continuation.instance_id()), (1, 0));
        assert_eq!(continuation.phase_shift(), &phase);
        let pointer = stored(&manager, guid) as *const WorldCreature;
        assert_eq!(
            stored(&manager, guid).runtime_elapsed_ms_like_cpp(),
            u64::from(diff)
        );
        assert_eq!(
            stored(&manager, guid).runtime_motion_master_ticks_like_cpp(),
            1
        );
        let result = manager
            .resume_movement_path(&tick, &mut token, continuation, Some(detour(&query)))
            .unwrap_or_else(|failure| panic!("{:?}", failure.error));
        let ActorMovementProgress::Complete(completion) = result else {
            panic!("terrain disabled");
        };
        let expected = synchronous.step_movement(
            diff,
            Some(target()),
            None,
            |_, _| true,
            |actual, map, instance, actual_phase| {
                assert_eq!((map, instance), (1, 0));
                assert_eq!(actual_phase, &phase);
                assert_eq!(actual, query);
                Some(detour(&actual))
            },
        );
        assert_same_step(completion.movement, expected);
        assert_eq!(
            (completion.guid, completion.key, completion.incarnation),
            (guid, MapKey::new(1, 0), token.incarnation())
        );
        assert_eq!(completion.position, synchronous.position());
        assert_eq!(
            completion.visibility_range,
            synchronous.visibility_range_like_cpp()
        );
        assert_eq!(completion.trace.entry, synchronous.entry());
        assert_eq!(completion.trace.state, synchronous.state());
        assert_eq!(stored(&manager, guid) as *const WorldCreature, pointer);
        assert!(token.actor_operation.is_none());
        assert_eq!(
            stored(&manager, guid).creature.ai_ownership(),
            synchronous.creature.ai_ownership()
        );
        assert_eq!(
            stored(&manager, guid).creature.unit().unit_state(),
            synchronous.creature.unit().unit_state()
        );
        assert_eq!(
            stored(&manager, guid).active_chase_path_poly_refs_like_cpp(),
            synchronous.active_chase_path_poly_refs_like_cpp()
        );
        assert_eq!(
            stored(&manager, guid).active_random_path_poly_refs_like_cpp(),
            synchronous.active_random_path_poly_refs_like_cpp()
        );
        // Observe the next real RNG-dependent destination; do not clone actor
        // or RNG to manufacture a matching post-resume stream.
        for _ in 0..4 {
            let actual = manager
                .find_map_mut(1, 0)
                .unwrap()
                .map_mut()
                .creature_actor_mut(guid)
                .unwrap()
                .pick_random_destination_from_current_position_like_cpp(3.0);
            assert_eq!(
                actual,
                synchronous.pick_random_destination_from_current_position_like_cpp(3.0)
            );
        }
        manager
            .try_finish_object_map::<LoadRecord>(
                &mut tick,
                token,
                None,
                None,
                MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
            )
            .unwrap_or_else(|(error, _)| panic!("{error:?}"));
        assert!(
            manager
                .prepare_next_object_map(
                    &mut tick,
                    MapObjectUpdateSelectionLikeCpp::WholeTypedStores
                )
                .unwrap()
                .is_none()
        );
        manager.finalize_object_tick(tick).unwrap();
        assert_eq!(
            manager.tick_coordination_like_cpp(),
            MapTickCoordinationStateLikeCpp::Idle
        );
    }
}

#[test]
fn stored_actor_terrain_sequence_keeps_source_and_one_prefix_until_completion() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_010, 200);
    let home = stored(&manager, guid).home_position();
    let phase = stored(&manager, guid).phase_shift().clone();
    let mut calls = 0;
    let mut progress = manager
        .prepare_movement(&tick, &mut token, guid, None, true, |_, _| {
            calls += 1;
            true
        })
        .unwrap();
    let mut stages = Vec::new();
    let completion = loop {
        assert_eq!(stored(&manager, guid).runtime_elapsed_ms_like_cpp(), 200);
        assert_eq!(
            stored(&manager, guid).runtime_motion_master_ticks_like_cpp(),
            1
        );
        assert_eq!(calls, 1);
        if !matches!(&progress, ActorMovementProgress::Complete(_)) {
            assert!(token.actor_operation.is_some());
        }
        progress = match progress {
            ActorMovementProgress::Complete(completion) => break completion,
            ActorMovementProgress::Pending(ActorMovementPending::StaticHeight(request)) => {
                let (query, continuation) = request.into_parts();
                stages.push(("static", query.point));
                assert_eq!(query.map_id, 1);
                assert_eq!(
                    query.probe_z,
                    query.point.z + wow_entities::Z_OFFSET_FIND_HEIGHT
                );
                manager
                    .resume_movement_static_height(
                        &tick,
                        &mut token,
                        continuation,
                        wow_entities::INVALID_HEIGHT,
                    )
                    .unwrap_or_else(|failure| panic!("{:?}", failure.error))
            }
            ActorMovementProgress::Pending(ActorMovementPending::GridHeight(request)) => {
                let (query, continuation) = request.into_parts();
                stages.push(("grid", query.point));
                manager
                    .resume_movement_grid_height(
                        &tick,
                        &mut token,
                        continuation,
                        query.point.z + 0.5,
                    )
                    .unwrap_or_else(|failure| panic!("{:?}", failure.error))
            }
            ActorMovementProgress::Pending(ActorMovementPending::Path(request)) => {
                let (query, continuation) = request.into_parts();
                assert_eq!((continuation.map_id(), continuation.instance_id()), (1, 0));
                assert_eq!(continuation.phase_shift(), &phase);
                stages.push(("path", query.destination));
                manager
                    .resume_movement_path(&tick, &mut token, continuation, Some(detour(&query)))
                    .unwrap_or_else(|failure| panic!("{:?}", failure.error))
            }
        };
    };
    assert_eq!(stages.len(), 11);
    assert_eq!(stages[0], ("static", home));
    assert_eq!(stages[1], ("grid", home));
    assert_eq!(stages[2].0, "path");
    assert_eq!(stages[3], ("static", stages[2].1));
    for pair in stages[3..].chunks_exact(2) {
        assert_eq!(pair[0].0, "static");
        assert_eq!(pair[1], ("grid", pair[0].1));
    }
    assert!(matches!(
        completion.movement,
        Some(CreatureMovementStep::Launch {
            source: CreatureMovementSource::Home,
            ..
        })
    ));
    assert!(token.actor_operation.is_none());
}

#[test]
fn terminal_projection_takes_home_health_flag_once_even_without_movement_packet() {
    let (mut manager, tick, mut token, guid) = fixture(CreatureMovementSource::Home, 450_011, 200);
    let ActorMovementProgress::Complete(launch) = manager
        .prepare_movement(&tick, &mut token, guid, None, false, |_, _| false)
        .unwrap()
    else {
        panic!("direct home launches without I/O");
    };
    assert!(matches!(
        launch.movement,
        Some(CreatureMovementStep::Launch { .. })
    ));
    assert!(launch.home_health_update.is_none());
    // The next real prefix finalizes the existing spline and reaches home;
    // no fixture sets the pending flag or fakes a completed generator.
    let ActorMovementProgress::Complete(first) = manager
        .prepare_movement(&tick, &mut token, guid, None, false, |_, _| false)
        .unwrap()
    else {
        panic!("home movement continues without I/O");
    };
    let first = if first.home_health_update.is_some() {
        first
    } else {
        // token diff remains 200; advance whole real operations until the
        // spline reaches its destination, never resample/rewrite token diff.
        let mut completion = first;
        for _ in 0..100 {
            if completion.home_health_update.is_some() {
                break;
            }
            completion = match manager
                .prepare_movement(&tick, &mut token, guid, None, false, |_, _| false)
                .unwrap()
            {
                ActorMovementProgress::Complete(completion) => completion,
                _ => panic!("direct home cannot request I/O"),
            };
        }
        completion
    };
    assert!(first.home_health_update.is_some());
    assert!(token.actor_operation.is_none());
    let ActorMovementProgress::Complete(second) = manager
        .prepare_movement(&tick, &mut token, guid, None, false, |_, _| false)
        .unwrap()
    else {
        panic!("idle completes");
    };
    assert!(second.home_health_update.is_none());
}
