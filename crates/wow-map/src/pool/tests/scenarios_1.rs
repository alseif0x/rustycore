//! Pool template and group selection regression scenarios, part 1 of 2.
//!
//! Moved out of the pool.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn pool_mgr_spawn_one_pool_recurses_with_real_child_plan_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 1));
    pool_mgr.insert_template_like_cpp(20, PoolTemplateDataLikeCpp::new(1, 1));

    let mut parent_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 10);
    parent_group.add_entry_like_cpp(PoolObjectLikeCpp::new(20, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 10, parent_group)
        .expect("test parent pool group");

    let mut child_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 20);
    child_group.add_entry_like_cpp(PoolObjectLikeCpp::new(200, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 20, child_group)
        .expect("test child creature group");

    let plan = pool_mgr
        .spawn_pool_plan_like_cpp(&mut spawns, 10, |_, _| 0.0, choose_first_indices)
        .expect("recursive pool spawn plan");

    let parent_pool_plan = plan
        .subplans
        .iter()
        .find(|subplan| subplan.kind == PoolMemberKindLikeCpp::Pool && subplan.pool_id == 10)
        .expect("parent pool typed subplan");
    let object_plan = parent_pool_plan
        .object_plan
        .as_ref()
        .expect("parent pool object plan");
    assert_eq!(
        object_plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 20,
        }]
    );
    assert_eq!(object_plan.child_pool_spawn_plans.len(), 1);
    assert!(
        object_plan.child_pool_spawn_plans[0]
            .subplans
            .iter()
            .any(|subplan| subplan.kind == PoolMemberKindLikeCpp::Creature
                && subplan.pool_id == 20
                && subplan
                    .object_plan
                    .as_ref()
                    .is_some_and(|child_object_plan| child_object_plan.actions
                        == vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
                            kind: PoolMemberKindLikeCpp::Creature,
                            guid: 200,
                        }]))
    );
    assert!(spawns.is_spawned_pool_like_cpp(20));
    assert!(spawns.is_spawned_creature_like_cpp(200));
}

#[test]
fn pool_group_spawn_object_explicit_roll_spawns_first_eligible_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 40.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(102, 60.0), 1);

    let plan = group.spawn_object_plan_like_cpp(&mut spawns, 1, 0, || 30.0, choose_first_indices);

    assert_eq!(plan.selected, vec![PoolObjectLikeCpp::new(101, 40.0)]);
    assert_eq!(
        plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::Creature,
            guid: 101,
        }]
    );
    assert!(spawns.is_spawned_creature_like_cpp(101));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);
}

#[test]
fn pool_group_spawn_object_explicit_spawned_miss_falls_back_to_equal_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::GameObject, 201, 7),
        Ok(())
    );
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(201, 100.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(202, 0.0), 1);

    let plan = group.spawn_object_plan_like_cpp(&mut spawns, 2, 0, || 50.0, choose_first_indices);

    assert_eq!(plan.selected, vec![PoolObjectLikeCpp::new(202, 0.0)]);
    assert_eq!(
        plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::GameObject,
            guid: 202,
        }]
    );
    assert!(spawns.is_spawned_gameobject_like_cpp(201));
    assert!(spawns.is_spawned_gameobject_like_cpp(202));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 2);
}

#[test]
fn pool_group_spawn_object_explicit_roll_miss_falls_back_to_equal_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(211, 40.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(212, 0.0), 1);

    let plan = group.spawn_object_plan_like_cpp(
        &mut spawns,
        1,
        0,
        || 80.0,
        |candidates, count| {
            assert_eq!(candidates, &[PoolObjectLikeCpp::new(212, 0.0)]);
            assert_eq!(count, 1);
            vec![0]
        },
    );

    assert_eq!(plan.selected, vec![PoolObjectLikeCpp::new(212, 0.0)]);
    assert_eq!(
        plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::GameObject,
            guid: 212,
        }]
    );
    assert!(!spawns.is_spawned_gameobject_like_cpp(211));
    assert!(spawns.is_spawned_gameobject_like_cpp(212));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);
}

#[test]
fn pool_group_spawn_object_equal_candidates_and_deterministic_selection_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 302, 7),
        Ok(())
    );
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(301, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(302, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(303, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(304, 0.0), 1);

    let mut observed_candidates = Vec::new();
    let plan = group.spawn_object_plan_like_cpp(
        &mut spawns,
        3,
        0,
        || 0.0,
        |candidates, count| {
            observed_candidates = candidates.to_vec();
            assert_eq!(count, 2);
            vec![1, 99, 1, 0]
        },
    );

    assert_eq!(
        observed_candidates,
        vec![
            PoolObjectLikeCpp::new(301, 0.0),
            PoolObjectLikeCpp::new(303, 0.0),
            PoolObjectLikeCpp::new(304, 0.0),
        ]
    );
    assert_eq!(
        plan.selected,
        vec![
            PoolObjectLikeCpp::new(303, 0.0),
            PoolObjectLikeCpp::new(301, 0.0),
        ]
    );
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 3);
    assert!(spawns.is_spawned_creature_like_cpp(301));
    assert!(spawns.is_spawned_creature_like_cpp(303));
    assert!(!spawns.is_spawned_creature_like_cpp(304));
}

#[test]
fn pool_group_spawn_object_trigger_selected_respawns_without_counter_change_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 401, 7),
        Ok(())
    );
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(401, 100.0), 1);

    let plan = group.spawn_object_plan_like_cpp(&mut spawns, 1, 401, || 50.0, choose_first_indices);

    assert_eq!(plan.selected, vec![PoolObjectLikeCpp::new(401, 100.0)]);
    assert!(plan.respawned_trigger);
    assert_eq!(plan.despawned_trigger, None);
    assert_eq!(
        plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::RespawnOne {
            kind: PoolMemberKindLikeCpp::Creature,
            guid: 401,
        }]
    );
    assert!(spawns.is_spawned_creature_like_cpp(401));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);
}

#[test]
fn pool_group_spawn_object_unselected_trigger_despawns_and_new_spawn_keeps_net_counter_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::GameObject, 501, 7),
        Ok(())
    );
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(501, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(502, 0.0), 1);

    let plan = group.spawn_object_plan_like_cpp(
        &mut spawns,
        1,
        501,
        || 0.0,
        |_candidates, count| {
            assert_eq!(count, 1);
            vec![1]
        },
    );

    assert_eq!(plan.selected, vec![PoolObjectLikeCpp::new(502, 0.0)]);
    assert_eq!(plan.despawned_trigger, Some(501));
    assert_eq!(
        plan.actions,
        vec![
            PoolSpawnObjectActionLikeCpp::SpawnOne {
                kind: PoolMemberKindLikeCpp::GameObject,
                guid: 502,
            },
            PoolSpawnObjectActionLikeCpp::DespawnOne {
                kind: PoolMemberKindLikeCpp::GameObject,
                guid: 501,
            },
        ]
    );
    assert!(!spawns.is_spawned_gameobject_like_cpp(501));
    assert!(spawns.is_spawned_gameobject_like_cpp(502));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);
}

#[test]
fn pool_group_spawn_object_count_non_positive_still_despawns_trigger_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 601, 7),
        Ok(())
    );
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 602, 7),
        Ok(())
    );
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 603, 7),
        Ok(())
    );
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(601, 0.0), 1);

    let plan = group.spawn_object_plan_like_cpp(
        &mut spawns,
        1,
        601,
        || 0.0,
        |_candidates, _count| vec![0],
    );

    assert!(plan.selected.is_empty());
    assert_eq!(plan.despawned_trigger, Some(601));
    assert_eq!(
        plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::DespawnOne {
            kind: PoolMemberKindLikeCpp::Creature,
            guid: 601,
        }]
    );
    assert!(!spawns.is_spawned_creature_like_cpp(601));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 2);
}

#[test]
fn pool_group_spawn_object_pool_kind_subpool_state_and_respawn_noop_like_cpp() {
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 7);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(701, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(702, 0.0), 1);

    let spawn_plan =
        group.spawn_object_plan_like_cpp(&mut spawns, 1, 0, || 0.0, |_candidates, _count| vec![0]);
    assert_eq!(
        spawn_plan.actions,
        vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 701,
        }]
    );
    assert!(spawns.is_spawned_pool_like_cpp(701));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);

    let respawn_plan = group.spawn_object_plan_like_cpp(
        &mut spawns,
        1,
        701,
        || 0.0,
        |_candidates, _count| vec![0],
    );
    assert!(respawn_plan.respawned_trigger);
    assert!(respawn_plan.actions.is_empty());
    assert!(spawns.is_spawned_pool_like_cpp(701));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);

    let despawn_plan = group.spawn_object_plan_like_cpp(
        &mut spawns,
        1,
        701,
        || 0.0,
        |_candidates, _count| vec![1],
    );
    assert_eq!(despawn_plan.despawned_trigger, Some(701));
    assert_eq!(
        despawn_plan.actions,
        vec![
            PoolSpawnObjectActionLikeCpp::SpawnOne {
                kind: PoolMemberKindLikeCpp::Pool,
                guid: 702,
            },
            PoolSpawnObjectActionLikeCpp::DespawnOne {
                kind: PoolMemberKindLikeCpp::Pool,
                guid: 701,
            },
        ]
    );
    assert!(!spawns.is_spawned_pool_like_cpp(701));
    assert!(spawns.is_spawned_pool_like_cpp(702));
    assert_eq!(spawns.get_spawned_objects_like_cpp(7), 1);
}

#[test]
fn pool_group_add_entry_buckets_match_cpp() {
    let mut group = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::Creature);

    group.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 25.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(2, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(3, 25.0), 2);

    assert_eq!(
        group.explicitly_chanced_like_cpp(),
        &[PoolObjectLikeCpp::new(1, 25.0)]
    );
    assert_eq!(
        group.equal_chanced_like_cpp(),
        &[
            PoolObjectLikeCpp::new(2, 0.0),
            PoolObjectLikeCpp::new(3, 25.0),
        ]
    );
}

#[test]
fn pool_group_check_pool_matches_cpp() {
    let mut valid_explicit = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::GameObject);
    valid_explicit.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 60.0), 1);
    valid_explicit.add_entry_like_cpp(PoolObjectLikeCpp::new(2, 40.0), 1);
    assert!(valid_explicit.check_pool_like_cpp());

    let mut invalid_explicit = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::GameObject);
    invalid_explicit.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 60.0), 1);
    assert!(!invalid_explicit.check_pool_like_cpp());

    let mut zero_total = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::GameObject);
    zero_total.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 0.0), 1);
    assert!(zero_total.check_pool_like_cpp());

    let mut equal_present = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::GameObject);
    equal_present.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 60.0), 1);
    equal_present.add_entry_like_cpp(PoolObjectLikeCpp::new(2, 0.0), 1);
    assert!(equal_present.check_pool_like_cpp());
}

#[test]
fn pool_group_empty_deep_check_matches_cpp() {
    let empty_creature = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::Creature);
    let mut creature_closure_calls = 0;
    assert!(empty_creature.is_empty_deep_check_like_cpp(|_| {
        creature_closure_calls += 1;
        false
    }));
    assert_eq!(creature_closure_calls, 0);

    let mut gameobject = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::GameObject);
    gameobject.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 0.0), 1);
    let mut gameobject_closure_calls = 0;
    assert!(!gameobject.is_empty_deep_check_like_cpp(|_| {
        gameobject_closure_calls += 1;
        true
    }));
    assert_eq!(gameobject_closure_calls, 0);

    let mut pool = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::Pool);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 50.0), 1);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(20, 0.0), 1);
    let mut visited = Vec::new();
    assert!(!pool.is_empty_deep_check_like_cpp(|child_pool_id| {
        visited.push(child_pool_id);
        child_pool_id != 20
    }));
    assert_eq!(visited, vec![10, 20]);

    let mut overflowing_pool = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::Pool);
    overflowing_pool.add_entry_like_cpp(PoolObjectLikeCpp::new(u64::from(u32::MAX) + 1, 1.0), 1);
    assert!(!overflowing_pool.is_empty_deep_check_like_cpp(|_| true));
}

#[test]
fn pool_group_remove_one_relation_matches_cpp() {
    let mut pool = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::Pool);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 50.0), 1);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 40.0), 1);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 0.0), 1);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 0.0), 2);
    pool.add_entry_like_cpp(PoolObjectLikeCpp::new(11, 0.0), 1);

    let removal = pool.remove_one_relation_like_cpp(10);
    assert_eq!(
        removal,
        PoolRelationRemovalLikeCpp {
            removed_explicit: true,
            removed_equal: true,
        }
    );
    assert_eq!(
        pool.explicitly_chanced_like_cpp(),
        &[PoolObjectLikeCpp::new(10, 40.0)]
    );
    assert_eq!(
        pool.equal_chanced_like_cpp(),
        &[
            PoolObjectLikeCpp::new(10, 0.0),
            PoolObjectLikeCpp::new(11, 0.0),
        ]
    );

    let mut creature = PoolGroupLikeCpp::new(PoolMemberKindLikeCpp::Creature);
    creature.add_entry_like_cpp(PoolObjectLikeCpp::new(10, 50.0), 1);
    assert_eq!(
        creature.remove_one_relation_like_cpp(10),
        PoolRelationRemovalLikeCpp::default()
    );
    assert_eq!(
        creature.explicitly_chanced_like_cpp(),
        &[PoolObjectLikeCpp::new(10, 50.0)]
    );
}

#[test]
fn pool_mgr_init_pools_for_map_no_autospawn_is_noop_like_cpp() {
    let mgr = PoolMgrLikeCpp::new();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut rolls = 0;
    let plan = mgr.init_pools_for_map_plan_like_cpp(
        571,
        &mut spawns,
        |_, _| {
            rolls += 1;
            0.0
        },
        choose_first_indices,
    );

    assert_eq!(plan.map_id, 571);
    assert_eq!(plan.attempted(), 0);
    assert_eq!(plan.planned(), 0);
    assert_eq!(plan.error_count(), 0);
    assert_eq!(rolls, 0);
    assert!(spawns.spawned_objects_like_cpp().is_empty());
}

#[test]
fn pool_mgr_init_pools_for_map_applies_autospawn_order_and_mutates_state_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(2, 571));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::GameObject,
        10,
        group_with_one(PoolMemberKindLikeCpp::GameObject, 10, 201),
    )
    .unwrap();
    mgr.insert_template_like_cpp(20, PoolTemplateDataLikeCpp::new(2, 571));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        20,
        group_with_one(PoolMemberKindLikeCpp::Creature, 20, 101),
    )
    .unwrap();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Pool,
        20,
        group_with_one(PoolMemberKindLikeCpp::Pool, 20, 301),
    )
    .unwrap();
    mgr.add_auto_spawn_pool_like_cpp(571, 20);
    mgr.add_auto_spawn_pool_like_cpp(571, 10);
    mgr.add_auto_spawn_pool_like_cpp(530, 99);
    let mut spawns = SpawnedPoolDataLikeCpp::new();

    let plan =
        mgr.init_pools_for_map_plan_like_cpp(571, &mut spawns, |_, _| 0.0, choose_first_indices);

    assert_eq!(
        plan.pools
            .iter()
            .map(|pool| pool.pool_id)
            .collect::<Vec<_>>(),
        vec![20, 10]
    );
    assert_eq!(plan.attempted(), 2);
    assert_eq!(plan.spawn_one_actions(), 3);
    assert!(spawns.is_spawned_pool_like_cpp(301));
    assert!(spawns.is_spawned_creature_like_cpp(101));
    assert!(spawns.is_spawned_gameobject_like_cpp(201));
    assert_eq!(spawns.get_spawned_objects_like_cpp(20), 2);
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 1);
}

#[test]
fn pool_mgr_init_pools_for_map_reports_errors_without_truncating_or_panicking_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        10,
        group_with_one(PoolMemberKindLikeCpp::Creature, 10, 101),
    )
    .unwrap();
    mgr.add_auto_spawn_pool_like_cpp(571, 10);
    let mut spawns = SpawnedPoolDataLikeCpp::new();

    let plan =
        mgr.init_pools_for_map_plan_like_cpp(571, &mut spawns, |_, _| 0.0, choose_first_indices);

    assert_eq!(plan.pools, Vec::<PoolSpawnPoolPlanLikeCpp>::new());
    assert_eq!(plan.attempted(), 1);
    assert_eq!(plan.error_count(), 1);
    assert_eq!(plan.errors[0].pool_id, Some(10));
    assert_eq!(
        plan.errors[0].error,
        PoolInitForMapErrorKindLikeCpp::PoolPlan(PoolMgrPlanErrorLikeCpp::MissingTemplate {
            pool_id: 10,
        })
    );
    assert!(!spawns.is_spawned_creature_like_cpp(101));

    let overflow = mgr.init_pools_for_map_plan_like_cpp(
        u32::MAX,
        &mut spawns,
        |_, _| 0.0,
        choose_first_indices,
    );
    assert_eq!(overflow.attempted(), 0);
    assert_eq!(overflow.error_count(), 1);
    assert_eq!(
        overflow.errors[0].error,
        PoolInitForMapErrorKindLikeCpp::MapIdOutOfI32Range
    );
}

#[test]
fn pool_mgr_spawn_pool_orders_pool_gameobject_creature_and_mutates_map_spawns_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(3, 571));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        10,
        group_with_one(PoolMemberKindLikeCpp::Creature, 10, 101),
    )
    .unwrap();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::GameObject,
        10,
        group_with_one(PoolMemberKindLikeCpp::GameObject, 10, 201),
    )
    .unwrap();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Pool,
        10,
        group_with_one(PoolMemberKindLikeCpp::Pool, 10, 301),
    )
    .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();

    let mut roll_calls = Vec::new();
    let plan = mgr
        .spawn_pool_plan_like_cpp(
            &mut spawns,
            10,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                0.0
            },
            choose_first_indices,
        )
        .unwrap();

    let kinds = plan
        .subplans
        .iter()
        .map(|subplan| subplan.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            PoolMemberKindLikeCpp::Pool,
            PoolMemberKindLikeCpp::GameObject,
            PoolMemberKindLikeCpp::Creature,
        ]
    );
    assert!(roll_calls.is_empty());
    assert!(spawns.is_spawned_pool_like_cpp(301));
    assert!(spawns.is_spawned_gameobject_like_cpp(201));
    assert!(spawns.is_spawned_creature_like_cpp(101));
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 3);
    assert_eq!(
        plan.subplans[0].object_plan.as_ref().unwrap().actions,
        vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 301,
        }]
    );
}

#[test]
fn pool_mgr_spawn_pool_uses_independent_explicit_rolls_per_top_level_kind_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(2, 571));

    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(201, 25.0), 1);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(202, 25.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 10, gameobject_group)
        .unwrap();

    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 25.0), 1);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(102, 25.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, creature_group)
        .unwrap();

    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut roll_calls = Vec::new();
    let plan = mgr
        .spawn_pool_plan_like_cpp(
            &mut spawns,
            10,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                match kind {
                    PoolMemberKindLikeCpp::Pool => 0.0,
                    PoolMemberKindLikeCpp::GameObject => 10.0,
                    PoolMemberKindLikeCpp::Creature => 30.0,
                }
            },
            choose_first_indices,
        )
        .unwrap();

    assert_eq!(
        roll_calls,
        vec![
            (PoolMemberKindLikeCpp::GameObject, 10),
            (PoolMemberKindLikeCpp::Creature, 10),
        ]
    );
    assert_eq!(
        plan.subplans[0].skip_reason,
        Some(PoolMgrPlanSkipReasonLikeCpp::MissingGroup)
    );
    assert!(spawns.is_spawned_gameobject_like_cpp(201));
    assert!(!spawns.is_spawned_gameobject_like_cpp(202));
    assert!(!spawns.is_spawned_creature_like_cpp(101));
    assert!(spawns.is_spawned_creature_like_cpp(102));

    let gameobject_plan = plan.subplans[1].object_plan.as_ref().unwrap();
    let creature_plan = plan.subplans[2].object_plan.as_ref().unwrap();
    assert_eq!(
        gameobject_plan.selected,
        vec![PoolObjectLikeCpp::new(201, 25.0)]
    );
    assert_eq!(
        creature_plan.selected,
        vec![PoolObjectLikeCpp::new(102, 25.0)]
    );
}

#[test]
fn pool_mgr_missing_template_errors_without_limit_zero_or_spawn_mutation_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        10,
        group_with_one(PoolMemberKindLikeCpp::Creature, 10, 101),
    )
    .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();

    let result = mgr.spawn_typed_pool_plan_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        &mut spawns,
        10,
        0,
        |_, _| 0.0,
        choose_first_indices,
    );

    assert_eq!(
        result,
        Err(PoolMgrPlanErrorLikeCpp::MissingTemplate { pool_id: 10 })
    );
    assert!(!spawns.is_spawned_creature_like_cpp(101));
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 0);
}

#[test]
fn pool_mgr_missing_and_empty_groups_are_noop_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::GameObject,
        10,
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10),
    )
    .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();

    let mut roll_calls = Vec::new();
    let plan = mgr
        .spawn_pool_plan_like_cpp(
            &mut spawns,
            10,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                0.0
            },
            choose_first_indices,
        )
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
    assert!(roll_calls.is_empty());
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 0);
}

#[test]
fn pool_mgr_equal_only_group_does_not_request_explicit_roll_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    mgr.insert_or_replace_group_like_cpp(
        PoolMemberKindLikeCpp::Creature,
        10,
        group_with_one(PoolMemberKindLikeCpp::Creature, 10, 101),
    )
    .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    let mut roll_calls = Vec::new();

    let plan = mgr
        .spawn_pool_plan_like_cpp(
            &mut spawns,
            10,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                0.0
            },
            choose_first_indices,
        )
        .unwrap();

    assert!(roll_calls.is_empty());
    assert!(plan.subplans[2].object_plan.is_some());
    assert!(spawns.is_spawned_creature_like_cpp(101));
}

#[test]
fn pool_mgr_count_non_positive_does_not_request_explicit_roll_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 100.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, group)
        .unwrap();
    let mut spawns = SpawnedPoolDataLikeCpp::new();
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 10),
        Ok(())
    );
    assert_eq!(
        spawns.add_spawn_like_cpp(SpawnObjectType::Creature, 102, 10),
        Ok(())
    );
    let mut roll_calls = Vec::new();

    let plan = mgr
        .spawn_typed_pool_plan_like_cpp(
            PoolMemberKindLikeCpp::Creature,
            &mut spawns,
            10,
            0,
            |kind, pool_id| {
                roll_calls.push((kind, pool_id));
                0.0
            },
            choose_first_indices,
        )
        .unwrap();

    assert!(roll_calls.is_empty());
    assert!(plan.object_plan.unwrap().selected.is_empty());
    assert_eq!(spawns.get_spawned_objects_like_cpp(10), 2);
}
