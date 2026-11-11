//! Loaded-grid output identity and success/error sequencing regressions.

use super::*;

fn fresh_creature_record(spawn_id: SpawnId, counter: i64) -> MapObjectRecord {
    let mut creature = test_creature_for_spawn(spawn_id, counter, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    MapObjectRecord::new_creature(creature).unwrap()
}

fn invalid_creature_record(spawn_id: SpawnId, counter: i64) -> MapObjectRecord {
    let mut record = fresh_creature_record(spawn_id, counter);
    record
        .object_mut()
        .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
    record
}

fn fact_pairs(records: &[LoadedGridObjectFacts]) -> Vec<(ObjectGuid, Option<ObjectGuid>)> {
    records
        .iter()
        .map(|record| (record.object_guid(), record.typed_creature_guid()))
        .collect()
}

#[test]
fn loaded_grid_area_trigger_facts_preserve_typed_identity_order_and_multiplicity() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    for spawn_id in 90_001..=90_007 {
        store.add_area_trigger_spawn(&spawn_data(
            SpawnObjectType::AreaTrigger,
            spawn_id,
            SpawnGroupTemplateData::legacy_group(),
        ));
    }
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    let typed = guid(HighGuid::Creature, 9_000_101);
    let stub = guid(HighGuid::Creature, 9_000_102);
    let gameobject = guid(HighGuid::GameObject, 9_000_103);
    let player = guid(HighGuid::Player, 9_000_104);
    let pre_add = guid(HighGuid::Creature, 9_000_110);
    let mut calls = Vec::new();
    let mut typed_allocation = 0;
    let mut player_allocation = 0;

    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::AreaTrigger);
            calls.push(spawn_id);
            if spawn_id > 90_001 {
                assert!(map.map_object_record(pre_add).is_some());
                assert!(map.map_object_record(typed).is_some());
            }
            let record = match spawn_id {
                90_001 => {
                    let record = fresh_creature_record(0, 9_000_101);
                    typed_allocation = record.creature().unwrap() as *const Creature as usize;
                    return Some(LoadedGridRespawnRecordsLikeCpp {
                        pre_add_records: vec![
                            fresh_creature_record(0, 9_000_110),
                            invalid_creature_record(0, 9_000_111),
                        ],
                        primary_record: record,
                    });
                }
                90_002 | 90_005 => MapObjectRecord::new(
                    AccessorObjectKind::Creature,
                    world_object_with_counter(HighGuid::Creature, 9_000_102, 571, 7, false),
                )
                .unwrap(),
                90_003 => {
                    let mut object = test_gameobject_for_spawn(0, 9_000_103);
                    object.world_mut().object_mut().remove_from_world();
                    MapObjectRecord::new_game_object(object).unwrap()
                }
                90_004 => {
                    let mut owner = test_player_for_viewpoint(9_000_104);
                    owner
                        .unit_mut()
                        .world_mut()
                        .object_mut()
                        .remove_from_world();
                    owner.set_money(987_654);
                    let record = MapObjectRecord::new_player(owner).unwrap();
                    player_allocation = record.player().unwrap() as *const Player as usize;
                    record
                }
                90_006 => invalid_creature_record(0, 9_000_106),
                90_007 => return None,
                _ => unreachable!(),
            };
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(record))
        },
    );

    assert_eq!(
        calls,
        vec![90_001, 90_002, 90_003, 90_004, 90_005, 90_006, 90_007]
    );
    assert_eq!(summary.metadata_entries, 7);
    assert_eq!(summary.pre_add_records_added, 1);
    assert_eq!(summary.add_to_map_errors, 2);
    assert_eq!(summary.load_record_missing, 1);
    assert_eq!(
        fact_pairs(&summary.loaded_grid_primary_records),
        vec![
            (typed, Some(typed)),
            (stub, None),
            (gameobject, None),
            (player, None),
            (stub, None),
        ]
    );
    let stored = map.map_object_record(typed).unwrap().creature().unwrap();
    assert_eq!(stored as *const Creature as usize, typed_allocation);
    let stored = map.map_object_record(player).unwrap().player().unwrap();
    assert_eq!(stored as *const Player as usize, player_allocation);
    assert_eq!(stored.money(), 987_654);
    assert_eq!(
        map.map_object_record(stub).unwrap().kind(),
        AccessorObjectKind::Creature
    );
    assert!(map.map_object_record(stub).unwrap().creature().is_none());
    assert!(
        map.map_object_record(guid(HighGuid::Creature, 9_000_106))
            .is_none()
    );
}

#[test]
fn loaded_grid_due_respawn_facts_follow_success_after_timer_and_pre_add_effects() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(9_010, SpawnGroupFlags::NONE);
    for (spawn_id, time) in [(90_101, 100), (90_102, 101), (90_103, 102)] {
        store.add_object_spawn(
            &spawn_data(SpawnObjectType::Creature, spawn_id, group.clone()),
            |_| false,
        );
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, spawn_id, time));
    }
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    let primary = guid(HighGuid::Creature, 9_010_101);
    let pre_add = guid(HighGuid::Creature, 9_010_110);
    let mut calls = Vec::new();
    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        102,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_, count| (0..count).collect(),
        true,
        |map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_ne!(map.get_respawn_time_like_cpp(object_type, spawn_id), 0);
            calls.push(spawn_id);
            if spawn_id > 90_101 {
                assert_eq!(map.get_respawn_time_like_cpp(object_type, 90_101), 0);
                assert!(map.map_object_record(primary).is_some());
                assert!(map.map_object_record(pre_add).is_some());
            }
            match spawn_id {
                90_101 => Some(LoadedGridRespawnRecordsLikeCpp {
                    pre_add_records: vec![fresh_creature_record(0, 9_010_110)],
                    primary_record: fresh_creature_record(spawn_id, 9_010_101),
                }),
                90_102 => Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    invalid_creature_record(spawn_id, 9_010_102),
                )),
                90_103 => None,
                _ => unreachable!(),
            }
        },
    );
    assert_eq!(calls, vec![90_101, 90_102, 90_103]);
    assert_eq!(
        fact_pairs(&summary.loaded_grid_primary_records),
        vec![(primary, Some(primary))]
    );
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 1);
    for spawn_id in 90_101..=90_103 {
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            0
        );
    }
}

#[test]
fn loaded_grid_spawn_group_facts_exclude_failed_adds_and_keep_loader_order() {
    let group = spawn_group(9_020, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        (90_201..=90_204)
            .map(|id| {
                spawn_data(
                    SpawnObjectType::Creature,
                    id,
                    SpawnGroupTemplateData::default_group(),
                )
            })
            .collect(),
    );
    let mut map = test_map();
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    for spawn_id in 90_201..=90_204 {
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, spawn_id, 999));
    }
    let typed = guid(HighGuid::Creature, 9_020_101);
    let stub = guid(HighGuid::Creature, 9_020_102);
    let mut calls = Vec::new();
    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        true,
        false,
        &store,
        |map, object_type, spawn_id, force| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert!(!force);
            assert_eq!(map.get_respawn_time_like_cpp(object_type, spawn_id), 0);
            calls.push(spawn_id);
            let record = match spawn_id {
                90_201 => fresh_creature_record(spawn_id, 9_020_101),
                90_202 => MapObjectRecord::new(
                    AccessorObjectKind::Creature,
                    world_object_with_counter(HighGuid::Creature, 9_020_102, 571, 7, false),
                )
                .unwrap(),
                90_203 => invalid_creature_record(spawn_id, 9_020_103),
                90_204 => return None,
                _ => unreachable!(),
            };
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(record))
        },
    );
    assert_eq!(calls, vec![90_201, 90_202, 90_203, 90_204]);
    assert_eq!(
        fact_pairs(&outcome.loaded_grid_primary_records),
        vec![(typed, Some(typed)), (stub, None)]
    );
    assert_eq!(outcome.metadata_entries, 4);
    assert_eq!(outcome.respawn_timers_removed, 4);
    assert_eq!(outcome.executed_loaded_grid_spawns, 2);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(
        outcome
            .load_plans
            .iter()
            .map(|p| p.spawn_id)
            .collect::<Vec<_>>(),
        calls
    );
}

#[test]
fn loaded_grid_pool_facts_keep_selection_order_and_preceding_pool_mutation() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let mut pool = PoolMgrLikeCpp::new();
    pool.insert_template_like_cpp(9_030, PoolTemplateDataLikeCpp::new(4, 571));
    let mut members = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 9_030);
    for spawn_id in 90_301..=90_304 {
        store.add_object_spawn(
            &spawn_data(
                SpawnObjectType::Creature,
                spawn_id,
                SpawnGroupTemplateData::legacy_group(),
            ),
            |_| false,
        );
        members.add_entry_like_cpp(PoolObjectLikeCpp::new(spawn_id, 0.0), 4);
    }
    pool.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 9_030, members)
        .unwrap();
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    let mut calls = Vec::new();
    let summary = map
        .spawn_pool_loaded_grid_records_like_cpp(
            &pool,
            9_030,
            &store,
            |_, _| 0.0,
            |_, count| (0..count).rev().collect(),
            |map, object_type, spawn_id| {
                assert_eq!(object_type, SpawnObjectType::Creature);
                assert_eq!(
                    map.pool_data_like_cpp().get_spawned_objects_like_cpp(9_030),
                    4
                );
                assert!(
                    map.pool_data_like_cpp()
                        .is_spawned_creature_like_cpp(spawn_id)
                );
                calls.push(spawn_id);
                match spawn_id {
                    90_304 => None,
                    90_303 => Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                        invalid_creature_record(spawn_id, 9_030_103),
                    )),
                    90_302 => Some(LoadedGridRespawnRecordsLikeCpp {
                        pre_add_records: vec![fresh_creature_record(0, 9_030_110)],
                        primary_record: fresh_creature_record(spawn_id, 9_030_102),
                    }),
                    90_301 => {
                        assert!(
                            map.map_object_record(guid(HighGuid::Creature, 9_030_110))
                                .is_some()
                        );
                        Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                            fresh_creature_record(spawn_id, 9_030_101),
                        ))
                    }
                    _ => unreachable!(),
                }
            },
        )
        .unwrap();
    let second = guid(HighGuid::Creature, 9_030_102);
    let first = guid(HighGuid::Creature, 9_030_101);
    assert_eq!(calls, vec![90_304, 90_303, 90_302, 90_301]);
    assert_eq!(
        fact_pairs(&summary.loaded_grid_primary_records),
        vec![(second, Some(second)), (first, Some(first))]
    );
    assert_eq!(summary.executed_loaded_grid_respawns, 2);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(summary.pool_spawn_action_load_plans.len(), 1);
    assert_eq!(summary.pool_spawn_action_load_plans[0].spawn_id, 90_304);
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(9_030),
        4
    );
}
