//! Creature scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn set_world_object_like_cpp_creature_not_in_world_does_not_enqueue_or_mutate() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(421020, 4210201, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(outcome.status, SetWorldObjectStatusLikeCpp::NotInWorld);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 1);
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.switch_processed, 0);
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn switch_list_on_moves_creature_from_grid_to_world_container_like_cpp() {
    let mut map = test_map();
    let spawn_id = 420010;
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4200101);
    assert!(
        local_cell_for_switch(&map, grid, cell)
            .grid_objects
            .creatures
            .contains(&guid)
    );

    let queued = map.add_object_to_switch_list_like_cpp(guid, true);
    assert_eq!(queued.status, AddObjectToSwitchListStatusLikeCpp::Queued);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(!local_cell.grid_objects.creatures.contains(&guid));
    assert!(local_cell.world_objects.creatures.contains(&guid));
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn switch_list_off_moves_temp_creature_from_world_to_grid_container_like_cpp() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 420020, 4200201);
    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert_eq!(
        map.remove_all_objects_in_remove_list_like_cpp()
            .switch_executed,
        1
    );
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, false).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_executed, 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn creature_relocation_visibility_plan_matches_cpp_player_and_creature_visits() {
    let source = guid(HighGuid::Creature, 1);
    let player_visible = guid(HighGuid::Player, 2);
    let player_needs_notify = guid(HighGuid::Player, 3);
    let creature_normal = guid(HighGuid::Creature, 4);
    let creature_needs_notify = guid(HighGuid::Creature, 5);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player_visible);
    nearby.world.players.insert(player_needs_notify);
    nearby.grid.creatures.insert(source);
    nearby.grid.creatures.insert(creature_normal);
    nearby.grid.creatures.insert(creature_needs_notify);

    let plan = CreatureRelocationVisibilityPlan::from_nearby_like_cpp(
        source,
        true,
        &nearby,
        [player_needs_notify],
        [creature_needs_notify],
    );

    assert_eq!(
        plan.player_visibility_updates,
        HashSet::from([player_visible])
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, player_visible))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, player_needs_notify))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, creature_normal))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(creature_normal, source))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, creature_needs_notify))
    );
    assert!(
        !plan
            .ai_relocation_checks
            .contains(&(creature_needs_notify, source))
    );
}
#[test]
fn creature_relocation_visibility_plan_skips_creature_visits_when_source_dead() {
    let source = guid(HighGuid::Creature, 1);
    let player = guid(HighGuid::Player, 2);
    let creature = guid(HighGuid::Creature, 3);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player);
    nearby.grid.creatures.insert(creature);

    let plan =
        CreatureRelocationVisibilityPlan::from_nearby_like_cpp(source, false, &nearby, [], []);

    assert_eq!(plan.player_visibility_updates, HashSet::from([player]));
    assert_eq!(plan.ai_relocation_checks, vec![(source, player)]);
}
#[test]
fn delayed_unit_relocation_plan_deduplicates_creatures_from_world_and_grid_sets() {
    let creature = guid(HighGuid::Creature, 1);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(creature);
    nearby.world.creatures.insert(creature);

    let plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(&nearby, [creature], [], []);

    assert_eq!(plan.creature_relocations, vec![creature]);
    assert!(plan.player_relocations.is_empty());
}
#[test]
fn ai_relocation_plan_for_player_checks_nearby_creatures_against_source_unit() {
    let player = guid(HighGuid::Player, 1);
    let world_creature = guid(HighGuid::Creature, 2);
    let grid_creature = guid(HighGuid::Creature, 3);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.creatures.insert(world_creature);
    nearby.grid.creatures.insert(grid_creature);

    let plan = AIRelocationPlan::from_nearby_like_cpp(player, false, &nearby);

    assert_eq!(
        plan.creature_unit_checks,
        vec![(world_creature, player), (grid_creature, player)]
    );
}
#[test]
fn ai_relocation_plan_for_creature_checks_both_cpp_directions() {
    let source = guid(HighGuid::Creature, 1);
    let other = guid(HighGuid::Creature, 2);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(source);
    nearby.grid.creatures.insert(other);

    let plan = AIRelocationPlan::from_nearby_like_cpp(source, true, &nearby);

    assert_eq!(
        plan.creature_unit_checks,
        vec![(other, source), (source, other)]
    );
}
#[test]
fn ai_relocation_plan_deduplicates_world_grid_creatures_and_skips_self_worker_noop() {
    let source = guid(HighGuid::Creature, 1);
    let other = guid(HighGuid::Creature, 2);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.creatures.insert(source);
    nearby.grid.creatures.insert(source);
    nearby.world.creatures.insert(other);
    nearby.grid.creatures.insert(other);

    let plan = AIRelocationPlan::from_nearby_like_cpp(source, false, &nearby);

    assert_eq!(plan.creature_unit_checks, vec![(other, source)]);
}
#[test]
fn reset_notify_flags_for_cells_like_cpp_resets_only_players_and_creatures() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let player_guid = player.guid();
    let creature = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object_with_counter(HighGuid::GameObject, 3, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    let player_cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();
    for guid in [player_guid, creature_guid, gameobject_guid] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let outcome = map.reset_notify_flags_for_cells_like_cpp([player_cell]);

    assert_eq!(outcome.reset_player_guids, vec![player_guid]);
    assert_eq!(outcome.reset_creature_guids, vec![creature_guid]);
    assert!(outcome.missing_guids.is_empty());
    assert!(
        !map.map_object(player_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(
        !map.map_object(creature_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(
        map.map_object(gameobject_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
}
#[test]
fn active_to_idle_stop_drains_guid_lifecycle_stoper_actions_into_creature_like_cpp() {
    let mut map = guid_unload_test_map();
    let coord = GridCoord::new(2, 3);
    assert!(map.ensure_grid_loaded(&cell_from_grid_center(coord)));

    let dynamic_object_guid = guid(HighGuid::DynamicObject, 4184);
    let area_trigger_guid = guid(HighGuid::AreaTrigger, 4185);
    let victim_guid = guid(HighGuid::Creature, 4186);
    let mut creature = test_creature_for_spawn(4184, 4184, true);
    let creature_guid = creature.unit().world().guid();
    creature.register_dynamic_object(dynamic_object_guid);
    creature.register_area_trigger(area_trigger_guid);
    creature.unit_mut().set_attacking(Some(victim_guid));
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let grid = map.get_ngrid_mut(coord).unwrap();
    grid.get_grid_type_mut(0, 0)
        .unwrap()
        .grid_objects
        .creatures
        .insert(creature_guid);
    grid.set_state(GridStateKind::Active);

    assert!(!map.update_grid_state_at(coord, 1001));

    let grid = map.get_ngrid(coord).unwrap();
    assert_eq!(grid.state(), GridStateKind::Idle);
    let creature = map
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap();
    assert!(!creature.is_in_combat());
    assert!(creature.dynamic_objects().is_empty());
    assert_eq!(
        creature.removed_dynamic_objects_from_grid_unload(),
        &[dynamic_object_guid]
    );
    assert!(creature.area_triggers().is_empty());
    assert_eq!(
        creature.removed_area_triggers_from_grid_unload(),
        &[area_trigger_guid]
    );
}
#[test]
fn unload_grid_refuses_world_creatures_and_active_neighbors_unless_forced() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);
    let cell = cell_from_grid_center(coord);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(coord)
        .unwrap()
        .get_grid_type_mut(0, 0)
        .unwrap()
        .world_objects
        .creatures
        .insert(ObjectGuid::new(1, 1));

    assert!(!map.unload_grid_at(coord, false));
    assert!(map.is_grid_loaded(coord));

    assert!(map.unload_grid_at(coord, true));
    assert!(map.get_ngrid(coord).is_none());
    assert_eq!(map.lifecycle().evacuates, 0);
    assert_eq!(map.lifecycle().cleans, 1);
    assert_eq!(map.lifecycle().unloads, 1);
    assert_eq!(map.terrain().unloads, vec![(61, 60)]);
}
