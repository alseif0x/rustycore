//! Catalog-to-entity rule composition; no Session fixture or clock needed.

use super::*;
use crate::quest::QUEST_FLAGS_DAILY_LIKE_CPP;
use wow_constants::quest::{
    QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP, QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP,
    QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};
use wow_entities::{
    PlayerQuestStatusRecord, represented_can_complete_quest_after_objective_like_cpp,
};

#[test]
fn catalog_objective_definition_and_rule_view_keep_the_same_contract() {
    let mut quest = quest_with_id(901);
    quest.objectives.push(QuestObjective {
        id: 9001,
        quest_id: quest.id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 25,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    // The compatibility path names the entity type itself, not a converted copy.
    let objective: &wow_entities::QuestObjective = &quest.objectives[0];
    assert_eq!(objective.condition_progress_limit_like_cpp(), 2);
    let mut status = PlayerQuestStatusRecord {
        quest_id: quest.id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![1],
        slot: 0,
    };
    let complete = |quest: &QuestTemplate, status: &PlayerQuestStatusRecord, rewarded| {
        represented_can_complete_quest_after_objective_like_cpp(
            status,
            &quest.objective_rules_like_cpp(),
            0,
            rewarded,
        )
    };
    assert!(!complete(&quest, &status, false));
    status.objective_counts[0] = 2;
    assert!(complete(&quest, &status, false));
    assert!(!complete(&quest, &status, true));
    quest.special_flags |= QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP;
    assert!(complete(&quest, &status, true));
    quest.flags |= QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP;
    assert!(!complete(&quest, &status, false));
    status.explored = true;
    assert!(complete(&quest, &status, false));
    quest.limit_time_secs = 10;
    assert!(!complete(&quest, &status, false));
    status.end_time_secs = 10;
    assert!(complete(&quest, &status, false));
    quest.id = 0;
    assert!(!complete(&quest, &status, false));
}

#[test]
fn quest_currency_source_classification_stays_with_catalog_model() {
    let mut quest = quest_with_id(902);
    assert_eq!(
        quest.currency_gain_source_like_cpp(),
        wow_constants::currency::CurrencyGainSourceLikeCpp::QuestReward
    );
    quest.flags |= QUEST_FLAGS_DAILY_LIKE_CPP;
    assert_eq!(
        quest.currency_gain_source_like_cpp(),
        wow_constants::currency::CurrencyGainSourceLikeCpp::DailyQuestReward
    );
    // C++ Player.cpp:14696-14705 checks IsDaily() before IsWorldQuest(), so a quest carrying
    // both flags classifies as a daily reward; clear the daily flag to exercise the world quest arm.
    quest.flags &= !QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.flags_ex = QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP;
    assert_eq!(
        quest.currency_gain_source_like_cpp(),
        wow_constants::currency::CurrencyGainSourceLikeCpp::WorldQuestReward
    );
    quest.flags_ex |= QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP;
    assert_eq!(
        quest.currency_gain_source_like_cpp(),
        wow_constants::currency::CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps
    );
}
