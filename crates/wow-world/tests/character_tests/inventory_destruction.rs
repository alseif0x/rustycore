use super::*;
use super::inventory_children::make_session_with_send_capacity;

#[test]
fn recursive_destroy_plans_child_and_parent_quest_removal_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let quest_id = 91_001;
    let child_entry = 700;
    let parent_entry = 600;
    let mut quest = super::quest_template(quest_id);
    quest.objectives = [child_entry, parent_entry]
        .into_iter()
        .enumerate()
        .map(|(index, entry_id)| wow_entities::QuestObjective {
            id: quest_id * 10 + index as u32,
            quest_id,
            obj_type: 1,
            order: index as u8,
            storage_index: index as i8,
            object_id: entry_id as i32,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp([
        quest,
    ])));
    insert_player_quest_gameplay_status_for_test(&mut session, 
        quest_id,
        wow_world::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![1, 1],
            slot: 0,
        },
    );

    let planned = inventory_destroy_quest_plan_for_test(&session, &[
            (wow_entities::INVENTORY_SLOT_BAG_START, 0, child_entry, 1),
            (wow_entities::INVENTORY_SLOT_BAG_0, wow_entities::INVENTORY_SLOT_BAG_START, parent_entry, 1),
        ])
        .expect("fixture canonical inventory owner");

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![0, 0]);
    assert_eq!(
        planned[0].status,
        wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
}
