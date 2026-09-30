//! Existing quest fixture inputs and adapters for loot's application scenarios.
use super::support::*;
pub(super) use std::collections::{HashMap, HashSet};
pub(super) use wow_loot::{LootStore, LootStoreItem, LootStoreKind, LootStores, LootTemplateRow};
pub(super) use wow_persistence::PersistenceOutcomeLikeCpp;
pub(super) use wow_world::handlers::quest::PlayerQuestStatus;
pub(super) use wow_world::test_fixtures::loot::*;
pub(super) use wow_world::test_fixtures::quest::*;
pub(super) const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 0x01;
pub(super) const QUEST_STATUS_REWARDED_LIKE_CPP: u8 = 6;
pub(super) use basic_quest_template_for_loot_test as test_quest_template;

pub(super) fn install_quest_bound_loot_objective_like_cpp(
    session: &mut WorldSession,
    quest_id: u32,
    item_id: u32,
    current_count: i32,
    required_count: i32,
) {
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: required_count,
        flags: 0,
        flags2: 1,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_quest_store_for_test(session, Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_gameplay_status_for_test(
        session,
        quest_id,
        wow_world::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![current_count],
            slot: 0,
        },
    );
}
