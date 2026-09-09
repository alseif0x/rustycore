//! Pool template and group selection regression scenarios, part 2 of 2.
//!
//! Moved out of the pool.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn pool_mgr_update_pool_child_pool_uses_mother_pool_branch_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(50, PoolTemplateDataLikeCpp::new(1, 571));
    let mut mother_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 50);
    mother_group.add_entry_like_cpp(PoolObjectLikeCpp::new(70, 0.0), 1);
    mother_group.add_entry_like_cpp(PoolObjectLikeCpp::new(71, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 50, mother_group)
        .unwrap();
    mgr.register_child_pool_relation_like_cpp(70, 50).unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    spawns.add_pool_spawn_like_cpp(70, 50);

    let plan = mgr
        .update_pool_plan_like_cpp(
            &mut spawns,
            70,
            SpawnObjectType::Creature,
            999,
            |_, _| 0.0,
            |_candidates, count| {
                assert_eq!(count, 1);
                vec![1]
            },
        )
        .unwrap();

    assert_eq!(plan.kind, PoolMemberKindLikeCpp::Pool);
    assert_eq!(plan.pool_id, 50);
    assert_eq!(plan.trigger_from, 70);
    let object_plan = plan.object_plan.unwrap();
    assert_eq!(object_plan.selected, vec![PoolObjectLikeCpp::new(71, 0.0)]);
    assert_eq!(object_plan.despawned_trigger, Some(70));
    assert!(!spawns.is_spawned_pool_like_cpp(70));
    assert!(spawns.is_spawned_pool_like_cpp(71));
}

#[test]
fn pool_mgr_update_pool_creature_and_gameobject_no_child_dispatch_typed_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    mgr.insert_template_like_cpp(20, PoolTemplateDataLikeCpp::new(1, 571));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        10,
        group_with_one(PoolMemberKindLikeCpp::Creature, 10, 101),
    )
    .unwrap();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::GameObject,
        20,
        group_with_one(PoolMemberKindLikeCpp::GameObject, 20, 201),
    )
    .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 10),
        Ok(())
    );
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::GameObject, 201, 20),
        Ok(())
    );

    let creature_plan = mgr
        .update_pool_plan_like_cpp(
            &mut spawns,
            10,
            SpawnObjectType::Creature,
            101,
            |_, _| 0.0,
            choose_first_indices,
        )
        .unwrap();
    let gameobject_plan = mgr
        .update_pool_plan_like_cpp(
            &mut spawns,
            20,
            SpawnObjectType::GameObject,
            201,
            |_, _| 0.0,
            choose_first_indices,
        )
        .unwrap();

    assert_eq!(creature_plan.kind, PoolMemberKindLikeCpp::Creature);
    assert_eq!(creature_plan.trigger_from, 101);
    assert!(creature_plan.object_plan.unwrap().respawned_trigger);
    assert_eq!(gameobject_plan.kind, PoolMemberKindLikeCpp::GameObject);
    assert_eq!(gameobject_plan.trigger_from, 201);
    assert!(gameobject_plan.object_plan.unwrap().respawned_trigger);
}

#[test]
fn pool_mgr_update_pool_areatrigger_is_unsupported_and_preserves_spawns_like_cpp() {
    let mgr = PoolMgrLikeCpp::new();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 10),
        Ok(())
    );

    let mut roll_calls = Vec::new();
    let result = mgr.update_pool_plan_like_cpp(
        &mut spawns,
        10,
        SpawnObjectType::AreaTrigger,
        301,
        |kind, pool_id| {
            roll_calls.push((kind, pool_id));
            0.0
        },
        choose_first_indices,
    );

    assert_eq!(
        result,
        Err(PoolMgrPlanErrorLikeCpp::UnsupportedSpawnType {
            spawn_type: SpawnObjectType::AreaTrigger,
        })
    );
    assert!(spawns.is_spawned_creature_like_cpp(101));
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 1);
    assert_eq!(
        mgr.is_part_of_a_pool_like_cpp(SpawnObjectType::AreaTrigger, 301),
        Ok(0)
    );
    assert!(roll_calls.is_empty());
}

#[test]
fn pool_mgr_update_pool_mother_branch_requests_lazy_pool_roll_only_when_explicit_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(50, PoolTemplateDataLikeCpp::new(1, 571));
    let mut mother_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 50);
    mother_group.add_entry_like_cpp(PoolObjectLikeCpp::new(70, 50.0), 1);
    mother_group.add_entry_like_cpp(PoolObjectLikeCpp::new(71, 50.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 50, mother_group)
        .unwrap();
    mgr.register_child_pool_relation_like_cpp(70, 50).unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    spawns.add_pool_spawn_like_cpp(70, 50);
    let mut roll_calls = Vec::new();

    let plan = mgr
        .update_pool_plan_like_cpp(
            &mut spawns,
            70,
            SpawnObjectType::Creature,
            999,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                60.0
            },
            choose_first_indices,
        )
        .unwrap();

    assert_eq!(roll_calls, vec![(PoolMemberKindLikeCpp::Pool, 50)]);
    assert_eq!(plan.kind, PoolMemberKindLikeCpp::Pool);
    assert_eq!(
        plan.object_plan.unwrap().selected,
        vec![PoolObjectLikeCpp::new(71, 50.0)]
    );
    assert!(!spawns.is_spawned_pool_like_cpp(70));
    assert!(spawns.is_spawned_pool_like_cpp(71));
}

#[test]
fn pool_mgr_update_pool_typed_branch_requests_lazy_roll_only_when_explicit_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 50.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(102, 50.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, group)
        .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 10),
        Ok(())
    );
    let mut roll_calls = Vec::new();

    let plan = mgr
        .update_pool_plan_like_cpp(
            &mut spawns,
            10,
            SpawnObjectType::Creature,
            101,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                60.0
            },
            choose_first_indices,
        )
        .unwrap();

    assert_eq!(roll_calls, vec![(PoolMemberKindLikeCpp::Creature, 10)]);
    assert_eq!(plan.kind, PoolMemberKindLikeCpp::Creature);
    assert_eq!(
        plan.object_plan.unwrap().selected,
        vec![PoolObjectLikeCpp::new(102, 50.0)]
    );
    assert!(!spawns.is_spawned_creature_like_cpp(101));
    assert!(spawns.is_spawned_creature_like_cpp(102));
}

#[test]
fn pool_mgr_is_empty_and_check_pool_preserve_cpp_order_and_result() {
    let mut mgr = PoolMgrLikeCpp::new();
    let mut invalid_gameobject =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10);
    invalid_gameobject.add_entry_like_cpp(PoolObjectLikeCpp::new(201, 60.0), 1);
    let mut invalid_creature = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    invalid_creature.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 50.0), 1);
    invalid_creature.add_entry_like_cpp(PoolObjectLikeCpp::new(102, 50.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 10, invalid_gameobject)
        .unwrap();
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, invalid_creature)
        .unwrap();

    assert!(!mgr.is_empty_like_cpp(10));
    assert!(!mgr.check_pool_like_cpp(10));

    let mut parent = PoolMgrLikeCpp::new();
    let mut parent_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 40);
    parent_group.add_entry_like_cpp(PoolObjectLikeCpp::new(41, 0.0), 1);
    parent
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 40, parent_group)
        .unwrap();
    assert!(parent.is_empty_like_cpp(40));
    parent
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::Creature,
            41,
            group_with_one(PoolMemberKindLikeCpp::Creature, 41, 411),
        )
        .unwrap();
    assert!(!parent.is_empty_like_cpp(40));

    let mut cyclic = PoolMgrLikeCpp::new();
    cyclic
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::Pool,
            1,
            group_with_one(PoolMemberKindLikeCpp::Pool, 1, 1),
        )
        .unwrap();
    assert!(!cyclic.is_empty_like_cpp(1));
}

#[test]
fn pool_mgr_builders_validate_group_kind_and_child_pool_overflow_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    assert_eq!(
        mgr.insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::Creature,
            10,
            PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10),
        ),
        Err(PoolMgrPlanErrorLikeCpp::WrongGroupKind {
            expected: PoolMemberKindLikeCpp::Creature,
            actual: PoolMemberKindLikeCpp::GameObject,
        })
    );
    assert_eq!(
        mgr.register_child_pool_relation_like_cpp(u64::from(u32::MAX) + 1, 10),
        Err(PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
            child_pool_id: u64::from(u32::MAX) + 1,
        })
    );
}

#[test]
fn pool_mgr_despawn_pool_order_and_bucket_order_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    let mut creatures = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    creatures.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 2);
    creatures.add_entry_like_cpp(PoolObjectLikeCpp::new(102, 100.0), 1);
    let mut gameobjects = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10);
    gameobjects.add_entry_like_cpp(PoolObjectLikeCpp::new(201, 0.0), 2);
    gameobjects.add_entry_like_cpp(PoolObjectLikeCpp::new(202, 100.0), 1);
    let mut pools = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 10);
    pools.add_entry_like_cpp(PoolObjectLikeCpp::new(20, 0.0), 2);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, creatures)
        .unwrap();
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 10, gameobjects)
        .unwrap();
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 10, pools)
        .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    spawns
        .add_spawn_like_cpp(SpawnObjectType::Creature, 101, 10)
        .unwrap();
    spawns
        .add_spawn_like_cpp(SpawnObjectType::Creature, 102, 10)
        .unwrap();
    spawns
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 201, 10)
        .unwrap();
    spawns
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 202, 10)
        .unwrap();
    spawns.add_pool_spawn_like_cpp(20, 10);

    let plan = mgr
        .despawn_pool_plan_like_cpp(&mut spawns, 10, false)
        .unwrap();

    assert_eq!(
        plan.subplans
            .iter()
            .map(|subplan| subplan.kind)
            .collect::<Vec<_>>(),
        vec![
            PoolMemberKindLikeCpp::Creature,
            PoolMemberKindLikeCpp::GameObject,
            PoolMemberKindLikeCpp::Pool,
        ]
    );
    assert_eq!(
        plan.subplans[0].object_plan.as_ref().unwrap().despawned,
        vec![101, 102]
    );
    assert_eq!(
        plan.subplans[1].object_plan.as_ref().unwrap().despawned,
        vec![201, 202]
    );
    assert_eq!(
        plan.subplans[2].object_plan.as_ref().unwrap().despawned,
        vec![20]
    );
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 0);
}

#[test]
fn pool_mgr_despawn_mutates_parent_and_nested_child_pool_state_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    let mut parent_pools = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 10);
    parent_pools.add_entry_like_cpp(PoolObjectLikeCpp::new(20, 0.0), 1);
    let mut child_creatures = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 20);
    child_creatures.add_entry_like_cpp(PoolObjectLikeCpp::new(2001, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 10, parent_pools)
        .unwrap();
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 20, child_creatures)
        .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    spawns.add_pool_spawn_like_cpp(20, 10);
    spawns
        .add_spawn_like_cpp(SpawnObjectType::Creature, 2001, 20)
        .unwrap();

    let plan = mgr
        .despawn_pool_plan_like_cpp(&mut spawns, 10, false)
        .unwrap();

    let pool_object_plan = plan.subplans[2].object_plan.as_ref().unwrap();
    assert_eq!(pool_object_plan.despawned, vec![20]);
    assert_eq!(pool_object_plan.child_pool_plans.len(), 1);
    assert_eq!(
        pool_object_plan.child_pool_plans[0].subplans[0]
            .object_plan
            .as_ref()
            .unwrap()
            .despawned,
        vec![2001]
    );
    assert!(!spawns.is_spawned_pool_like_cpp(20));
    assert!(!spawns.is_spawned_creature_like_cpp(2001));
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 0);
    assert_eq!(spawns.get_spawned_objects_like_cpp(20), 0);
}

#[test]
fn pool_mgr_despawn_missing_and_empty_groups_skip_without_template_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::GameObject,
        10,
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10),
    )
    .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();

    let plan = mgr
        .despawn_pool_plan_like_cpp(&mut spawns, 10, true)
        .unwrap();

    assert_eq!(
        plan.subplans[0].skip_reason,
        Some(PoolMgrPlanSkipReasonLikeCpp::MissingGroup)
    );
    assert_eq!(
        plan.subplans[1].skip_reason,
        Some(PoolMgrPlanSkipReasonLikeCpp::EmptyGroup)
    );
    assert_eq!(
        plan.subplans[2].skip_reason,
        Some(PoolMgrPlanSkipReasonLikeCpp::MissingGroup)
    );
}

#[test]
fn pool_group_despawn_delete_respawn_time_only_for_unspawned_creature_and_go_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    spawns
        .add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7)
        .unwrap();
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(102, 0.0), 1);

    let plan = group
        .despawn_object_plan_like_cpp(&mut spawns, 999, true, |_, _, _| unreachable!())
        .unwrap();

    assert!(spawns.is_spawned_creature_like_cpp(101));
    assert_eq!(plan.despawned, Vec::<u64>::new());
    assert_eq!(
        plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
            kind: PoolMemberKindLikeCpp::Creature,
            guid: 102,
        }]
    );
    assert_eq!(
        plan.removed_respawn_times,
        vec![(PoolMemberKindLikeCpp::Creature, 102)]
    );

    let mut pool_spawns = SpawnedPoolDataLikeCpp::new();
    let pool_group = group_with_one(PoolMemberKindLikeCpp::Pool, 7, 20);
    let pool_plan = pool_group
        .despawn_object_plan_like_cpp(&mut pool_spawns, 0, true, |_, _, _| unreachable!())
        .unwrap();
    assert!(pool_plan.actions.is_empty());
    assert!(pool_plan.removed_respawn_times.is_empty());
}

#[test]
fn pool_mgr_despawn_child_pool_overflow_and_cycle_are_typed_errors_like_cpp() {
    let mut overflow_mgr = PoolMgrLikeCpp::new();
    let mut overflow_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 10);
    let overflowing_child = u64::from(u32::MAX) + 1;
    overflow_group.add_entry_like_cpp(PoolObjectLikeCpp::new(overflowing_child, 0.0), 1);
    overflow_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 10, overflow_group)
        .unwrap();
    let mut overflow_spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        overflow_mgr.despawn_pool_plan_like_cpp(&mut overflow_spawns, 10, false),
        Err(PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
            child_pool_id: overflowing_child,
        })
    );

    let mut cyclic = PoolMgrLikeCpp::new();
    cyclic
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::Pool,
            1,
            group_with_one(PoolMemberKindLikeCpp::Pool, 1, 1),
        )
        .unwrap();
    let mut cyclic_spawns = SpawnedPoolDataLikeCpp::new();
    cyclic_spawns.add_pool_spawn_like_cpp(1, 1);
    assert_eq!(
        cyclic.despawn_pool_plan_like_cpp(&mut cyclic_spawns, 1, false),
        Err(PoolMgrPlanErrorLikeCpp::ChildPoolCycle { pool_id: 1 })
    );
}

#[test]
fn pool_mgr_autospawn_tracks_top_level_non_child_only_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 530));
    mgr.insert_template_like_cpp(20, PoolTemplateDataLikeCpp::new(1, 530));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        10,
        group_with_one(PoolMemberKindLikeCpp::Creature, 10, 1001),
    )
    .unwrap();
    let mut parent_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 20);
    parent_group.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 20, parent_group)
        .unwrap();
    mgr.register_child_pool_relation_like_cpp(10, 20).unwrap();

    assert_eq!(mgr.top_level_auto_spawn_candidate_like_cpp(10), None);
    assert_eq!(mgr.top_level_auto_spawn_candidate_like_cpp(20), Some(530));
    mgr.add_auto_spawn_pool_like_cpp(530, 20);
    assert_eq!(mgr.auto_spawn_pools_for_map_like_cpp(530), &[20]);
    assert!(mgr.auto_spawn_pools_for_map_like_cpp(-1).is_empty());
}
