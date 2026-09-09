//! Gameobject scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_object_summon_gameobject_player_owner_branch_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4822201);
    let owner_guid = owner.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    let position = Position::new(4.0, 5.0, 6.0, 0.75);
    let template = summon_gameobject_template_like_cpp(4822202, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.world_object_summon_gameobject_like_cpp(
        owner_guid,
        template,
        position,
        45,
        GameObjectSummonTypeLikeCpp::TimedDespawn,
    );

    assert_eq!(
        outcome.status,
        WorldObjectSummonGameObjectStatusLikeCpp::CreatedAddedToMap
    );
    assert_eq!(outcome.low_guid, Some(1));
    assert!(!outcome.phase_inherit_represented);
    assert!(!outcome.spawned_by_default_forced_false);
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| add.inserted));
    let add_owner = outcome
        .add_owner
        .expect("player summoner always calls Unit::AddGameObject");
    assert!(add_owner.registered_owned_gameobject);
    assert!(add_owner.owner_guid_set);
    let guid = outcome.guid.unwrap();

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
    assert_eq!(gameobject.respawn_time(), 45);
    assert_eq!(gameobject.world().position(), position);
    assert!(gameobject.world().object().is_in_world());
    assert!(!gameobject.spawned_by_default());
    assert_eq!(gameobject.spell_id(), 0);
}
#[test]
fn world_object_summon_gameobject_unit_timed_despawn_forces_non_default_like_cpp() {
    let mut map = test_map();
    let owner = test_creature_for_spawn(48223, 4822301, true);
    let owner_guid = owner.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(owner).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822302, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.world_object_summon_gameobject_like_cpp(
        owner_guid,
        template,
        Position::xyz(7.0, 8.0, 9.0),
        12,
        GameObjectSummonTypeLikeCpp::TimedDespawn,
    );

    assert_eq!(
        outcome.status,
        WorldObjectSummonGameObjectStatusLikeCpp::CreatedAddedToMap
    );
    assert!(outcome.add_owner.is_none());
    assert!(outcome.spawned_by_default_forced_false);
    let guid = outcome.guid.unwrap();
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert!(!gameobject.spawned_by_default());
    assert_eq!(gameobject.respawn_time(), 12);
}
#[test]
fn world_object_summon_gameobject_not_in_world_does_not_consume_guid_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4822401);
    let owner_guid = owner.guid();
    owner
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822402, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.world_object_summon_gameobject_like_cpp(
        owner_guid,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        30,
        GameObjectSummonTypeLikeCpp::TimedOrCorpseDespawn,
    );

    assert_eq!(
        outcome.status,
        WorldObjectSummonGameObjectStatusLikeCpp::SummonerNotInWorld
    );
    assert!(outcome.guid.is_none());
    assert!(outcome.add_to_map.is_none());
    assert!(outcome.add_owner.is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
}
#[test]
fn unit_remove_gameobjects_by_spell_filters_owner_list_without_slot_cleanup_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4821401);
    let owner_guid = owner.guid();
    let mut matched_gameobject = test_gameobject_for_spawn(48214, 4821402);
    let matched_guid = matched_gameobject.world().guid();
    matched_gameobject.set_spell_id(4821410);
    let mut kept_gameobject = test_gameobject_for_spawn(48214, 4821403);
    let kept_guid = kept_gameobject.world().guid();
    kept_gameobject.set_spell_id(4821420);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(matched_gameobject).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(kept_gameobject).unwrap())
        .unwrap();

    assert!(
        map.gameobject_add_to_owner_slot_like_cpp(owner_guid, matched_guid, 1)
            .slot_set
    );
    assert!(
        map.gameobject_add_to_owner_like_cpp(owner_guid, kept_guid)
            .registered_owned_gameobject
    );

    let remove_by_spell = map.unit_remove_gameobjects_by_spell_like_cpp(owner_guid, 4821410, false);

    assert_eq!(remove_by_spell.owner_guid, owner_guid);
    assert_eq!(remove_by_spell.spell_id, 4821410);
    assert!(!remove_by_spell.delete_requested);
    assert!(remove_by_spell.owner_found_as_unit_like);
    assert_eq!(remove_by_spell.owned_entries_before, 2);
    assert_eq!(remove_by_spell.matched_entries, 1);
    assert_eq!(remove_by_spell.owner_guid_cleared, 1);
    assert_eq!(remove_by_spell.respawn_time_cleared, 0);
    assert_eq!(remove_by_spell.owner_list_entries_removed, 1);
    assert_eq!(remove_by_spell.delete_outcomes, 0);
    assert!(!remove_by_spell.object_slot_cleanup_represented);
    assert!(!remove_by_spell.aura_cleanup_represented);
    assert!(!remove_by_spell.cooldown_event_represented);
    assert!(!remove_by_spell.creature_ai_callback_represented);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![kept_guid]
    );
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[1],
        matched_guid
    );
    let matched = map
        .map_object_record(matched_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(matched.owner_guid(), ObjectGuid::EMPTY);
    let kept = map
        .map_object_record(kept_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(kept.owner_guid(), owner_guid);
}
#[test]
fn unit_remove_gameobjects_by_spell_delete_path_sets_respawn_zero_and_delete_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4821501);
    let owner_guid = owner.guid();
    let mut gameobject = test_gameobject_for_spawn(48215, 4821502);
    let guid = gameobject.world().guid();
    gameobject.set_spell_id(4821510);
    gameobject.set_respawn_time(60);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    assert!(
        map.gameobject_add_to_owner_like_cpp(owner_guid, guid)
            .registered_owned_gameobject
    );

    let remove_by_spell = map.unit_remove_gameobjects_by_spell_like_cpp(owner_guid, 0, true);

    assert!(remove_by_spell.delete_requested);
    assert_eq!(remove_by_spell.owned_entries_before, 1);
    assert_eq!(remove_by_spell.matched_entries, 1);
    assert_eq!(remove_by_spell.owner_guid_cleared, 1);
    assert_eq!(remove_by_spell.respawn_time_cleared, 1);
    assert_eq!(remove_by_spell.owner_list_entries_removed, 1);
    assert_eq!(remove_by_spell.delete_outcomes, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(gameobject.respawn_time(), 0);
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
}
#[test]
fn gameobject_remove_from_owner_clears_owner_before_model_remove_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4820101);
    let owner_guid = owner.guid();

    let mut gameobject = test_gameobject_for_spawn(48201, 4820102);
    let guid = gameobject.world().guid();
    let removed_aura = AppliedAuraRef::new(482001, owner_guid, 0, 0x1);
    let removed_owned_aura = OwnedAuraRef::new(482001, owner_guid, None);
    let kept_aura = AppliedAuraRef::new(482002, owner_guid, 1, 0x1);
    owner
        .unit_mut()
        .subsystems_mut()
        .control
        .register_owned_gameobject_like_cpp(guid);
    assert!(
        owner
            .unit_mut()
            .subsystems_mut()
            .control
            .set_gameobject_slot(1, guid)
    );
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(removed_aura);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_owned(removed_owned_aura);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(kept_aura);
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();

    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_owner_guid_like_cpp(owner_guid);
    gameobject.set_spell_id(482001);
    gameobject.set_represented_gameobject_model_like_cpp(true);
    map.insert_gameobject_model_like_cpp(key);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_owner = outcome
        .gameobject_remove_from_owner
        .expect("exact typed in-world GameObject should expose RemoveFromOwner boundary");

    assert_eq!(remove_owner.guid, guid);
    assert_eq!(remove_owner.owner_guid_before, owner_guid);
    assert_eq!(remove_owner.owner_guid_after, ObjectGuid::EMPTY);
    assert!(remove_owner.owner_found_as_unit_like);
    assert!(remove_owner.cleared_owner);
    assert_eq!(remove_owner.spell_id, 482001);
    assert!(remove_owner.unit_side_effects_represented);
    assert!(remove_owner.unit_owned_gameobject_list_removed);
    assert!(remove_owner.unit_object_slot_cleared);
    assert!(remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 1);
    assert!(!remove_owner.cooldown_event_represented);
    assert!(!remove_owner.creature_ai_callback_represented);
    assert!(outcome.gameobject_model_remove.is_some());
    assert!(!map.contains_gameobject_model_like_cpp(key));
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
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .gameobject_slots
            .iter()
            .all(ObjectGuid::is_empty)
    );
    assert!(!owner.unit().subsystems().auras.has_applied(removed_aura));
    assert!(owner.unit().subsystems().auras.has_applied(kept_aura));
    assert_eq!(
        owner.unit().subsystems().auras.removed_auras,
        vec![removed_aura.aura_ref()]
    );
    assert!(
        !owner
            .unit()
            .subsystems()
            .auras
            .has_owned(removed_owned_aura)
    );
}
#[test]
fn gameobject_remove_from_owner_clears_lost_owner_fallback_like_cpp() {
    let mut map = test_map();
    let missing_owner_guid = ObjectGuid::create_player(1, 4820201);
    let mut gameobject = test_gameobject_for_spawn(48202, 4820202);
    let guid = gameobject.world().guid();
    gameobject.set_owner_guid_like_cpp(missing_owner_guid);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_owner = outcome.gameobject_remove_from_owner.unwrap();

    assert_eq!(remove_owner.owner_guid_before, missing_owner_guid);
    assert_eq!(remove_owner.owner_guid_after, ObjectGuid::EMPTY);
    assert!(!remove_owner.owner_found_as_unit_like);
    assert!(remove_owner.cleared_owner);
    assert!(!remove_owner.unit_side_effects_represented);
    assert!(!remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 0);
}
#[test]
fn gameobject_remove_from_owner_noops_empty_owner_generic_and_not_in_world_like_cpp() {
    let mut empty_owner_map = test_map();
    let empty_owner_gameobject = test_gameobject_for_spawn(48203, 4820301);
    let empty_owner_guid = empty_owner_gameobject.world().guid();
    empty_owner_map
        .insert_map_object_record(MapObjectRecord::new_game_object(empty_owner_gameobject).unwrap())
        .unwrap();

    let empty_owner_removed = empty_owner_map
        .remove_from_map_like_cpp(empty_owner_guid, true)
        .unwrap();
    let empty_owner = empty_owner_removed.gameobject_remove_from_owner.unwrap();
    assert_eq!(empty_owner.owner_guid_before, ObjectGuid::EMPTY);
    assert_eq!(empty_owner.owner_guid_after, ObjectGuid::EMPTY);
    assert!(!empty_owner.owner_found_as_unit_like);
    assert!(!empty_owner.cleared_owner);
    assert!(!empty_owner.aura_cleanup_represented);
    assert_eq!(empty_owner.aura_cleanup_removed_count, 0);

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4820302, 571, 7, false);
    let generic_guid = generic_object.guid();
    generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();
    let generic_removed = generic_map
        .remove_from_map_like_cpp(generic_guid, true)
        .unwrap();
    assert!(generic_removed.gameobject_remove_from_owner.is_none());

    let mut not_in_world_map = test_map();
    let mut not_in_world_gameobject = test_gameobject_for_spawn(48203, 4820303);
    let not_in_world_guid = not_in_world_gameobject.world().guid();
    not_in_world_gameobject.set_owner_guid_like_cpp(ObjectGuid::create_player(1, 4820304));
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
    assert!(not_in_world_removed.gameobject_remove_from_owner.is_none());
}
#[test]
fn gameobject_linked_trap_remove_runs_before_owner_store_extraction_like_cpp() {
    let mut map = test_map();
    let mut trap = test_gameobject_for_spawn(48301, 4830101);
    let trap_guid = trap.world().guid();
    trap.set_represented_gameobject_model_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();

    let mut owner = test_gameobject_for_spawn(48302, 4830102);
    let owner_guid = owner.world().guid();
    owner.set_linked_trap_like_cpp(trap_guid);
    owner.set_represented_gameobject_model_like_cpp(true);
    let owner_model_key = RepresentedGameObjectModelKeyLikeCpp { owner_guid };
    map.insert_gameobject_model_like_cpp(owner_model_key);
    map.insert_map_object_record(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(owner_guid, true).unwrap();
    let linked_trap = outcome.gameobject_linked_trap_remove.expect(
        "exact typed in-world GameObject should expose linked-trap RemoveFromWorld evidence",
    );

    assert_eq!(linked_trap.guid, owner_guid);
    assert_eq!(linked_trap.linked_trap_guid, Some(trap_guid));
    assert!(linked_trap.owner_present_before_linked_trap_remove);
    assert!(!linked_trap.linked_trap_removed);
    assert!(linked_trap.linked_trap_remove_queued);
    assert!(!linked_trap.linked_trap_missing_or_self);
    assert!(!linked_trap.linked_trap_cycle_guarded);
    assert!(linked_trap.despawn_or_unsummon_scheduler_represented);
    assert!(!linked_trap.object_accessor_fanout_represented);
    assert!(outcome.gameobject_model_remove.is_some());
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(!map.contains_gameobject_model_like_cpp(owner_model_key));
}
#[test]
fn gameobject_linked_trap_remove_cycle_guard_allows_single_nested_remove_like_cpp() {
    let mut map = test_map();
    let mut owner = test_gameobject_for_spawn(48307, 4830401);
    let owner_guid = owner.world().guid();
    let mut trap = test_gameobject_for_spawn(48308, 4830402);
    let trap_guid = trap.world().guid();

    owner.set_linked_trap_like_cpp(trap_guid);
    trap.set_linked_trap_like_cpp(owner_guid);
    map.insert_map_object_record(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(owner_guid, true).unwrap();
    let linked_trap = outcome.gameobject_linked_trap_remove.expect(
        "exact typed in-world GameObject should expose linked-trap RemoveFromWorld evidence",
    );

    assert_eq!(linked_trap.guid, owner_guid);
    assert_eq!(linked_trap.linked_trap_guid, Some(trap_guid));
    assert!(linked_trap.owner_present_before_linked_trap_remove);
    assert!(!linked_trap.linked_trap_removed);
    assert!(linked_trap.linked_trap_remove_queued);
    assert!(!linked_trap.linked_trap_missing_or_self);
    assert!(!linked_trap.linked_trap_cycle_guarded);
    assert!(linked_trap.despawn_or_unsummon_scheduler_represented);
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_linked_trap_remove_noops_missing_self_and_empty_like_cpp() {
    let mut missing_map = test_map();
    let missing_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 0, 4830201);
    let mut missing_owner = test_gameobject_for_spawn(48303, 4830202);
    let missing_owner_guid = missing_owner.world().guid();
    missing_owner.set_linked_trap_like_cpp(missing_trap_guid);
    missing_map
        .insert_map_object_record(MapObjectRecord::new_game_object(missing_owner).unwrap())
        .unwrap();

    let missing_outcome = missing_map
        .remove_from_map_like_cpp(missing_owner_guid, true)
        .unwrap();
    let missing = missing_outcome.gameobject_linked_trap_remove.unwrap();
    assert_eq!(missing.linked_trap_guid, Some(missing_trap_guid));
    assert!(!missing.linked_trap_removed);
    assert!(missing.linked_trap_missing_or_self);
    assert!(!missing.linked_trap_cycle_guarded);
    assert!(missing_map.map_object_record(missing_owner_guid).is_none());

    let mut self_map = test_map();
    let mut self_owner = test_gameobject_for_spawn(48304, 4830203);
    let self_guid = self_owner.world().guid();
    self_owner.set_linked_trap_like_cpp(self_guid);
    self_map
        .insert_map_object_record(MapObjectRecord::new_game_object(self_owner).unwrap())
        .unwrap();

    let self_outcome = self_map.remove_from_map_like_cpp(self_guid, true).unwrap();
    let self_linked = self_outcome.gameobject_linked_trap_remove.unwrap();
    assert_eq!(self_linked.linked_trap_guid, Some(self_guid));
    assert!(!self_linked.linked_trap_removed);
    assert!(self_linked.linked_trap_missing_or_self);
    assert!(!self_linked.linked_trap_cycle_guarded);
    assert!(self_map.map_object_record(self_guid).is_none());

    let mut empty_map = test_map();
    let empty_owner = test_gameobject_for_spawn(48305, 4830204);
    let empty_guid = empty_owner.world().guid();
    empty_map
        .insert_map_object_record(MapObjectRecord::new_game_object(empty_owner).unwrap())
        .unwrap();

    let empty_outcome = empty_map
        .remove_from_map_like_cpp(empty_guid, true)
        .unwrap();
    let empty = empty_outcome.gameobject_linked_trap_remove.unwrap();
    assert_eq!(empty.linked_trap_guid, None);
    assert!(!empty.linked_trap_removed);
    assert!(empty.linked_trap_missing_or_self);
    assert!(empty_map.map_object_record(empty_guid).is_none());
}
#[test]
fn gameobject_linked_trap_remove_skips_not_in_world_and_generic_paths_like_cpp() {
    let mut not_in_world_map = test_map();
    let mut not_in_world_gameobject = test_gameobject_for_spawn(48306, 4830301);
    let not_in_world_guid = not_in_world_gameobject.world().guid();
    not_in_world_gameobject.set_linked_trap_like_cpp(ObjectGuid::create_world_object(
        HighGuid::GameObject,
        0,
        1,
        571,
        7,
        0,
        4830302,
    ));
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
    assert!(not_in_world_removed.gameobject_linked_trap_remove.is_none());

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4830303, 571, 7, false);
    let generic_guid = generic_object.guid();
    generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();

    let generic_removed = generic_map
        .remove_from_map_like_cpp(generic_guid, true)
        .unwrap();
    assert!(generic_removed.gameobject_linked_trap_remove.is_none());
}
#[test]
fn dynamic_tree_gameobject_add_without_model_evidence_leaves_tree_empty_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45102, 4510201);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert!(outcome.gameobject_model_insert.is_none());
    assert!(outcome.gameobject_collision_enable.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let summary = map.update_dynamic_tree_like_cpp(250);
    assert!(summary.empty);
    assert_eq!(summary.unbalanced_after, 0);
}
#[test]
fn dynamic_tree_gameobject_already_in_world_add_does_not_insert_model_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45103, 4510301);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.gameobject_model_insert.is_none());
    assert!(outcome.gameobject_collision_enable.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
}
#[test]
fn dynamic_tree_gameobject_add_chest_ready_enables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45201, 4520101);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Ready, None);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.new_collision_enabled, Some(true));
}
#[test]
fn dynamic_tree_gameobject_add_chest_non_ready_disables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45202, 4520201);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, None);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.new_collision_enabled, Some(false));
}
#[test]
fn dynamic_tree_gameobject_add_non_chest_ready_state_enables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45203, 4520301);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_state(GoState::Ready);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.new_collision_enabled, Some(true));
}
#[test]
fn dynamic_tree_gameobject_add_non_chest_active_state_disables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45204, 4520401);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_state(GoState::Active);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.new_collision_enabled, Some(false));
}
#[test]
fn dynamic_tree_gameobject_remove_consumes_contained_model_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45104, 4510401);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    assert!(map.contains_gameobject_model_like_cpp(key));

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();

    let remove = outcome
        .gameobject_model_remove
        .expect("contained represented model should be removed before final map removal");
    assert_eq!(
        remove.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(remove.model_count_before, 1);
    assert_eq!(remove.model_count_after, 0);
    assert_eq!(remove.unbalanced_before, 1);
    assert_eq!(remove.unbalanced_after, 2);
    assert!(!map.contains_gameobject_model_like_cpp(key));
}
#[test]
fn dynamic_tree_gameobject_remove_missing_key_is_guarded_noop_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45105, 4510501);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_represented_gameobject_model_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.mark_dynamic_tree_unbalanced_for_tests_like_cpp(5);

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();

    assert!(outcome.gameobject_model_remove.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let summary = map.update_dynamic_tree_like_cpp(250);
    assert!(summary.empty);
    assert_eq!(summary.unbalanced_before, 5);
    assert_eq!(summary.unbalanced_after, 5);
}
#[test]
fn dynamic_tree_transport_add_excludes_immediate_gameobject_model_insert_like_cpp() {
    let mut map = test_map();
    let mut transport = test_transport_for_update(4510601, false);
    let guid = transport.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    transport
        .game_object_mut()
        .set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.gameobject_model_insert.is_none());
    assert!(outcome.gameobject_collision_enable.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
}
#[test]
fn dynamic_tree_gameobject_update_model_removes_old_and_inserts_new_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45301, 4530101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.update_gameobject_model_like_cpp(guid, true, false);

    assert_eq!(outcome.status, GameObjectUpdateModelStatusLikeCpp::Updated);
    assert!(outcome.old_model_present);
    assert!(outcome.old_model_registered);
    let remove = outcome.old_model_remove.unwrap();
    assert_eq!(
        remove.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(remove.model_count_before, 1);
    assert_eq!(remove.model_count_after, 0);
    assert_eq!(remove.unbalanced_before, 1);
    assert_eq!(remove.unbalanced_after, 2);
    let insert = outcome.new_model_insert.unwrap();
    assert_eq!(
        insert.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(insert.model_count_before, 0);
    assert_eq!(insert.model_count_after, 1);
    assert_eq!(insert.unbalanced_before, 2);
    assert_eq!(insert.unbalanced_after, 3);
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(!gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}
#[test]
fn dynamic_tree_gameobject_update_model_to_no_model_removes_old_without_insert_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45302, 4530201);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.update_gameobject_model_like_cpp(guid, false, true);

    assert_eq!(outcome.status, GameObjectUpdateModelStatusLikeCpp::Updated);
    assert!(outcome.old_model_present);
    assert!(outcome.old_model_registered);
    assert_eq!(
        outcome.old_model_remove.unwrap().status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert!(outcome.new_model_insert.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(!gameobject.has_represented_gameobject_model_like_cpp());
    assert!(!gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}
