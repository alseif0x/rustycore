pub use crate::player::quest_persistence_test_fixture::{
    PlayerQuestLoadStageFixtureLikeCpp, PlayerQuestPersistencePortFixtureLikeCpp,
};

pub fn player_quest_persistence_port_with_load_rows_for_test(
    active: Vec<wow_persistence::PlayerQuestActivePersistenceRowLikeCpp>,
    objectives: Vec<wow_persistence::PlayerQuestObjectivePersistenceRowLikeCpp>,
    rewarded: Vec<wow_persistence::PlayerQuestIdPersistenceRowLikeCpp>,
    daily: Vec<wow_persistence::PlayerQuestDailyPersistenceRowLikeCpp>,
    weekly: Vec<wow_persistence::PlayerQuestIdPersistenceRowLikeCpp>,
    monthly: Vec<wow_persistence::PlayerQuestIdPersistenceRowLikeCpp>,
) -> std::sync::Arc<PlayerQuestPersistencePortFixtureLikeCpp> {
    std::sync::Arc::new(PlayerQuestPersistencePortFixtureLikeCpp {
        active,
        objectives,
        rewarded,
        daily,
        weekly,
        monthly,
        ..PlayerQuestPersistencePortFixtureLikeCpp::default()
    })
}

pub fn player_quest_load_stages_for_test(
    fixture: &PlayerQuestPersistencePortFixtureLikeCpp,
) -> Vec<PlayerQuestLoadStageFixtureLikeCpp> {
    fixture.stages.lock().unwrap().clone()
}

pub fn player_quest_status_requests_for_test(
    fixture: &PlayerQuestPersistencePortFixtureLikeCpp,
) -> Vec<wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp> {
    fixture.status_requests.lock().unwrap().clone()
}

pub fn set_quest_store_for_test(
    session: &mut crate::session::WorldSession,
    store: std::sync::Arc<wow_data::quest::QuestStore>,
) {
    session.quests.store = Some(store);
}

pub fn represented_quest_statuses_for_save_for_test(
    session: &crate::session::WorldSession,
) -> Vec<(u32, u8)> {
    session.represented_quest_statuses_for_save_like_cpp()
}

pub fn represented_quest_status_persistence_for_test(
    session: &crate::session::WorldSession,
    status: &crate::handlers::quest::PlayerQuestStatus,
) -> wow_persistence::QuestStatusPersistenceLikeCpp {
    session.represented_quest_status_persistence_like_cpp(status)
}

pub async fn save_quest_to_db_for_test(
    session: &crate::session::WorldSession,
    quest_id: u32,
    status: u8,
) {
    crate::handlers::quest::save_quest_to_db_for_test(session, quest_id, status).await;
}

pub async fn load_player_quests_for_test(session: &mut crate::session::WorldSession) {
    session.load_player_quests().await;
}

pub fn remove_represented_active_rewarded_duplicates_for_test(
    session: &mut crate::session::WorldSession,
) -> Vec<u32> {
    session.remove_represented_active_rewarded_duplicates_like_cpp()
}
