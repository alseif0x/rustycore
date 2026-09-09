//! Session scenarios exercising the represented persistence responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn logout_save_snapshot_uses_canonical_health_when_session_mirror_is_stale_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 75);
    let position = Position::new(77.0, 88.0, 99.0, 2.5);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CanonicalDamaged".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_player_health_like_cpp(100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(41);
        })
        .unwrap();

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("snapshot should exist");

    assert_eq!(snapshot.health, 41);
    assert_eq!(snapshot.max_health, 100);
    assert_eq!(session.player_health_like_cpp(), 41);
}
#[test]
fn logout_save_snapshot_ignores_stale_test_fixture_death_and_reads_canonical_player_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 76);
    let position = Position::new(77.0, 88.0, 99.0, 2.5);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SessionDead".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_player_health_like_cpp(100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
        })
        .unwrap();

    // C++ has a single Player object. Deliberately poison the test-only legacy
    // fixture and prove SaveToDB still reads the canonical owner.
    session.player_health_like_cpp = 0;
    session.player_alive_like_cpp = false;

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("snapshot should exist");

    assert_eq!(snapshot.health, 100);
    assert_eq!(snapshot.max_health, 100);
    assert_eq!(session.player_health_like_cpp(), 100);
}
#[test]
fn player_save_transaction_plan_orders_represented_statements_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 5010);
    let powers = loaded_character_power_snapshot_like_cpp([321, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let snapshot = PlayerSaveToDbSnapshotLikeCpp {
        guid,
        map_id: 571,
        instance_id: 7,
        position: Position::new(11.0, 22.0, 33.0, 1.5),
        level: 70,
        xp: 12_345,
        money: 67_890,
        health: 444,
        max_health: 555,
        powers,
    };
    session.set_player_guid(Some(guid));
    session.set_player_level_like_cpp(70);
    session.set_player_xp_like_cpp(12_345);
    session.set_player_gold_like_cpp(67_890);
    session.set_loaded_player_powers_like_cpp([321, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(session.set_complete_player_skill_records_like_cpp(
        HashMap::from([(
            762,
            RepresentedPlayerSkillLikeCpp {
                skill_id: 762,
                step: 1,
                value: 75,
                max: 75,
                profession_slot: -1,
                state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        )]),
        1,
    ));
    session.player_quests.insert(
        8_888,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 8_888,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: true,
            accept_time_secs: 123,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    let request = session
        .current_player_character_save_request_like_cpp(&snapshot, 1_000)
        .expect("determinate tracker should allow a full-save request");

    assert_eq!(request.player_guid, guid.counter() as u64);
    assert_eq!(request.character.level, 70);
    assert_eq!(request.character.xp, 12_345);
    assert_eq!(request.character.money, 67_890);
    assert_eq!(
        request.character.powers,
        Some([321, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    );
    assert_eq!(
        request.skills.as_deref(),
        Some(
            [wow_persistence::PlayerSkillSaveLikeCpp {
                skill_id: 762,
                value: 75,
                max: 75,
                profession_slot: -1,
            }]
            .as_slice()
        )
    );
    // The SQLx-free request has no quest-status field: C++ `_SaveQuestStatus`
    // consumes only its dirty set, which this represented snapshot does not own.
}
#[test]
fn ambiguous_absolute_money_commit_requires_exact_changed_row_evidence_like_cpp() {
    assert_eq!(
        reconcile_absolute_player_money_commit_like_cpp(100, 75, Some(75)),
        AbsolutePlayerMoneyCommitReconciliationLikeCpp::Committed
    );
    assert_eq!(
        reconcile_absolute_player_money_commit_like_cpp(100, 75, Some(100)),
        AbsolutePlayerMoneyCommitReconciliationLikeCpp::RolledBack
    );
    assert_eq!(
        reconcile_absolute_player_money_commit_like_cpp(100, 75, Some(90)),
        AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate
    );
    assert_eq!(
        reconcile_absolute_player_money_commit_like_cpp(100, 75, None),
        AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate
    );
    assert_eq!(
        reconcile_absolute_player_money_commit_like_cpp(100, 100, Some(100)),
        AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate,
        "an unchanged money row cannot prove whether bundled non-money statements committed"
    );
}
#[tokio::test]
async fn failed_exclusive_money_persistence_never_publishes_runtime_and_reopens_admission() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 5_012);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(100);
    session.set_loot_money_persistence_test_result_like_cpp(false);
    let tracker = session.durable_loot_money_persistence_tracker_like_cpp();

    assert_eq!(
        session
            .mutate_and_persist_player_gold_exclusive_like_cpp(|money| money + 25)
            .await,
        None
    );
    assert_eq!(session.player_gold_like_cpp(), 100);
    assert!(
        tracker.begin_like_cpp().is_ok(),
        "a definite persistence failure must drop the save fence and reopen payout admission"
    );
}
#[tokio::test]
async fn indeterminate_money_state_blocks_full_save_reconciliation_like_cpp() {
    let (mut session, _, _) = make_session();
    let tracker = session.durable_loot_money_persistence_tracker_like_cpp();
    tracker.mark_indeterminate_like_cpp();

    assert!(
        !session
            .reconcile_durable_loot_money_before_save_like_cpp()
            .await
    );
    assert!(tracker.is_indeterminate_like_cpp());
    assert!(
        session
            .current_player_character_save_request_like_cpp(
                &PlayerSaveToDbSnapshotLikeCpp {
                    guid: ObjectGuid::create_player(1, 50_013),
                    map_id: 0,
                    instance_id: 0,
                    position: Position::default(),
                    level: 1,
                    xp: 0,
                    money: 0,
                    health: 1,
                    max_health: 1,
                    powers: empty_character_power_snapshot_like_cpp(),
                },
                0,
            )
            .is_none(),
        "an indeterminate money transaction must prevent every partial full-save plan"
    );
}
#[test]
fn cancelled_money_commit_fence_permanently_closes_admission_before_disconnect_save() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    {
        let _armed = PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(&tracker));
    }

    assert!(tracker.is_indeterminate_like_cpp());
    assert!(tracker.begin_like_cpp().is_err());

    let safe_tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    {
        let mut known_outcome =
            PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(&safe_tracker));
        known_outcome.disarm_like_cpp();
    }
    assert!(!safe_tracker.is_indeterminate_like_cpp());
    assert!(safe_tracker.begin_like_cpp().is_ok());
}
#[test]
fn player_save_plan_marks_dirty_state_only_after_commit_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 5011);
    let snapshot = PlayerSaveToDbSnapshotLikeCpp {
        guid,
        map_id: 571,
        instance_id: 7,
        position: Position::new(11.0, 22.0, 33.0, 1.5),
        level: 70,
        xp: 12_345,
        money: 67_890,
        health: 444,
        max_health: 555,
        powers: loaded_character_power_snapshot_like_cpp([321, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    };
    session.set_player_guid(Some(guid));
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id: 13_337,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::New,
            },
        ])
    );

    session.tutorials_loaded_coherently_like_cpp = true;
    session.tutorials_loaded_from_db_like_cpp = false;
    session.tutorials_changed_like_cpp = true;

    session.mark_represented_equipment_sets_loaded_like_cpp();
    let mut changed_equipment = RepresentedEquipmentSetLikeCpp::equipment(
        9,
        0,
        RepresentedEquipmentSetUpdateStateLikeCpp::Changed,
    );
    changed_equipment.guid = 900;
    session.insert_represented_equipment_set_like_cpp(900, changed_equipment);

    session
        .reputation_mgr_like_cpp_mut()
        .insert_state_for_test_like_cpp(crate::reputation::mgr::FactionStateLikeCpp {
            standing: 123,
            flags: ReputationFlagsLikeCpp::VISIBLE,
            ..crate::reputation::mgr::FactionStateLikeCpp::new_like_cpp(
                85,
                14,
                ReputationFlagsLikeCpp::VISIBLE,
            )
        });

    let request = session
        .current_player_character_save_request_like_cpp(&snapshot, 1_000)
        .expect("determinate tracker should allow a full-save request");
    let committed = request.committed_groups_like_cpp();

    assert!(committed.tutorials_changed);
    assert!(committed.player_spells);
    assert!(matches!(
        &request.spells,
        Some(wow_persistence::PlayerSpellSaveGroupLikeCpp::Complete { rows, .. })
            if rows.iter().any(|spell| spell.spell_id == 13_337)
    ));
    assert!(committed.equipment_sets);
    assert!(committed.reputation);
    assert!(session.tutorials_changed_like_cpp);
    assert!(!session.tutorials_loaded_from_db_like_cpp);
    assert_eq!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .and_then(|rows| rows.get(&13_337).copied())
            .map(|spell| spell.state),
        Some(RepresentedPlayerSpellStateLikeCpp::New),
        "a failed full-save transaction must leave _SaveSpells state dirty for retry"
    );
    assert_eq!(
        session
            .represented_equipment_set_like_cpp(900)
            .expect("equipment set")
            .state,
        RepresentedEquipmentSetUpdateStateLikeCpp::Changed,
        "failed transaction must leave equipment-set state dirty for retry"
    );
    assert!(
        session
            .reputation_mgr_like_cpp()
            .get_state(14)
            .expect("reputation state")
            .need_save,
        "failed transaction must leave reputation state dirty for retry"
    );

    session.mark_current_player_save_to_db_committed_like_cpp(&committed);

    assert!(!session.tutorials_changed_like_cpp);
    assert_eq!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .and_then(|rows| rows.get(&13_337).copied())
            .map(|spell| spell.state),
        Some(RepresentedPlayerSpellStateLikeCpp::Unchanged),
        "successful Player::SaveToDB consumes the same dirty state as C++ _SaveSpells"
    );
    assert!(session.tutorials_loaded_from_db_like_cpp);
    assert_eq!(
        session
            .represented_equipment_set_like_cpp(900)
            .expect("equipment set")
            .state,
        RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged
    );
    assert!(
        !session
            .reputation_mgr_like_cpp()
            .get_state(14)
            .expect("reputation state")
            .need_save
    );
}
#[test]
fn player_save_requires_complete_skill_authority_before_delete_all_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 5012);
    let snapshot = PlayerSaveToDbSnapshotLikeCpp {
        guid,
        map_id: 571,
        instance_id: 7,
        position: Position::new(11.0, 22.0, 33.0, 1.5),
        level: 70,
        xp: 12_345,
        money: 67_890,
        health: 444,
        max_health: 555,
        powers: loaded_character_power_snapshot_like_cpp([321, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    };
    session.set_player_guid(Some(guid));
    let skill = RepresentedPlayerSkillLikeCpp {
        skill_id: 164,
        step: 1,
        value: 75,
        max: 150,
        profession_slot: -1,
        state: RepresentedPlayerSkillStateLikeCpp::Changed,
    };
    session.set_player_skill_records_like_cpp(HashMap::from([(skill.skill_id, skill)]));
    assert!(session.player_skill_records_loaded_like_cpp());
    assert!(session.complete_player_skill_records_like_cpp().is_none());

    let partial_request = session
        .current_player_character_save_request_like_cpp(&snapshot, 1_000)
        .expect("partial skill authority skips the table instead of blocking other saves");
    assert!(partial_request.skills.is_none());
    assert!(!partial_request.committed_groups_like_cpp().player_skills);

    assert!(
        session.set_complete_player_skill_records_like_cpp(
            HashMap::from([(skill.skill_id, skill)]),
            1,
        )
    );
    let complete_request = session
        .current_player_character_save_request_like_cpp(&snapshot, 1_000)
        .expect("complete skill authority is safe to persist");
    assert!(complete_request.committed_groups_like_cpp().player_skills);
    assert!(
        complete_request
            .skills
            .as_ref()
            .is_some_and(|skills| skills.iter().any(|skill| skill.skill_id == 164))
    );
}
#[test]
fn loaded_xp_is_clamped_below_next_level_threshold_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_next_level_xp_like_cpp(1_000);
    session.set_player_xp_like_cpp(1_000);

    session.clamp_loaded_player_xp_to_next_level_like_cpp();

    assert_eq!(session.player_xp_like_cpp(), 999);

    session.set_player_xp_like_cpp(998);
    session.clamp_loaded_player_xp_to_next_level_like_cpp();
    assert_eq!(session.player_xp_like_cpp(), 998);
}
#[test]
fn load_rested_xp_preserves_saved_bonus_and_state_before_offline_accrual_like_cpp() {
    let (mut capped, _, _) = make_session();
    capped.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    capped.set_player_next_level_xp_like_cpp(1_000);

    capped.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 900.0);

    assert_eq!(capped.represented_xp_rest_bonus_like_cpp(), 900.0);
    assert_eq!(capped.represented_xp_rest_threshold_like_cpp(), 900);
    assert_eq!(
        capped.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );

    let (mut max_level, _, _) = make_session();
    max_level.set_loaded_player_identity_like_cpp(1, 1, 8, 80, 0);
    max_level.set_player_next_level_xp_like_cpp(1_000);

    max_level.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 100.0);

    assert_eq!(max_level.represented_xp_rest_bonus_like_cpp(), 100.0);
    assert_eq!(max_level.represented_xp_rest_threshold_like_cpp(), 100);
    assert_eq!(
        max_level.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );
}
#[test]
fn load_rested_xp_repairs_legacy_rust_zero_state_without_clamping_bonus() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);

    session.load_represented_xp_rest_bonus_like_cpp(0, 123.5);

    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 123.5);
    assert_eq!(session.represented_xp_rest_threshold_like_cpp(), 123);
}
#[test]
fn rest_state_save_player_flags_preserves_current_bits_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1AF);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.ensure_login_player_controller_like_cpp(
        guid,
        "RestFlags".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_AFK_LIKE_CPP);
            player.set_player_flag(PLAYER_FLAGS_DND_LIKE_CPP);
        })
        .expect("canonical player exists");

    assert_eq!(
        session.represented_player_flags_for_rest_state_save_like_cpp(),
        PLAYER_FLAGS_AFK_LIKE_CPP | PLAYER_FLAGS_DND_LIKE_CPP
    );

    session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0);
    assert_eq!(
        session.represented_player_flags_for_rest_state_save_like_cpp(),
        PLAYER_FLAGS_AFK_LIKE_CPP | PLAYER_FLAGS_DND_LIKE_CPP | PLAYER_FLAGS_RESTING_LIKE_CPP
    );

    session.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP);
    assert_eq!(
        session.represented_player_flags_for_rest_state_save_like_cpp(),
        PLAYER_FLAGS_AFK_LIKE_CPP | PLAYER_FLAGS_DND_LIKE_CPP
    );
}
#[test]
fn rest_state_save_player_flags_preserves_loaded_db_bits_without_canonical_like_cpp() {
    let (mut session, _, _) = make_session();
    session
        .set_loaded_player_flags_like_cpp(PLAYER_FLAGS_GHOST_LIKE_CPP | PLAYER_FLAGS_AFK_LIKE_CPP);

    assert_eq!(
        session.represented_player_flags_for_rest_state_save_like_cpp(),
        PLAYER_FLAGS_GHOST_LIKE_CPP | PLAYER_FLAGS_AFK_LIKE_CPP
    );

    session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0);
    assert_eq!(
        session.represented_player_flags_for_rest_state_save_like_cpp(),
        PLAYER_FLAGS_GHOST_LIKE_CPP | PLAYER_FLAGS_AFK_LIKE_CPP | PLAYER_FLAGS_RESTING_LIKE_CPP
    );
}
#[test]
fn loaded_player_flags_rehydrate_canonical_player_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B2);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session
        .set_loaded_player_flags_like_cpp(PLAYER_FLAGS_GHOST_LIKE_CPP | PLAYER_FLAGS_AFK_LIKE_CPP);
    session.ensure_login_player_controller_like_cpp(
        guid,
        "LoadedFlags".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);

    session.apply_loaded_player_flags_to_canonical_like_cpp();

    assert!(
        session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_GHOST_LIKE_CPP)
            .unwrap_or(false)
    );
    assert!(
        session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_AFK_LIKE_CPP)
            .unwrap_or(false)
    );
}
#[test]
fn load_rest_bonus_preserves_db_resting_flag_until_zone_rebuild_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B4);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_loaded_player_flags_like_cpp(
        PLAYER_FLAGS_AFK_LIKE_CPP | PLAYER_FLAGS_RESTING_LIKE_CPP,
    );
    session.ensure_login_player_controller_like_cpp(
        guid,
        "LoadedResting".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.apply_loaded_player_flags_to_canonical_like_cpp();

    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    assert!(
        session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(false),
        "C++ RestMgr::LoadRestBonus updates RestInfo but does not clear loaded PlayerFlags"
    );
    assert_eq!(
        session.represented_player_flags_for_create_like_cpp().0,
        PLAYER_FLAGS_AFK_LIKE_CPP | PLAYER_FLAGS_RESTING_LIKE_CPP
    );
}
#[test]
fn player_create_flags_use_loaded_and_canonical_bits_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B3);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session
        .set_loaded_player_flags_like_cpp(PLAYER_FLAGS_GHOST_LIKE_CPP | PLAYER_FLAGS_AFK_LIKE_CPP);
    session.set_loaded_player_flags_ex_like_cpp(0x04);

    assert_eq!(
        session.represented_player_flags_for_create_like_cpp(),
        (
            PLAYER_FLAGS_GHOST_LIKE_CPP | PLAYER_FLAGS_AFK_LIKE_CPP,
            0x04
        )
    );

    session.ensure_login_player_controller_like_cpp(
        guid,
        "CreateFlags".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.apply_loaded_player_flags_to_canonical_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_DND_LIKE_CPP);
        })
        .expect("canonical player exists");
    session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0);

    assert_eq!(
        session.represented_player_flags_for_create_like_cpp(),
        (
            PLAYER_FLAGS_GHOST_LIKE_CPP
                | PLAYER_FLAGS_AFK_LIKE_CPP
                | PLAYER_FLAGS_DND_LIKE_CPP
                | PLAYER_FLAGS_RESTING_LIKE_CPP,
            0x04,
        )
    );
}
#[test]
fn player_save_timer_marks_periodic_save_due_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_save_interval_ms_like_cpp(100);

    assert_eq!(session.next_player_save_ms_like_cpp, 100);
    assert!(!session.pending_periodic_player_save_like_cpp);

    session.update_player_save_timer_like_cpp(99);
    assert_eq!(session.next_player_save_ms_like_cpp, 1);
    assert!(!session.pending_periodic_player_save_like_cpp);

    session.update_player_save_timer_like_cpp(1);
    assert_eq!(session.next_player_save_ms_like_cpp, 0);
    assert!(
        session.pending_periodic_player_save_like_cpp,
        "C++ Player::Update calls SaveToDB when m_nextSave expires; Rust marks the async save pending"
    );
}
#[test]
fn load_rest_state_clears_stale_rest_flags_between_characters_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B1);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.ensure_login_player_controller_like_cpp(
        guid,
        "RestSwap".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);

    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, 42));
    assert!(session.represented_is_resting_like_cpp());
    assert_eq!(session.represented_inn_area_trigger_id_like_cpp, 42);
    assert_ne!(session.represented_rest_time_secs_like_cpp, 0);

    // C++ applies the new character row's PlayerFlags to the new Player before
    // RestMgr::LoadRestBonus. Keep that provenance explicit: the runtime rest
    // mask is still stale here, while the persisted flag for the new row is not.
    session.set_loaded_player_flags_like_cpp(0);
    assert!(session.represented_is_resting_like_cpp());
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    assert!(!session.represented_is_resting_like_cpp());
    assert_eq!(session.represented_inn_area_trigger_id_like_cpp, 0);
    assert_eq!(session.represented_rest_time_secs_like_cpp, 0);
    assert!(
        !session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(true)
    );
}
#[test]
fn saved_raf_linked_rest_state_is_preserved_until_bonus_is_normalized_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);

    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RAF_LINKED_LIKE_CPP, 0.0);

    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);

    session.add_represented_xp_rest_bonus_like_cpp(25.0);

    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP,
        "C++ RestMgr::SetRestBonus only keeps REST_STATE_RAF_LINKED when RAF currently applies"
    );
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 25.0);
}
#[test]
fn represented_explored_zones_load_preserves_canonical_snapshot_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE101);

    assert_eq!(
        session.load_represented_explored_zones_like_cpp("1 2 5 6"),
        2
    );
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Explorer".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );

    let player = session.initial_player_fixture_like_cpp().unwrap();
    assert_eq!(
        player.explored_zones_block_like_cpp(0),
        Some(0x0000_0002_0000_0001)
    );
    assert_eq!(
        player.explored_zones_block_like_cpp(1),
        Some(0x0000_0006_0000_0005)
    );
    assert_eq!(
        session
            .represented_explored_zones_db_string_like_cpp()
            .expect("test Player explored-zones owner resolves")
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>(),
        vec!["1", "2", "5", "6"]
    );
}
#[test]
fn represented_resurrection_health_syncs_canonical_before_save_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 0xE114);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Resurrected".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        1,
        80,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    let _ = session.sync_canonical_player_health_like_cpp(0, 120);

    session.apply_represented_resurrection_health_like_cpp(42);
    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("save snapshot");

    assert_eq!(snapshot.health, 42);
    assert_eq!(snapshot.max_health, 120);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((42, 120))
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Alive)
    );
}
#[test]
fn character_talent_load_filters_invalid_rows_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([
        test_talent_entry_like_cpp(101, 2, 50_101),
        test_talent_entry_like_cpp(102, 0, 0),
        test_talent_entry_like_cpp(103, 0, 50_103),
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(50_101, test_spell_info_like_cpp(50_101));
    session.set_spell_store(Arc::new(spell_store));
    let talent_tabs = install_test_talent_tab_store_like_cpp(&mut session);

    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 2, 0));
    assert!(!session.load_represented_talent_row_like_cpp(&talent_tabs, 999, 0, 0));
    assert!(!session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 9, 0));
    assert!(!session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 2, 4));
    assert!(!session.load_represented_talent_row_like_cpp(&talent_tabs, 102, 0, 0));
    assert!(!session.load_represented_talent_row_like_cpp(&talent_tabs, 103, 0, 0));

    let packet = session.represented_update_talent_data_packet_like_cpp();
    assert_eq!(
        packet.groups[0].talents,
        vec![wow_packet::packets::misc::TalentInfoLikeCpp {
            talent_id: 101,
            rank: 2,
        }]
    );
}
