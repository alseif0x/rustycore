use super::*;

#[test]
fn missing_and_system_groups_preserve_summary_and_never_load_or_activate() {
    for system in [false, true] {
        let group = group(510, SpawnGroupFlags::SYSTEM);
        let selected = system.then_some(&group);
        let mut old = map(true);
        let mut current = map(true);
        let store = SpawnStore::new();
        let expected = old.original_group(selected, true, true, &store, |_, _, _, _| {
            panic!("blocked group")
        });
        let actual = current.spawn_group_spawn_loaded_grid_records_like_cpp(
            selected,
            true,
            true,
            &store,
            |_, _, _, _| panic!("blocked group"),
        );
        assert_record_summary(actual, expected);
        let owned =
            current.spawn_group_spawn_materialized(selected, true, true, &store, |_, _, _, _| {
                panic!("blocked group")
            });
        assert!(owned.attempts.is_empty());
        assert_eq!(owned.summary.blocked_missing_group, usize::from(!system));
        assert_eq!(owned.summary.blocked_system_group, usize::from(system));
    }
}

#[test]
fn timer_live_difficulty_grid_map_and_area_gates_match_original_without_loading() {
    for gate in 0..6 {
        let group = group(511, SpawnGroupFlags::NONE);
        let mut data = spawn(
            if gate == 5 {
                SpawnObjectType::AreaTrigger
            } else {
                SpawnObjectType::Creature
            },
            111,
        );
        if gate == 2 {
            data.spawn_difficulties = vec![2];
        }
        if gate == 4 {
            data.map_id = 530;
        }
        let store = store(&[group.clone()], vec![(511, data)]);
        let mut old = map(gate != 3);
        let mut current = map(gate != 3);
        for shared in [&mut old, &mut current] {
            if gate == 0 {
                timer(shared, 111);
            }
            if gate == 1 {
                shared.insert_map_object_record(record(111, 571)).unwrap();
            }
        }
        let expected = old.original_group(Some(&group), false, false, &store, |_, _, _, _| {
            panic!("gated")
        });
        let actual = current.spawn_group_spawn_loaded_grid_records_like_cpp(
            Some(&group),
            false,
            false,
            &store,
            |_, _, _, _| panic!("gated"),
        );
        assert_record_summary(actual, expected);
        let owned = current.spawn_group_spawn_materialized(
            Some(&group),
            false,
            false,
            &store,
            |_, _, _, _| panic!("gated"),
        );
        assert!(owned.attempts.is_empty());
        assert!(owned.summary.load_plans.is_empty());
        match gate {
            0 => assert_eq!(owned.summary.skipped_respawn_timer_active, 1),
            1 => assert_eq!(owned.summary.skipped_live_object_active, 1),
            2 => assert_eq!(owned.summary.skipped_difficulty_mismatch, 1),
            3 => assert_eq!(owned.summary.skipped_unloaded_grid, 1),
            4 => assert_eq!(owned.summary.metadata_entries, 0),
            _ => assert_eq!(owned.summary.skipped_no_respawn_map, 1),
        }
    }
}

#[test]
fn force_and_ignore_remove_timer_before_difficulty_gate_and_preserve_loader_force() {
    for (ignore, force) in [(true, false), (false, true)] {
        let group = group(512, SpawnGroupFlags::NONE);
        let mut mismatched = spawn(SpawnObjectType::Creature, 113);
        mismatched.spawn_difficulties = vec![2];
        let store = store(
            &[group.clone()],
            vec![
                (512, spawn(SpawnObjectType::Creature, 112)),
                (512, mismatched),
            ],
        );
        let mut shared = map(true);
        timer(&mut shared, 112);
        timer(&mut shared, 113);
        let mut calls = Vec::new();
        let outcome = shared.spawn_group_spawn_materialized(
            Some(&group),
            ignore,
            force,
            &store,
            |map, kind, id, passed_force| {
                assert_eq!(map.get_respawn_time_like_cpp(kind, id), 0);
                calls.push((id, passed_force));
                Ok(None)
            },
        );
        assert_eq!(calls, vec![(112, force)]);
        assert_eq!(outcome.summary.respawn_timers_removed, 2);
        assert_eq!(outcome.summary.skipped_difficulty_mismatch, 1);
        assert_eq!(
            shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 113),
            0
        );
        assert_eq!(group_plan(&outcome.attempts[0].plan).force, force);
    }
}

#[test]
fn ignore_clears_timer_but_keeps_live_creature_and_gameobject_blocks() {
    for kind in [SpawnObjectType::Creature, SpawnObjectType::GameObject] {
        let group = group(513, SpawnGroupFlags::NONE);
        let store = store(&[group.clone()], vec![(513, spawn(kind, 114))]);
        let mut shared = map(true);
        let incoming = match kind {
            SpawnObjectType::Creature => record(114, 571),
            SpawnObjectType::GameObject => {
                let mut object = wow_entities::GameObject::new();
                object
                    .world_mut()
                    .object_mut()
                    .create(ObjectGuid::create_world_object(
                        HighGuid::GameObject,
                        0,
                        1,
                        571,
                        7,
                        42,
                        114,
                    ));
                object.world_mut().set_map(571, 7).unwrap();
                object.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
                object.set_spawn_id(114);
                MapObjectRecord::new_game_object(object).unwrap()
            }
            SpawnObjectType::AreaTrigger => unreachable!(),
        };
        shared.insert_map_object_record(incoming).unwrap();
        shared.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: kind,
            spawn_id: 114,
            entry: 42,
            respawn_time: 12345,
            grid_id: 0,
        });
        let outcome = shared.spawn_group_spawn_materialized(
            Some(&group),
            true,
            false,
            &store,
            |_, _, _, _| panic!("ignore clears timer but keeps live blocker"),
        );
        assert_eq!(outcome.summary.respawn_timers_removed, 1);
        assert_eq!(outcome.summary.skipped_live_object_active, 1);
        assert_eq!(shared.get_respawn_time_like_cpp(kind, 114), 0);
        assert_eq!(shared.map_object_count(), 1);
        assert!(outcome.attempts.is_empty());
    }
}
