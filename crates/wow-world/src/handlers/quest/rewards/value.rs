//! Existing scalar bindings to canonical domain valuation.
use super::*;

pub(in crate::handlers::quest) fn reputation_rank_from_standing_like_cpp(standing: i32) -> u8 {
    reputation_rank_from_standing_data_like_cpp(standing).as_u8()
}


pub(in crate::handlers::quest) fn player_quest_level_like_cpp(quest: &wow_data::quest::QuestTemplate, player_level: u8) -> i32 {
    wow_progression::effective_quest_level(&quest.reward_rules(), || player_level)
}
