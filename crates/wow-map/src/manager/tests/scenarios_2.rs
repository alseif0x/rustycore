//! Managed-map lifecycle and updater regression scenarios, part 2 of 4.
//!
//! Moved out of the manager.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn map_manager_game_object_update_with_pool_context_updates_pool_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid =
        insert_pool_compatible_owner_created_game_object_for_update(&mut manager, 4642201, 464);
    let (spawn_store, pool_mgr) =
        gameobject_pool_update_context_for_manager_like_cpp(464, 4642201, 4642202);

    assert_eq!(
        manager.update_with_pool_update_context(1, &spawn_store, &pool_mgr),
        Some(1)
    );

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.last_game_objects_update_summary(), {
        let mut expected_summary = GameObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            despawn_remove_queued: 0,
            despawn_pool_updated: 1,
            loot_cleared: 1,
            summoned_expired_deletes: 1,
            summoned_expired_respawn_time_zeroed: 1,
            summoned_expired_despawn_represented: 1,
            summoned_expired_go_state_ready: 1,
            generic_visual_despawn_represented: 1,
            ..GameObjectsUpdateSummaryLikeCpp::default()
        };
        expected_summary
            .generic_visual_despawn_guids
            .push(game_object_guid);
        expected_summary
    });
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    // Without the loaded-grid loader from 031dx, pool-data advances and the
    // represented `Despawn1Object<GameObject>` removes the trigger, but the
    // live replacement is not materialized.
    assert!(
        managed_map
            .map()
            .map_object_record(game_object_guid)
            .is_none()
    );
    assert!(
        managed_map
            .map()
            .map_object_record(guid(HighGuid::GameObject, 4642202, 1, 0))
            .is_none()
    );
    assert!(
        managed_map
            .map()
            .pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4642202)
    );
}

#[test]
fn map_manager_game_object_update_with_pool_loaded_grid_context_adds_replacement_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid =
        insert_pool_compatible_owner_created_game_object_for_update(&mut manager, 4642301, 464);
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .ensure_grid_loaded(&crate::map::cell_from_world(0.0, 0.0));
    let (spawn_store, pool_mgr) =
        gameobject_pool_update_context_for_manager_like_cpp(464, 4642301, 4642302);

    let replacement_guid = guid(HighGuid::GameObject, 4642302, 1, 0);
    let mut loader_calls = 0usize;
    assert_eq!(
        manager.update_with_pool_update_loaded_grid_records_context(
            1,
            &spawn_store,
            &pool_mgr,
            |_, object_type, spawn_id| {
                loader_calls += 1;
                assert_eq!(object_type, SpawnObjectType::GameObject);
                assert_eq!(spawn_id, 4642302);
                let mut game_object = GameObject::new();
                game_object
                    .world_mut()
                    .object_mut()
                    .create(replacement_guid);
                game_object.world_mut().set_map(1, 0).unwrap();
                game_object
                    .world_mut()
                    .relocate(Position::xyz(0.0, 0.0, 0.0));
                game_object.world_mut().object_mut().add_to_world();
                game_object.set_spawn_id(spawn_id);
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_game_object(game_object).unwrap(),
                ))
            },
        ),
        Some(1)
    );

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(loader_calls, 1);
    assert_eq!(managed_map.last_game_objects_update_summary(), {
        let mut expected_summary = GameObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            despawn_remove_queued: 0,
            despawn_pool_updated: 1,
            loot_cleared: 1,
            summoned_expired_deletes: 1,
            summoned_expired_respawn_time_zeroed: 1,
            summoned_expired_despawn_represented: 1,
            summoned_expired_go_state_ready: 1,
            generic_visual_despawn_represented: 1,
            ..GameObjectsUpdateSummaryLikeCpp::default()
        };
        expected_summary
            .generic_visual_despawn_guids
            .push(game_object_guid);
        expected_summary
    });
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    // Loaded-grid replacement materialization is wired here. With the
    // spawn-id index populated before AddToMap, the represented
    // `Despawn1Object<GameObject>` removes the trigger before inserting the
    // replacement.
    assert!(
        managed_map
            .map()
            .map_object_record(game_object_guid)
            .is_none()
    );
    assert!(
        managed_map
            .map()
            .map_object_record(replacement_guid)
            .is_some()
    );
    assert_eq!(
        managed_map
            .map()
            .gameobject_spawn_id_store_count_like_cpp(4642302),
        1
    );
    assert!(
        managed_map
            .map()
            .pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4642302)
    );
}

#[test]
fn map_manager_game_object_update_skips_not_in_world_and_keeps_delay_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid = insert_game_object_for_update(&mut manager, 4380301, 10, false);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_game_objects_update_summary(),
        GameObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            despawn_remove_queued: 0,
            missing_or_stale: 0,
            not_game_object: 0,
            not_in_world: 1,
            linked_traps_removed: 0,
            loot_cleared: 0,
            goober_spell_casts_represented: 0,
            goober_users_cleared: 0,
            goober_state_reset: 0,
            goober_nodespawn_returns: 0,
            ..GameObjectsUpdateSummaryLikeCpp::default()
        }
    );
    let game_object = managed_map
        .map()
        .get_typed_game_object(game_object_guid)
        .unwrap();
    assert_eq!(game_object.despawn_delay(), 10);
}

#[test]
fn map_manager_game_object_update_summary_ignores_non_game_objects_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid = insert_game_object_for_update(&mut manager, 4380401, 0, true);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4380402, 10, true);
    let creature_guid = insert_creature_for_update(&mut manager, 4380403, true);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4380404, 10, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_game_objects_update_summary(),
        GameObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            despawn_remove_queued: 0,
            missing_or_stale: 0,
            not_game_object: 0,
            not_in_world: 0,
            linked_traps_removed: 0,
            loot_cleared: 0,
            goober_spell_casts_represented: 0,
            goober_users_cleared: 0,
            goober_state_reset: 0,
            goober_nodespawn_returns: 0,
            ..GameObjectsUpdateSummaryLikeCpp::default()
        }
    );
    assert!(
        managed_map
            .map()
            .map_object_record(game_object_guid)
            .is_some()
    );
    assert!(
        managed_map
            .map()
            .map_object_record(dynamic_object_guid)
            .is_some()
    );
    assert!(managed_map.map().map_object_record(creature_guid).is_some());
    assert!(
        managed_map
            .map()
            .map_object_record(area_trigger_guid)
            .is_some()
    );
}

#[test]
fn map_manager_transport_update_visits_live_transport_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let transport_guid = insert_transport_for_update(&mut manager, 4390101, true, 1);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(
        managed_map.last_transports_update_summary(),
        TransportsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            unsupported_no_period: 0,
            missing_or_stale: 0,
            not_transport: 0,
            not_in_world: 0,
            position_updates_represented: 1,
            just_stopped: 0,
        }
    );
    let transport = managed_map
        .map()
        .map_object_record(transport_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 101);
}

#[test]
fn map_manager_transport_update_visits_not_in_world_transport_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let transport_guid = insert_transport_for_update(&mut manager, 4390201, false, 2);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_transports_update_summary(),
        TransportsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            unsupported_no_period: 0,
            missing_or_stale: 0,
            not_transport: 0,
            not_in_world: 0,
            position_updates_represented: 0,
            just_stopped: 0,
        }
    );
    let transport = managed_map
        .map()
        .map_object_record(transport_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 101);
}

#[test]
fn map_manager_update_dynamic_object_slice_ignores_creature_and_gameobject_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330401, 10, true);
    let creature_guid = guid(HighGuid::Creature, 4330402, 1, 0);
    let game_object_guid = guid(HighGuid::GameObject, 4330403, 1, 0);
    {
        let managed_map = manager.find_map_mut(1, 0).unwrap();
        let mut creature = Creature::new(false);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(creature_guid);
        creature.unit_mut().world_mut().set_map(1, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(12.0, 22.0, 32.0));
        creature.unit_mut().world_mut().object_mut().add_to_world();
        managed_map
            .map_mut()
            .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
            .unwrap();

        let mut game_object = GameObject::new();
        game_object
            .world_mut()
            .object_mut()
            .create(game_object_guid);
        game_object.world_mut().set_map(1, 0).unwrap();
        game_object
            .world_mut()
            .relocate(Position::xyz(13.0, 23.0, 33.0));
        game_object.world_mut().object_mut().add_to_world();
        managed_map
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object).unwrap(),
            )
            .unwrap();
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_dynamic_objects_update_summary(),
        DynamicObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_dynamic_object: 0,
            not_in_world: 0,
        }
    );
    assert!(managed_map.map().map_object_record(creature_guid).is_some());
    assert!(
        managed_map
            .map()
            .map_object_record(game_object_guid)
            .is_some()
    );
    assert_eq!(
        managed_map
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        9
    );
}

#[test]
fn map_manager_update_consumes_send_object_updates_before_personal_phase_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid = guid(HighGuid::GameObject, 4450401, 1, 0);
    {
        let managed_map = manager.find_map_mut(1, 0).unwrap();
        let mut game_object = GameObject::new();
        game_object
            .world_mut()
            .object_mut()
            .create(game_object_guid);
        game_object.world_mut().set_map(1, 0).unwrap();
        game_object
            .world_mut()
            .relocate(Position::xyz(13.0, 23.0, 33.0));
        game_object.world_mut().object_mut().add_to_world();
        game_object.world_mut().object_mut().set_scale(2.0);
        managed_map
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object).unwrap(),
            )
            .unwrap();
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_send_object_updates_summary_like_cpp(),
        SendObjectUpdatesSummaryLikeCpp {
            queued_before: 1,
            processed: 1,
            cleared_update_masks: 1,
            skipped_not_in_world: 0,
            missing_or_stale: 0,
            fanout_not_represented: 1,
            dynamic_object_values_updates: Vec::new(),
            player_values_updates: Vec::new(),
            unit_values_updates: Vec::new(),
        }
    );
    let object = managed_map
        .map()
        .map_object(game_object_guid)
        .unwrap()
        .object();
    assert!(!object.is_object_updated());
    assert!(object.changed_fields().is_empty());
}

#[test]
fn map_manager_update_visits_live_creature_with_default_context_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 4350101, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(
        managed_map.last_creatures_update_summary(),
        CreatureUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            skipped_missing: 0,
            skipped_non_creature: 0,
            skipped_not_in_world: 0,
            actions_recorded: 3,
        }
    );
    assert!(
        !managed_map
            .map()
            .map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
}

#[test]
fn map_manager_update_skips_not_in_world_creature_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 4350201, false);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_creatures_update_summary(),
        CreatureUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            skipped_missing: 0,
            skipped_non_creature: 0,
            skipped_not_in_world: 1,
            actions_recorded: 0,
        }
    );
    assert!(
        managed_map
            .map()
            .map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
}

#[test]
fn map_manager_update_creature_slice_ignores_other_families_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 4350301, true);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4350302, 10, true);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4350303, 10, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.last_creatures_update_summary().visited, 1);
    assert_eq!(managed_map.last_creatures_update_summary().updated, 1);
    assert!(
        !managed_map
            .map()
            .map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
    assert_eq!(
        managed_map
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        9
    );
    assert_eq!(
        managed_map
            .map()
            .map_object_record(area_trigger_guid)
            .unwrap()
            .area_trigger()
            .unwrap()
            .duration_ms(),
        9
    );
}

#[test]
fn map_manager_update_visits_live_area_trigger_without_expiry_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340101, 10, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_area_triggers_update_summary(),
        AreaTriggersUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_area_trigger: 0,
            not_in_world: 0,
        }
    );
    let area_trigger = managed_map
        .map()
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert_eq!(area_trigger.duration_ms(), 9);
    assert_eq!(area_trigger.time_since_created_ms(), 1);
}

#[test]
fn map_manager_update_expires_area_trigger_then_delayed_update_drains_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340201, 1, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_area_triggers_update_summary(),
        AreaTriggersUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            expired_remove_queued: 1,
            missing_or_stale: 0,
            not_area_trigger: 0,
            not_in_world: 0,
        }
    );
    assert!(
        managed_map
            .map()
            .map_object_record(area_trigger_guid)
            .is_none()
    );
}

#[test]
fn map_manager_update_skips_not_in_world_area_trigger_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340301, 10, false);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_area_triggers_update_summary(),
        AreaTriggersUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_area_trigger: 0,
            not_in_world: 1,
        }
    );
    let area_trigger = managed_map
        .map()
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert_eq!(area_trigger.duration_ms(), 10);
    assert_eq!(area_trigger.time_since_created_ms(), 0);
}

#[test]
fn map_manager_update_area_trigger_slice_ignores_other_families_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340401, 10, true);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4340402, 10, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_area_triggers_update_summary(),
        AreaTriggersUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_area_trigger: 0,
            not_in_world: 0,
        }
    );
    assert_eq!(
        managed_map
            .map()
            .map_object_record(area_trigger_guid)
            .unwrap()
            .area_trigger()
            .unwrap()
            .duration_ms(),
        9
    );
    assert_eq!(
        managed_map
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        9
    );
}

#[test]
fn map_manager_update_visits_live_conversation_without_expiry_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let conversation_guid = insert_conversation_for_update(&mut manager, 4360101, 10, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_conversations_update_summary(),
        ConversationsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_conversation: 0,
            not_in_world: 0,
        }
    );
    let conversation = managed_map
        .map()
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert_eq!(conversation.duration_ms(), 9);
    assert!(!conversation.is_removed());
}

#[test]
fn map_manager_update_expires_conversation_then_delayed_update_drains_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let conversation_guid = insert_conversation_for_update(&mut manager, 4360201, 1, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_conversations_update_summary(),
        ConversationsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            expired_remove_queued: 1,
            missing_or_stale: 0,
            not_conversation: 0,
            not_in_world: 0,
        }
    );
    assert!(
        managed_map
            .map()
            .map_object_record(conversation_guid)
            .is_none()
    );
}

#[test]
fn map_manager_update_visits_live_scene_object_without_removal_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let scene_object_guid = insert_scene_object_for_update(
        &mut manager,
        4370101,
        true,
        Some(guid(HighGuid::Cast, 4370102, 1, 0)),
    );

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_scene_objects_update_summary(),
        SceneObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            remove_queued: 0,
            missing_or_stale: 0,
            not_scene_object: 0,
            not_in_world: 0,
        }
    );
    assert!(
        managed_map
            .map()
            .map_object_record(scene_object_guid)
            .unwrap()
            .scene_object()
            .is_some()
    );
}

#[test]
fn map_manager_update_queues_scene_object_removal_then_delayed_update_drains_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let scene_object_guid =
        insert_scene_object_for_update(&mut manager, 4370201, true, ObjectGuid::EMPTY.into());
    {
        let managed_map = manager.find_map_mut(1, 0).unwrap();
        let summary = managed_map
            .map_mut()
            .update_scene_objects_like_cpp(1, |_guid, _scene| SceneObjectUpdateContextLikeCpp {
                creator_exists: false,
                linked_aura_exists: true,
            });
        assert_eq!(summary.visited, 1);
        assert_eq!(summary.remove_queued, 1);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 1);
        assert!(
            managed_map
                .map()
                .map_object_record(scene_object_guid)
                .is_some()
        );
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert!(
        managed_map
            .map()
            .map_object_record(scene_object_guid)
            .is_none()
    );
}

#[test]
fn update_destroys_unloadable_maps_before_delayed_update() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_map_entry(
        33,
        7,
        1,
        ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    manager.find_map_mut(33, 7).unwrap().set_can_unload(true);
    manager.init_instance_ids(10);
    for instance_id in 1..=7 {
        manager.register_instance_id(instance_id);
    }

    assert_eq!(manager.update(1), Some(1));

    assert!(manager.find_map(33, 7).is_none());
    assert_eq!(manager.next_instance_id(), 7);
}

#[test]
fn create_instanced_map_registers_instance_id_like_cpp() {
    let mut manager = MapManager::default();
    manager.create_map_entry(
        631,
        1,
        3,
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );
    manager.create_map_entry(489, 2, 0, ManagedMapKind::Battleground);

    assert_eq!(manager.generate_instance_id(), Some(3));
}

#[test]
fn instance_id_allocator_reuses_lowest_freed_id() {
    let mut allocator = InstanceIdAllocator::new();
    allocator.init_instance_ids(3);
    allocator.register_instance_id(1);
    allocator.register_instance_id(2);

    assert_eq!(allocator.generate_instance_id(), Some(3));
    allocator.free_instance_id(2);
    assert_eq!(allocator.generate_instance_id(), Some(2));
}

#[test]
fn scheduled_script_counter_saturates_on_decrease() {
    let mut manager = MapManager::default();

    manager.increase_scheduled_scripts_count();
    assert!(manager.is_script_scheduled());
    manager.decrease_scheduled_script_count_by(2);
    assert!(!manager.is_script_scheduled());
}

#[test]
fn activated_map_updater_uses_schedule_and_wait_path() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    manager.map_updater_mut().activate(2);

    assert_eq!(manager.update(1), Some(1));

    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.update_calls(), &[1]);
    assert_eq!(manager.map_updater().scheduled_updates(), 1);
    assert_eq!(manager.map_updater().wait_calls(), 1);
    assert!(manager.map_updater().activated());

    manager.map_updater_mut().deactivate();
    assert!(!manager.map_updater().activated());
}
