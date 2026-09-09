//! Creature scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn creature_vehicle_add_to_map_already_in_world_has_no_reset_evidence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(403, 40301, true);
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9005);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.creature_vehicle_reset.is_none());
}
#[test]
fn creature_vehicle_add_to_map_installs_local_vehicle_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(397, 39701, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9001, true);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    let install = outcome.creature_vehicle_install.unwrap();
    assert_eq!(install.kit_id, Some(9001));
    assert!(install.had_kit);
    assert_eq!(install.previous_installed, Some(false));
    assert!(install.installed);
    assert!(install.script_on_install_represented);
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(stored.unit().world().object().is_in_world());
    let kit = stored.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id, 9001);
    assert!(kit.active);
    assert!(kit.installed);
}
#[test]
fn creature_vehicle_add_to_map_without_kit_has_no_install_evidence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(398, 39801, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.creature_vehicle_install.is_none());
}
#[test]
fn creature_vehicle_add_to_map_already_in_world_does_not_install_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(399, 39901, true);
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9002, true);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.creature_vehicle_install.is_none());
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    let kit = stored.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id, 9002);
    assert!(!kit.installed);
}
#[test]
fn creature_add_to_world_add_to_map_tail_initializes_clears_move_and_visibility_flags_like_cpp() {
    let mut map = test_map();
    let mut stale_creature = test_creature_for_spawn(478, 47801, true);
    stale_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = stale_creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(stale_creature).unwrap())
        .unwrap();
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, Position::xyz(40.0, 41.0, 42.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
            .is_some()
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        1
    );

    let mut creature = test_creature_for_spawn(478, 47801, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900478)));
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9478);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.creature_unit_add_to_world.is_some());
    assert!(outcome.creature_search_formation.is_some());
    assert!(outcome.creature_aim_initialize.is_some());
    assert!(outcome.creature_vehicle_reset.is_some());
    assert!(outcome.creature_vehicle_install.is_some());
    assert!(outcome.creature_zone_script_create.is_some());
    let tail = outcome.add_to_map_tail.unwrap();
    assert!(tail.initialize_object_represented);
    assert!(tail.pending_move_state_cleared);
    assert!(!tail.no_pending_move_state);
    assert!(!tail.add_to_active_represented);
    assert!(!tail.add_to_active_skipped_runtime_gap);
    assert!(tail.set_is_new_object_true);
    assert!(tail.update_object_visibility_on_create_represented);
    assert!(tail.update_object_visibility_on_create_runtime_gap);
    assert!(tail.set_is_new_object_false);
    assert!(!tail.final_is_new_object);
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
            .is_none()
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    let drain = map.move_all_creatures_in_move_list_like_cpp();
    assert_eq!(drain.processed, 0);
    assert_eq!(drain.relocated, 0);
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(stored.unit().world().object().is_in_world());
    assert!(!stored.unit().world().object().is_new_object());
}
#[test]
fn add_to_map_generic_creature_and_already_in_world_do_not_overclaim_tail_like_cpp() {
    let mut map = test_map();
    let generic = world_object_with_counter(HighGuid::Creature, 47803, 571, 7, false);
    let generic_guid = generic.guid();
    map.insert_map_object(AccessorObjectKind::Creature, generic)
        .unwrap();
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(generic_guid, Position::xyz(60.0, 61.0, 62.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    let generic_again = world_object_with_counter(HighGuid::Creature, 47803, 571, 7, false);

    let generic_outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, generic_again)
        .unwrap();

    assert!(generic_outcome.creature_unit_add_to_world.is_none());
    assert!(generic_outcome.creature_search_formation.is_none());
    assert!(generic_outcome.creature_aim_initialize.is_none());
    assert!(generic_outcome.creature_vehicle_reset.is_none());
    assert!(generic_outcome.creature_vehicle_install.is_none());
    assert!(generic_outcome.creature_zone_script_create.is_none());
    assert!(generic_outcome.add_to_map_tail.is_none());
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, generic_guid)
            .is_some()
    );

    let mut creature = test_creature_for_spawn(479, 47901, true);
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900479)));
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9479);
    let already_guid = creature.guid();
    let already = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(already.already_in_world);
    assert!(already.creature_unit_add_to_world.is_none());
    assert!(already.creature_search_formation.is_none());
    assert!(already.creature_aim_initialize.is_none());
    assert!(already.creature_vehicle_reset.is_none());
    assert!(already.creature_vehicle_install.is_none());
    assert!(already.creature_zone_script_create.is_none());
    assert!(already.add_to_map_tail.is_none());
    let stored = map
        .map_object_record(already_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(stored.unit().world().object().is_in_world());
    assert!(!stored.unit().world().object().is_new_object());
}
#[test]
fn creature_vehicle_remove_from_map_unit_remove_from_world_uninstalls_local_vehicle_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(46701, 4670101, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9101, true);
    let guid = creature.guid();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(added.creature_vehicle_install.is_some());

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(removed.was_in_world);
    let remove = removed.creature_vehicle_remove.unwrap();
    let unit_remove = removed.creature_unit_remove_from_world.unwrap();
    assert_eq!(unit_remove.guid, guid);
    assert!(unit_remove.was_in_world);
    assert_eq!(unit_remove.vehicle_remove, Some(remove));
    assert!(unit_remove.world_object_removed);
    assert_eq!(remove.kit_id, Some(9101));
    assert!(remove.had_kit);
    assert_eq!(remove.previous_installed, Some(true));
    assert!(remove.on_remove_from_world);
    assert!(!remove.send_set_vehicle_rec_id_zero_represented);
    assert!(remove.uninstall_represented);
    assert!(remove.remove_all_passengers_represented);
    assert!(remove.script_on_uninstall_represented);
    assert!(remove.kit_cleared);
    assert!(removed.object.is_some());
}
#[test]
fn creature_vehicle_remove_from_map_delete_keeps_uninstall_evidence_without_object_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(46702, 4670201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9102, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    let remove = removed.creature_vehicle_remove.unwrap();
    let unit_remove = removed.creature_unit_remove_from_world.unwrap();
    assert_eq!(unit_remove.vehicle_remove, Some(remove));
    assert!(unit_remove.world_object_removed);
    assert_eq!(remove.kit_id, Some(9102));
    assert!(remove.kit_cleared);
    assert!(removed.object.is_none());
}
#[test]
fn creature_vehicle_remove_from_map_not_in_world_does_not_consume_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(46703, 4670301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9103, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let stored = map
        .entity_world
        .get_mut(&guid)
        .and_then(MapObjectRecord::creature_mut)
        .unwrap();
    stored
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(removed.creature_unit_remove_from_world.is_none());
    assert!(removed.creature_vehicle_remove.is_none());
    assert!(removed.object.is_some());
}
#[test]
fn creature_vehicle_remove_from_map_non_creature_has_no_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(46704, 4670401);
    gameobject.world_mut().object_mut().remove_from_world();
    let guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(removed.creature_unit_remove_from_world.is_none());
    assert!(removed.creature_vehicle_remove.is_none());
}
#[test]
fn remove_from_map_delete_detaches_creature_loot_authority_before_typed_drop() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_101);
    let mut creature = test_creature_for_spawn(484_101, 4_841_010, true);
    let guid = creature.guid();
    assert!(
        creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 17, player,
            ))
            .installed()
    );
    let retained_authority = creature.loot_authority_like_cpp().clone();
    let lease = poll_immediately_ready(retained_authority.reserve_money_like_cpp(player))
        .expect("the live Creature authority must grant the uncontended lease");
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    assert!(removed.object.is_none());
    assert_eq!(
        retained_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}
#[test]
fn insert_map_object_record_detaches_displaced_creature_authority_for_same_guid() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_103);
    let mut displaced_creature = test_creature_for_spawn(484_103, 4_841_030, true);
    let guid = displaced_creature.guid();
    assert!(
        displaced_creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 23, player,
            ))
            .installed()
    );
    let displaced_authority = displaced_creature.loot_authority_like_cpp().clone();
    map.insert_map_object_record(MapObjectRecord::new_creature(displaced_creature).unwrap())
        .unwrap();
    let lease = poll_immediately_ready(displaced_authority.reserve_money_like_cpp(player))
        .expect("the displaced Creature authority must grant the uncontended lease");

    let mut replacement = test_creature_for_spawn(484_104, 4_841_030, true);
    assert!(
        replacement
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 29, player,
            ))
            .installed()
    );
    let replacement_authority = replacement.loot_authority_like_cpp().clone();
    let displaced = map
        .insert_map_object_record(MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap()
        .expect("same-GUID insert must return the displaced Creature record");

    assert_eq!(
        displaced_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(
        displaced
            .creature()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&displaced_authority)
    );
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement_authority)
    );
    assert_eq!(
        replacement_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}
#[test]
fn remove_list_enqueue_creature_marks_destroyed_cleans_and_keeps_record_like_cpp() {
    let mut map = test_map();
    let spawn_id = 41901;
    let mut creature = test_creature_for_spawn(spawn_id, 4190101, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.add_object_to_remove_list_like_cpp(guid);

    assert_eq!(outcome.guid, guid);
    assert!(outcome.queued);
    assert!(!outcome.duplicate);
    assert_eq!(outcome.cleanup_before_delete_count, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    assert!(
        map.exact_cell_guids_like_cpp(added.cell)
            .grid
            .creatures
            .contains(&guid)
    );
    let creature = map.get_typed_creature(guid).unwrap();
    assert!(creature.unit().world().object().is_destroyed_object());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
}
#[test]
fn remove_list_drain_physically_removes_creature_and_second_cleanup_like_cpp() {
    let mut map = test_map();
    let spawn_id = 41902;
    let mut creature = test_creature_for_spawn(spawn_id, 4190201, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.add_object_to_remove_list_like_cpp(guid).queued);

    let outcome = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(outcome.processed, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.creature_second_cleanup_count, 1);
    assert_eq!(outcome.missing_or_stale, 0);
    assert_eq!(outcome.remove_errors, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 0);
    assert!(
        !map.exact_cell_guids_like_cpp(added.cell)
            .grid
            .creatures
            .contains(&guid)
    );
}
#[test]
fn unload_grid_at_false_consumes_creature_gameobject_area_trigger_move_lists_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44106, 4410601, true);
    let creature_guid = creature.guid();
    let gameobject = test_gameobject_for_spawn(44106, 4410602);
    let gameobject_guid = gameobject.world().guid();
    let area_trigger = test_area_trigger_for_update(4410603, 10_000, true);
    let area_guid = area_trigger.world().guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
    .unwrap();
    let same_cell_new_position = Position::xyz(2.0, 3.0, 4.0);
    map.add_creature_to_move_list_like_cpp(creature_guid, same_cell_new_position);
    map.add_game_object_to_move_list_like_cpp(gameobject_guid, same_cell_new_position);
    map.add_area_trigger_to_move_list_like_cpp(area_guid, same_cell_new_position);

    let unload_position = Position::xyz(3_000.0, 3_000.0, 0.0);
    map.load_grid(unload_position.x, unload_position.y);
    let unload_cell = Cell::from_world(unload_position.x, unload_position.y);
    let unload_grid = GridCoord::new(unload_cell.grid_x(), unload_cell.grid_y());

    assert!(map.unload_grid_at(unload_grid, false));

    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        0
    );
    assert_eq!(
        map.map_object(creature_guid).unwrap().position(),
        same_cell_new_position
    );
    assert_eq!(
        map.map_object(gameobject_guid).unwrap().position(),
        same_cell_new_position
    );
    assert_eq!(
        map.map_object(area_guid).unwrap().position(),
        same_cell_new_position
    );
    assert_eq!(map.lifecycle().evacuates, 1);
}
#[test]
fn creature_update_in_world_consumes_runtime_plan_once_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350101);
    let creature = test_creature_for_spawn(43501, 4350101, true);
    assert!(creature.trigger_just_appeared());
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let first = map.update_creature_like_cpp(
        creature_guid,
        1,
        1_000,
        CreatureRuntimeUpdateContext::default(),
    );
    let second = map.update_creature_like_cpp(
        creature_guid,
        1,
        1_001,
        CreatureRuntimeUpdateContext::default(),
    );

    assert_eq!(first.status, CreatureUpdateStatusLikeCpp::Updated);
    assert!(
        first
            .plan
            .as_ref()
            .unwrap()
            .contains(wow_entities::CreatureRuntimeAction::NotifyJustAppeared)
    );
    assert!(
        !map.map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
    assert_eq!(second.status, CreatureUpdateStatusLikeCpp::Updated);
    assert!(
        !second
            .plan
            .as_ref()
            .unwrap()
            .contains(wow_entities::CreatureRuntimeAction::NotifyJustAppeared)
    );
}
#[test]
fn creature_update_not_in_world_skips_without_mutation_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350201);
    let mut creature = test_creature_for_spawn(43502, 4350201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    assert!(creature.trigger_just_appeared());
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.update_creature_like_cpp(
        creature_guid,
        1,
        1_000,
        CreatureRuntimeUpdateContext::default(),
    );

    assert_eq!(outcome.status, CreatureUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.actions_recorded, 0);
    assert!(
        map.map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
}
#[test]
fn creature_update_snapshot_ignores_gameobject_areatrigger_dynamicobject_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350301);
    let mut creature = test_creature_for_spawn(43503, 4350301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(43504, 4350302)).unwrap(),
    )
    .unwrap();
    let mut area_trigger = test_area_trigger_for_update(4350303, 10, true);
    area_trigger.set_duration(10);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4350304);
    dynamic_object.set_duration(10);
    let dynamic_object_guid = dynamic_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let mut observed_snapshot = None;
    let summary = map.update_creatures_like_cpp(1, 1_000, |guid, snapshot| {
        observed_snapshot = Some((guid, snapshot));
        CreatureRuntimeUpdateContext::default()
    });

    assert_eq!(summary.visited, 1);
    assert_eq!(summary.updated, 1);
    assert_eq!(summary.skipped_non_creature, 0);
    assert!(summary.actions_recorded > 0);
    let (observed_guid, observed_snapshot) =
        observed_snapshot.expect("the loaded Creature should be snapshotted exactly once");
    assert_eq!(observed_guid, creature_guid);
    assert_eq!(observed_snapshot.guid, creature_guid);
    assert_eq!(observed_snapshot.position, Position::xyz(1.0, 2.0, 3.0));
    assert_eq!(
        (observed_snapshot.health, observed_snapshot.max_health),
        (100, 100)
    );
    assert!(observed_snapshot.is_alive);
    assert!(observed_snapshot.is_in_world);
    assert!(
        !map.map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
    assert_eq!(
        map.map_object_record(area_trigger_guid)
            .unwrap()
            .area_trigger()
            .unwrap()
            .duration_ms(),
        10
    );
    assert_eq!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        10
    );
}
#[test]
fn creature_update_snapshot_skips_unloaded_grid_records_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350311);
    let mut creature = test_creature_for_spawn(43503, 4350311, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(map.unload_grid_at(added.grid, true));
    assert!(map.get_ngrid(added.grid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());

    let summary = map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
        CreatureRuntimeUpdateContext::default()
    });

    assert_eq!(summary.visited, 0);
    assert_eq!(summary.updated, 0);
    assert_eq!(summary.actions_recorded, 0);
}
#[test]
fn creature_update_context_resolver_affects_plan_like_cpp() {
    let mut default_map = test_map();
    let mut default_creature = test_creature_for_spawn(43504, 4350401, true);
    default_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    default_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(default_creature).unwrap(),
        )
        .unwrap();
    let default_summary = default_map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
        CreatureRuntimeUpdateContext::default()
    });

    let mut disabled_ai_map = test_map();
    let mut disabled_ai_creature = test_creature_for_spawn(43504, 4350402, true);
    disabled_ai_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    disabled_ai_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(disabled_ai_creature).unwrap(),
        )
        .unwrap();
    let disabled_ai_summary =
        disabled_ai_map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
            CreatureRuntimeUpdateContext {
                ai_enabled: false,
                ..CreatureRuntimeUpdateContext::default()
            }
        });

    assert_eq!(default_summary.visited, 1);
    assert_eq!(disabled_ai_summary.visited, 1);
    assert!(default_summary.actions_recorded > disabled_ai_summary.actions_recorded);
}
#[test]
fn player_remove_from_world_viewpoint_creature_consumes_shared_vision_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4930201);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 493020, 4930202);
    player.set_farsight_object_like_cpp(target_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.get_typed_creature_mut(target_guid)
        .unwrap()
        .unit_mut()
        .add_player_to_vision_like_cpp(player_guid);

    let removed = map.remove_from_map_like_cpp(player_guid, false).unwrap();

    let cleanup = removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedUnitViewpoint
    );
    let player_set_viewpoint = cleanup.player_set_viewpoint.unwrap();
    assert_eq!(
        player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(!player_set_viewpoint.update_visibility_requested);
    assert!(player_set_viewpoint.set_seer_requested);
    assert_eq!(
        player_set_viewpoint.set_world_object,
        Some(SetWorldObjectOutcomeLikeCpp {
            guid: target_guid,
            on: false,
            status: SetWorldObjectStatusLikeCpp::Delegated(
                AddObjectToSwitchListStatusLikeCpp::Queued
            ),
        })
    );
    assert!(map.map_object_record(player_guid).is_none());
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), Some(false));
    assert!(removed.object.unwrap().guid() == player_guid);
}
#[test]
fn set_world_object_like_cpp_creature_in_world_enqueues_and_drain_executes() {
    let mut map = test_map();
    let spawn_id = 421010;
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4210101);

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(outcome.guid, guid);
    assert_eq!(outcome.on, true);
    assert_eq!(
        outcome.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 1);
    assert_eq!(map.pending_switch_like_cpp(guid), Some(true));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
    assert!(
        local_cell_for_switch(&map, grid, cell)
            .grid_objects
            .creatures
            .contains(&guid)
    );

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
