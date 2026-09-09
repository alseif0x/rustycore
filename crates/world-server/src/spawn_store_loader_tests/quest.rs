//! Quest scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_quest_condition_metadata_load_skips_out_of_range_and_last_row_wins_like_cpp() {
    let events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut quest_conditions = BTreeMap::new();
    let mut report = GameEventQuestConditionLoadReportLikeCpp::default();

    apply_game_event_quest_condition_row_like_cpp(
        GameEventQuestConditionRowLikeCpp {
            quest_id: 7000,
            event_id: 1,
            condition_id: 10,
            num: 1.25,
        },
        &events,
        &mut quest_conditions,
        &mut report,
    );
    apply_game_event_quest_condition_row_like_cpp(
        GameEventQuestConditionRowLikeCpp {
            quest_id: 7000,
            event_id: 2,
            condition_id: 20,
            num: 2.5,
        },
        &events,
        &mut quest_conditions,
        &mut report,
    );
    apply_game_event_quest_condition_row_like_cpp(
        GameEventQuestConditionRowLikeCpp {
            quest_id: 8000,
            event_id: 3,
            condition_id: 30,
            num: 4.0,
        },
        &events,
        &mut quest_conditions,
        &mut report,
    );

    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.overwrites, 1);
    assert_eq!(report.skipped_out_of_range_event, 1);
    assert_eq!(
        quest_conditions.get(&7000),
        Some(&GameEventQuestConditionRecordLikeCpp {
            quest_id: 7000,
            event_id: 2,
            condition_id: 20,
            num: 2.5,
        })
    );
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_event_quest_conditions_like_cpp(quest_conditions.clone());
    assert_eq!(
        metadata.game_event_quest_condition_like_cpp(7000),
        quest_conditions.get(&7000)
    );
    assert!(!quest_conditions.contains_key(&8000));
}
#[test]
fn game_event_quest_complete_missing_mapping_does_not_mutate_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping { quest_id: 7000 }
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}
#[test]
fn game_event_quest_complete_inactive_event_does_not_mutate_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        ),
        7000,
        1,
        10,
        1.0,
    );

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id: 1 }
        )
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}
#[test]
fn game_event_quest_complete_non_world_conditions_does_not_mutate_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event(1, GameEventStateLikeCpp::Normal, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        ),
        7000,
        1,
        10,
        1.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::NotWorldConditions {
                event_id: 1,
                state_raw: GameEventStateLikeCpp::Normal as u8,
            }
        )
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}
#[test]
fn game_event_quest_complete_missing_condition_does_not_mutate_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        ),
        7000,
        1,
        99,
        1.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::MissingCondition {
                event_id: 1,
                condition_id: 99,
            }
        )
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}
#[test]
fn game_event_quest_complete_increments_clamps_and_emits_condition_save_evidence_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event_with_condition(
                event(257, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
                10,
                condition(3.0, 1.0),
            ),
            20,
            condition(4.0, 1.0),
        ),
        7000,
        257,
        10,
        5.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(257);

    let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100);

    assert_eq!(
        outcome,
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::Progressed(
                GameEventConditionProgressSummaryLikeCpp {
                    event_id: 257,
                    condition_id: 10,
                    done_before: 1.0,
                    done_after: 3.0,
                    req_num: 3.0,
                    persistence_event_id: 1,
                    completed_event: false,
                    check_outcome: GameEventConditionCheckOutcomeLikeCpp::NotCompleted {
                        event_id: 257,
                        blocking_condition_id: 20,
                    },
                    save_world_event_state_requested: false,
                    force_game_event_update_requested: false,
                }
            )
        )
    );
}
#[test]
fn game_event_quest_complete_all_conditions_done_requests_save_and_force_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event_with_condition(
                event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
                10,
                condition(3.0, 1.0),
            ),
            20,
            condition(4.0, 4.0),
        ),
        7000,
        1,
        10,
        2.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100);

    assert!(matches!(
        outcome,
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::Progressed(
                GameEventConditionProgressSummaryLikeCpp {
                    completed_event: true,
                    save_world_event_state_requested: true,
                    force_game_event_update_requested: true,
                    check_outcome: GameEventConditionCheckOutcomeLikeCpp::Completed(
                        GameEventConditionCheckSummaryLikeCpp {
                            state_after_raw,
                            next_start_after: 400,
                            ..
                        }
                    ),
                    ..
                }
            )
        ) if state_after_raw == GameEventStateLikeCpp::WorldNextPhase as u8
    ));
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldNextPhase as u8);
    assert_eq!(event.next_start, 400);
}
#[test]
fn game_event_quest_sizing_accessors_and_out_of_range_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(2, 100, 7000),
        &mut quests,
        &mut report,
    );
    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(3, 101, 7001),
        &mut quests,
        &mut report,
    );

    assert_eq!(quests.creature_records_like_cpp(0).unwrap(), &[]);
    assert_eq!(quests.creature_records_like_cpp(1).unwrap(), &[]);
    assert_eq!(
        quests.creature_records_like_cpp(2).unwrap()[0].quest_id,
        7000
    );
    assert_eq!(quests.creature_records_like_cpp(3), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 1);
}
#[test]
fn game_event_quest_valid_event_accepts_high_quest_id_no_template_validation_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(1, 100, u32::MAX),
        &mut quests,
        &mut report,
    );

    let records = quests.creature_records_like_cpp(1).unwrap();
    assert_eq!(records[0].quest_id, u32::MAX);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 0);
}
#[test]
fn game_event_quest_relation_events_touched_counts_non_empty_buckets_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = CanonicalSpawnStoreLoadReport::default();

    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(1, 100, 7000),
        &mut quests,
        &mut report.game_event_quest_relations.creature,
    );
    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(3, 101, 7001),
        &mut quests,
        &mut report.game_event_quest_relations.creature,
    );
    apply_game_event_gameobject_quest_relation_row_like_cpp(
        game_event_quest_row(2, 200, 8000),
        &mut quests,
        &mut report.game_event_quest_relations.gameobject,
    );

    report.game_event_quest_relations.creature.events_touched = quests
        .creature_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();
    report.game_event_quest_relations.gameobject.events_touched = quests
        .gameobject_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    assert_eq!(report.game_event_quest_relations.creature.events_touched, 2);
    assert_eq!(
        report.game_event_quest_relations.gameobject.events_touched,
        1
    );
}
#[test]
fn game_event_quest_canonical_metadata_accessors_expose_both_families_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(quests.push_creature_record_like_cpp(
        1,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: 100,
            quest_id: 7000,
        },
    ));
    assert!(quests.push_gameobject_record_like_cpp(
        1,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: 200,
            quest_id: 8000,
        },
    ));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_quest_relations_like_cpp(quests);

    assert_eq!(
        metadata.game_event_creature_quests_like_cpp(1).unwrap()[0].giver_id,
        100
    );
    assert_eq!(
        metadata.game_event_gameobject_quests_like_cpp(1).unwrap()[0].giver_id,
        200
    );
    assert_eq!(metadata.game_event_creature_quests_like_cpp(2), None);
    assert_eq!(metadata.game_event_gameobject_quests_like_cpp(2), None);
}
#[test]
fn game_event_quest_activation_inserts_active_relations_and_duplicates_like_cpp() {
    let mut metadata = game_event_quest_cache_metadata_like_cpp(
        1,
        &[(1, 100, 7000), (1, 100, 7000)],
        &[(1, 200, 8000), (1, 200, 8001)],
    );

    let summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, true);

    assert_eq!(summary.creature_records_seen, 2);
    assert_eq!(summary.gameobject_records_seen, 2);
    assert_eq!(summary.creature_inserted, 2);
    assert_eq!(summary.gameobject_inserted, 2);
    assert_eq!(
        metadata.game_event_active_creature_quest_relations_like_cpp(100),
        &[
            game_event_quest_relation_record(100, 7000),
            game_event_quest_relation_record(100, 7000),
        ]
    );
    assert_eq!(
        metadata
            .game_event_active_gameobject_quest_relations_like_cpp(200)
            .iter()
            .map(|record| record.quest_id)
            .collect::<Vec<_>>(),
        vec![8000, 8001]
    );
}
#[test]
fn game_event_quest_deactivation_removes_first_matching_relation_like_cpp() {
    let mut metadata =
        game_event_quest_cache_metadata_like_cpp(1, &[(1, 100, 7000)], &[(1, 200, 8000)]);
    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);
    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);

    let summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);

    assert_eq!(summary.creature_removed, 1);
    assert_eq!(summary.gameobject_removed, 1);
    assert_eq!(
        metadata.game_event_active_creature_quest_relations_like_cpp(100),
        &[game_event_quest_relation_record(100, 7000)]
    );
    assert_eq!(
        metadata.game_event_active_gameobject_quest_relations_like_cpp(200),
        &[game_event_quest_relation_record(200, 8000)]
    );
}
#[test]
fn game_event_quest_deactivation_skips_when_other_active_event_has_same_quest_like_cpp() {
    let mut metadata = game_event_quest_cache_metadata_like_cpp(
        2,
        &[(1, 100, 7000), (2, 101, 7000)],
        &[(1, 200, 8000), (2, 201, 8000)],
    );
    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(2);

    let summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);

    assert_eq!(summary.creature_skipped_active_other_event, 1);
    assert_eq!(summary.gameobject_skipped_active_other_event, 1);
    assert_eq!(summary.creature_removed, 0);
    assert_eq!(summary.gameobject_removed, 0);
    assert_eq!(
        metadata.game_event_active_creature_quest_relations_like_cpp(100),
        &[game_event_quest_relation_record(100, 7000)]
    );
    assert_eq!(
        metadata.game_event_active_gameobject_quest_relations_like_cpp(200),
        &[game_event_quest_relation_record(200, 8000)]
    );
}
#[test]
fn game_event_quest_deactivation_remove_miss_and_missing_bucket_are_no_panic_like_cpp() {
    let mut metadata =
        game_event_quest_cache_metadata_like_cpp(1, &[(1, 100, 7000)], &[(1, 200, 8000)]);

    let miss_summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    assert_eq!(miss_summary.creature_remove_misses, 1);
    assert_eq!(miss_summary.gameobject_remove_misses, 1);

    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);
    metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    let no_match_summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    assert_eq!(no_match_summary.creature_remove_misses, 1);
    assert_eq!(no_match_summary.gameobject_remove_misses, 1);

    let mut no_match_metadata = game_event_quest_cache_metadata_like_cpp(
        2,
        &[(1, 100, 7000), (2, 100, 9000)],
        &[(1, 200, 8000), (2, 200, 9000)],
    );
    no_match_metadata.update_game_event_quest_relation_cache_like_cpp(2, true);
    let no_match_summary =
        no_match_metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    assert_eq!(no_match_summary.creature_no_match, 1);
    assert_eq!(no_match_summary.gameobject_no_match, 1);

    let missing_summary = metadata.update_game_event_quest_relation_cache_like_cpp(2, false);
    assert!(missing_summary.creature_missing_event_bucket);
    assert!(missing_summary.gameobject_missing_event_bucket);
}
