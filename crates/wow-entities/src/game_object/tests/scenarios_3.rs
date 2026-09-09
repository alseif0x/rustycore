//! GameObject template, loot and runtime state regression scenarios, part 3 of 3.
//!
//! Moved out of the game_object.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn gameobject_loot_for_player_matches_cpp_shared_vs_personal_precedence() {
    let first = ObjectGuid::create_player(1, 7);
    let second = ObjectGuid::create_player(1, 8);
    let mut go = GameObject::new();

    assert_eq!(go.loot_for_player_like_cpp(first), None);

    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(5, 0));
    assert_eq!(
        go.loot_for_player_like_cpp(first),
        Some(&GameObjectOwnedLoot::new(5, 0))
    );
    assert_eq!(
        go.loot_for_player_like_cpp(second),
        Some(&GameObjectOwnedLoot::new(5, 0))
    );

    go.set_personal_loot_like_cpp(first, GameObjectOwnedLoot::new(0, 1));
    assert_eq!(
        go.loot_for_player_like_cpp(first),
        Some(&GameObjectOwnedLoot::new(0, 1))
    );
    assert_eq!(go.loot_for_player_like_cpp(second), None);

    go.set_personal_loot_like_cpp(second, GameObjectOwnedLoot::new(9, 0));
    assert_eq!(
        go.loot_for_player_like_cpp(second),
        Some(&GameObjectOwnedLoot::new(9, 0))
    );
}

#[test]
fn gameobject_clear_loot_clears_owned_loot_unique_users_and_use_count_like_cpp() {
    let mut go = GameObject::new();
    let first = ObjectGuid::new(1, 100);
    let second = ObjectGuid::new(1, 200);

    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(1, 1));
    go.set_personal_loot_like_cpp(first, GameObjectOwnedLoot::new(0, 1));
    assert!(go.add_unique_use_like_cpp(first));
    assert!(!go.add_unique_use_like_cpp(first));
    assert!(go.add_unique_use_like_cpp(second));
    go.add_use_like_cpp();
    go.add_use_like_cpp();

    assert_eq!(
        go.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(1, 1))
    );
    assert_eq!(go.personal_loot_count_like_cpp(), 1);
    assert_eq!(go.unique_user_count_like_cpp(), 2);
    assert_eq!(go.use_times(), 5);
    assert!(!go.is_fully_looted_like_cpp());

    go.clear_loot_like_cpp();

    assert_eq!(go.shared_loot_like_cpp(), None);
    assert_eq!(go.personal_loot_count_like_cpp(), 0);
    assert_eq!(go.unique_user_count_like_cpp(), 0);
    assert_eq!(go.use_times(), 0);
    assert!(go.is_fully_looted_like_cpp());
}

#[test]
fn gameobject_clear_personal_loot_preserves_shared_loot_like_cpp() {
    let player = ObjectGuid::create_player(1, 10);
    let mut go = GameObject::new();
    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(3, 0));
    go.set_personal_loot_like_cpp(player, GameObjectOwnedLoot::new(0, 1));

    go.clear_personal_loot_like_cpp();

    assert_eq!(
        go.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(3, 0))
    );
    assert_eq!(go.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        go.loot_for_player_like_cpp(player),
        Some(&GameObjectOwnedLoot::new(3, 0))
    );
}

#[test]
fn gameobject_add_unique_use_increments_use_times_before_unique_insert_like_cpp() {
    let mut go = GameObject::new();
    let player = ObjectGuid::new(1, 100);

    assert_eq!(go.use_times(), 0);
    assert_eq!(go.unique_user_count_like_cpp(), 0);

    assert!(go.add_unique_use_like_cpp(player));
    assert_eq!(go.use_times(), 1);
    assert_eq!(go.unique_user_count_like_cpp(), 1);

    assert!(!go.add_unique_use_like_cpp(player));
    assert_eq!(go.use_times(), 2);
    assert_eq!(go.unique_user_count_like_cpp(), 1);
}

#[test]
fn gameobject_data_setters_mark_cpp_bits() {
    let mut go = GameObject::new();

    go.set_display_id(1234);
    go.set_faction(35);
    go.set_go_state(GoState::Ready);
    go.set_go_type(3);
    go.set_flags(0x20);
    go.set_level(70);
    go.set_percent_health(80);
    go.set_art_kit(4);
    go.set_custom_param(99);
    let owner = ObjectGuid::create_player(1, 42);
    go.set_created_by(owner);

    assert_eq!(go.data().display_id, 1234);
    assert_eq!(go.owner_guid(), owner);
    assert_eq!(go.data().created_by, owner);
    assert_eq!(go.data().faction_template, 35);
    assert_eq!(go.data().state, GoState::Ready as i8);
    assert_eq!(go.data().type_id, 3);
    assert_eq!(go.data().flags, 0x20);
    assert_eq!(go.data().level, 70);
    assert_eq!(go.data().percent_health, 80);
    assert_eq!(go.data().art_kit, 4);
    assert_eq!(go.data().custom_param, 99);
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_PARENT_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_DISPLAY_ID_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_CREATED_BY_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_FACTION_TEMPLATE_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_STATE_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_TYPE_ID_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_FLAGS_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_LEVEL_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_PERCENT_HEALTH_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_ART_KIT_BIT)
    );
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_CUSTOM_PARAM_BIT)
    );
}

#[test]
fn gameobject_set_owner_guid_like_cpp_updates_created_by_and_spawned_default() {
    let mut go = GameObject::new();
    let owner = ObjectGuid::create_player(1, 48201);
    go.set_spawned_by_default(true);

    go.set_owner_guid_like_cpp(owner);

    assert_eq!(go.owner_guid(), owner);
    assert_eq!(go.data().created_by, owner);
    assert!(!go.spawned_by_default());
    assert!(
        go.game_object_data_changes_mask()
            .is_set(GAME_OBJECT_DATA_CREATED_BY_BIT)
    );

    go.set_spawned_by_default(true);
    go.clear_owner_guid_like_cpp();

    assert_eq!(go.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(go.data().created_by, ObjectGuid::EMPTY);
    assert!(!go.spawned_by_default());
}

#[test]
fn loot_state_tracks_unit_for_any_state_and_none_clears_like_cpp() {
    let mut go = GameObject::new();
    let unit = ObjectGuid::new(7, 11);

    for state in [
        LootState::Activated,
        LootState::Ready,
        LootState::JustDeactivated,
    ] {
        go.set_loot_state(state, Some(unit));
        assert_eq!(go.loot_state(), state);
        assert_eq!(go.loot_state_unit_guid(), unit);
    }

    go.set_loot_state(LootState::Ready, None);
    assert_eq!(go.loot_state(), LootState::Ready);
    assert_eq!(go.loot_state_unit_guid(), ObjectGuid::EMPTY);
}

#[test]
fn spell_id_and_spawn_fields_match_cpp_base_behaviour() {
    let mut go = GameObject::new();

    go.set_spell_id(123);
    go.set_spawn_id(99);
    go.set_respawn_delay_time(45);
    go.set_respawn_time(1000);
    go.set_spawned_by_default(true);
    go.set_cooldown_time(77);
    go.set_respawn_compatibility_mode(true);

    assert_eq!(go.spell_id(), 123);
    assert_eq!(go.spawn_id(), 99);
    assert_eq!(go.respawn_delay_time(), 45);
    assert_eq!(go.respawn_time(), 1000);
    assert!(go.spawned_by_default());
    assert_eq!(go.cooldown_time(), 77);
    assert!(go.respawn_compatibility_mode());

    go.set_spawned_by_default(true);
    go.set_spell_id(0);
    assert_eq!(go.spell_id(), 0);
    assert!(!go.spawned_by_default());
}

#[test]
fn path_progress_for_client_preserves_cpp_dynamic_flag_change_state() {
    let mut go = GameObject::new();

    go.set_path_progress_for_client(0.5);
    assert_eq!(go.world().object().dynamic_flags() >> 16, 32_767);
    assert!(
        !go.world()
            .object()
            .changed_fields()
            .contains(ObjectChangedFields::DYNAMIC_FLAGS)
    );

    go.world_mut().object_mut().set_dynamic_flag(0x4);
    go.set_path_progress_for_client(1.0);
    assert_eq!(go.world().object().dynamic_flags() & 0xFFFF, 0x4);
    assert_eq!(go.world().object().dynamic_flags() >> 16, 65_535);
    assert!(
        go.world()
            .object()
            .changed_fields()
            .contains(ObjectChangedFields::DYNAMIC_FLAGS)
    );
}

#[test]
fn values_update_sets_gameobject_object_type_bit() {
    let mut go = GameObject::new();

    go.set_display_id(1234);
    let update = go.values_update();

    assert!(update.has_data());
    assert_eq!(update.changed_object_type_mask, 1 << TYPEID_GAME_OBJECT);
    let game_object_data = update.game_object_data.unwrap();
    assert_eq!(game_object_data.values.display_id, 1234);
    assert!(
        game_object_data
            .mask
            .is_set(GAME_OBJECT_DATA_DISPLAY_ID_BIT)
    );
}

#[test]
fn gameobject_grid_unload_helpers_apply_represented_state() {
    let mut go = GameObject::new();
    go.world_mut().set_current_cell(3, 4);

    go.set_destroyed_object(true);
    go.request_respawn_relocation_from_grid_unload();
    go.cleanup_before_delete();
    go.request_delete_from_grid_unload();

    assert!(go.world().object().is_destroyed_object());
    assert!(go.grid_unload_respawn_relocation_requested());
    assert_eq!(go.cleanup_before_delete_count(), 1);
    assert!(go.grid_unload_delete_requested());
    assert_eq!(go.world().current_cell(), None);
    assert!(!go.world().object().is_in_grid());
}
