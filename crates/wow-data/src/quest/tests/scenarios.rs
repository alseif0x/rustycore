//! Quest store regressions.
//!
//! Moved out of quest.rs under #683; every test is unchanged.

use super::*;

#[test]
fn decoded_catalog_rows_apply_flags_objectives_and_relations_like_cpp() {
    let quest = quest_with_id(42);
    let objective = QuestObjective {
        id: 7,
        quest_id: 42,
        obj_type: QUEST_OBJECTIVE_AREATRIGGER_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: 99,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    };
    let mut store = QuestStore::from_template_rows_like_cpp(vec![quest]);
    store.apply_special_flag_rows_like_cpp(vec![(42, QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP)]);
    store.apply_seasonal_relation_rows_like_cpp(vec![(42, 9)]);
    store.apply_objective_rows_like_cpp(vec![objective]);
    store.apply_creature_starter_rows_like_cpp(vec![(100, 42), (100, 404)]);
    store.apply_creature_ender_rows_like_cpp(vec![(101, 42)]);
    store.apply_gameobject_starter_rows_like_cpp(vec![(200, 42)]);
    store.apply_gameobject_ender_rows_like_cpp(vec![(201, 42)]);

    let quest = store.get(42).unwrap();
    assert!(quest.is_monthly_like_cpp());
    assert!(quest.is_repeatable());
    assert_eq!(quest.event_id_for_quest_like_cpp(), 9);
    assert_eq!(quest.objectives.len(), 1);
    assert_eq!(store.starter_quests.get(&100).unwrap(), &[42]);
    assert_eq!(store.ender_quests.get(&101).unwrap(), &[42]);
    assert_eq!(store.gameobject_starter_quests.get(&200).unwrap(), &[42]);
    assert_eq!(store.gameobject_ender_quests.get(&201).unwrap(), &[42]);
}

#[test]
fn quest_dependent_previous_positive_prev_pushes_current_like_cpp() {
    let previous = quest_with_id(100);
    let mut current = quest_with_id(200);
    current.prev_quest_id = 100;

    let store = QuestStore::from_quests_like_cpp([previous, current]);

    assert_eq!(store.get(200).unwrap().dependent_previous_quests, vec![100]);
}

#[test]
fn quest_dependent_previous_negative_prev_does_not_push_like_cpp() {
    let previous = quest_with_id(100);
    let mut current = quest_with_id(200);
    current.prev_quest_id = -100;

    let store = QuestStore::from_quests_like_cpp([previous, current]);

    assert!(store.get(200).unwrap().dependent_previous_quests.is_empty());
}

#[test]
fn quest_next_quest_id_pushes_source_into_target_dependent_previous_like_cpp() {
    let mut source = quest_with_id(100);
    source.next_quest_id = 200;
    let target = quest_with_id(200);

    let store = QuestStore::from_quests_like_cpp([source, target]);

    assert_eq!(store.get(200).unwrap().dependent_previous_quests, vec![100]);
}

#[test]
fn quest_missing_breadcrumb_target_zeroes_breadcrumb_for_quest_id_like_cpp() {
    let mut breadcrumb = quest_with_id(100);
    breadcrumb.breadcrumb_for_quest_id = 999;

    let store = QuestStore::from_quests_like_cpp([breadcrumb]);

    assert_eq!(store.get(100).unwrap().breadcrumb_for_quest_id, 0);
}

#[test]
fn quest_breadcrumb_chain_informs_each_target_of_source_breadcrumb_like_cpp() {
    let mut source = quest_with_id(100);
    source.breadcrumb_for_quest_id = 200;
    let mut middle = quest_with_id(200);
    middle.breadcrumb_for_quest_id = 300;
    let target = quest_with_id(300);

    let store = QuestStore::from_quests_like_cpp([source, middle, target]);

    assert!(
        store
            .get(200)
            .unwrap()
            .dependent_breadcrumb_quests
            .contains(&100)
    );
    assert!(
        store
            .get(300)
            .unwrap()
            .dependent_breadcrumb_quests
            .contains(&100)
    );
}

#[test]
fn quest_breadcrumb_loop_clears_lowest_source_current_link_without_panic_like_cpp() {
    let mut first = quest_with_id(100);
    first.breadcrumb_for_quest_id = 200;
    let mut second = quest_with_id(200);
    second.breadcrumb_for_quest_id = 100;

    let store = QuestStore::from_quests_like_cpp([first, second]);

    assert_eq!(store.get(100).unwrap().breadcrumb_for_quest_id, 0);
}

#[test]
fn quest_missing_prev_and_next_do_not_fabricate_dependent_previous_like_cpp() {
    let mut quest = quest_with_id(100);
    quest.prev_quest_id = 777;
    quest.next_quest_id = 888;

    let store = QuestStore::from_quests_like_cpp([quest]);

    assert!(store.get(100).unwrap().dependent_previous_quests.is_empty());
}

#[test]
fn seasonal_sort_negative_non_repeatable_is_seasonal_like_cpp() {
    assert!(quest_with_sort_and_flags(-QUEST_SORT_SEASONAL_LIKE_CPP, 0, 0).is_seasonal_like_cpp());
}

#[test]
fn seasonal_sort_repeatable_after_object_mgr_normalization_is_not_seasonal_like_cpp() {
    assert!(
        quest_with_sort_and_flags(
            -QUEST_SORT_SEASONAL_LIKE_CPP,
            0,
            QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP
        )
        .is_repeatable()
    );
    assert!(
        !quest_with_sort_and_flags(
            -QUEST_SORT_SEASONAL_LIKE_CPP,
            0,
            QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP
        )
        .is_seasonal_like_cpp()
    );
    assert!(
        !quest_with_sort_and_flags(-QUEST_SORT_SEASONAL_LIKE_CPP, QUEST_FLAGS_DAILY_LIKE_CPP, 0)
            .is_seasonal_like_cpp()
    );
    assert!(
        !quest_with_sort_and_flags(
            -QUEST_SORT_SEASONAL_LIKE_CPP,
            QUEST_FLAGS_WEEKLY_LIKE_CPP,
            0
        )
        .is_seasonal_like_cpp()
    );
    assert!(
        !quest_with_sort_and_flags(
            -QUEST_SORT_SEASONAL_LIKE_CPP,
            0,
            QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP
        )
        .is_seasonal_like_cpp()
    );
}

#[test]
fn seasonal_object_mgr_normalization_masks_special_flags_and_daily_weekly_prefers_weekly_like_cpp()
{
    let disallowed_special_bit = 0x0000_0020;
    let quest = quest_with_sort_and_flags(
        -QUEST_SORT_SEASONAL_LIKE_CPP,
        QUEST_FLAGS_DAILY_LIKE_CPP | QUEST_FLAGS_WEEKLY_LIKE_CPP,
        disallowed_special_bit,
    );

    assert_eq!(quest.flags & QUEST_FLAGS_DAILY_LIKE_CPP, 0);
    assert_ne!(quest.flags & QUEST_FLAGS_WEEKLY_LIKE_CPP, 0);
    assert_eq!(quest.special_flags & disallowed_special_bit, 0);
    assert!(quest.is_repeatable());
    assert!(!quest.is_seasonal_like_cpp());
}

#[test]
fn seasonal_sort_i32_min_is_not_seasonal_and_does_not_panic_like_cpp() {
    assert!(!quest_with_sort_and_flags(i32::MIN, 0, 0).is_seasonal_like_cpp());
}

#[test]
fn non_seasonal_sort_is_not_seasonal_like_cpp() {
    assert!(!quest_with_sort_and_flags(-101, 0, 0).is_seasonal_like_cpp());
    assert!(!quest_with_sort_and_flags(QUEST_SORT_SEASONAL_LIKE_CPP, 0, 0).is_seasonal_like_cpp());
}

#[test]
fn love_is_in_the_air_sort_is_seasonal_like_cpp() {
    assert!(
        quest_with_sort_and_flags(-QUEST_SORT_LOVE_IS_IN_THE_AIR_LIKE_CPP, 0, 0)
            .is_seasonal_like_cpp()
    );
}

#[test]
fn event_id_for_quest_defaults_zero_like_cpp() {
    assert_eq!(
        quest_with_sort_and_flags(-QUEST_SORT_SEASONAL_LIKE_CPP, 0, 0)
            .event_id_for_quest_like_cpp(),
        0
    );
}

#[test]
fn quest_source_item_missing_zeroes_item_id_but_keeps_count_like_cpp() {
    let mut quest = quest_with_id(300);
    quest.source_item_id = 700;
    quest.source_item_count = 4;

    quest.normalize_source_item_spell_like_cpp(|_| false, |_| true);

    assert_eq!(quest.source_item_id, 0);
    // C++ uses an inner if/else-if; after clearing the id, it does not run the outer
    // `sourceItemId == 0 && count > 0` branch in the same iteration.
    assert_eq!(quest.source_item_count, 4);
}

#[test]
fn quest_source_item_existing_with_zero_count_sets_one_like_cpp() {
    let mut quest = quest_with_id(301);
    quest.source_item_id = 701;
    quest.source_item_count = 0;

    quest.normalize_source_item_spell_like_cpp(|item_id| item_id == 701, |_| true);

    assert_eq!(quest.source_item_id, 701);
    assert_eq!(quest.source_item_count, 1);
}

#[test]
fn quest_source_item_zero_with_positive_count_clears_count_like_cpp() {
    let mut quest = quest_with_id(302);
    quest.source_item_id = 0;
    quest.source_item_count = 3;

    quest.normalize_source_item_spell_like_cpp(|_| true, |_| true);

    assert_eq!(quest.source_item_id, 0);
    assert_eq!(quest.source_item_count, 0);
}

#[test]
fn quest_source_spell_invalid_zeroes_spell_like_cpp() {
    let mut quest = quest_with_id(303);
    quest.source_spell_id = 900;

    quest.normalize_source_item_spell_like_cpp(|_| true, |_| false);

    assert_eq!(quest.source_spell_id, 0);
}

#[test]
fn quest_source_spell_valid_is_preserved_like_cpp() {
    let mut quest = quest_with_id(304);
    quest.source_spell_id = 901;

    quest.normalize_source_item_spell_like_cpp(|_| true, |spell_id| spell_id == 901);

    assert_eq!(quest.source_spell_id, 901);
}

#[test]
fn quest_source_fields_can_be_constructed_raw_before_normalization() {
    let mut quest = quest_with_id(305);
    quest.source_item_id = 702;
    quest.source_item_count = 0;
    quest.source_spell_id = 902;

    assert_eq!(quest.source_item_id, 702);
    assert_eq!(quest.source_item_count, 0);
    assert_eq!(quest.source_spell_id, 902);
}

#[test]
fn quest_store_source_item_spell_normalization_uses_caller_predicates_like_cpp() {
    let mut invalid_item = quest_with_id(306);
    invalid_item.source_item_id = 703;
    invalid_item.source_item_count = 5;
    let mut valid_item_zero_count = quest_with_id(307);
    valid_item_zero_count.source_item_id = 704;
    valid_item_zero_count.source_item_count = 0;
    let mut invalid_spell = quest_with_id(308);
    invalid_spell.source_spell_id = 903;

    let mut store =
        QuestStore::from_quests_like_cpp([invalid_item, valid_item_zero_count, invalid_spell]);
    store.normalize_source_item_spell_metadata_like_cpp(|item_id| item_id == 704, |_| false);

    assert_eq!(store.get(306).unwrap().source_item_id, 0);
    assert_eq!(store.get(306).unwrap().source_item_count, 5);
    assert_eq!(store.get(307).unwrap().source_item_id, 704);
    assert_eq!(store.get(307).unwrap().source_item_count, 1);
    assert_eq!(store.get(308).unwrap().source_spell_id, 0);
}

#[test]
fn quest_pool_non_pooled_quest_is_active_like_cpp() {
    let store = QuestStore::from_quests_like_cpp([daily_quest(100)]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(&store, [], []);

    assert!(!pools.is_quest_pooled_like_cpp(100));
    assert!(pools.is_quest_active_like_cpp(100));
}

#[test]
fn quest_pool_pooled_saved_active_quest_is_active_like_cpp() {
    let store = QuestStore::from_quests_like_cpp([daily_quest(101), daily_quest(102)]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
        &store,
        [
            QuestPoolMemberRowLikeCpp {
                quest_id: 101,
                pool_id: 7,
                pool_index: 0,
                num_active: Some(1),
            },
            QuestPoolMemberRowLikeCpp {
                quest_id: 102,
                pool_id: 7,
                pool_index: 1,
                num_active: Some(1),
            },
        ],
        [QuestPoolSavedActiveRowLikeCpp {
            pool_id: 7,
            quest_id: 101,
        }],
    );

    assert!(pools.is_quest_pooled_like_cpp(101));
    assert!(pools.is_quest_active_like_cpp(101));
}

#[test]
fn quest_pool_pooled_not_saved_active_quest_is_inactive_like_cpp() {
    let store = QuestStore::from_quests_like_cpp([daily_quest(103), daily_quest(104)]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
        &store,
        [
            QuestPoolMemberRowLikeCpp {
                quest_id: 103,
                pool_id: 8,
                pool_index: 0,
                num_active: Some(1),
            },
            QuestPoolMemberRowLikeCpp {
                quest_id: 104,
                pool_id: 8,
                pool_index: 1,
                num_active: Some(1),
            },
        ],
        [QuestPoolSavedActiveRowLikeCpp {
            pool_id: 8,
            quest_id: 103,
        }],
    );

    assert!(pools.is_quest_pooled_like_cpp(104));
    assert!(!pools.is_quest_active_like_cpp(104));
}

#[test]
fn quest_pool_saved_first_quest_activates_entire_member_index_like_cpp() {
    let store = QuestStore::from_quests_like_cpp([daily_quest(201), daily_quest(202)]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
        &store,
        [
            QuestPoolMemberRowLikeCpp {
                quest_id: 201,
                pool_id: 12,
                pool_index: 0,
                num_active: Some(1),
            },
            QuestPoolMemberRowLikeCpp {
                quest_id: 202,
                pool_id: 12,
                pool_index: 0,
                num_active: Some(1),
            },
        ],
        [QuestPoolSavedActiveRowLikeCpp {
            pool_id: 12,
            quest_id: 201,
        }],
    );

    assert!(pools.is_quest_pooled_like_cpp(201));
    assert!(pools.is_quest_pooled_like_cpp(202));
    assert!(pools.is_quest_active_like_cpp(201));
    assert!(pools.is_quest_active_like_cpp(202));
}

#[test]
fn quest_pool_saved_non_first_quest_does_not_activate_member_index_like_cpp() {
    let store = QuestStore::from_quests_like_cpp([daily_quest(203), daily_quest(204)]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
        &store,
        [
            QuestPoolMemberRowLikeCpp {
                quest_id: 203,
                pool_id: 13,
                pool_index: 0,
                num_active: Some(1),
            },
            QuestPoolMemberRowLikeCpp {
                quest_id: 204,
                pool_id: 13,
                pool_index: 0,
                num_active: Some(1),
            },
        ],
        [QuestPoolSavedActiveRowLikeCpp {
            pool_id: 13,
            quest_id: 204,
        }],
    );

    assert!(pools.is_quest_pooled_like_cpp(203));
    assert!(pools.is_quest_pooled_like_cpp(204));
    assert!(!pools.is_quest_active_like_cpp(203));
    assert!(!pools.is_quest_active_like_cpp(204));
}

#[test]
fn quest_pool_missing_quest_and_pool_rows_skip_without_panic_like_cpp() {
    let store = QuestStore::from_quests_like_cpp([weekly_quest(105)]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
        &store,
        [
            QuestPoolMemberRowLikeCpp {
                quest_id: 999,
                pool_id: 9,
                pool_index: 0,
                num_active: Some(1),
            },
            QuestPoolMemberRowLikeCpp {
                quest_id: 105,
                pool_id: 10,
                pool_index: 0,
                num_active: None,
            },
        ],
        [QuestPoolSavedActiveRowLikeCpp {
            pool_id: 404,
            quest_id: 105,
        }],
    );

    assert!(!pools.is_quest_pooled_like_cpp(105));
    assert!(pools.is_quest_active_like_cpp(105));
    assert!(pools.is_quest_active_like_cpp(999));
}

#[test]
fn quest_pool_non_daily_weekly_monthly_member_is_skipped_and_active_like_cpp() {
    let mut normal = quest_with_sort_and_flags(0, 0, 0);
    normal.id = 106;
    let store = QuestStore::from_quests_like_cpp([normal]);
    let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
        &store,
        [QuestPoolMemberRowLikeCpp {
            quest_id: 106,
            pool_id: 11,
            pool_index: 0,
            num_active: Some(1),
        }],
        [QuestPoolSavedActiveRowLikeCpp {
            pool_id: 11,
            quest_id: 106,
        }],
    );

    assert!(!pools.is_quest_pooled_like_cpp(106));
    assert!(pools.is_quest_active_like_cpp(106));
}

#[test]
fn gameobject_quest_relations_skip_missing_quests_and_stay_separate_from_creatures_like_cpp() {
    let mut quest_two = quest_with_sort_and_flags(0, 0, 0);
    quest_two.id = 2;
    let mut store =
        QuestStore::from_quests_like_cpp([quest_with_sort_and_flags(0, 0, 0), quest_two]);

    store.starter_quests.entry(10).or_default().push(1);
    store.ender_quests.entry(10).or_default().push(2);

    assert!(store.insert_gameobject_starter_relation_like_cpp(1000, 1));
    assert!(store.insert_gameobject_ender_relation_like_cpp(1000, 2));
    assert!(!store.insert_gameobject_starter_relation_like_cpp(1000, 999));
    assert!(!store.insert_gameobject_ender_relation_like_cpp(1000, 999));

    assert_eq!(
        store
            .quests_for_gameobject_starter(1000)
            .into_iter()
            .map(|quest| quest.id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        store
            .quests_for_gameobject_ender(1000)
            .into_iter()
            .map(|quest| quest.id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(
        store
            .quests_for_starter(10)
            .into_iter()
            .map(|quest| quest.id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        store
            .quests_for_ender(10)
            .into_iter()
            .map(|quest| quest.id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(store.gameobject_has_start_quests(1000));
    assert!(store.gameobject_has_end_quests(1000));
    assert!(!store.gameobject_has_start_quests(2000));
    assert!(!store.gameobject_has_end_quests(2000));
    assert!(!store.npc_has_start_quests(1000));
    assert!(!store.npc_has_end_quests(1000));
}
