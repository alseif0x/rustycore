//! Gameobject scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn dynamic_respawn_gameobject_ceil_scales_and_clamps_to_minimum() {
    let context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    let scaled = apply_dynamic_mode_respawn_scaling_like_cpp(241, context);

    assert_eq!(scaled.delay_secs, 91);
    assert!(scaled.was_scaled());

    let clamped = apply_dynamic_mode_respawn_scaling_like_cpp(120, context);
    assert_eq!(clamped.delay_secs, 60);
    assert!(clamped.was_scaled());
}
#[test]
fn map_object_store_can_hold_typed_gameobject_entity_like_cpp() {
    let mut map = test_map();
    let mut gameobject = GameObject::new();
    let guid = guid(HighGuid::GameObject, 77);
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(123);
    gameobject.world_mut().set_map(571, 7).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    gameobject.set_created_by(ObjectGuid::create_player(1, 42));

    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    assert_eq!(map.get_game_object(guid).unwrap().guid(), guid);
    assert_eq!(
        map.get_typed_game_object(guid).unwrap().owner_guid(),
        ObjectGuid::create_player(1, 42)
    );
}
#[test]
fn add_to_map_typed_gameobject_tail_initializes_and_clears_move_like_cpp() {
    let mut map = test_map();
    let mut stale_gameobject = test_gameobject_for_spawn(478, 47802);
    stale_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = stale_gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(stale_gameobject).unwrap())
        .unwrap();
    assert_eq!(
        map.add_game_object_to_move_list_like_cpp(guid, Position::xyz(50.0, 51.0, 52.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );

    let mut gameobject = test_gameobject_for_spawn(478, 47802);
    gameobject.world_mut().object_mut().remove_from_world();
    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let tail = outcome.add_to_map_tail.unwrap();
    assert!(tail.initialize_object_represented);
    assert!(tail.pending_move_state_cleared);
    assert!(tail.set_is_new_object_true);
    assert!(tail.update_object_visibility_on_create_represented);
    assert!(tail.update_object_visibility_on_create_runtime_gap);
    assert!(tail.set_is_new_object_false);
    assert!(!tail.final_is_new_object);
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, guid)
            .is_none()
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    let drain = map.move_all_game_objects_in_move_list_like_cpp();
    assert_eq!(drain.processed, 0);
    assert_eq!(drain.relocated, 0);
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(stored.world().object().is_in_world());
    assert!(!stored.world().object().is_new_object());
}
#[test]
fn add_map_object_record_to_map_like_cpp_preserves_typed_gameobject_spawn_index() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(396, 39602);
    gameobject.world_mut().object_mut().remove_from_world();
    let guid = gameobject.world().guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.inserted);
    assert!(!outcome.already_in_world);
    assert!(outcome.inserted_into_cell);
    assert!(map.get_gameobject_by_spawn_id_like_cpp(396).is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .is_some()
    );

    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.grid_objects.gameobjects.contains(&guid));
}
#[test]
fn remove_from_map_delete_detaches_gameobject_loot_authority_before_typed_drop() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_102);
    let mut gameobject = test_gameobject_for_spawn(484_102, 4_841_020);
    let guid = gameobject.world().guid();
    assert!(
        gameobject
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 19, player,
            ))
            .installed()
    );
    let retained_authority = gameobject.loot_authority_like_cpp().clone();
    let lease = poll_immediately_ready(retained_authority.reserve_money_like_cpp(player))
        .expect("the live GameObject authority must grant the uncontended lease");
    gameobject.world_mut().object_mut().remove_from_world();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
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
fn insert_map_object_record_detaches_displaced_gameobject_authority_for_same_guid() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_104);
    let mut displaced_gameobject = test_gameobject_for_spawn(484_105, 4_841_040);
    let guid = displaced_gameobject.world().guid();
    assert!(
        displaced_gameobject
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 31, player,
            ))
            .installed()
    );
    let displaced_authority = displaced_gameobject.loot_authority_like_cpp().clone();
    map.insert_map_object_record(MapObjectRecord::new_game_object(displaced_gameobject).unwrap())
        .unwrap();
    let lease = poll_immediately_ready(displaced_authority.reserve_money_like_cpp(player))
        .expect("the displaced GameObject authority must grant the uncontended lease");

    let mut replacement = test_gameobject_for_spawn(484_106, 4_841_040);
    assert!(
        replacement
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 37, player,
            ))
            .installed()
    );
    let replacement_authority = replacement.loot_authority_like_cpp().clone();
    let displaced = map
        .insert_map_object_record(MapObjectRecord::new_game_object(replacement).unwrap())
        .unwrap()
        .expect("same-GUID insert must return the displaced GameObject record");

    assert_eq!(
        displaced_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(
        displaced
            .game_object()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&displaced_authority)
    );
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
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
fn remove_from_map_like_cpp_visibility_on_destroy_runs_for_not_in_world_gameobject_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48411, 4841101);
    gameobject.world_mut().object_mut().remove_from_world();
    let guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(!removed.was_in_world);
    assert!(!removed.cxx_in_world);
    assert!(
        removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );
}
#[test]
fn switch_list_non_unit_gameobject_enqueue_is_ignored_like_cpp() {
    let mut map = test_map();
    let gameobject = test_gameobject_for_spawn(420050, 4200501);
    let guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.add_object_to_switch_list_like_cpp(guid, true);

    assert_eq!(
        outcome.status,
        AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_just_deactivated_queues_linked_trap_delete_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580101, 571, 7, false);
    let mut trap = game_object_with_counter(4580102, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::JustDeactivated, None);
    owner.set_respawn_delay_time(0);
    owner.set_linked_trap_like_cpp(trap_guid);
    trap.set_loot_state(LootState::Ready, None);
    trap.set_go_state(GoState::Active);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(owner_guid, 1, 1_000);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.linked_trap_guid, Some(trap_guid));
    assert!(!outcome.linked_trap_removed);
    assert!(outcome.linked_trap_remove_queued);
    assert!(!outcome.linked_trap_missing_or_self);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(outcome.loot_cleared);
    let trap_after_update = map
        .map_object_record(trap_guid)
        .and_then(MapObjectRecord::game_object)
        .expect("linked trap should stay in map until remove-list drain");
    assert_eq!(trap_after_update.loot_state(), LootState::NotReady);
    assert_eq!(trap_after_update.data().state, GoState::Ready as i8);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.removed, 1);
    assert!(map.map_object_record(trap_guid).is_none());
}
#[test]
fn gameobject_update_just_deactivated_clears_owned_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4590101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let personal_guid = guid(HighGuid::Player, 4590191);
    let unique_guid = guid(HighGuid::Player, 4590192);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(7, 2));
    gameobject.set_personal_loot_like_cpp(personal_guid, GameObjectOwnedLoot::new(11, 3));
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));
    gameobject.add_use_like_cpp();
    assert!(gameobject.shared_loot_like_cpp().is_some());
    assert_eq!(gameobject.personal_loot_count_like_cpp(), 1);
    assert_eq!(gameobject.unique_user_count_like_cpp(), 1);
    assert_eq!(gameobject.use_times(), 2);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(outcome.generic_not_ready);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}
#[test]
fn gameobject_update_just_deactivated_goober_spell_represents_casts_and_clears_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let personal_guid = guid(HighGuid::Player, 4600191);
    let first_unique_guid = guid(HighGuid::Player, 4600192);
    let second_unique_guid = guid(HighGuid::Player, 4600193);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        spell_id: 12345,
        ..GooberUseSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(17, 4));
    gameobject.set_personal_loot_like_cpp(personal_guid, GameObjectOwnedLoot::new(19, 5));
    assert!(gameobject.add_unique_use_like_cpp(first_unique_guid));
    assert!(gameobject.add_unique_use_like_cpp(second_unique_guid));
    gameobject.add_use_like_cpp();

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, Some(12345));
    assert_eq!(outcome.goober_spell_casts_represented, 2);
    assert!(outcome.goober_users_cleared);
    assert!(!outcome.goober_state_reset);
    assert!(!outcome.goober_nodespawn_return);
    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.non_consumed_set_ready);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}
#[test]
fn gameobject_update_just_deactivated_goober_lock_resets_state_and_clears_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_go_state(GoState::Active);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        lock_id: 77,
        ..GooberUseSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(23, 1));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.goober_state_reset);
    assert_eq!(canonical.data().state, GoState::Ready as i8);
    assert!(outcome.loot_cleared);
    assert!(canonical.shared_loot_like_cpp().is_none());
}
#[test]
fn gameobject_update_just_deactivated_goober_nodespawn_returns_before_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let unique_guid = guid(HighGuid::Player, 4600391);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_flags(gameobject.data().flags | GO_FLAG_NODESPAWN);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        spell_id: 23456,
        auto_close_ms: 5000,
        ..GooberUseSource::default()
    }));
    gameobject.set_go_state(GoState::Active);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(29, 2));
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, Some(23456));
    assert_eq!(outcome.goober_spell_casts_represented, 1);
    assert!(outcome.goober_users_cleared);
    assert!(outcome.goober_state_reset);
    assert!(outcome.goober_nodespawn_return);
    assert!(!outcome.loot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(canonical.data().state, GoState::Ready as i8);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}
#[test]
fn gameobject_update_just_deactivated_goober_nodespawn_without_source_returns_before_clearloot_like_cpp()
 {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600351, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let unique_guid = guid(HighGuid::Player, 4600352);
    let personal_guid = guid(HighGuid::Player, 4600353);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_flags(gameobject.data().flags | GO_FLAG_NODESPAWN);
    gameobject.set_represented_goober_use_source_like_cpp(None);
    gameobject.set_go_state(GoState::Active);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(30, 2));
    gameobject.set_personal_loot_like_cpp(personal_guid, GameObjectOwnedLoot::new(31, 1));
    gameobject.add_use_like_cpp();
    gameobject.add_use_like_cpp();
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));
    let use_times_before = gameobject.use_times();

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, None);
    assert_eq!(outcome.goober_spell_casts_represented, 0);
    assert!(!outcome.goober_users_cleared);
    assert!(!outcome.goober_state_reset);
    assert!(outcome.goober_nodespawn_return);
    assert!(!outcome.loot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(canonical.data().state, GoState::Active as i8);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 1);
    assert_eq!(canonical.unique_user_count_like_cpp(), 1);
    assert_eq!(canonical.use_times(), use_times_before);
}
#[test]
fn gameobject_update_just_deactivated_goober_without_spell_clears_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600401, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let unique_guid = guid(HighGuid::Player, 4600491);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource::default()));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(31, 3));
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, None);
    assert_eq!(outcome.goober_spell_casts_represented, 0);
    assert!(!outcome.goober_users_cleared);
    assert!(!outcome.goober_state_reset);
    assert!(!outcome.goober_nodespawn_return);
    assert!(outcome.loot_cleared);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}
#[test]
fn gameobject_update_non_consumed_chest_restock_returns_after_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 45,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(41, 2));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.non_consumed_restock_armed);
    assert!(!outcome.non_consumed_set_ready);
    assert!(outcome.non_consumed_update_visibility_represented);
    assert!(outcome.non_consumed_update_dynamic_flags_represented);
    assert!(!outcome.non_consumed_source_missing);
    assert_eq!(canonical.restock_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_non_consumed_chest_without_restock_sets_ready_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 0,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(43, 1));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(!outcome.non_consumed_restock_armed);
    assert!(outcome.non_consumed_set_ready);
    assert!(outcome.non_consumed_update_visibility_represented);
    assert!(!outcome.non_consumed_update_dynamic_flags_represented);
    assert_eq!(canonical.restock_time(), 0);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_non_consumed_goober_sets_ready_after_prebranch_and_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        consumable: false,
        lock_id: 88,
        ..GooberUseSource::default()
    }));
    gameobject.set_go_state(GoState::Active);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(47, 1));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.goober_state_reset);
    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.non_consumed_set_ready);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_consumable_chest_or_goober_does_not_take_non_consumed_return_like_cpp() {
    let mut map = test_map();
    let mut chest = game_object_with_counter(4610401, 571, 7, false);
    let mut goober = game_object_with_counter(4610402, 571, 7, false);
    let chest_guid = chest.world().guid();
    let goober_guid = goober.world().guid();
    chest.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    chest.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 90,
        chest_consumable: true,
        ..GameObjectLootSource::default()
    }));
    chest.set_loot_state(LootState::JustDeactivated, None);
    goober.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    goober.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        consumable: true,
        ..GooberUseSource::default()
    }));
    goober.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(chest).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(goober).unwrap())
        .unwrap();

    let chest_outcome = map.update_game_object_like_cpp(chest_guid, 1, 1_000);
    let goober_outcome = map.update_game_object_like_cpp(goober_guid, 1, 1_000);

    assert!(!chest_outcome.non_consumed_chest_or_goober_return);
    assert!(!chest_outcome.non_consumed_restock_armed);
    assert!(!chest_outcome.non_consumed_set_ready);
    assert!(!goober_outcome.non_consumed_chest_or_goober_return);
    assert!(!goober_outcome.non_consumed_set_ready);
}
#[test]
fn gameobject_update_spell_created_expired_deletes_after_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610501, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_go_state(GoState::Active);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 30,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_spell_id(123);
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.loot_cleared);
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.summoned_expired_respawn_time_zeroed);
    assert!(outcome.summoned_expired_despawn_represented);
    assert!(outcome.summoned_expired_go_state_ready);
    assert!(outcome.remove_list.as_ref().is_some_and(|list| list.queued));
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(canonical.data().state, GoState::Ready as i8);
    assert_eq!(canonical.respawn_time(), 0);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert!(!outcome.non_consumed_restock_armed);
    assert!(!outcome.non_consumed_set_ready);
    assert!(!outcome.non_consumed_update_visibility_represented);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_update_owner_created_expired_deletes_after_clearloot_like_cpp() {
    let mut map = test_map();
    let owner_guid = guid(HighGuid::Player, 4620191);
    let mut gameobject = game_object_with_counter(4620101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 30,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_created_by(owner_guid);
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.loot_cleared);
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.summoned_expired_respawn_time_zeroed);
    assert!(outcome.remove_list.as_ref().is_some_and(|list| list.queued));
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_update_new_flag_drop_owner_new_flag_command_is_represented_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4620201, 571, 7, false);
    let owner_guid = owner.world().guid();
    owner.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG as u8);
    let mut drop = game_object_with_counter(4620202, 571, 7, false);
    let drop_guid = drop.world().guid();
    drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    drop.set_created_by(owner_guid);
    drop.set_respawn_time(0);
    drop.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(drop).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(drop_guid, 1, 1_000);

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.new_flag_drop_owner_in_base_command_represented);
    assert!(!outcome.new_flag_drop_owner_missing_or_empty);
    assert!(!outcome.new_flag_drop_owner_wrong_kind);
    assert!(!outcome.new_flag_drop_owner_not_new_flag);
    assert!(map.map_object_record(owner_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_update_new_flag_drop_missing_and_wrong_owner_are_explicit_noops_like_cpp() {
    let mut missing_map = test_map();
    let mut missing_drop = game_object_with_counter(4620301, 571, 7, false);
    let missing_drop_guid = missing_drop.world().guid();
    missing_drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    missing_drop.set_created_by(guid(HighGuid::GameObject, 4620399));
    missing_drop.set_respawn_time(0);
    missing_drop.set_loot_state(LootState::JustDeactivated, None);
    missing_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(missing_drop).unwrap(),
        )
        .unwrap();

    let missing_outcome = missing_map.update_game_object_like_cpp(missing_drop_guid, 1, 1_000);

    assert!(missing_outcome.summoned_expired_delete);
    assert!(missing_outcome.new_flag_drop_owner_missing_or_empty);
    assert!(!missing_outcome.new_flag_drop_owner_in_base_command_represented);

    let mut wrong_kind_map = test_map();
    let creature = test_creature_for_spawn(4620401, 4620401, true);
    let creature_guid = creature.guid();
    let mut wrong_kind_drop = game_object_with_counter(4620402, 571, 7, false);
    let wrong_kind_drop_guid = wrong_kind_drop.world().guid();
    wrong_kind_drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    wrong_kind_drop.set_created_by(creature_guid);
    wrong_kind_drop.set_respawn_time(0);
    wrong_kind_drop.set_loot_state(LootState::JustDeactivated, None);
    wrong_kind_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    wrong_kind_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(wrong_kind_drop).unwrap(),
        )
        .unwrap();

    let wrong_kind_outcome =
        wrong_kind_map.update_game_object_like_cpp(wrong_kind_drop_guid, 1, 1_000);

    assert!(wrong_kind_outcome.summoned_expired_delete);
    assert!(wrong_kind_outcome.new_flag_drop_owner_wrong_kind);
    assert!(!wrong_kind_outcome.new_flag_drop_owner_in_base_command_represented);

    let mut not_new_flag_map = test_map();
    let mut owner = game_object_with_counter(4620501, 571, 7, false);
    let owner_guid = owner.world().guid();
    owner.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    let mut not_new_flag_drop = game_object_with_counter(4620502, 571, 7, false);
    let not_new_flag_drop_guid = not_new_flag_drop.world().guid();
    not_new_flag_drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    not_new_flag_drop.set_created_by(owner_guid);
    not_new_flag_drop.set_respawn_time(0);
    not_new_flag_drop.set_loot_state(LootState::JustDeactivated, None);
    not_new_flag_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    not_new_flag_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(not_new_flag_drop).unwrap(),
        )
        .unwrap();

    let not_new_flag_outcome =
        not_new_flag_map.update_game_object_like_cpp(not_new_flag_drop_guid, 1, 1_000);

    assert!(not_new_flag_outcome.summoned_expired_delete);
    assert!(not_new_flag_outcome.new_flag_drop_owner_not_new_flag);
    assert!(!not_new_flag_outcome.new_flag_drop_owner_in_base_command_represented);
}
