//! Original loot quest application cases use the existing Quest fixtures.
use super::quest_support::*;
use super::support::*;
use wow_loot::LootConditionRowLikeCpp;

fn loot_condition(condition_type_or_reference: i32, value1: u32, value2: u32, value3: u32) -> LootConditionRowLikeCpp {
    LootConditionRowLikeCpp { else_group: 0, condition_type_or_reference, condition_target: 0, value1, value2, value3, string_value1: String::new(), negative: false, script_name: String::new() }
}

#[test]
fn represented_personal_loot_remote_has_quest_for_item_drop_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    install_limited_test_item_template(&mut session, 7002, 0);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(200);
    quest.item_drop[0] = 7002;
    quest.item_drop_quantity[0] = 4;
    quest_store.quests.insert(200, quest);
    set_quest_store_for_test(&mut session, Arc::new(quest_store));

    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(200, wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut inventory_item_counts = HashMap::new();
    inventory_item_counts.insert(7002, 3);
    let mut remote_context = LootConditionPlayer::remote(1, 1, 0, 80)
        .with_known_spells(Vec::new())
        .with_inventory_and_progress(active_quest_statuses, HashMap::new(), inventory_item_counts)
        .with_rewarded_quests(HashSet::new());

    assert!(loot_item_quest_allowed_for_test(&session, 7002, true, 0, 0, Some(&remote_context)));

    remote_context.set_item_count(7002, 4);
    assert!(!loot_item_quest_allowed_for_test(&session, 7002, true, 0, 0, Some(&remote_context)));
}

#[test]
fn represented_personal_loot_remote_has_quest_for_item_objective_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    install_limited_test_item_template(&mut session, 7001, 0);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(100);
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id: 100,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 7001,
        amount: 3,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest_store.quests.insert(100, quest);
    set_quest_store_for_test(&mut session, Arc::new(quest_store));

    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut active_quest_objective_counts = HashMap::new();
    active_quest_objective_counts.insert(100, vec![2]);
    let mut remote_context = LootConditionPlayer::remote(1, 1, 0, 80)
        .with_known_spells(Vec::new())
        .with_inventory_and_progress(active_quest_statuses, active_quest_objective_counts, HashMap::new())
        .with_rewarded_quests(HashSet::new());

    assert!(loot_item_quest_allowed_for_test(&session, 7001, true, 0, 0, Some(&remote_context)));

    remote_context.set_objective_counts(100, vec![3]);
    assert!(!loot_item_quest_allowed_for_test(&session, 7001, true, 0, 0, Some(&remote_context)));
}

#[test]
fn represented_personal_loot_remote_quest_and_spell_conditions_use_registry_like_cpp() {
    let (session, _) = make_session_with_send_capacity(1);
    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    active_quest_statuses.insert(200, wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP);
    let mut rewarded_quests = HashSet::new();
    rewarded_quests.insert(300);
    let remote_context = LootConditionPlayer::remote(1, 1, 0, 80)
        .with_known_spells(vec![12_345])
        .with_inventory_and_progress(active_quest_statuses, HashMap::new(), HashMap::new())
        .with_rewarded_quests(rewarded_quests);

    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(9, 100, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(28, 200, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(8, 300, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(14, 400, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        remote_context.quest_status(300),
        QUEST_STATUS_REWARDED_LIKE_CPP
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(14, 300, 0, 0),
            &remote_context,
        ),
        Some(false),
        "C++ Player::GetQuestStatus returns REWARDED before QUEST_STATUS_NONE"
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(25, 12_345, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_remote_loot_condition_for_test(&session, 
            &loot_condition(47, 100, 0x08, 0),
            &remote_context,
        ),
        Some(true)
    );
}

