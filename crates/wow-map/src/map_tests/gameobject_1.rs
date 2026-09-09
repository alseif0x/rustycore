//! Gameobject scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn dynamic_tree_gameobject_add_consumes_explicit_model_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45101, 4510101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let insert = outcome
        .gameobject_model_insert
        .expect("explicit represented model should insert dynamic-tree key");
    assert_eq!(
        insert.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(insert.model_count_before, 0);
    assert_eq!(insert.model_count_after, 1);
    assert_eq!(insert.unbalanced_before, 0);
    assert_eq!(insert.unbalanced_after, 1);
    let collision = outcome
        .gameobject_collision_enable
        .expect("represented model should record EnableCollision evidence");
    assert!(collision.represented_model_present);
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.previous_collision_enabled, None);
    assert_eq!(collision.new_collision_enabled, Some(false));
    assert!(map.contains_gameobject_model_like_cpp(key));
}
#[test]
fn add_to_map_exact_gameobject_preinserts_canonical_store_and_spawn_index_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(47901, 4790101);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert_eq!(
        outcome.gameobject_store_inserted_before_add_to_world,
        Some(true)
    );
    assert_eq!(
        outcome.gameobject_spawn_indexed_before_add_to_world,
        Some(true)
    );
    assert!(outcome.gameobject_model_insert.is_some());
    assert!(outcome.gameobject_collision_enable.is_some());
    assert!(outcome.add_to_map_tail.is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .is_some()
    );
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(47901)
            .contains(&guid)
    );
}
#[test]
fn add_to_map_exact_gameobject_model_collision_and_world_state_mutate_canonical_record_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(47902, 4790201);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_state(GoState::Ready);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let canonical = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(canonical.world().object().is_in_world());
    assert!(!canonical.world().object().is_new_object());
    assert_eq!(
        canonical.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
    let tail = outcome.add_to_map_tail.unwrap();
    assert!(tail.set_is_new_object_true);
    assert!(tail.set_is_new_object_false);
    assert!(!tail.final_is_new_object);
}
#[test]
fn active_non_player_add_remove_gameobject_updates_set_and_unload_lock_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48501, 4850101);
    let guid = gameobject.world().guid();
    let respawn_position = gameobject.stationary_position();
    let respawn_cell = Cell::from_world(respawn_position.x, respawn_position.y);
    let respawn_grid = GridCoord::new(respawn_cell.grid_x(), respawn_cell.grid_y());
    map.ensure_grid_loaded(&cell_from_grid_center(respawn_grid));
    gameobject.world_mut().set_active(true);
    gameobject.world_mut().object_mut().remove_from_world();

    let add = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    let add_active = add
        .add_to_map_tail
        .unwrap()
        .add_to_active
        .expect("active exact typed GameObject should consume AddToActive seam");

    assert_eq!(
        add_active.status,
        ActiveNonPlayerMutationStatusLikeCpp::Mutated
    );
    assert!(add_active.inserted_in_active_set);
    assert!(map.is_active_non_player_like_cpp(guid));
    assert_eq!(map.active_non_players_count_like_cpp(), 1);
    let add_lock = add_active.unload_lock.unwrap();
    assert_eq!(add_lock.spawn_id, 48501);
    assert_eq!(add_lock.respawn_grid, Some(respawn_grid));
    assert!(add_lock.lock_incremented);
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        1
    );

    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_active = remove
        .remove_from_active
        .expect("active exact typed GameObject should consume RemoveFromActive seam");
    assert!(remove_active.removed_from_active_set);
    assert!(!map.is_active_non_player_like_cpp(guid));
    assert_eq!(map.active_non_players_count_like_cpp(), 0);
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        0
    );
}
#[test]
fn add_to_map_gameobject_non_exact_paths_do_not_emit_typed_preinsert_evidence_like_cpp() {
    let mut already_map = test_map();
    let mut already_gameobject = test_gameobject_for_spawn(47903, 4790301);
    already_gameobject.set_represented_gameobject_model_like_cpp(true);
    let already_outcome = already_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(already_gameobject).unwrap(),
        )
        .unwrap();
    assert!(already_outcome.already_in_world);
    assert_eq!(
        already_outcome.gameobject_store_inserted_before_add_to_world,
        None
    );
    assert_eq!(
        already_outcome.gameobject_spawn_indexed_before_add_to_world,
        None
    );

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4790302, 571, 7, false);
    let generic_outcome = generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();
    assert!(!generic_outcome.already_in_world);
    assert_eq!(
        generic_outcome.gameobject_store_inserted_before_add_to_world,
        None
    );
    assert_eq!(
        generic_outcome.gameobject_spawn_indexed_before_add_to_world,
        None
    );
}
#[test]
fn gameobject_zone_script_create_precedes_store_insert_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48001, 4800101);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let zone_script = outcome
        .gameobject_zone_script_create
        .expect("exact typed GameObject should expose represented ZoneScript create boundary");
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback_boundary);
    assert!(!zone_script.script_dispatch_represented);
    assert!(!zone_script.object_store_present_before_callback);
    assert!(!zone_script.spawn_index_present_before_callback);
    assert_eq!(
        outcome.gameobject_store_inserted_before_add_to_world,
        Some(true)
    );
    assert_eq!(
        outcome.gameobject_spawn_indexed_before_add_to_world,
        Some(true)
    );
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .is_some()
    );
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(48001)
            .contains(&guid)
    );
}
#[test]
fn gameobject_zone_script_create_skips_already_in_world_and_generic_paths_like_cpp() {
    let mut already_map = test_map();
    let already_gameobject = test_gameobject_for_spawn(48002, 4800201);
    let already_outcome = already_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(already_gameobject).unwrap(),
        )
        .unwrap();
    assert!(already_outcome.already_in_world);
    assert!(already_outcome.gameobject_zone_script_create.is_none());

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4800202, 571, 7, false);
    let generic_outcome = generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();
    assert!(!generic_outcome.already_in_world);
    assert!(generic_outcome.gameobject_zone_script_create.is_none());

    let mut non_gameobject_map = test_map();
    let mut creature = test_creature_for_spawn(48003, 4800301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let non_gameobject = non_gameobject_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(non_gameobject.gameobject_zone_script_create.is_none());
}
#[test]
fn gameobject_zone_script_remove_snapshots_before_model_spawn_unindex_like_cpp() {
    let mut map = test_map();
    let spawn_id = 48101;
    let mut gameobject = test_gameobject_for_spawn(spawn_id, 4810101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    assert!(map.map_object_record(guid).is_some());
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            .contains(&guid)
    );
    assert!(map.contains_gameobject_model_like_cpp(key));

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();

    let zone_script = outcome.gameobject_zone_script_remove.expect(
        "exact typed in-world GameObject should expose represented ZoneScript remove boundary",
    );
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback_boundary);
    assert!(!zone_script.script_dispatch_represented);
    assert!(zone_script.model_remove_pending_before_callback);
    assert!(zone_script.spawn_index_present_before_callback);
    assert!(outcome.gameobject_model_remove.is_some());
    assert!(map.map_object_record(guid).is_none());
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            .is_empty()
    );
    assert!(!map.contains_gameobject_model_like_cpp(key));
}
#[test]
fn gameobject_zone_script_remove_skips_generic_and_not_in_world_like_cpp() {
    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4810201, 571, 7, false);
    let generic_guid = generic_object.guid();
    generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();

    let generic_removed = generic_map
        .remove_from_map_like_cpp(generic_guid, true)
        .unwrap();

    assert!(generic_removed.gameobject_zone_script_remove.is_none());

    let mut not_in_world_map = test_map();
    let mut not_in_world_gameobject = test_gameobject_for_spawn(48102, 4810202);
    let not_in_world_guid = not_in_world_gameobject.world().guid();
    not_in_world_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(
            MapObjectRecord::new_game_object(not_in_world_gameobject).unwrap(),
        )
        .unwrap();

    let not_in_world_removed = not_in_world_map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();

    assert!(not_in_world_removed.gameobject_zone_script_remove.is_none());
}
#[test]
fn gameobject_add_to_owner_registers_owner_list_and_guid_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4820601);
    let owner_guid = owner.guid();
    let gameobject = test_gameobject_for_spawn(48206, 4820602);
    let guid = gameobject.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let add_owner = map.gameobject_add_to_owner_like_cpp(owner_guid, guid);

    assert_eq!(add_owner.guid, guid);
    assert_eq!(add_owner.owner_guid, owner_guid);
    assert!(add_owner.owner_found_as_unit_like);
    assert!(add_owner.gameobject_found);
    assert_eq!(add_owner.owner_guid_before, ObjectGuid::EMPTY);
    assert_eq!(add_owner.owner_guid_after, owner_guid);
    assert!(add_owner.gameobject_owner_empty_before);
    assert!(add_owner.registered_owned_gameobject);
    assert!(add_owner.owner_guid_set);
    assert!(!add_owner.cooldown_start_represented);
    assert!(!add_owner.creature_ai_callback_represented);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![guid]
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), owner_guid);
}
#[test]
fn gameobject_add_to_owner_noops_for_missing_owner_or_preowned_gameobject_like_cpp() {
    let mut preowned_map = test_map();
    let owner = test_player_for_viewpoint(4820901);
    let owner_guid = owner.guid();
    let existing_owner_guid = ObjectGuid::create_player(1, 4820903);
    let mut gameobject = test_gameobject_for_spawn(48209, 4820902);
    let guid = gameobject.world().guid();
    gameobject.set_owner_guid_like_cpp(existing_owner_guid);

    preowned_map
        .insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    preowned_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let preowned = preowned_map.gameobject_add_to_owner_like_cpp(owner_guid, guid);
    assert!(preowned.owner_found_as_unit_like);
    assert!(preowned.gameobject_found);
    assert_eq!(preowned.owner_guid_before, existing_owner_guid);
    assert_eq!(preowned.owner_guid_after, existing_owner_guid);
    assert!(!preowned.gameobject_owner_empty_before);
    assert!(!preowned.registered_owned_gameobject);
    assert!(!preowned.owner_guid_set);

    let mut missing_owner_map = test_map();
    let gameobject = test_gameobject_for_spawn(48210, 4821002);
    let guid = gameobject.world().guid();
    missing_owner_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let missing = missing_owner_map
        .gameobject_add_to_owner_like_cpp(ObjectGuid::create_player(1, 4821001), guid);
    assert!(!missing.owner_found_as_unit_like);
    assert!(missing.gameobject_found);
    assert!(!missing.registered_owned_gameobject);
    assert_eq!(missing.owner_guid_after, ObjectGuid::EMPTY);
}
#[test]
fn gameobject_add_to_owner_slot_sets_effect_summon_slot_tail_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4821101);
    let owner_guid = owner.guid();
    let gameobject = test_gameobject_for_spawn(48211, 4821102);
    let guid = gameobject.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let add_slot = map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 2);

    assert!(add_slot.add_owner.registered_owned_gameobject);
    assert_eq!(add_slot.slot, 2);
    assert_eq!(add_slot.slot_previous_guid, ObjectGuid::EMPTY);
    assert!(add_slot.slot_set);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(owner.unit().subsystems().control.gameobject_slots[2], guid);
}
#[test]
fn gameobject_add_to_owner_slot_keeps_cpp_guards_visible() {
    let mut invalid_slot_map = test_map();
    let owner = test_player_for_viewpoint(4821201);
    let owner_guid = owner.guid();
    let gameobject = test_gameobject_for_spawn(48212, 4821202);
    let guid = gameobject.world().guid();
    invalid_slot_map
        .insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    invalid_slot_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let invalid_slot = invalid_slot_map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 99);
    assert!(invalid_slot.add_owner.registered_owned_gameobject);
    assert!(!invalid_slot.slot_set);
    let owner = invalid_slot_map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .gameobject_slots
            .iter()
            .all(ObjectGuid::is_empty)
    );

    let mut preowned_map = test_map();
    let owner = test_player_for_viewpoint(4821301);
    let owner_guid = owner.guid();
    let existing_owner_guid = ObjectGuid::create_player(1, 4821303);
    let mut gameobject = test_gameobject_for_spawn(48213, 4821302);
    let guid = gameobject.world().guid();
    gameobject.set_owner_guid_like_cpp(existing_owner_guid);
    preowned_map
        .insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    preowned_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let preowned_slot = preowned_map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 1);
    assert!(!preowned_slot.add_owner.registered_owned_gameobject);
    assert!(!preowned_slot.slot_set);
    let owner = preowned_map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[1],
        ObjectGuid::EMPTY
    );
}
#[test]
fn gameobject_prepare_owner_slot_for_summon_recast_preserves_owner_auras_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821601);
    let owner_guid = owner.guid();
    let spell_id = 4821610;
    let recast_aura = AppliedAuraRef::new(spell_id, owner_guid, 0, 0x1);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(recast_aura);
    let mut gameobject = test_gameobject_for_spawn(48216, 4821602);
    let guid = gameobject.world().guid();
    gameobject.set_spell_id(spell_id);
    gameobject.set_respawn_time(60);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    assert!(
        map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 0)
            .slot_set
    );

    let cleanup = map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 0, spell_id);

    assert_eq!(cleanup.owner_guid, owner_guid);
    assert_eq!(cleanup.slot, 0);
    assert_eq!(cleanup.slot_guid_before, guid);
    assert!(cleanup.slot_had_guid);
    assert!(cleanup.gameobject_found);
    assert!(cleanup.recast_spell_id_cleared);
    assert!(cleanup.unit_pointer_owner_match);
    assert!(cleanup.respawn_time_cleared);
    assert!(cleanup.slot_cleared);
    assert!(!cleanup.cooldown_event_represented);
    let remove_owner = cleanup.remove_from_owner.unwrap();
    assert_eq!(remove_owner.spell_id, 0);
    assert!(remove_owner.unit_owned_gameobject_list_removed);
    assert!(remove_owner.unit_object_slot_cleared);
    assert!(!remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 0);
    assert!(cleanup.delete_outcome.is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[0],
        ObjectGuid::EMPTY
    );
    assert!(owner.unit().subsystems().auras.has_applied(recast_aura));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(gameobject.spell_id(), 0);
    assert_eq!(gameobject.respawn_time(), 0);
}
#[test]
fn gameobject_prepare_owner_slot_for_summon_different_spell_removes_old_aura_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821701);
    let owner_guid = owner.guid();
    let old_spell_id = 4821710;
    let new_spell_id = 4821720;
    let old_aura = AppliedAuraRef::new(old_spell_id, owner_guid, 0, 0x1);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(old_aura);
    let mut gameobject = test_gameobject_for_spawn(48217, 4821702);
    let guid = gameobject.world().guid();
    gameobject.set_spell_id(old_spell_id);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    assert!(
        map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 1)
            .slot_set
    );

    let cleanup =
        map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 1, new_spell_id);

    assert!(cleanup.gameobject_found);
    assert!(!cleanup.recast_spell_id_cleared);
    assert!(cleanup.unit_pointer_owner_match);
    let remove_owner = cleanup.remove_from_owner.unwrap();
    assert_eq!(remove_owner.spell_id, old_spell_id);
    assert!(remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 1);
    assert!(cleanup.delete_outcome.is_some());

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(!owner.unit().subsystems().auras.has_applied(old_aura));
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[1],
        ObjectGuid::EMPTY
    );
}
#[test]
fn gameobject_prepare_owner_slot_for_summon_clears_missing_guid_without_delete_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821801);
    let owner_guid = owner.guid();
    let missing_guid = guid(HighGuid::GameObject, 4821802);
    assert!(
        owner
            .unit_mut()
            .subsystems_mut()
            .control
            .set_gameobject_slot(3, missing_guid)
    );
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();

    let cleanup = map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 3, 4821810);

    assert!(cleanup.owner_found_as_unit_like);
    assert_eq!(cleanup.slot_guid_before, missing_guid);
    assert!(cleanup.slot_had_guid);
    assert!(!cleanup.gameobject_found);
    assert!(!cleanup.recast_spell_id_cleared);
    assert!(!cleanup.unit_pointer_owner_match);
    assert!(cleanup.remove_from_owner.is_none());
    assert!(!cleanup.respawn_time_cleared);
    assert!(cleanup.delete_outcome.is_none());
    assert!(cleanup.slot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[3],
        ObjectGuid::EMPTY
    );

    let invalid_slot =
        map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 99, 4821810);
    assert!(invalid_slot.owner_found_as_unit_like);
    assert_eq!(invalid_slot.slot_guid_before, ObjectGuid::EMPTY);
    assert!(!invalid_slot.slot_had_guid);
    assert!(!invalid_slot.slot_cleared);
}
#[test]
fn gameobject_prepare_owner_slot_for_summon_owner_mismatch_keeps_object_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821901);
    let owner_guid = owner.guid();
    let other_owner_guid = ObjectGuid::create_player(1, 4821903);
    let spell_id = 4821910;
    let guid = guid(HighGuid::GameObject, 4821902);
    assert!(
        owner
            .unit_mut()
            .subsystems_mut()
            .control
            .set_gameobject_slot(2, guid)
    );
    let mut gameobject = test_gameobject_for_spawn(48219, 4821902);
    gameobject.set_owner_guid_like_cpp(other_owner_guid);
    gameobject.set_spell_id(spell_id);
    gameobject.set_respawn_time(90);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let cleanup = map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 2, spell_id);

    assert!(cleanup.gameobject_found);
    assert!(cleanup.recast_spell_id_cleared);
    assert!(!cleanup.unit_pointer_owner_match);
    assert!(cleanup.remove_from_owner.is_none());
    assert!(!cleanup.respawn_time_cleared);
    assert!(cleanup.delete_outcome.is_none());
    assert!(cleanup.slot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);

    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), other_owner_guid);
    assert_eq!(gameobject.spell_id(), 0);
    assert_eq!(gameobject.respawn_time(), 90);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[2],
        ObjectGuid::EMPTY
    );
}
#[test]
fn gameobject_summon_object_for_owner_slot_creates_adds_and_slots_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4822001);
    let owner_guid = owner.guid();
    owner.unit_mut().set_faction(1735);
    owner.unit_mut().set_level(47);
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    let position = Position::new(10.0, 11.0, 12.0, 1.25);
    let template = summon_gameobject_template_like_cpp(4822002, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.gameobject_summon_object_for_owner_slot_like_cpp(
        owner_guid, 1, 4822010, template, position, 12_345,
    );

    assert_eq!(
        outcome.status,
        GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
    );
    assert_eq!(outcome.low_guid, Some(1));
    let guid = outcome
        .guid
        .expect("summon should allocate a GameObject guid");
    assert_eq!(guid.entry(), 4822002);
    assert_eq!(outcome.respawn_time_secs, Some(12));
    assert_eq!(outcome.caster_faction, Some(1735));
    assert_eq!(outcome.caster_level, Some(47));
    assert!(!outcome.phase_inherit_represented);
    assert!(outcome.execute_log_represented);
    assert!(!outcome.cooldown_event_represented);
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| add.inserted));
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| {
        add.gameobject_store_inserted_before_add_to_world == Some(true)
            && add
                .add_to_map_tail
                .as_ref()
                .is_some_and(|tail| tail.update_object_visibility_on_create_represented)
    }));
    let add_owner_slot = outcome.add_owner_slot.unwrap();
    assert!(add_owner_slot.add_owner.registered_owned_gameobject);
    assert!(add_owner_slot.slot_set);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(owner.unit().subsystems().control.gameobject_slots[1], guid);
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![guid]
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), owner_guid);
    assert_eq!(gameobject.spell_id(), 4822010);
    assert_eq!(gameobject.respawn_time(), 12);
    assert_eq!(gameobject.data().faction_template, 1735);
    assert_eq!(gameobject.data().level, 47);
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(gameobject.world().position(), position);
    assert_eq!(
        gameobject.local_rotation_like_cpp(),
        gameobject_local_rotation_from_orientation_like_cpp(position.orientation)
    );
    assert_eq!(gameobject.spawn_id(), 0);
    assert!(!gameobject.respawn_compatibility_mode());
}
#[test]
fn gameobject_summon_object_for_owner_slot_missing_owner_does_not_consume_guid_like_cpp() {
    let mut map = test_map();
    let template = summon_gameobject_template_like_cpp(4822102, 5);

    let outcome = map.gameobject_summon_object_for_owner_slot_like_cpp(
        ObjectGuid::create_player(1, 4822101),
        0,
        4822110,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        -1,
    );

    assert_eq!(
        outcome.status,
        GameObjectSummonObjectForOwnerSlotStatusLikeCpp::MissingOwner
    );
    assert!(outcome.guid.is_none());
    assert!(outcome.add_to_map.is_none());
    assert!(outcome.add_owner_slot.is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
}
#[test]
fn world_object_summon_gameobject_position_keeps_explicit_coords_like_cpp() {
    let source = Position::new(10.0, 20.0, 30.0, 1.5);

    let outcome = world_object_summon_gameobject_position_from_coords_like_cpp(
        source, 2.0, 4.0, 5.0, 6.0, 0.75,
    );

    assert_eq!(outcome.position, Position::new(4.0, 5.0, 6.0, 0.75));
    assert!(!outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(!outcome.collision_los_adjustment_represented);
}
#[test]
fn world_object_summon_gameobject_position_zero_coords_use_close_point_like_cpp() {
    let source = Position::new(10.0, 20.0, 30.0, 0.0);

    let outcome = world_object_summon_gameobject_position_from_coords_like_cpp(
        source, 1.25, 0.0, 0.0, 0.0, 0.75,
    );

    assert_eq!(outcome.position, Position::new(12.5, 20.0, 30.0, 0.0));
    assert!(outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(!outcome.collision_los_adjustment_represented);

    let source = Position::new(10.0, 20.0, 30.0, std::f32::consts::FRAC_PI_2);
    let y_outcome = world_object_summon_gameobject_position_from_coords_like_cpp(
        source, 2.0, 0.0, 0.0, 0.0, 0.0,
    );
    assert!((y_outcome.position.x - 10.0).abs() < 0.00001);
    assert!((y_outcome.position.y - 24.0).abs() < 0.00001);
    assert_eq!(y_outcome.position.z, 30.0);
    assert_eq!(y_outcome.position.orientation, std::f32::consts::FRAC_PI_2);
}
