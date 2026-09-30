//! Catalog record and borrowed rule-view projections.

use super::*;
use crate::quest::QUEST_FLAGS_DAILY_LIKE_CPP;
use wow_constants::quest::{
    QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP, QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP,
};

#[test]
fn quest_template_builds_data_owned_objective_view_like_cpp() {
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
    let objective: &QuestObjective = &quest.objectives[0];
    assert_eq!(objective.condition_progress_limit_like_cpp(), 2);
    let rules = quest.objective_rules_like_cpp();
    assert_eq!(rules.id(), quest.id);
    assert_eq!(rules.flags(), quest.flags);
    assert_eq!(rules.limit_time_secs(), quest.limit_time_secs);
    assert!(!rules.is_repeatable_like_cpp());
    assert_eq!(rules.objectives(), quest.objectives.as_slice());
    quest.special_flags |= QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP;
    assert!(quest.objective_rules_like_cpp().is_repeatable_like_cpp());
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
