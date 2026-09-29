pub use crate::player::quest_persistence_test_fixture::{
    PlayerQuestLoadStageFixtureLikeCpp, PlayerQuestPersistencePortFixtureLikeCpp,
    PlayerQuestRewardPersistencePortFixtureLikeCpp,
};
pub use crate::session::quest_dialog::RepresentedQuestCompleteStatusUpdateLikeCpp;

pub fn represented_quest_complete_status_updates_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestCompleteStatusUpdateLikeCpp> {
    session
        .represented_quest_complete_status_updates_like_cpp()
        .to_vec()
}

pub fn represented_timed_quest_removals_for_test(
    session: &crate::session::WorldSession,
) -> Vec<u32> {
    session.represented_timed_quest_removals_like_cpp().to_vec()
}

pub fn represented_quest_reward_skill_updates_for_test(
    session: &crate::session::WorldSession,
) -> Vec<(u32, u32)> {
    session
        .represented_quest_reward_skill_updates_like_cpp()
        .to_vec()
}

pub fn player_quest_reward_persistence_fixture_with_outcome_for_test(
    outcome: wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp,
) -> PlayerQuestRewardPersistencePortFixtureLikeCpp {
    PlayerQuestRewardPersistencePortFixtureLikeCpp::with_outcome(outcome)
}

pub fn player_quest_reward_requests_for_test(
    fixture: &PlayerQuestRewardPersistencePortFixtureLikeCpp,
) -> Vec<wow_persistence::PlayerQuestRewardDurableRequestLikeCpp> {
    fixture.requests.lock().unwrap().clone()
}

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

pub fn first_free_quest_slot_for_test(
    session: &crate::session::WorldSession,
) -> Option<u8> {
    session.first_free_quest_slot_like_cpp()
}

pub fn insert_player_quest_gameplay_status_for_test(
    session: &mut crate::session::WorldSession,
    quest_id: u32,
    status: crate::handlers::quest::PlayerQuestStatus,
) {
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.insert_status_like_cpp(quest_id, status);
        })
        .expect("test Player quest owner");
}

pub fn set_player_quest_gameplay_rewarded_for_test(
    session: &mut crate::session::WorldSession,
    quest_id: u32,
) {
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.set_rewarded_like_cpp(quest_id, true);
        })
        .expect("test Player quest owner");
}

pub fn faction_store_for_test(
    session: &crate::session::WorldSession,
) -> Option<std::sync::Arc<wow_data::progression_rewards::FactionStore>> {
    session.faction_store().cloned()
}

pub fn add_currency_quest_reward_for_test(
    session: &mut crate::session::WorldSession,
    currency_id: u32,
    amount: u32,
    gain_source: wow_constants::currency::CurrencyGainSourceLikeCpp,
) -> Result<Option<()>, ()> {
    session
        .add_currency_quest_reward_like_cpp(currency_id, amount, gain_source)
        .map(|delta| delta.map(|_| ()))
}

pub fn set_player_skill_values_for_test(
    session: &mut crate::session::WorldSession,
    skill_values: std::collections::HashMap<u16, u16>,
) -> bool {
    session.set_player_skill_values_like_cpp(skill_values)
}

pub async fn apply_quest_item_added_objective_progress_for_test(
    session: &mut crate::session::WorldSession,
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> Vec<u32> {
    session
        .apply_quest_item_added_objective_progress_like_cpp(entry_id, quest_log_item_id, count)
        .await
}

pub async fn apply_quest_source_item_bound_objective_progress_for_object_for_test(
    session: &mut crate::session::WorldSession,
    quest_store: &wow_data::quest::QuestStore,
    object_id: i32,
    count_i32: i32,
) -> Vec<(u32, i32)> {
    session
        .apply_quest_source_item_bound_objective_progress_for_object_for_test(
            quest_store,
            object_id,
            count_i32,
        )
        .await
}

pub fn set_represented_df_quest_for_test(
    session: &mut crate::session::WorldSession,
    quest_id: u32,
    present: bool,
) {
    session.set_represented_df_quest_like_cpp_for_test(quest_id, present);
}

pub fn can_complete_repeatable_quest_represented_bounded_for_test(
    session: &crate::session::WorldSession,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    session.can_complete_repeatable_quest_represented_bounded_like_cpp(quest)
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

pub fn represented_inventory_item_counts_for_test(
    session: &crate::session::WorldSession,
) -> Option<std::collections::HashMap<u32, u32>> {
    session.represented_inventory_item_counts_like_cpp()
}

pub fn player_currency_quantity_for_test(
    session: &crate::session::WorldSession,
    currency_id: u32,
) -> Option<u32> {
    session.player_currency_quantity(currency_id)
}

pub fn represented_can_delay_teleport_for_test(
    session: &crate::session::WorldSession,
) -> bool {
    session.represented_can_delay_teleport_like_cpp()
}

pub fn plan_item_transfer_quest_persistence_for_test(
    session: &crate::session::WorldSession,
    removed_entries_in_order: &[u32],
    post_removal_non_bank_counts: &[(u32, u32)],
    added_items_in_order: &[(u32, u32, u32)],
) -> Vec<crate::handlers::quest::PlayerQuestStatus> {
    session.plan_item_transfer_quest_persistence_like_cpp(
        removed_entries_in_order,
        post_removal_non_bank_counts,
        added_items_in_order,
    )
}

pub fn plan_item_transfer_withdrawal_for_test(
    session: &crate::session::WorldSession,
    removed_entries_in_order: &[u32],
    post_removal_non_bank_counts: &[(u32, u32)],
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> (bool, Vec<crate::handlers::quest::PlayerQuestStatus>) {
    let mut plan = session.begin_item_transfer_quest_persistence_like_cpp(
        removed_entries_in_order,
        post_removal_non_bank_counts,
    );
    let credited = session.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
        &mut plan,
        entry_id,
        quest_log_item_id,
        count,
    );
    let statuses = session.finish_item_transfer_quest_persistence_like_cpp(plan);
    (credited, statuses)
}

pub fn plan_bank_item_quest_persistence_for_test(
    session: &crate::session::WorldSession,
    entry_id: u32,
    quest_log_item_id: u32,
    moving_to_bank: bool,
    post_move_non_bank_count: u32,
    added_count: u32,
) -> Vec<crate::handlers::quest::PlayerQuestStatus> {
    session.plan_bank_item_quest_persistence_like_cpp(
        entry_id,
        quest_log_item_id,
        moving_to_bank,
        post_move_non_bank_count,
        added_count,
    )
}

pub fn plan_quest_source_item_bound_objective_statuses_for_test(
    session: &crate::session::WorldSession,
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> Option<Vec<crate::handlers::quest::PlayerQuestStatus>> {
    session
        .plan_quest_source_item_bound_objective_persistence_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
        )
        .map(|plan| plan.statuses)
}
