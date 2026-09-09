//! Instance lifecycle state regression scenarios, part 1 of 2.
//!
//! Moved out of the lib.rs root under #658; every test is unchanged.

use super::*;

#[test]
fn map_db2_entries_key_and_binding_match_cpp() {
    let entries = raid_entries();

    assert_eq!(entries.key(), (631, 7));
    assert!(entries.is_instance_id_bound());
    assert!(!flex_entries().is_instance_id_bound());
    assert!(
        !MapDb2Entries {
            reset_interval: MapDifficultyResetInterval::Anytime,
            ..entries
        }
        .has_reset_schedule()
    );
}

#[test]
fn map_db2_entries_from_stores_match_cpp_fields() {
    let maps = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 631,
        instance_type: wow_data::map::MAP_RAID,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
        flags2: 0,
    }]);
    let difficulties = wow_data::MapDifficultyStore::from_entries([wow_data::MapDifficultyEntry {
        id: 900,
        message: String::new(),
        map_id: 631,
        difficulty_id: 15,
        lock_id: 7,
        reset_interval: 2,
        max_players: 25,
        flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
    }]);

    let entries = MapDb2Entries::from_stores_like_cpp(&maps, &difficulties, 631, 15).unwrap();

    assert_eq!(
        entries,
        MapDb2Entries {
            map_id: 631,
            difficulty_id: 15,
            lock_id: 7,
            reset_interval: MapDifficultyResetInterval::Weekly,
            max_players: 25,
            is_flex_locking: true,
            is_using_encounter_locks: true,
        }
    );
    assert!(MapDb2Entries::from_stores_like_cpp(&maps, &difficulties, 631, 3).is_none());
}

#[test]
fn map_db2_entries_from_downscaled_stores_match_cpp_fields() {
    let maps = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 33,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]);
    let difficulties = wow_data::DifficultyStore::from_entries([
        wow_data::DifficultyEntry {
            id: 5,
            instance_type: 1,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        wow_data::DifficultyEntry {
            id: 2,
            instance_type: 1,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
    ]);
    let map_difficulties =
        wow_data::MapDifficultyStore::from_entries([wow_data::MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 33,
            difficulty_id: 2,
            lock_id: 9,
            reset_interval: 1,
            max_players: 5,
            flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
        }]);

    let entries = MapDb2Entries::from_downscaled_stores_like_cpp(
        &maps,
        &map_difficulties,
        &difficulties,
        33,
        5,
    )
    .unwrap();

    assert_eq!(
        entries,
        MapDb2Entries {
            map_id: 33,
            difficulty_id: 2,
            lock_id: 9,
            reset_interval: MapDifficultyResetInterval::Daily,
            max_players: 5,
            is_flex_locking: false,
            is_using_encounter_locks: true,
        }
    );
}

#[test]
fn reset_schedule_default_matches_cpp_world_config_defaults() {
    assert_eq!(
        ResetSchedule::default(),
        ResetSchedule {
            hour: 8,
            week_day: 2,
        }
    );
}

#[test]
fn next_reset_time_daily_and_weekly_match_cpp_hour_rules() {
    let daily = MapDb2Entries {
        reset_interval: MapDifficultyResetInterval::Daily,
        ..raid_entries()
    };
    let schedule = ResetSchedule {
        hour: 9,
        week_day: 2,
    };
    let day10_08 = 10 * 86_400 + 8 * 3_600;
    let day10_10 = 10 * 86_400 + 10 * 3_600;

    assert_eq!(
        next_reset_time_at(&daily, schedule, day10_08),
        10 * 86_400 + 9 * 3_600
    );
    assert_eq!(
        next_reset_time_at(&daily, schedule, day10_10),
        11 * 86_400 + 9 * 3_600
    );

    let weekly = raid_entries();
    let tuesday_08 = 5 * 86_400 + 8 * 3_600;
    let tuesday_10 = 5 * 86_400 + 10 * 3_600;

    assert_eq!(
        next_reset_time_at(&weekly, schedule, tuesday_08),
        5 * 86_400 + 9 * 3_600
    );
    assert_eq!(
        next_reset_time_at(&weekly, schedule, tuesday_10),
        12 * 86_400 + 9 * 3_600
    );
}

#[test]
fn create_instance_lock_for_new_instance_stores_temporary_new_lock_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    let lock = mgr
        .create_instance_lock_for_new_instance_at(
            player(1),
            &entries,
            9001,
            ResetSchedule::default(),
            100,
        )
        .unwrap();

    assert_eq!(lock.instance_id, 9001);
    assert!(lock.is_new);
    assert!(mgr.statistics().instance_count == 1);
    assert!(
        mgr.find_active_instance_lock_at(player(1), &entries, 100)
            .unwrap()
            .is_new
    );
    assert_eq!(mgr.statistics().player_count, 0);
}

#[test]
fn find_active_instance_lock_honors_extended_expired_and_temporary_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(100, None),
        ResetSchedule::default(),
        100,
    );
    mgr.instance_locks_by_player
        .get_mut(&player(1))
        .unwrap()
        .get_mut(&entries.key())
        .unwrap()
        .expiry_time = 10;
    assert!(
        mgr.find_active_instance_lock_at(player(1), &entries, 100)
            .is_none()
    );

    mgr.instance_locks_by_player
        .get_mut(&player(1))
        .unwrap()
        .get_mut(&entries.key())
        .unwrap()
        .extended = true;
    assert!(
        mgr.find_active_instance_lock_at(player(1), &entries, 100)
            .is_some()
    );

    mgr.create_instance_lock_for_new_instance_at(
        player(2),
        &entries,
        200,
        ResetSchedule::default(),
        100,
    );
    assert!(
        mgr.find_active_instance_lock_at(player(2), &entries, 100)
            .is_some()
    );
}

#[test]
fn set_active_instance_lock_instance_id_updates_active_lock_like_cpp() {
    let entries = flex_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.create_instance_lock_for_new_instance_at(
        player(1),
        &entries,
        9001,
        ResetSchedule::default(),
        100,
    );
    assert!(mgr.set_active_instance_lock_instance_id_at(player(1), &entries, 100, 9002));

    assert_eq!(
        mgr.find_active_instance_lock_at(player(1), &entries, 100)
            .unwrap()
            .instance_id,
        9002
    );
}

#[test]
fn set_active_instance_lock_instance_id_skips_expired_permanent_like_cpp() {
    let entries = flex_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(9001, None),
        ResetSchedule::default(),
        100,
    );
    mgr.instance_locks_by_player
        .get_mut(&player(1))
        .unwrap()
        .get_mut(&entries.key())
        .unwrap()
        .expiry_time = 10;

    assert!(!mgr.set_active_instance_lock_instance_id_at(player(1), &entries, 100, 9002));
    assert!(
        mgr.find_active_instance_lock_at(player(1), &entries, 100)
            .is_none()
    );
}

#[test]
fn update_instance_lock_promotes_temporary_and_merges_masks_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.create_instance_lock_for_new_instance_at(
        player(1),
        &entries,
        9001,
        ResetSchedule::default(),
        100,
    );
    let lock = mgr
        .update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(9001, Some(1)),
            ResetSchedule::default(),
            100,
        )
        .unwrap();

    assert_eq!(lock.instance_id, 9001);
    assert!(!lock.is_new);
    assert_eq!(lock.data.data, "bosses:1");
    assert_eq!(lock.data.completed_encounters_mask, 0b110);
    assert_eq!(lock.data.entrance_world_safe_loc_id, 42);
    assert!(
        !mgr.temporary_instance_locks_by_player
            .contains_key(&player(1))
    );
    assert_eq!(mgr.statistics().player_count, 1);
}

#[test]
fn update_instance_lock_plan_appends_delete_insert_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();
    let mut plan = InstanceLockPersistencePlanLikeCpp::default();

    let lock = mgr
        .update_instance_lock_for_player_with_persistence_at(
            &mut plan,
            player(1),
            &entries,
            update_event(9001, Some(1)),
            ResetSchedule::default(),
            100,
        )
        .unwrap();

    assert_eq!(plan.len(), 2);
    assert!(matches!(
        plan.mutations.as_slice(),
        [
            InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock { .. },
            InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock { .. }
        ]
    ));
    assert_eq!(lock.instance_id, 9001);
    assert_eq!(lock.data.completed_encounters_mask, 0b110);
}

#[test]
fn update_instance_lock_replaces_expired_non_extended_lock_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        InstanceLockUpdateEvent {
            instance_completed_encounters_mask: 0b1000,
            completed_encounter_bit: Some(0),
            ..update_event(100, None)
        },
        ResetSchedule::default(),
        100,
    );
    let old_lock = mgr
        .instance_locks_by_player
        .get_mut(&player(1))
        .unwrap()
        .get_mut(&entries.key())
        .unwrap();
    old_lock.expiry_time = 10;
    old_lock.data.completed_encounters_mask = 0b1001;

    let new_lock = mgr
        .update_instance_lock_for_player_at(
            player(1),
            &entries,
            InstanceLockUpdateEvent {
                instance_completed_encounters_mask: 0,
                completed_encounter_bit: Some(2),
                ..update_event(200, None)
            },
            ResetSchedule::default(),
            100,
        )
        .unwrap();

    assert_eq!(new_lock.instance_id, 200);
    assert_eq!(new_lock.data.completed_encounters_mask, 0b100);
}

#[test]
fn load_from_rows_reconstructs_shared_and_character_locks_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    let issues = mgr.load_from_rows_like_cpp(
        [SharedInstanceLockRow {
            instance_id: 9001,
            data: "shared".to_string(),
            completed_encounters_mask: 0b1010,
            entrance_world_safe_loc_id: 77,
        }],
        [CharacterInstanceLockRow {
            player_guid_counter: 55,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
            instance_id: 9001,
            difficulty_id: entries.difficulty_id,
            data: "player".to_string(),
            completed_encounters_mask: 0b0010,
            entrance_world_safe_loc_id: 11,
            expiry_time: 500,
            extended: true,
        }],
        |_, _| Some(entries),
    );

    assert!(issues.is_empty());
    let locks =
        mgr.get_instance_locks_for_player(ObjectGuid::create_global(HighGuid::Player, 0, 55));
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].data.data, "player");
    assert_eq!(locks[0].instance_initialization_data().data, "shared");
    assert!(locks[0].extended);
    assert_eq!(mgr.statistics().instance_count, 1);
    assert_eq!(mgr.statistics().player_count, 1);
    assert_eq!(mgr.registered_instance_ids_like_cpp_order(), vec![9001]);
}

#[test]
fn raid_info_locks_for_player_match_cpp_send_raid_info_fields() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.load_from_rows_like_cpp(
        [SharedInstanceLockRow {
            instance_id: 9001,
            data: String::new(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
        }],
        [CharacterInstanceLockRow {
            player_guid_counter: 55,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
            instance_id: 9001,
            difficulty_id: entries.difficulty_id,
            data: "player".to_string(),
            completed_encounters_mask: 0b101,
            entrance_world_safe_loc_id: 0,
            expiry_time: 500,
            extended: false,
        }],
        |_, _| Some(entries),
    );

    let views = mgr.get_raid_info_locks_for_player_at(
        ObjectGuid::create_global(HighGuid::Player, 0, 55),
        100,
        ResetSchedule::default(),
        |_, _| Some(entries),
    );

    assert_eq!(
        views,
        vec![InstanceRaidInfoLock {
            instance_id: 9001,
            map_id: entries.map_id,
            difficulty_id: u32::from(entries.difficulty_id),
            time_remaining: 400,
            completed_mask: 0b101,
            locked: true,
            extended: false,
        }]
    );
}

#[test]
fn raid_info_locks_extend_effective_expiry_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.load_from_rows_like_cpp(
        [SharedInstanceLockRow {
            instance_id: 9001,
            data: String::new(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
        }],
        [CharacterInstanceLockRow {
            player_guid_counter: 55,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
            instance_id: 9001,
            difficulty_id: entries.difficulty_id,
            data: String::new(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
            expiry_time: 500,
            extended: true,
        }],
        |_, _| Some(entries),
    );

    let views = mgr.get_raid_info_locks_for_player_at(
        ObjectGuid::create_global(HighGuid::Player, 0, 55),
        100,
        ResetSchedule::default(),
        |_, _| Some(entries),
    );

    assert_eq!(
        views[0].time_remaining as u64,
        400 + entries.reset_interval.raid_duration_secs()
    );
    assert!(views[0].locked);
    assert!(views[0].extended);
}

#[test]
fn load_from_rows_skips_missing_shared_instance_data_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    let issues = mgr.load_from_rows_like_cpp(
        [],
        [CharacterInstanceLockRow {
            player_guid_counter: 55,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
            instance_id: 9001,
            difficulty_id: entries.difficulty_id,
            data: "player".to_string(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
            expiry_time: 500,
            extended: false,
        }],
        |_, _| Some(entries),
    );

    assert_eq!(
        issues,
        vec![InstanceLockLoadIssue::MissingSharedInstanceData {
            player_guid_counter: 55,
            instance_id: 9001
        }]
    );
    assert!(
        mgr.get_instance_locks_for_player(ObjectGuid::create_global(HighGuid::Player, 0, 55))
            .is_empty()
    );
    assert_eq!(mgr.registered_instance_ids_like_cpp_order(), vec![9001]);
}

#[test]
fn cleanup_unreferenced_shared_instance_data_matches_cpp_delete_path() {
    let mut mgr = InstanceLockMgr::default();

    let issues = mgr.load_from_rows_like_cpp(
        [SharedInstanceLockRow {
            instance_id: 9001,
            data: "orphan".to_string(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
        }],
        [],
        |_, _| None,
    );

    assert!(issues.is_empty());
    assert_eq!(mgr.statistics().instance_count, 1);

    let mutation = mgr
        .cleanup_unreferenced_shared_instance_lock_data_like_cpp(9001)
        .unwrap();

    assert_eq!(
        mutation,
        InstanceLockPersistenceMutationLikeCpp::DeleteSharedInstance { instance_id: 9001 }
    );
    assert_eq!(mgr.statistics().instance_count, 0);
}

#[test]
fn cleanup_keeps_referenced_shared_instance_data_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.load_from_rows_like_cpp(
        [SharedInstanceLockRow {
            instance_id: 9001,
            data: "shared".to_string(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
        }],
        [CharacterInstanceLockRow {
            player_guid_counter: 55,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
            instance_id: 9001,
            difficulty_id: entries.difficulty_id,
            data: "player".to_string(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
            expiry_time: 500,
            extended: false,
        }],
        |_, _| Some(entries),
    );

    assert!(
        mgr.cleanup_unreferenced_shared_instance_lock_data_like_cpp(9001)
            .is_none()
    );
    assert_eq!(mgr.statistics().instance_count, 1);
}

#[test]
fn update_shared_instance_lock_plan_appends_delete_insert_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();
    let mut plan = InstanceLockPersistencePlanLikeCpp::default();

    mgr.create_instance_lock_for_new_instance_at(
        player(1),
        &entries,
        9001,
        ResetSchedule::default(),
        100,
    );

    let shared = mgr
        .update_shared_instance_lock_with_persistence(&mut plan, update_event(9001, Some(2)))
        .unwrap();

    assert_eq!(plan.len(), 2);
    assert!(matches!(
        plan.mutations.as_slice(),
        [
            InstanceLockPersistenceMutationLikeCpp::DeleteSharedInstance { .. },
            InstanceLockPersistenceMutationLikeCpp::InsertSharedInstance { .. }
        ]
    ));
    assert_eq!(shared.instance_id, 9001);
    assert_eq!(shared.data.completed_encounters_mask, 0b100);
}

#[test]
fn can_join_instance_lock_blocks_different_non_encounter_instance_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(100, None),
        ResetSchedule::default(),
        100,
    );
    let target_lock = InstanceLock::new(entries.map_id, entries.difficulty_id, 10_000, 200);

    assert_eq!(
        mgr.can_join_instance_lock_at(player(1), &entries, &target_lock, 100),
        TransferAbortReason::LockedToDifferentInstance
    );
}

#[test]
fn can_join_instance_lock_checks_flex_completed_masks_like_cpp() {
    let entries = flex_entries();
    let mut mgr = InstanceLockMgr::default();

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        InstanceLockUpdateEvent {
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: Some(2),
            ..update_event(100, None)
        },
        ResetSchedule::default(),
        100,
    );
    let target_lock = InstanceLock {
        data: InstanceLockData {
            completed_encounters_mask: 0,
            ..InstanceLockData::default()
        },
        ..InstanceLock::new(entries.map_id, entries.difficulty_id, 10_000, 100)
    };

    assert_eq!(
        mgr.can_join_instance_lock_at(player(1), &entries, &target_lock, 100),
        TransferAbortReason::AlreadyCompletedEncounter
    );
}

#[test]
fn update_instance_lock_extension_plan_appends_update_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();
    let mut plan = InstanceLockPersistencePlanLikeCpp::default();
    let schedule = ResetSchedule::default();
    let now = 10 * 86_400;

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(100, None),
        schedule,
        now,
    );

    let (old_expiry, new_expiry) = mgr
        .update_instance_lock_extension_for_player_with_persistence_at(
            &mut plan,
            player(1),
            &entries,
            true,
            schedule,
            now,
        )
        .unwrap();

    assert_eq!(plan.len(), 1);
    assert!(new_expiry > old_expiry);
}

#[test]
fn reset_instance_locks_skips_in_use_and_expires_reset_locks_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();
    let schedule = ResetSchedule::default();
    let now = 10 * 86_400;

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(100, None),
        schedule,
        now,
    );
    mgr.update_instance_lock_for_player_at(
        player(2),
        &entries,
        update_event(200, None),
        schedule,
        now,
    );
    mgr.instance_locks_by_player
        .get_mut(&player(2))
        .unwrap()
        .get_mut(&entries.key())
        .unwrap()
        .is_in_use = true;
    let entries_by_key = HashMap::from([(entries.key(), entries)]);

    let reset_one = mgr.reset_instance_locks_for_player_at(
        player(1),
        None,
        None,
        &entries_by_key,
        schedule,
        now,
    );
    assert_eq!(reset_one.reset.len(), 1);
    assert!(reset_one.failed_to_reset.is_empty());
    assert!(
        mgr.find_active_instance_lock_at(player(1), &entries, now)
            .is_none()
    );

    let reset_two = mgr.reset_instance_locks_for_player_at(
        player(2),
        None,
        None,
        &entries_by_key,
        schedule,
        now,
    );
    assert!(reset_two.reset.is_empty());
    assert_eq!(reset_two.failed_to_reset.len(), 1);
}

#[test]
fn reset_instance_locks_plan_appends_force_expire_like_cpp() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();
    let mut plan = InstanceLockPersistencePlanLikeCpp::default();
    let schedule = ResetSchedule::default();
    let now = 10 * 86_400;

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(100, None),
        schedule,
        now,
    );
    let entries_by_key = HashMap::from([(entries.key(), entries)]);

    let result = mgr.reset_instance_locks_for_player_with_persistence_at(
        &mut plan,
        player(1),
        None,
        None,
        &entries_by_key,
        schedule,
        now,
    );

    assert_eq!(result.reset.len(), 1);
    assert_eq!(plan.len(), 1);
    assert!(
        mgr.find_active_instance_lock_at(player(1), &entries, now)
            .is_none()
    );
}

#[test]
fn player_lock_map_difficulties_are_unique_and_sorted() {
    let entries = raid_entries();
    let mut mgr = InstanceLockMgr::default();
    let schedule = ResetSchedule::default();
    let now = 10 * 86_400;

    mgr.update_instance_lock_for_player_at(
        player(1),
        &entries,
        update_event(100, None),
        schedule,
        now,
    );

    assert_eq!(mgr.player_lock_map_difficulties(player(1)), vec![(631, 4)]);
    assert!(mgr.player_lock_map_difficulties(player(2)).is_empty());
}

#[test]
fn instance_script_create_sets_all_bosses_not_started_like_cpp() {
    let mut script = InstanceScriptBase::new(4, 3);

    script.create_like_cpp();

    assert_eq!(script.boss_state(0), EncounterState::NotStarted);
    assert_eq!(script.boss_state(1), EncounterState::NotStarted);
    assert_eq!(script.boss_state(2), EncounterState::NotStarted);
}

#[test]
fn instance_script_save_data_matches_cpp_json_shape() {
    let mut script = InstanceScriptBase::new(4, 2);
    script.set_header("TEST");
    script.set_boss_state_like_cpp(0, EncounterState::Done);
    script.set_boss_state_like_cpp(1, EncounterState::InProgress);
    script.register_persistent_value_like_cpp("Kills", PersistentInstanceScriptValue::I64(7));
    script.register_persistent_value_like_cpp("Ratio", PersistentInstanceScriptValue::F64(2.5));

    assert_eq!(
        script.get_save_data_like_cpp(),
        "{\"Header\":\"TEST\",\"BossStates\":[3,1],\"AdditionalData\":{\"Kills\":7,\"Ratio\":2.5}}"
    );
}

#[test]
fn instance_script_load_normalizes_transient_boss_states_like_cpp() {
    let mut script = InstanceScriptBase::new(4, 5);
    script.set_header("TEST");
    script.create_like_cpp();

    script
        .load_save_data_like_cpp(
            "{\"Header\":\"TEST\",\"BossStates\":[1,2,3,4,5],\"AdditionalData\":{}}",
        )
        .unwrap();

    assert_eq!(script.boss_state(0), EncounterState::NotStarted);
    assert_eq!(script.boss_state(1), EncounterState::NotStarted);
    assert_eq!(script.boss_state(2), EncounterState::Done);
    assert_eq!(script.boss_state(3), EncounterState::NotStarted);
    assert_eq!(script.boss_state(4), EncounterState::NotStarted);
}
