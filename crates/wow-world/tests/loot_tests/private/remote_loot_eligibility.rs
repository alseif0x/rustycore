//! Original remote loot-condition catalog integration cases.
use super::support::*;
use std::collections::HashMap;
use wow_data::quest::{QUEST_REWARD_REPUTATIONS_COUNT, QuestTemplate};
use wow_loot::LootConditionRowLikeCpp;
use wow_world::test_fixtures::loot::{
    LootConditionPlayer, evaluate_remote_loot_condition_for_test,
};

fn loot_condition(
    condition_type_or_reference: i32,
    value1: u32,
    value2: u32,
    value3: u32,
) -> LootConditionRowLikeCpp {
    LootConditionRowLikeCpp {
        else_group: 0,
        condition_type_or_reference,
        condition_target: 0,
        value1,
        value2,
        value3,
        string_value1: String::new(),
        negative: false,
        script_name: String::new(),
    }
}

fn test_quest_template(id: u32) -> QuestTemplate {
    wow_world::test_fixtures::quest_template_row_for_test(id, 0, String::new())
}

#[test]
fn represented_personal_loot_remote_context_uses_registry_fields_like_cpp() {
    let (session, _) = make_session_with_send_capacity(1);
    let remote_context = LootConditionPlayer::remote(1, 1, 0, 80);

    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(6, 469, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(15, 1, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(16, 1, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(20, 0, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(27, 70, 3, 0),
            &remote_context,
        ),
        Some(true)
    );
}

#[test]
fn represented_personal_loot_remote_inventory_and_objective_conditions_use_registry_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(100);
    quest.objectives.push(QuestObjective {
        id: 11,
        quest_id: 100,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 7001,
        amount: 7,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest_store.quests.insert(100, quest);
    session.set_quest_store(Arc::new(quest_store));
    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut active_quest_objective_counts = HashMap::new();
    active_quest_objective_counts.insert(100, vec![5]);
    let mut inventory_item_counts = HashMap::new();
    inventory_item_counts.insert(9001, 2);
    let remote_context = LootConditionPlayer::remote(1, 1, 0, 80).with_inventory_and_progress(
        active_quest_statuses,
        active_quest_objective_counts,
        inventory_item_counts,
    );

    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(2, 9001, 2, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(2, 9001, 3, 0),
            &remote_context,
        ),
        Some(false)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(2, 9001, 2, 1),
            &remote_context,
        ),
        None
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(48, 11, 0, 5),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(
            &session,
            &loot_condition(48, 11, 0, 4),
            &remote_context,
        ),
        Some(false)
    );
}
