//! Gameobject scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_status_query_canonical_gameobject_starter_uses_go_relation_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1003]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9103, 1003));
    session.set_quest_store(Arc::new(store));
    let guid = gameobject_guid(9103, 3);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9103);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::TRIVIAL));
}
#[tokio::test]
async fn quest_giver_status_query_canonical_gameobject_completed_ender_uses_go_relation_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1004]);
    assert!(store.insert_gameobject_ender_relation_like_cpp(9104, 1004));
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        1004,
        PlayerQuestStatus {
            quest_id: 1004,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = gameobject_guid(9104, 4);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9104);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::CAN_REWARD)
    );
}
#[tokio::test]
async fn quest_giver_status_multiple_visible_gameobject_starter_uses_go_relation_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[2003]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9203, 2003));
    store.starter_quests.entry(9203).or_default().push(2999);
    session.set_quest_store(Arc::new(store));
    let guid = gameobject_guid(9203, 203);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9203);
    attach_map_manager(&mut session, manager);
    mark_visible_gameobject_questgiver(&mut session, guid);

    session.handle_quest_giver_status_multiple_query().await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}
