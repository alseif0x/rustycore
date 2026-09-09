//! Gameobject scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn dynamic_tree_gameobject_update_model_not_in_world_is_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45303, 4530301);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    gameobject.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.update_gameobject_model_like_cpp(guid, false, false);

    assert_eq!(
        outcome.status,
        GameObjectUpdateModelStatusLikeCpp::NotInWorld
    );
    assert!(outcome.old_model_present);
    assert!(outcome.old_model_registered);
    assert!(outcome.old_model_remove.is_none());
    assert!(outcome.new_model_insert.is_none());
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        gameobject.data().flags & GO_FLAG_MAP_OBJECT,
        GO_FLAG_MAP_OBJECT
    );
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}
#[test]
fn dynamic_tree_gameobject_update_model_missing_and_wrong_kind_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4530401);
    let creature = test_creature_for_spawn(45304, 4530402, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4530403, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.update_gameobject_model_like_cpp(missing_guid, true, true);
    let wrong_kind = map.update_gameobject_model_like_cpp(creature_guid, true, true);
    let untyped = map.update_gameobject_model_like_cpp(untyped_guid, true, true);

    assert_eq!(
        missing.status,
        GameObjectUpdateModelStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectUpdateModelStatusLikeCpp::WrongKind
    );
    assert_eq!(
        untyped.status,
        GameObjectUpdateModelStatusLikeCpp::WrongKind
    );
    assert!(missing.new_model_insert.is_none());
    assert!(wrong_kind.new_model_insert.is_none());
    assert!(untyped.new_model_insert.is_none());
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}
#[test]
fn gameobject_display_set_in_world_writes_field_then_updates_model_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45401, 4540101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_display_id(111);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.set_gameobject_display_id_like_cpp(guid, 777, true, false);

    assert_eq!(outcome.status, GameObjectSetDisplayIdStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_display_id, Some(111));
    assert_eq!(outcome.new_display_id, Some(777));
    let update_model = outcome.update_model.unwrap();
    assert_eq!(
        update_model.status,
        GameObjectUpdateModelStatusLikeCpp::Updated
    );
    assert_eq!(
        update_model.old_model_remove.unwrap().status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(
        update_model.new_model_insert.unwrap().status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().display_id, 777);
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(!gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}
#[test]
fn gameobject_display_set_not_in_world_preserves_old_model_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45402, 4540201);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_display_id(222);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    gameobject.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.set_gameobject_display_id_like_cpp(guid, 888, false, false);

    assert_eq!(outcome.status, GameObjectSetDisplayIdStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_display_id, Some(222));
    assert_eq!(outcome.new_display_id, Some(888));
    let update_model = outcome.update_model.unwrap();
    assert_eq!(
        update_model.status,
        GameObjectUpdateModelStatusLikeCpp::NotInWorld
    );
    assert!(update_model.old_model_present);
    assert!(update_model.old_model_registered);
    assert!(update_model.old_model_remove.is_none());
    assert!(update_model.new_model_insert.is_none());
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().display_id, 888);
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}
#[test]
fn gameobject_display_set_missing_wrong_kind_and_untyped_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4540301);
    let creature = test_creature_for_spawn(45403, 4540302, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4540303, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.set_gameobject_display_id_like_cpp(missing_guid, 777, true, true);
    let wrong_kind = map.set_gameobject_display_id_like_cpp(creature_guid, 777, true, true);
    let untyped = map.set_gameobject_display_id_like_cpp(untyped_guid, 777, true, true);

    assert_eq!(
        missing.status,
        GameObjectSetDisplayIdStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectSetDisplayIdStatusLikeCpp::WrongKind
    );
    assert_eq!(
        untyped.status,
        GameObjectSetDisplayIdStatusLikeCpp::WrongKind
    );
    assert!(missing.update_model.is_none());
    assert!(wrong_kind.update_model.is_none());
    assert!(untyped.update_model.is_none());
    assert_eq!(missing.previous_display_id, None);
    assert_eq!(wrong_kind.previous_display_id, None);
    assert_eq!(untyped.previous_display_id, None);
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}
#[test]
fn gameobject_display_set_does_not_infer_model_from_nonzero_display_id_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45404, 4540401);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_display_id(0);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_display_id_like_cpp(guid, 999, false, true);

    assert_eq!(outcome.status, GameObjectSetDisplayIdStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_display_id, Some(0));
    assert_eq!(outcome.new_display_id, Some(999));
    let update_model = outcome.update_model.unwrap();
    assert_eq!(
        update_model.status,
        GameObjectUpdateModelStatusLikeCpp::Updated
    );
    assert!(!update_model.old_model_present);
    assert!(update_model.old_model_remove.is_none());
    assert!(update_model.new_model_insert.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().display_id, 999);
    assert!(!gameobject.has_represented_gameobject_model_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
}
#[test]
fn gameobject_set_go_state_ready_enables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45501, 4550101);
    let guid = gameobject.world().guid();
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(!outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, Some(true));
    let collision = outcome.collision_enable.unwrap();
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.previous_collision_enabled, None);
    assert_eq!(collision.new_collision_enabled, Some(true));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}
#[test]
fn gameobject_set_go_state_active_disables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45502, 4550201);
    let guid = gameobject.world().guid();
    gameobject.set_go_state(GoState::Ready);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Active);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Ready as i8));
    assert_eq!(outcome.new_state, Some(GoState::Active as i8));
    assert_eq!(outcome.in_world_for_collision_branch, Some(true));
    let collision = outcome.collision_enable.unwrap();
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.previous_collision_enabled, Some(true));
    assert_eq!(collision.new_collision_enabled, Some(false));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Active as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}
#[test]
fn gameobject_set_go_state_not_in_world_writes_state_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45503, 4550301);
    let guid = gameobject.world().guid();
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    gameobject.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(!outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, Some(false));
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}
#[test]
fn gameobject_set_go_state_transport_type_writes_state_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45504, 4550401);
    let guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_TRANSPORT as u8);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(false);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, None);
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}
#[test]
fn gameobject_set_go_state_map_obj_transport_writes_state_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45506, 4550601);
    let guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, None);
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.data().type_id,
        GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT as i8
    );
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}
#[test]
fn gameobject_set_go_state_missing_wrong_kind_and_untyped_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4550501);
    let creature = test_creature_for_spawn(45505, 4550502, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4550503, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.set_gameobject_go_state_like_cpp(missing_guid, GoState::Ready);
    let wrong_kind = map.set_gameobject_go_state_like_cpp(creature_guid, GoState::Ready);
    let untyped = map.set_gameobject_go_state_like_cpp(untyped_guid, GoState::Ready);

    assert_eq!(
        missing.status,
        GameObjectSetGoStateStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectSetGoStateStatusLikeCpp::WrongKind
    );
    assert_eq!(untyped.status, GameObjectSetGoStateStatusLikeCpp::WrongKind);
    assert_eq!(missing.previous_state, None);
    assert_eq!(wrong_kind.previous_state, None);
    assert_eq!(untyped.previous_state, None);
    assert!(missing.collision_enable.is_none());
    assert!(wrong_kind.collision_enable.is_none());
    assert!(untyped.collision_enable.is_none());
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}
#[test]
fn gameobject_set_loot_state_chest_activated_arms_restock_and_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45601, 4560101);
    let go_guid = gameobject.world().guid();
    let unit = guid(HighGuid::Player, 4560199);
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_go_state(GoState::Active);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_loot_state_like_cpp(
        go_guid,
        LootState::Activated,
        Some(unit),
        1_000,
        30,
        true,
    );

    assert_eq!(outcome.status, GameObjectSetLootStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_loot_state, Some(LootState::NotReady));
    assert_eq!(outcome.new_loot_state, Some(LootState::Activated));
    assert_eq!(outcome.new_loot_state_unit_guid, Some(unit));
    assert!(outcome.ai_on_loot_state_changed_not_represented);
    assert!(outcome.restock_armed);
    assert_eq!(outcome.previous_restock_time, Some(0));
    assert_eq!(outcome.new_restock_time, Some(1_030));
    let collision = outcome.collision_enable.unwrap();
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.new_collision_enabled, Some(true));
    let gameobject = map
        .map_object_record(go_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.restock_time(), 1_030);
    assert_eq!(gameobject.loot_state_unit_guid(), unit);
}
#[test]
fn gameobject_set_loot_state_restock_requires_changed_and_zero_previous_like_cpp() {
    let mut map = test_map();
    let mut unchanged = test_gameobject_for_spawn(45602, 4560201);
    let unchanged_guid = unchanged.world().guid();
    unchanged.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    map.insert_map_object_record(MapObjectRecord::new_game_object(unchanged).unwrap())
        .unwrap();

    let unchanged_outcome = map.set_gameobject_loot_state_like_cpp(
        unchanged_guid,
        LootState::Activated,
        None,
        2_000,
        60,
        false,
    );
    assert!(!unchanged_outcome.restock_armed);
    assert_eq!(unchanged_outcome.new_restock_time, Some(0));

    let mut already_restocking = test_gameobject_for_spawn(45603, 4560301);
    let already_guid = already_restocking.world().guid();
    already_restocking.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    already_restocking.set_restock_time_like_cpp(77);
    map.insert_map_object_record(MapObjectRecord::new_game_object(already_restocking).unwrap())
        .unwrap();

    let already_outcome = map.set_gameobject_loot_state_like_cpp(
        already_guid,
        LootState::Activated,
        None,
        2_000,
        60,
        true,
    );
    assert!(!already_outcome.restock_armed);
    assert_eq!(already_outcome.previous_restock_time, Some(77));
    assert_eq!(already_outcome.new_restock_time, Some(77));
}
#[test]
fn gameobject_set_loot_state_door_writes_loot_but_preserves_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45604, 4560401);
    let guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_DOOR as u8);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(false);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome =
        map.set_gameobject_loot_state_like_cpp(guid, LootState::Ready, None, 3_000, 60, true);

    assert_eq!(outcome.status, GameObjectSetLootStateStatusLikeCpp::Updated);
    assert_eq!(outcome.new_loot_state, Some(LootState::Ready));
    assert!(outcome.door_type_early_return);
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.loot_state(), LootState::Ready);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}
#[test]
fn gameobject_set_loot_state_model_collision_condition_matches_cpp() {
    let mut map = test_map();
    let cases = [
        (4560501, GoState::Active, LootState::Ready, true),
        (4560502, GoState::Active, LootState::Activated, true),
        (4560503, GoState::Active, LootState::JustDeactivated, true),
        (4560504, GoState::Ready, LootState::Activated, false),
    ];

    for (counter, go_state, loot_state, expected_collision) in cases {
        let mut gameobject = test_gameobject_for_spawn(counter as SpawnId, counter);
        let guid = gameobject.world().guid();
        gameobject.set_go_state(go_state);
        gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
        gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
        map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
            .unwrap();

        let outcome =
            map.set_gameobject_loot_state_like_cpp(guid, loot_state, None, 4_000, 0, true);

        assert_eq!(outcome.status, GameObjectSetLootStateStatusLikeCpp::Updated);
        let collision = outcome.collision_enable.unwrap();
        assert_eq!(collision.requested_enable, expected_collision);
        assert_eq!(collision.new_collision_enabled, Some(expected_collision));
    }
}
#[test]
fn gameobject_set_loot_state_missing_wrong_kind_and_untyped_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4560601);
    let creature = test_creature_for_spawn(45606, 4560602, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4560603, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.set_gameobject_loot_state_like_cpp(
        missing_guid,
        LootState::Ready,
        None,
        5_000,
        10,
        true,
    );
    let wrong_kind = map.set_gameobject_loot_state_like_cpp(
        creature_guid,
        LootState::Ready,
        None,
        5_000,
        10,
        true,
    );
    let untyped = map.set_gameobject_loot_state_like_cpp(
        untyped_guid,
        LootState::Ready,
        None,
        5_000,
        10,
        true,
    );

    assert_eq!(
        missing.status,
        GameObjectSetLootStateStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectSetLootStateStatusLikeCpp::WrongKind
    );
    assert_eq!(
        untyped.status,
        GameObjectSetLootStateStatusLikeCpp::WrongKind
    );
    assert_eq!(missing.previous_loot_state, None);
    assert_eq!(wrong_kind.previous_loot_state, None);
    assert_eq!(untyped.previous_loot_state, None);
    assert!(missing.collision_enable.is_none());
    assert!(wrong_kind.collision_enable.is_none());
    assert!(untyped.collision_enable.is_none());
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}
#[test]
fn grid_unload_actions_apply_to_map_owned_gameobject_record() {
    let mut map = test_map();
    let go_guid = guid(HighGuid::GameObject, 3712);
    let mut gameobject = test_gameobject_for_spawn(372, 3712);
    gameobject.world_mut().set_current_cell(5, 6);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcomes = apply_grid_unload_actions(
        &mut map,
        [
            GridUnloadAction::GameObjectRespawnRelocation(go_guid),
            GridUnloadAction::CleanupsBeforeDelete(GridObjectKind::GameObject, go_guid),
            GridUnloadAction::DeleteObject(GridObjectKind::GameObject, go_guid),
        ],
    );

    assert_eq!(outcomes, vec![GridUnloadApplyOutcome::Applied; 3]);
    assert_eq!(map.map_object_count(), 1);
    let gameobject = map
        .map_object_record(go_guid)
        .unwrap()
        .game_object()
        .unwrap();
    assert!(gameobject.grid_unload_respawn_relocation_requested());
    assert_eq!(gameobject.cleanup_before_delete_count(), 1);
    assert!(gameobject.grid_unload_delete_requested());
    assert_eq!(gameobject.world().current_cell(), None);
}
#[test]
fn process_respawns_loaded_grid_gameobject_loader_adds_record_and_removes_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(398, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 39801, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 39801, 100));
    let expected_guid = guid(HighGuid::GameObject, 3980101);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 39801);
            let mut gameobject = test_gameobject_for_spawn(39801, 3980101);
            gameobject.world_mut().object_mut().remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(gameobject).unwrap(),
            ))
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 39801),
        0
    );
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(39801), 1);
    let record = map.map_object_record(expected_guid).unwrap();
    assert!(record.object().object().is_in_world());
    assert!(record.game_object().is_some());
    assert!(map.get_gameobject_by_spawn_id_like_cpp(39801).is_some());
    let cell = Cell::from_world(record.object().position().x, record.object().position().y);
    let grid = map
        .get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
        .unwrap();
    let local_cell = grid
        .get_grid_type(cell.cell_x(), cell.cell_y())
        .expect("record inserted into target cell");
    assert!(local_cell.grid_objects.gameobjects.contains(&expected_guid));
}
#[test]
fn check_respawn_live_object_guard_gameobject_same_spawn_clears_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(24, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 54, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(54, 54)).unwrap(),
    )
    .unwrap();
    let mut info = respawn_info(SpawnObjectType::GameObject, 54, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}
#[test]
fn world_object_by_spawn_id_gameobject_prefers_spawned_then_fallback_like_cpp() {
    let mut map = test_map();
    let despawned_guid = guid(HighGuid::GameObject, 7801);
    let spawned_guid = guid(HighGuid::GameObject, 7802);
    let mut despawned = test_gameobject_for_spawn(78, 7801);
    despawned.set_respawn_delay_time(30);
    despawned.set_respawn_time(100);
    despawned.set_spawned_by_default(true);
    let mut spawned = test_gameobject_for_spawn(78, 7802);
    spawned.set_respawn_delay_time(30);
    spawned.set_respawn_time(100);
    spawned.set_spawned_by_default(false);

    map.insert_map_object_record(MapObjectRecord::new_game_object(despawned).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(spawned).unwrap())
        .unwrap();

    assert_eq!(
        map.gameobject_spawn_id_store_guids_like_cpp(78),
        vec![despawned_guid, spawned_guid]
    );
    assert_eq!(
        map.get_gameobject_by_spawn_id_like_cpp(78)
            .unwrap()
            .world()
            .guid(),
        spawned_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 78)
            .unwrap()
            .guid(),
        spawned_guid
    );

    assert!(map.remove_map_object(spawned_guid).is_some());
    assert_eq!(
        map.get_gameobject_by_spawn_id_like_cpp(78)
            .unwrap()
            .world()
            .guid(),
        despawned_guid
    );
}
#[test]
fn spawn_id_store_gameobject_same_spawn_blocks_until_removed_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(28, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 61, group), |_| {
        false
    });
    let gameobject_guid = guid(HighGuid::GameObject, 6101);

    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(61, 6101)).unwrap(),
    )
    .unwrap();
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(61), 1);

    let mut blocked_info = respawn_info(SpawnObjectType::GameObject, 61, 100);
    let blocked =
        map.check_respawn_live_object_guard_like_cpp(&mut blocked_info, &store, false, |_, _| {
            false
        });
    assert_eq!(
        blocked,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn
    );
    assert_eq!(blocked_info.respawn_time, 0);

    assert!(map.remove_map_object(gameobject_guid).is_some());
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(61), 0);

    let mut allowed_info = respawn_info(SpawnObjectType::GameObject, 61, 100);
    let allowed =
        map.check_respawn_live_object_guard_like_cpp(&mut allowed_info, &store, false, |_, _| {
            false
        });
    assert_eq!(allowed, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(allowed_info.respawn_time, 100);
}
#[test]
fn spawn_group_spawn_loaded_grid_loader_some_inserts_gameobject_like_cpp() {
    let group = spawn_group(3947, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            207,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, object_type, spawn_id, force| {
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 207);
            assert!(!force);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 207)).unwrap(),
            ))
        },
    );

    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 0);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(207), 1);
}
#[test]
fn process_respawns_composite_live_gameobject_blocker_deletes_due_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(68, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 101, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(101, 101)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 101, 10));

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.deleted_live_object_blocker, 1);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 101)
            .is_none()
    );
}
