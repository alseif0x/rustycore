//! Call the existing application catalog predicate without duplicating its lookup order.

pub fn quest_dependent_previous_blocks_for_test(
    quest_store: &wow_data::quest::QuestStore,
    quest: &wow_data::quest::QuestTemplate,
    rewarded: &std::collections::HashSet<u32>,
) -> bool {
    crate::handlers::quest::represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
        quest_store, quest, rewarded,
    )
}
