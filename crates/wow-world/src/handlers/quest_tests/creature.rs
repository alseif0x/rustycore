//! Creature scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn quest_giver_creature_id_is_zero_for_gameobject_sources_like_cpp() {
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(creature_guid(15513, 27)),
        15513
    );
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(gameobject_guid(180516, 301)),
        0
    );
}
#[test]
fn query_quest_completion_builds_creature_then_masked_go_entries_like_cpp() {
    let mut store = store_with_quests(&[77]);
    store.ender_quests.entry(1234).or_default().push(77);
    store.ender_quests.entry(12).or_default().push(77);
    store
        .gameobject_ender_quests
        .entry(0x5678)
        .or_default()
        .push(77);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[77]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 77);
    assert_eq!(response[0].npcs, vec![12, 1234, 0x8000_5678u32 as i32]);
}
#[test]
fn query_quest_completion_skips_negative_missing_and_oversized_creature_entries_like_cpp() {
    let mut store = store_with_quests(&[5]);
    store
        .ender_quests
        .entry(i32::MAX as u32 + 1)
        .or_default()
        .push(5);
    store
        .gameobject_ender_quests
        .entry(u32::MAX)
        .or_default()
        .push(5);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[-1, 999, 5]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 5);
    assert_eq!(response[0].npcs, vec![-1]);
}
#[tokio::test]
async fn quest_giver_status_query_canonical_creature_starter_sends_available_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1001]);
    store.starter_quests.entry(9001).or_default().push(1001);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9001, 1);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9001);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::TRIVIAL));
}
#[tokio::test]
async fn quest_giver_status_query_canonical_creature_starter_sends_quest_when_not_trivial_like_cpp()
{
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(1006);
    quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9006).or_default().push(1006);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9006, 6);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9006);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}
#[tokio::test]
async fn quest_giver_status_query_canonical_creature_starter_sends_future_when_visible_but_low_level_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(1007);
    quest.quest_level = 85;
    quest.min_level = 85;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9007).or_default().push(1007);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9007, 7);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9007);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::FUTURE));
}
#[tokio::test]
async fn quest_giver_status_query_canonical_creature_completed_ender_sends_can_reward_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1002]);
    store.ender_quests.entry(9002).or_default().push(1002);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        1002,
        PlayerQuestStatus {
            quest_id: 1002,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = creature_guid(9002, 2);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9002);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::CAN_REWARD)
    );
}
#[tokio::test]
async fn quest_giver_status_query_gameobject_ignores_creature_relation_for_same_entry_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1005]);
    store.starter_quests.entry(9105).or_default().push(1005);
    store.ender_quests.entry(9105).or_default().push(1005);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        1005,
        PlayerQuestStatus {
            quest_id: 1005,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = gameobject_guid(9105, 5);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9105);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}
#[tokio::test]
async fn quest_giver_status_multiple_visible_canonical_creature_starter_sends_available_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[2002]);
    store.starter_quests.entry(9202).or_default().push(2002);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9202, 202);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9202);
    attach_map_manager(&mut session, manager);
    mark_visible(&mut session, guid);

    session.handle_quest_giver_status_multiple_query().await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}
