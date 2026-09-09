//! Persistence scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn process_respawns_pool_spawn_one_pool_applies_real_child_spawn_plan_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(34, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 80, active), |_| {
        false
    });
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Pool,
        pool_id: 180,
        trigger_from: 0,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
                kind: PoolMemberKindLikeCpp::Pool,
                guid: 181,
            }],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: false,
            child_pool_spawn_plans: vec![PoolSpawnPoolPlanLikeCpp {
                pool_id: 181,
                subplans: vec![PoolTypedSpawnPlanLikeCpp {
                    kind: PoolMemberKindLikeCpp::Creature,
                    pool_id: 181,
                    trigger_from: 0,
                    max_limit: Some(1),
                    object_plan: Some(PoolSpawnObjectPlanLikeCpp {
                        actions: vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
                            kind: PoolMemberKindLikeCpp::Creature,
                            guid: 80,
                        }],
                        selected: vec![],
                        despawned_trigger: None,
                        respawned_trigger: false,
                        ..PoolSpawnObjectPlanLikeCpp::default()
                    }),
                    skip_reason: None,
                }],
            }],
            child_pool_despawn_plans: vec![],
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
        &plan,
        &store,
        &mut summary,
        Some(&mut |_, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 80);
            let mut creature = test_creature_for_spawn(spawn_id, 8001, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        }),
    );

    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_action_load_plans, vec![]);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(80), 1);
}
#[test]
fn spawn_pool_facade_mutates_pool_data_and_executes_loaded_grid_loader_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(530, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 530101, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(5301, crate::pool::PoolTemplateDataLikeCpp::new(1, 571));
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5301);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(530101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5301, creature_group)
        .expect("test creature pool group");
    let mut loader_calls = 0usize;

    let summary = map
        .spawn_pool_loaded_grid_records_like_cpp(
            &pool_mgr,
            5301,
            &store,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
            |_, object_type, spawn_id| {
                loader_calls += 1;
                assert_eq!(object_type, SpawnObjectType::Creature);
                assert_eq!(spawn_id, 530101);
                let mut creature = test_creature_for_spawn(spawn_id, 53010101, true);
                creature
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .remove_from_world();
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_creature(creature).unwrap(),
                ))
            },
        )
        .expect("spawn pool facade plan");

    assert_eq!(loader_calls, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(530101)
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(530101), 1);
}
#[test]
fn spawn_pool_facade_filters_unloaded_grid_without_calling_loader_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(531, SpawnGroupFlags::NONE);
    let mut unloaded = spawn_data(SpawnObjectType::Creature, 530201, active);
    unloaded.spawn_point = crate::spawn::SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0);
    store.add_object_spawn(&unloaded, |_| false);
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(5302, crate::pool::PoolTemplateDataLikeCpp::new(1, 571));
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5302);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(530201, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5302, creature_group)
        .expect("test creature pool group");
    let mut loader_calls = 0usize;

    let summary = map
        .spawn_pool_loaded_grid_records_like_cpp(
            &pool_mgr,
            5302,
            &store,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
            |_, _, _| {
                loader_calls += 1;
                None
            },
        )
        .expect("spawn pool facade plan");

    assert_eq!(loader_calls, 0);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(530201)
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(530201), 0);
}
#[test]
fn process_respawns_pool_plan_error_preserves_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(14, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 47, active), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 47, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 55);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(47, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 55, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 47, 55)
        .expect("test spawn pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(summary.processed_pool_timers, 0);
    assert_eq!(
        summary.blocked_pool_plan_errors,
        vec![PoolMgrPlanErrorLikeCpp::MissingTemplate { pool_id: 55 }]
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 47),
        100
    );
    assert!(!map.pool_data_like_cpp().is_spawned_creature_like_cpp(47));
}
#[test]
fn add_to_map_like_cpp_active_world_object_loads_grid_and_world_container() {
    let mut map = test_map();
    let mut object = WorldObject::new(true, TypeId::DynamicObject, TypeMask::DYNAMIC_OBJECT);
    object.object_mut().create(guid(HighGuid::DynamicObject, 2));
    object.set_map(571, 7).unwrap();
    object.relocate(Position::xyz(20.0, 20.0, 3.0));
    object.set_active(true);
    let guid = object.guid();

    let outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::DynamicObject, object)
        .unwrap();

    assert!(outcome.grid_loaded);
    assert!(!outcome.grid_created);
    assert!(map.is_grid_loaded(outcome.grid));
    assert_eq!(map.lifecycle().loads, 1);
    let grid = map.get_ngrid(outcome.grid).unwrap();
    assert_eq!(grid.state(), GridStateKind::Active);
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.world_objects.dynamic_objects.contains(&guid));
    assert!(!cell.grid_objects.dynamic_objects.contains(&guid));
}
#[test]
fn switch_list_unloaded_grid_does_not_create_grid_and_drains_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(420070, 4200701, true);
    let guid = creature.guid();
    let cell = Cell::from_world(1.0, 2.0);
    let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.get_ngrid(grid).is_none());

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_invalid_or_unloaded_grid, 1);
    assert!(map.get_ngrid(grid).is_none());
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn relocate_map_object_like_cpp_blocks_normal_object_to_unloaded_grid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(700.0, 20.0, 5.0))
        .unwrap();

    assert!(!outcome.relocated);
    assert!(outcome.blocked_by_unloaded_grid);
    assert_eq!(
        map.get_creature(guid).unwrap().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );
    let old_grid = map.get_ngrid(added.grid).unwrap();
    let old_cell = old_grid
        .get_grid_type(
            added.cell.x_coord % MAX_NUMBER_OF_CELLS,
            added.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(old_cell.grid_objects.creatures.contains(&guid));
}
#[test]
fn relocate_map_object_like_cpp_active_object_loads_new_grid_and_moves() {
    let mut map = test_map();
    let mut object = WorldObject::new(true, TypeId::DynamicObject, TypeMask::DYNAMIC_OBJECT);
    object.object_mut().create(guid(HighGuid::DynamicObject, 3));
    object.set_map(571, 7).unwrap();
    object.relocate(Position::xyz(20.0, 20.0, 3.0));
    object.set_active(true);
    let guid = object.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::DynamicObject, object)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(700.0, 20.0, 5.0))
        .unwrap();

    assert!(outcome.relocated);
    assert!(outcome.moved_between_cells);
    assert_ne!(outcome.old_grid, outcome.new_grid);
    assert!(outcome.loaded_grid);
    assert!(map.is_grid_loaded(outcome.new_grid));
    assert_eq!(
        map.get_dynamic_object(guid).unwrap().position(),
        Position::xyz(700.0, 20.0, 5.0)
    );
}
#[test]
fn nearby_cell_guids_like_cpp_visits_existing_cells_without_loading_grids() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object(HighGuid::GameObject, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();

    let nearby = map.nearby_cell_guids_like_cpp(0.0, 0.0, 70.0);

    assert_eq!(nearby.visited_cells, 16);
    assert_eq!(nearby.len(), 2);
    assert!(nearby.grid.creatures.contains(&creature_guid));
    assert!(nearby.grid.gameobjects.contains(&gameobject_guid));
    assert_eq!(map.terrain().loads.len(), 1);

    let far = map.nearby_cell_guids_like_cpp(700.0, 700.0, 0.0);
    assert_eq!(far.visited_cells, 1);
    assert!(far.is_empty());
    assert_eq!(map.terrain().loads.len(), 1);
}
#[test]
fn delayed_unit_relocation_plan_selects_only_units_needing_notify_like_cpp() {
    let creature_notify = guid(HighGuid::Creature, 1);
    let creature_normal = guid(HighGuid::Creature, 2);
    let world_creature_notify = guid(HighGuid::Creature, 3);
    let player_notify = guid(HighGuid::Player, 4);
    let player_normal = guid(HighGuid::Player, 5);
    let player_invalid_viewpoint = guid(HighGuid::Player, 6);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(creature_notify);
    nearby.grid.creatures.insert(creature_normal);
    nearby.world.creatures.insert(world_creature_notify);
    nearby.world.players.insert(player_notify);
    nearby.world.players.insert(player_normal);
    nearby.world.players.insert(player_invalid_viewpoint);

    let plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(
        &nearby,
        [creature_notify, world_creature_notify],
        [player_notify, player_invalid_viewpoint],
        [player_invalid_viewpoint],
    );

    assert_eq!(
        plan.creature_relocations,
        vec![creature_notify, world_creature_notify]
    );
    assert_eq!(plan.player_relocations, vec![player_notify]);
    assert_eq!(
        plan.skipped_invalid_viewpoints,
        vec![player_invalid_viewpoint]
    );
}
#[test]
fn object_update_plan_for_nearby_like_cpp_selects_in_world_updateable_objects_only() {
    let mut map = test_map();
    let player = world_object(HighGuid::Player, 571, 7, true);
    let player_guid = player.guid();
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let creature_guid = creature.guid();
    let gameobject = world_object(HighGuid::GameObject, 571, 7, true);
    let gameobject_guid = gameobject.guid();
    let dynamic_not_in_world = world_object(HighGuid::DynamicObject, 571, 7, false);
    let dynamic_guid = dynamic_not_in_world.guid();
    let missing_conversation = guid(HighGuid::Conversation, 9);
    map.insert_map_object(AccessorObjectKind::Player, player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::GameObject, gameobject)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::DynamicObject, dynamic_not_in_world)
        .unwrap();

    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player_guid);
    nearby.grid.creatures.insert(creature_guid);
    nearby.grid.gameobjects.insert(gameobject_guid);
    nearby.grid.dynamic_objects.insert(dynamic_guid);
    nearby.grid.conversations.insert(missing_conversation);

    let plan = map.object_update_plan_for_nearby_like_cpp(&nearby, 42);

    assert_eq!(plan.diff_ms, 42);
    assert_eq!(plan.update_guids, vec![creature_guid, gameobject_guid]);
}
#[test]
fn object_update_plan_for_nearby_like_cpp_deduplicates_world_and_grid_objects() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let creature_guid = creature.guid();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();
    let mut nearby = NearbyCellGuids::default();
    nearby.world.creatures.insert(creature_guid);
    nearby.grid.creatures.insert(creature_guid);

    let plan = map.object_update_plan_for_nearby_like_cpp(&nearby, 1);

    assert_eq!(plan.update_guids, vec![creature_guid]);
}
#[test]
fn map_update_visit_plan_like_cpp_filters_sources_by_cpp_in_world_guards() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, true);
    let player_guid = player.guid();
    let offline_player = world_object_with_counter(HighGuid::Player, 2, 571, 7, false);
    let offline_player_guid = offline_player.guid();
    let viewpoint = world_object_with_counter(HighGuid::Creature, 3, 571, 7, true);
    let viewpoint_guid = viewpoint.guid();
    let far_combat = world_object_with_counter(HighGuid::Creature, 4, 571, 7, true);
    let far_combat_guid = far_combat.guid();
    let offline_aura = world_object_with_counter(HighGuid::Creature, 5, 571, 7, false);
    let offline_aura_guid = offline_aura.guid();
    let active_non_player = world_object_with_counter(HighGuid::DynamicObject, 6, 571, 7, true);
    let active_non_player_guid = active_non_player.guid();
    let transport = world_object_with_counter(HighGuid::Transport, 7, 571, 7, false);
    let transport_guid = transport.guid();

    map.insert_map_object(AccessorObjectKind::Player, player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Player, offline_player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, viewpoint)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, far_combat)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, offline_aura)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::DynamicObject, active_non_player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Transport, transport)
        .unwrap();

    let plan = map.map_update_visit_plan_like_cpp(
        [
            MapUpdatePlayerSources {
                player_guid,
                viewpoint_guid: Some(viewpoint_guid),
                far_combat_unit_guids: vec![far_combat_guid],
                far_aura_caster_guids: vec![offline_aura_guid],
                far_summon_guids: vec![],
            },
            MapUpdatePlayerSources {
                player_guid: offline_player_guid,
                viewpoint_guid: Some(far_combat_guid),
                far_combat_unit_guids: vec![viewpoint_guid],
                far_aura_caster_guids: vec![],
                far_summon_guids: vec![],
            },
        ],
        [active_non_player_guid, offline_aura_guid],
        [transport_guid],
        50,
    );

    assert_eq!(plan.diff_ms, 50);
    assert_eq!(plan.session_update_players, vec![player_guid]);
    assert_eq!(plan.player_update_guids, vec![player_guid]);
    assert_eq!(plan.transport_update_guids, vec![transport_guid]);
    assert_eq!(
        plan.nearby_visit_centers
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([
            player_guid,
            viewpoint_guid,
            far_combat_guid,
            active_non_player_guid
        ])
    );
    assert!(plan.process_relocation_notifies);
}
#[test]
fn map_update_visit_plan_like_cpp_processes_relocation_notifies_only_for_players_or_active_non_players()
 {
    let mut map = test_map();
    let transport = world_object_with_counter(HighGuid::Transport, 7, 571, 7, false);
    let transport_guid = transport.guid();
    map.insert_map_object(AccessorObjectKind::Transport, transport)
        .unwrap();

    let plan = map.map_update_visit_plan_like_cpp(
        std::iter::empty::<MapUpdatePlayerSources>(),
        std::iter::empty::<ObjectGuid>(),
        [transport_guid],
        1,
    );

    assert_eq!(plan.transport_update_guids, vec![transport_guid]);
    assert!(!plan.process_relocation_notifies);
}
#[test]
fn process_relocation_notifies_plan_like_cpp_waits_for_active_grid_timer() {
    let mut map = test_map();
    let grid = GridCoord::new(2, 3);
    map.ensure_grid_created(grid);
    map.get_ngrid_mut(grid)
        .unwrap()
        .set_state(GridStateKind::Active);
    let marked = CellCoord::new(2 * MAX_NUMBER_OF_CELLS, 3 * MAX_NUMBER_OF_CELLS);

    let plan = map.process_relocation_notifies_plan_like_cpp([marked], 999, 1000);

    assert!(plan.delayed_relocation_cells.is_empty());
    assert!(plan.reset_notify_cells.is_empty());
    assert!(plan.reset_timer_grids.is_empty());
}
#[test]
fn process_relocation_notifies_plan_like_cpp_visits_marked_cells_and_resets_timer() {
    let mut map = test_map();
    let active_grid = GridCoord::new(2, 3);
    let idle_grid = GridCoord::new(4, 5);
    map.ensure_grid_created(active_grid);
    map.ensure_grid_created(idle_grid);
    map.get_ngrid_mut(active_grid)
        .unwrap()
        .set_state(GridStateKind::Active);
    map.get_ngrid_mut(idle_grid)
        .unwrap()
        .set_state(GridStateKind::Idle);
    let marked_a = CellCoord::new(2 * MAX_NUMBER_OF_CELLS, 3 * MAX_NUMBER_OF_CELLS);
    let marked_b = CellCoord::new(2 * MAX_NUMBER_OF_CELLS + 1, 3 * MAX_NUMBER_OF_CELLS);
    let marked_idle = CellCoord::new(4 * MAX_NUMBER_OF_CELLS, 5 * MAX_NUMBER_OF_CELLS);

    let plan = map.process_relocation_notifies_plan_like_cpp(
        [marked_b, marked_idle, marked_a],
        1000,
        1000,
    );

    assert_eq!(plan.diff_ms, 1000);
    assert_eq!(plan.delayed_relocation_cells, vec![marked_a, marked_b]);
    assert_eq!(plan.reset_notify_cells, vec![marked_a, marked_b]);
    assert_eq!(plan.reset_timer_grids, vec![active_grid]);
    assert_eq!(
        map.get_ngrid(active_grid)
            .unwrap()
            .info()
            .relocation_timer()
            .expire_time_ms(),
        1000
    );
}
#[test]
fn ensure_grid_created_sets_idle_grid_and_loads_reversed_terrain_coords() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);

    assert!(map.ensure_grid_created(coord));
    assert!(!map.ensure_grid_created(coord));

    let grid = map.get_ngrid(coord).unwrap();
    assert_eq!(grid.grid_id(), 2 * MAX_NUMBER_OF_GRIDS + 3);
    assert_eq!(grid.state(), GridStateKind::Idle);
    assert!(!grid.grid_object_data_loaded());
    assert_eq!(map.terrain().loads, vec![(61, 60)]);
}
#[test]
fn ensure_grid_loaded_marks_loaded_before_object_loader_hook() {
    let mut map = test_map();
    let cell = cell_from_grid_center(GridCoord::new(2, 3));

    assert!(map.ensure_grid_loaded(&cell));
    assert!(!map.ensure_grid_loaded(&cell));

    assert!(map.is_grid_loaded(GridCoord::new(2, 3)));
    assert_eq!(map.lifecycle().loads, 1);
}
#[test]
fn registered_loaded_corpse_tracks_grid_load_and_unload_like_cpp() {
    let mut map = Map::new(571, 0, 0, 60_000);
    let position = Position::new(10.0, 20.0, 30.0, 1.5);
    let cell = Cell::from_world(position.x, position.y);
    let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
    let guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 1);
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePve, 1_000);
    corpse.world_mut().object_mut().create(guid);
    corpse.world_mut().set_map(571, 0).unwrap();
    corpse.world_mut().relocate(position);

    assert!(!map.register_loaded_corpse_like_cpp(corpse).unwrap());
    assert!(
        !map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(
        map.nearby_cell_guids_like_cpp(position.x, position.y, 1.0)
            .world
            .corpses
            .is_empty()
    );

    assert!(map.ensure_grid_loaded(&cell));
    assert!(
        map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(
        map.nearby_cell_guids_like_cpp(position.x, position.y, 1.0)
            .world
            .corpses
            .contains(&guid)
    );

    assert!(map.unload_grid_at(grid, true));
    assert!(
        !map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(map.ensure_grid_loaded(&cell));
    assert!(
        map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
}
#[test]
fn loaded_grid_coords_only_reports_object_data_loaded_grids_like_cpp() {
    let mut map = test_map();
    let created_only = GridCoord::new(2, 3);
    let loaded_a = GridCoord::new(4, 5);
    let loaded_b = GridCoord::new(4, 6);

    assert!(map.ensure_grid_created(created_only));
    assert!(map.ensure_grid_loaded(&cell_from_grid_center(loaded_b)));
    assert!(map.ensure_grid_loaded(&cell_from_grid_center(loaded_a)));

    assert_eq!(map.loaded_grid_coords_like_cpp(), vec![loaded_a, loaded_b]);
}
#[test]
fn active_object_loading_sets_grid_active_and_short_expiry() {
    let mut map = test_map();
    let cell = cell_from_grid_center(GridCoord::new(2, 3));

    assert!(map.ensure_grid_loaded_for_active_object(&cell, ActiveObjectKind::NonPlayer));

    let grid = map.get_ngrid(GridCoord::new(2, 3)).unwrap();
    assert_eq!(grid.state(), GridStateKind::Active);
    assert_eq!(grid.info().time_tracker().remaining_ms(), 100);
    assert!(map.active_objects_near_grid(grid));
}
#[test]
fn unload_grid_applies_guid_lifecycle_actions_to_canonical_map_objects_like_cpp() {
    let mut map = guid_unload_test_map();
    let coord = GridCoord::new(2, 3);
    let cell = cell_from_grid_center(coord);
    assert!(map.ensure_grid_loaded(&cell));

    let creature = test_creature_for_spawn(4181, 4181, true);
    let creature_guid = creature.unit().world().guid();
    let gameobject = test_gameobject_for_spawn(4182, 4182);
    let gameobject_guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let grid_cell = map
        .get_ngrid_mut(coord)
        .unwrap()
        .get_grid_type_mut(0, 0)
        .unwrap();
    grid_cell.grid_objects.creatures.insert(creature_guid);
    grid_cell.grid_objects.gameobjects.insert(gameobject_guid);

    assert!(map.unload_grid_at(coord, true));

    assert!(map.get_ngrid(coord).is_none());
    assert_eq!(map.terrain().unloads, vec![(61, 60)]);
    assert_eq!(map.map_object_count(), 2);

    let creature = map
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap();
    assert!(creature.unit().world().object().is_destroyed_object());
    assert_eq!(creature.cleanup_before_delete_count(), 2);
    assert!(creature.grid_unload_delete_requested());
    assert!(!creature.grid_unload_respawn_relocation_requested());

    let gameobject = map
        .map_object_record(gameobject_guid)
        .unwrap()
        .game_object()
        .unwrap();
    assert!(gameobject.world().object().is_destroyed_object());
    assert_eq!(gameobject.cleanup_before_delete_count(), 2);
    assert!(gameobject.grid_unload_delete_requested());
    assert!(!gameobject.grid_unload_respawn_relocation_requested());
}
#[test]
fn update_grid_state_at_removes_grid_when_removal_unloads_successfully() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);
    map.ensure_grid_loaded(&cell_from_grid_center(coord));
    map.get_ngrid_mut(coord)
        .unwrap()
        .set_state(GridStateKind::Removal);

    assert!(map.update_grid_state_at(coord, 1001));

    assert!(map.get_ngrid(coord).is_none());
    assert_eq!(map.lifecycle().evacuates, 1);
    assert_eq!(map.lifecycle().cleans, 1);
    assert_eq!(map.lifecycle().unloads, 1);
}
#[test]
fn grid_id_loaded_uses_cpp_public_grid_id_decomposition() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);
    map.ensure_grid_loaded(&cell_from_grid_center(coord));

    assert!(is_grid_id_loaded(&map, 3 * MAX_NUMBER_OF_GRIDS + 2));
}
