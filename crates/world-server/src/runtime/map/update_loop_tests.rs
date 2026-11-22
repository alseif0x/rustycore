use super::*;
use std::sync::Arc;

fn empty_respawn_caches() -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Default::default(),
        sparring_store: Default::default(),
        difficulty_store: Default::default(),
        base_stats_store: Default::default(),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: Default::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Default::default(),
        creature_addon_store: Default::default(),
        spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Default::default(),
        gameobject_override_store: Default::default(),
    }
}

fn cleanup_test_entries(
    map_id: u32,
    difficulty_id: u8,
    lock_id: u32,
) -> wow_instances::MapDb2Entries {
    wow_instances::MapDb2Entries {
        map_id,
        difficulty_id,
        lock_id,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 25,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    }
}

fn cleanup_test_lock_context(
    key: wow_map::MapKey,
    difficulty_id: u8,
    owner: wow_core::ObjectGuid,
) -> wow_map::CreateMapInstanceLockContext {
    wow_map::CreateMapInstanceLockContext {
        instance_id: key.instance_id,
        difficulty_id,
        token: u64::from(key.map_id) << 32 | u64::from(key.instance_id),
        owner_guid_counter: owner.counter() as u64,
    }
}

fn create_cleanup_test_map(
    manager: &mut wow_map::MapManager,
    key: wow_map::MapKey,
    context: Option<wow_map::CreateMapInstanceLockContext>,
    owning_group_guid: Option<u64>,
) {
    let map = manager.create_map_entry(
        key.map_id,
        key.instance_id,
        context.map_or(1, |context| context.difficulty_id),
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: context.is_some(),
        },
    );
    map.set_instance_lock_context(context);
    if let Some(group_guid) = owning_group_guid {
        assert!(map.try_set_owning_group_like_cpp(group_guid));
    }
    map.set_can_unload(true);
}

#[test]
fn destroyed_map_cleanup_clears_exact_locks_and_groups_idempotently_like_cpp() {
    let realm_id = 7;
    let owner = wow_core::ObjectGuid::create_player(realm_id, 42);
    let persistent_key = wow_map::MapKey::new(631, 100);
    let temporary_key = wow_map::MapKey::new(632, 200);
    let solo_key = wow_map::MapKey::new(633, 300);
    let unrelated_key = wow_map::MapKey::new(999, 999);
    let persistent_entries = cleanup_test_entries(631, 3, 10);
    let temporary_entries = cleanup_test_entries(632, 4, 20);
    let unrelated_entries = cleanup_test_entries(999, 2, 30);
    let mut lock_mgr = wow_instances::InstanceLockMgr::default();
    let load_issues = lock_mgr.load_from_rows_like_cpp(
        realm_id,
        [],
        [wow_instances::CharacterInstanceLockRow {
            player_guid_counter: owner.counter() as u64,
            map_id: persistent_entries.map_id,
            lock_id: persistent_entries.lock_id,
            instance_id: 100,
            difficulty_id: persistent_entries.difficulty_id,
            data: String::new(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
            expiry_time: 500,
            extended: false,
        }],
        |_, _| Some(persistent_entries),
    );
    assert!(load_issues.is_empty());
    assert!(
        lock_mgr
            .find_instance_lock_exact_like_cpp(owner, 631, 3, 100)
            .is_some()
    );
    assert!(
        lock_mgr
            .find_instance_lock_exact_like_cpp(
                wow_core::ObjectGuid::create_player(1, 42),
                631,
                3,
                100
            )
            .is_none()
    );
    lock_mgr.create_instance_lock_for_new_instance_at(
        owner,
        &temporary_entries,
        200,
        wow_instances::ResetSchedule::default(),
        100,
    );
    lock_mgr.update_instance_lock_for_player_at(
        owner,
        &unrelated_entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 999,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        100,
    );
    for (entries, instance_id) in [
        (&persistent_entries, 100),
        (&temporary_entries, 200),
        (&unrelated_entries, 999),
    ] {
        assert!(lock_mgr.set_instance_lock_in_use_like_cpp(
            owner,
            entries.map_id,
            entries.difficulty_id,
            instance_id,
            true,
        ));
    }
    let lock_mgr = Arc::new(std::sync::RwLock::new(lock_mgr));

    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    let persistent_group = wow_social::group::GroupInfo::new(owner);
    let persistent_group_guid = persistent_group.group_guid;
    group_registry.register_group_like_cpp(persistent_group_guid, persistent_group);
    let temporary_group = wow_social::group::GroupInfo::new(owner);
    let temporary_group_guid = temporary_group.group_guid;
    group_registry.register_group_like_cpp(temporary_group_guid, temporary_group);
    let unrelated_group = wow_social::group::GroupInfo::new(owner);
    let unrelated_group_guid = unrelated_group.group_guid;
    group_registry.register_group_like_cpp(unrelated_group_guid, unrelated_group);
    for (group_guid, key) in [
        (persistent_group_guid, persistent_key),
        (temporary_group_guid, temporary_key),
        (unrelated_group_guid, unrelated_key),
    ] {
        group_registry
            .link_owned_instance_transition_like_cpp(group_guid, key.map_id, key.instance_id)
            .unwrap();
    }

    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 1);
    create_cleanup_test_map(
        &mut manager,
        persistent_key,
        Some(cleanup_test_lock_context(persistent_key, 3, owner)),
        Some(persistent_group_guid),
    );
    create_cleanup_test_map(
        &mut manager,
        temporary_key,
        Some(cleanup_test_lock_context(temporary_key, 4, owner)),
        Some(temporary_group_guid),
    );
    create_cleanup_test_map(&mut manager, solo_key, None, None);
    let plan = manager
        .begin_tick_like_cpp(1)
        .into_started()
        .expect("destruction tick");
    assert_eq!(plan.destroyed_maps_like_cpp().len(), 3);
    let owners = CanonicalMapLifecycleOwnersLikeCpp::new(
        Arc::clone(&lock_mgr),
        Arc::clone(&group_registry),
        realm_id,
    );

    let summary = consume_destroyed_map_cleanup_like_cpp(plan.destroyed_maps_like_cpp(), &owners);

    assert_eq!(
        summary,
        DestroyedMapCleanupSummaryLikeCpp {
            locks_cleared: 2,
            lock_cleanup_failures: 0,
            group_links_removed: 2,
        }
    );
    let locks = lock_mgr.read().unwrap();
    for (entries, expected) in [
        (&persistent_entries, false),
        (&temporary_entries, false),
        (&unrelated_entries, true),
    ] {
        assert_eq!(
            locks
                .find_active_instance_lock_at(owner, entries, 100)
                .unwrap()
                .is_in_use,
            expected
        );
    }
    drop(locks);
    for group_guid in [persistent_group_guid, temporary_group_guid] {
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .owned_instances_like_cpp()
                .count(),
            0
        );
    }
    assert_eq!(
        group_registry
            .get(&unrelated_group_guid)
            .unwrap()
            .owned_instances_like_cpp()
            .count(),
        1
    );

    let repeated = consume_destroyed_map_cleanup_like_cpp(plan.destroyed_maps_like_cpp(), &owners);
    assert_eq!(
        repeated,
        DestroyedMapCleanupSummaryLikeCpp {
            locks_cleared: 2,
            lock_cleanup_failures: 0,
            group_links_removed: 0,
        }
    );
}

#[test]
fn destroyed_map_lock_failure_does_not_skip_group_unlink_like_cpp() {
    let realm_id = 7;
    let owner = wow_core::ObjectGuid::create_player(realm_id, 42);
    let key = wow_map::MapKey::new(631, 100);
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    let group = wow_social::group::GroupInfo::new(owner);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    group_registry
        .link_owned_instance_transition_like_cpp(group_guid, key.map_id, key.instance_id)
        .unwrap();
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 1);
    create_cleanup_test_map(
        &mut manager,
        key,
        Some(cleanup_test_lock_context(key, 3, owner)),
        Some(group_guid),
    );
    let plan = manager
        .begin_tick_like_cpp(1)
        .into_started()
        .expect("destruction tick");
    let owners = CanonicalMapLifecycleOwnersLikeCpp::new(
        Arc::new(std::sync::RwLock::new(
            wow_instances::InstanceLockMgr::default(),
        )),
        Arc::clone(&group_registry),
        realm_id,
    );

    let summary = consume_destroyed_map_cleanup_like_cpp(plan.destroyed_maps_like_cpp(), &owners);

    assert_eq!(
        summary,
        DestroyedMapCleanupSummaryLikeCpp {
            locks_cleared: 0,
            lock_cleanup_failures: 1,
            group_links_removed: 1,
        }
    );
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .owned_instances_like_cpp()
            .count(),
        0
    );
}

#[test]
fn map_session_pass_player_recovery_prelude_db_intents_survive_abandon_without_replay() {
    let map_key = wow_map::MapKey::new(571, 0);
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(map_key.map_id, map_key.instance_id);
    let plan =
        canonical_map_tick_begin_like_cpp(&mut manager, 100).expect("canonical map tick begins");
    let spawn_metadata = crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::default();
    let conditions = wow_data::ConditionEntriesByTypeStore::default();
    let maps = wow_data::MapStore::from_entries(std::iter::empty());
    let caches = empty_respawn_caches();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);
    let mut prelude = prepare_map_recovery_prelude(
        &mut manager,
        plan.plan,
        &mut scheduler,
        &spawn_metadata,
        &conditions,
        &maps,
        &caches,
    )
    .expect("respawn prelude owns the admitted plan");

    let save_key = wow_persistence::RespawnPersistenceKeyLikeCpp {
        object_type_raw: wow_map::SpawnObjectType::Creature as u16,
        spawn_id: 701,
        map_id: u16::try_from(map_key.map_id).expect("fixture map ID fits persistence key"),
        instance_id: map_key.instance_id,
    };
    let delete_key = wow_persistence::RespawnPersistenceKeyLikeCpp {
        object_type_raw: wow_map::SpawnObjectType::GameObject as u16,
        spawn_id: 702,
        map_id: u16::try_from(map_key.map_id).expect("fixture map ID fits persistence key"),
        instance_id: map_key.instance_id,
    };
    prelude
        .summary_mut()
        .respawn_db_saves
        .push(super::super::RespawnDbSaveLikeCpp {
            object_type: wow_map::SpawnObjectType::Creature,
            spawn_id: save_key.spawn_id,
            respawn_time: 1234,
            map_id: save_key.map_id,
            instance_id: save_key.instance_id,
            mutation: wow_persistence::RespawnPersistenceMutationLikeCpp::Save {
                key: save_key,
                respawn_time: 1234,
            },
        });
    prelude
        .summary_mut()
        .respawn_db_deletes
        .push(super::super::RespawnDbDeleteLikeCpp {
            object_type: wow_map::SpawnObjectType::GameObject,
            spawn_id: delete_key.spawn_id,
            map_id: delete_key.map_id,
            instance_id: delete_key.instance_id,
            mutation: wow_persistence::RespawnPersistenceMutationLikeCpp::Delete {
                key: delete_key,
            },
        });

    let writer = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    submit_respawn_db_mutations(prelude.summary_mut(), &writer);
    assert!(prelude.summary().respawn_db_saves.is_empty());
    assert!(prelude.summary().respawn_db_deletes.is_empty());

    // A second call cannot replay drained prelude work. The queue remains
    // producer-owned across the later tick abandonment; this test performs no
    // database I/O.
    submit_respawn_db_mutations(prelude.summary_mut(), &writer);
    manager.abandon_tick_like_cpp(prelude.into_plan());
    let state = writer
        .mailbox
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.queue.pending_len(), 2);
    assert!(state.queue.pending.contains_key(&(save_key, None)));
    assert!(state.queue.pending.contains_key(&(delete_key, None)));
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::Idle
    );
}
