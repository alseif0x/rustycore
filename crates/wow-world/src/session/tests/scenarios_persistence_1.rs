//! Session scenarios exercising the represented persistence responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn save_first_durable_money_completion_preserves_and_drains_money_event() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_003);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(100);
    let tracker = session.durable_loot_money_persistence_tracker_like_cpp();
    let mut worker = tracker.begin_like_cpp().expect("payout admitted first");
    let applied = Arc::new(AtomicBool::new(false));
    let published = Arc::new(AtomicBool::new(false));

    worker.commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
        durable_money_before: 100,
        durable_money_after: 107,
        durable_applied_amount: 7,
        applied: Arc::clone(&applied),
    });
    let guard = session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("known payout remains writable");
    drop(guard);

    assert_eq!(session.player_gold_like_cpp(), 107);
    assert!(applied.load(Ordering::Acquire));
    assert_eq!(
        session
            .represented_quest_objective_progress_events_like_cpp
            .len(),
        1,
        "save reconciliation must retain the exact MoneyChanged transition"
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyLootMoneyLikeCpp(
            ApplyLootMoneyLikeCppCommand {
                recipient: player_guid,
                loot_owner: ObjectGuid::EMPTY,
                loot_obj: ObjectGuid::EMPTY,
                amount: 7,
                durable_applied_amount: Arc::new(AtomicU64::new(7)),
                durable_persistence_tracker: Arc::clone(&tracker),
                sole_looter: true,
                authority: OwnedLootAuthority::new(),
                authority_generation: 0,
                authority_committed: Arc::new(AtomicBool::new(false)),
                send_coin_removed: Arc::new(AtomicBool::new(false)),
                applied,
                published,
            },
        ))
        .expect("completion command queues");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        session
            .represented_quest_objective_progress_events_like_cpp
            .is_empty(),
        "packet publication must drain the save-first MoneyChanged event"
    );
}
#[test]
fn load_represented_player_difficulties_accepts_selectable_cpp_types() {
    let (mut session, _, _) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(2, MAP_INSTANCE_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
    ])));

    session.load_represented_player_difficulties_like_cpp(2, 15, 4);

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 4);
    assert_eq!(
        session
            .represented_dungeon_difficulty_packet_like_cpp()
            .expect("test Player difficulty owner resolves")
            .difficulty_id,
        2
    );
}
#[test]
fn load_represented_player_difficulties_normalizes_invalid_values_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(2, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            15,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
        difficulty_entry(4, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
    ])));

    session.load_represented_player_difficulties_like_cpp(2, 15, 4);

    assert_eq!(
        session.represented_dungeon_difficulty_id_like_cpp(),
        DIFFICULTY_NORMAL_LIKE_CPP
    );
    assert_eq!(
        session.represented_raid_difficulty_id_like_cpp(),
        DIFFICULTY_NORMAL_RAID_LIKE_CPP
    );
    assert_eq!(
        session.represented_legacy_raid_difficulty_id_like_cpp(),
        DIFFICULTY_10_N_LIKE_CPP
    );
}
#[test]
fn load_represented_player_difficulties_without_store_uses_cpp_defaults() {
    let (mut session, _, _) = make_session();

    session.load_represented_player_difficulties_like_cpp(2, 15, 4);

    assert_eq!(
        session.represented_dungeon_difficulty_id_like_cpp(),
        DIFFICULTY_NORMAL_LIKE_CPP
    );
    assert_eq!(
        session.represented_raid_difficulty_id_like_cpp(),
        DIFFICULTY_NORMAL_RAID_LIKE_CPP
    );
    assert_eq!(
        session.represented_legacy_raid_difficulty_id_like_cpp(),
        DIFFICULTY_10_N_LIKE_CPP
    );
}
#[test]
fn load_represented_group_difficulties_overrides_player_values_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.dungeon_difficulty_id = 2;
    group.raid_difficulty_id = 15;
    group.legacy_raid_difficulty_id = 4;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(2, MAP_INSTANCE_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
    ])));
    session.load_represented_player_difficulties_like_cpp(1, 14, 3);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.load_represented_group_difficulties_like_cpp());

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 4);
}
#[test]
fn load_represented_group_difficulties_without_group_preserves_player_values_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(2, MAP_INSTANCE_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
    ])));
    session.load_represented_player_difficulties_like_cpp(2, 15, 4);

    assert!(!session.load_represented_group_difficulties_like_cpp());

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 4);
}
#[test]
fn load_represented_group_difficulties_missing_registry_entry_preserves_values_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(2, MAP_INSTANCE_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
    ])));
    session.load_represented_player_difficulties_like_cpp(2, 15, 4);
    session.group_guid = Some(77);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::new(PendingInvites::default()),
    );

    assert!(!session.load_represented_group_difficulties_like_cpp());

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 4);
}
#[test]
fn load_represented_group_by_db_store_id_sets_group_and_difficulties_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    assert!(group.change_member_group_like_cpp(leader, 3));
    group.db_store_id = 80_928;
    group.dungeon_difficulty_id = 2;
    group.raid_difficulty_id = 15;
    group.legacy_raid_difficulty_id = 4;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(2, MAP_INSTANCE_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
    ])));
    session.load_represented_player_difficulties_like_cpp(1, 14, 3);
    session.set_player_guid(Some(leader));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.load_represented_group_by_db_store_id_like_cpp(80_928));

    assert_eq!(session.group_guid, Some(group_guid));
    assert_eq!(session.represented_subgroup_like_cpp(), Some(3));
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 4);
}
#[test]
fn load_represented_group_by_db_store_id_clears_missing_group_like_cpp() {
    let (mut session, _, _) = make_session();
    session.group_guid = Some(123);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::new(PendingInvites::default()),
    );

    assert!(!session.load_represented_group_by_db_store_id_like_cpp(80_929));
    assert_eq!(session.group_guid, None);
    assert_eq!(session.represented_subgroup_like_cpp(), None);
}
#[test]
fn reset_group_update_sequence_starts_loaded_group_at_one_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));

    assert!(session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(2)
    );
}
#[test]
fn represented_group_leader_flag_is_set_for_loaded_leader_like_cpp() {
    let (mut session, _, player_guid) = session_with_canonical_player_for_away_like_cpp();
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));

    assert!(session.apply_represented_group_leader_flag_like_cpp());

    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(
            player_guid,
            PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP
        ),
        Some(true)
    );
}
#[test]
fn load_character_reputation_rows_like_cpp_merges_rows_after_identity_and_store() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    let mut faction = FactionEntry::for_test_like_cpp(72, 4);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));

    assert!(session.load_character_reputation_rows_like_cpp([
        crate::reputation::mgr::CharacterReputationRowLikeCpp {
            faction_id: 72,
            standing: 3500,
            flags: (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR).bits(),
        },
    ]));

    let state = session
        .reputation_mgr_like_cpp()
        .get_state(4)
        .expect("reputation state");
    assert_eq!(state.standing, 3500);
    assert!(state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
    assert!(state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
    assert!(!state.need_save);
}
#[test]
fn load_seasonal_quest_status_clears_stale_state_and_resets_changed_on_empty_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_seasonal_quest_status_like_cpp(9, 12_345, 100);
    session.seasonal_quest_changed_like_cpp = true;
    let quest_store = seasonal_quest_store_like_cpp([12_345]);

    let outcome = session.load_seasonal_quest_status_like_cpp([], Some(&quest_store), None);

    assert!(session.seasonal_quests_like_cpp.is_empty());
    assert!(!session.seasonal_quest_changed_like_cpp);
    assert_eq!(outcome.rows_seen, 0);
    assert_eq!(outcome.seasonal_quest_changed, false);
}
#[test]
fn load_seasonal_quest_status_valid_row_populates_and_blocks_can_take_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest = seasonal_test_quest_template(12_345, -376, 9);
    let quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest.clone()]);
    let quest_v2_store = seasonal_quest_v2_store_like_cpp([(12_345, 65)]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: 9,
            completed_time: 100,
        }],
        Some(&quest_store),
        Some(&quest_v2_store),
    );

    assert_eq!(outcome.inserted, 1);
    assert_eq!(outcome.completed_bit_set, 1);
    assert_eq!(
        session.represented_quest_completed_bits_like_cpp,
        BTreeSet::from([65])
    );
    assert_eq!(
        session
            .seasonal_quests_like_cpp
            .get(&9)
            .and_then(|bucket| bucket.get(&12_345)),
        Some(&100)
    );
    assert!(!session.can_take_quest(&quest));
}
#[test]
fn load_seasonal_quest_status_missing_quest_v2_store_inserts_but_skips_bit_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([12_345]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: 9,
            completed_time: 100,
        }],
        Some(&quest_store),
        None,
    );

    assert_eq!(outcome.inserted, 1);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 1);
    assert!(session.represented_quest_completed_bits_like_cpp.is_empty());
}
#[test]
fn load_seasonal_quest_status_zero_unique_bit_inserts_but_skips_bit_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([12_345]);
    let quest_v2_store = seasonal_quest_v2_store_like_cpp([(12_345, 0)]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: 9,
            completed_time: 100,
        }],
        Some(&quest_store),
        Some(&quest_v2_store),
    );

    assert_eq!(outcome.inserted, 1);
    assert_eq!(outcome.completed_bit_skipped_zero_unique_bit, 1);
    assert!(session.represented_quest_completed_bits_like_cpp.is_empty());
}
#[test]
fn load_seasonal_quest_status_repeated_bit_counts_no_change_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([12_345]);
    let quest_v2_store = seasonal_quest_v2_store_like_cpp([(12_345, 65)]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [
            SeasonalQuestStatusDbRowLikeCpp {
                quest_id: 12_345,
                event_id: 9,
                completed_time: 100,
            },
            SeasonalQuestStatusDbRowLikeCpp {
                quest_id: 12_345,
                event_id: 9,
                completed_time: 200,
            },
        ],
        Some(&quest_store),
        Some(&quest_v2_store),
    );

    assert_eq!(outcome.completed_bit_set, 1);
    assert_eq!(outcome.completed_bit_no_change_or_noop, 1);
}
#[test]
fn initial_canonical_player_preserves_loaded_seasonal_unique_bit_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 12_345)));
    session.set_loaded_player_name_like_cpp("SeasonalBit".to_string());
    session.player_position = Some(Position::new(1.0, 2.0, 3.0, 0.0));
    let quest_store = seasonal_quest_store_like_cpp([12_345]);
    let quest_v2_store = seasonal_quest_v2_store_like_cpp([(12_345, 65)]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: 9,
            completed_time: 100,
        }],
        Some(&quest_store),
        Some(&quest_v2_store),
    );

    assert_eq!(outcome.completed_bit_set, 1);
    let player = session
        .initial_player_fixture_like_cpp()
        .expect("canonical player snapshot");
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(1));
}
#[test]
fn load_seasonal_quest_status_skips_missing_quest_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([1]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 2,
            event_id: 9,
            completed_time: 100,
        }],
        Some(&quest_store),
        Some(&seasonal_quest_v2_store_like_cpp([(2, 65)])),
    );

    assert_eq!(outcome.rows_seen, 1);
    assert_eq!(outcome.skipped_missing_quest, 1);
    assert!(session.seasonal_quests_like_cpp.is_empty());
    assert!(!session.seasonal_quest_changed_like_cpp);
}
#[test]
fn load_seasonal_quest_status_duplicate_event_quest_last_row_wins_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([12_345]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [
            SeasonalQuestStatusDbRowLikeCpp {
                quest_id: 12_345,
                event_id: 9,
                completed_time: 100,
            },
            SeasonalQuestStatusDbRowLikeCpp {
                quest_id: 12_345,
                event_id: 9,
                completed_time: 200,
            },
        ],
        Some(&quest_store),
        Some(&seasonal_quest_v2_store_like_cpp([(12_345, 65)])),
    );

    assert_eq!(outcome.inserted, 1);
    assert_eq!(outcome.replaced, 1);
    assert_eq!(
        session
            .seasonal_quests_like_cpp
            .get(&9)
            .and_then(|bucket| bucket.get(&12_345)),
        Some(&200)
    );
}
#[test]
fn load_seasonal_quest_status_event_out_of_range_is_skipped_not_truncated_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([12_345]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: u32::from(u16::MAX) + 1,
            completed_time: 100,
        }],
        Some(&quest_store),
        Some(&seasonal_quest_v2_store_like_cpp([(12_345, 65)])),
    );

    assert_eq!(outcome.skipped_event_out_of_range, 1);
    assert!(session.seasonal_quests_like_cpp.is_empty());
}
#[test]
fn load_seasonal_quest_status_negative_completed_time_is_skipped_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest_store = seasonal_quest_store_like_cpp([12_345]);

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: 9,
            completed_time: -1,
        }],
        Some(&quest_store),
        Some(&seasonal_quest_v2_store_like_cpp([(12_345, 65)])),
    );

    assert_eq!(outcome.skipped_negative_completed_time, 1);
    assert!(session.seasonal_quests_like_cpp.is_empty());
}
#[test]
fn load_seasonal_quest_status_without_quest_store_skips_rows_like_cpp() {
    let (mut session, _, _) = make_session();

    let outcome = session.load_seasonal_quest_status_like_cpp(
        [SeasonalQuestStatusDbRowLikeCpp {
            quest_id: 12_345,
            event_id: 9,
            completed_time: 100,
        }],
        None,
        Some(&seasonal_quest_v2_store_like_cpp([(12_345, 65)])),
    );

    assert_eq!(outcome.skipped_no_quest_store, 1);
    assert!(session.seasonal_quests_like_cpp.is_empty());
    assert!(!session.seasonal_quest_changed_like_cpp);
}
#[test]
fn account_mount_load_adds_faction_counterpart_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_battlenet_account_id(77);
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_definition_store_like_cpp(Arc::new(
        wow_data::MountDefinitionStoreLikeCpp::from_entries([(100, 101)]),
    ));

    session.set_account_mounts_like_cpp(vec![wow_packet::packets::misc::AccountMount {
        spell_id: 100,
        flags: 2,
    }]);

    assert_eq!(
        session.account_mount_rows_like_cpp(),
        vec![
            wow_packet::packets::misc::AccountMount {
                spell_id: 100,
                flags: 2
            },
            wow_packet::packets::misc::AccountMount {
                spell_id: 101,
                flags: 2
            },
        ],
        "C++ CollectionMgr::AddMount recursively stores the faction-specific counterpart with the same flags"
    );
    assert!(session.known_spells_like_cpp().contains(&100));
    assert!(session.known_spells_like_cpp().contains(&101));
    assert_eq!(
        session.account_mount_save_rows_like_cpp(),
        Some(vec![
            AccountMountSaveRowLikeCpp {
                bnet_account_id: 77,
                mount_spell_id: 100,
                flags: 2,
            },
            AccountMountSaveRowLikeCpp {
                bnet_account_id: 77,
                mount_spell_id: 101,
                flags: 2,
            },
        ]),
        "C++ CollectionMgr::SaveAccountMounts persists the final std::map collection, including faction-specific mounts"
    );
}
#[test]
fn canonical_player_persistent_capabilities_follow_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_569);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CapabilityOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_represented_at_login_flags_like_cpp(0x24));
    assert_eq!(
        session.mutate_player_persistent_capability_state_like_cpp(|state| {
            state.weapon_proficiency = 0x10;
            state.armor_proficiency = 0x20;
        }),
        Some(())
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_represented_at_login_flags_like_cpp(),
        Some(0x24)
    );

    let replacement_state = wow_entities::PlayerPersistentCapabilityStateLikeCpp {
        at_login_flags: 0x40,
        weapon_proficiency: 0x80,
        armor_proficiency: 0x100,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().persistent_capabilities = replacement_state;
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_represented_at_login_flags_like_cpp(), None);
    assert!(!session.set_represented_at_login_flags_like_cpp(0x2));
    assert_eq!(
        session.mutate_player_persistent_capability_state_like_cpp(|state| {
            state.weapon_proficiency = 0;
            state.armor_proficiency = 0;
        }),
        None
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().persistent_capabilities
            }),
        Some(replacement_state)
    );
}
