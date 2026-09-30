//! Quest persistence planning for item transfers.

use super::*;
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP as QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP as QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
};

#[test]
fn aggregate_item_removal_plan_persists_all_objectives_in_one_status() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_400;
    let first_item_id = 19_900;
    let second_item_id = 19_901;
    let mut quest = quest_template(quest_id);
    quest.objectives = [first_item_id, second_item_id]
        .into_iter()
        .enumerate()
        .map(|(index, item_id)| QuestObjective {
            id: quest_id * 10 + index as u32,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: index as u8,
            storage_index: index as i8,
            object_id: item_id,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    with_player_quest_status_mut_for_test(&mut session, quest_id, |status| {
        status.objective_counts = vec![1, 1];
    })
    .expect("active quest");

    let removed_entries = [first_item_id as u32, second_item_id as u32];
    let planned = plan_item_transfer_quest_persistence_for_test(
        &session,
        &removed_entries,
        &[(first_item_id as u32, 0), (second_item_id as u32, 0)],
        &[],
    );

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].quest_id, quest_id);
    assert_eq!(planned[0].status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![0, 0]);
}

#[test]
fn mixed_item_transfer_quest_plan_applies_withdrawal_after_deposit() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_403;
    let item_id = 19_903;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    with_player_quest_status_mut_for_test(&mut session, quest_id, |status| {
        status.objective_counts = vec![1];
    })
    .expect("active quest");

    let planned = plan_item_transfer_quest_persistence_for_test(
        &session,
        &[item_id as u32],
        &[(item_id as u32, 0)],
        &[(item_id as u32, 0, 1)],
    );

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![1]);
}

#[test]
fn quest_bound_withdrawal_plan_consumes_credit_without_physical_item() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_404;
    let item_id = 19_904;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 1,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot(&mut session, quest_id, 0);
    let (credited, planned) =
        plan_item_transfer_withdrawal_for_test(&session, &[], &[], item_id as u32, 0, 1);
    assert!(credited);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![1]);
}
