use crate::session::WorldSession;
use wow_core::{ObjectGuid, ObjectGuidGenerator};

pub async fn credit_quest_value_for_test(
    session: &mut WorldSession, generator: &ObjectGuidGenerator,
    objective_type: u8, object_id: i32, add: i32, victim: ObjectGuid,
) {
    session.update_represented_storing_value_quest_objective_progress_like_cpp(
        generator, objective_type, object_id, add, victim,
    ).await;
}

pub async fn credit_quest_flag_for_test(
    session: &mut WorldSession, generator: &ObjectGuidGenerator,
    objective_type: u8, object_id: i32, add: i32,
) {
    session.update_represented_storing_flag_quest_objective_progress_like_cpp(
        generator, objective_type, object_id, add,
    ).await;
}

pub async fn update_quest_money_objectives_for_test(
    session: &mut WorldSession, generator: &ObjectGuidGenerator, old: u64, new: u64,
) {
    session.update_represented_money_quest_objective_progress_like_cpp(generator, old, new).await;
}

pub async fn update_quest_currency_objectives_for_test(
    session: &mut WorldSession, generator: &ObjectGuidGenerator, currency: u32, change: i32,
) {
    session.update_represented_currency_quest_objective_progress_like_cpp(generator, currency, change).await;
}

pub async fn update_quest_reputation_objectives_for_test(
    session: &mut WorldSession, generator: &ObjectGuidGenerator,
    objective_type: u8, faction: u32, change: i32,
) {
    session.update_represented_reputation_quest_objective_progress_like_cpp(
        generator, objective_type, faction, change,
    ).await;
}

pub fn quest_compatibility_statuses_for_test(
    session: &WorldSession,
) -> std::collections::HashMap<u32, wow_entities::PlayerQuestStatusRecord> {
    session.quest_test_fixture_like_cpp.player_quests.clone()
}
