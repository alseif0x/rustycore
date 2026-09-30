//! Complete represented final quest-sharing decision, separate from CanTakeQuest.
//!
//! TrinityCore a5f8da2e, QuestHandler.cpp:724–743 and Player.cpp:15342–15385.
//! Retain Rust's partial sharing policy: rewarded candidates always reject,
//! stored NONE allows, and exclusive peers omit weekly/seasonal checks.

use super::PlayerQuestGameplayState;
use wow_constants::quest::QUEST_STATUS_NONE_LIKE_CPP;
use wow_data_model::quest::QuestEligibilityRules;

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    pub fn sharing_acceptance_after_expansion<'a, I>(
        quest: &QuestEligibilityRules<'_>,
        peers: impl FnOnce() -> I,
        mut status: impl FnMut(u32) -> Option<u8>,
        mut rewarded: impl FnMut(u32) -> bool,
        mut dungeon_finder: impl FnMut(u32) -> bool,
        mut daily: impl FnMut(u32) -> bool,
    ) -> bool
    where
        I: IntoIterator<Item = QuestEligibilityRules<'a>>,
    {
        let receiver_status = status(quest.id()).unwrap_or(QUEST_STATUS_NONE_LIKE_CPP);
        if rewarded(quest.id()) || receiver_status != QUEST_STATUS_NONE_LIKE_CPP {
            return false;
        }

        if quest.exclusive_group() <= 0 {
            return true;
        }

        for peer in peers()
            .into_iter()
            .filter(|candidate| candidate.exclusive_group() == quest.exclusive_group())
        {
            if peer.id() == quest.id() {
                continue;
            }

            if peer.is_dungeon_finder() && dungeon_finder(peer.id()) {
                return false;
            }
            if peer.is_daily() && daily(peer.id()) {
                return false;
            }

            if status(peer.id()).unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
                != QUEST_STATUS_NONE_LIKE_CPP
            {
                return false;
            }

            if !(quest.is_repeatable() && peer.is_repeatable()) && rewarded(peer.id()) {
                return false;
            }
        }

        true
    }
}
