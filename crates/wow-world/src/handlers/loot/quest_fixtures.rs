//! Existing quest-owner operations used by the complete loot eligibility fixtures.
use super::*;

pub async fn generate_quest_loot_for_test(session: &mut WorldSession, loot: u32, player: ObjectGuid) -> Option<Vec<LootEntry>> {
    session.generate_represented_creature_loot_items_for_player_like_cpp(loot, player).await
}

pub fn loot_item_quest_allowed_for_test(session: &WorldSession, item: u32, needs_quest: bool, flags_cu: u32, quest_log_item_id: i32, remote: Option<&LootConditionPlayer>) -> bool {
    let metadata = ItemTemplateAddonLootMetadataLikeCpp { flags_cu, quest_log_item_id };
    match remote {
        Some(remote) => remote.quest_item_allowed(session, item, needs_quest, flags_cu, quest_log_item_id),
        None => session.item_loot_quest_status_allows_like_cpp(item, needs_quest, metadata),
    }
}

pub fn incomplete_loot_item_objective_for_test(session: &WorldSession, item: u32) -> bool {
    session.has_incomplete_quest_objective_for_item_like_cpp(item)
}

pub async fn advance_loot_item_objectives_for_test(session: &mut WorldSession, item: u32, quest_log_item: u32, count: u32) -> Vec<u32> {
    let generators = session.id_generators_for_test_like_cpp();
    session.apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(generators.item.as_ref(), item, quest_log_item, count).await
}

pub fn remove_loot_item_objectives_for_test(session: &mut WorldSession, item: u32) -> Option<Vec<u32>> {
    session.apply_quest_item_removed_like_cpp(item)
}
