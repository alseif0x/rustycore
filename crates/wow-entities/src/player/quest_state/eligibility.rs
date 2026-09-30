//! Player-owned quest acceptance predicates.
//!
//! The application resolves catalog rows and calls these stages in its existing
//! order. This module reads only the supplied immutable quest view and the
//! canonical quest-state snapshot.
//!
//! TrinityCore SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, Player.cpp:
//! Previous (15088), Status (15285), ExclusiveGroup (15342), Day (15387),
//! Week (15403), Seasonal (15412), Month (15439), DependentBreadcrumb (15197).
//! Retained Rust gaps: active-key presence also blocks stored NONE, and exclusive
//! peers check DF and daily independently; C++ calls SatisfyQuestDay there.

use super::PlayerQuestGameplayState;
use wow_constants::quest::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP,
    QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};
use wow_data_model::quest::{QuestDayCooldownBlock, QuestEligibilityRules, QuestStatusBlock};

mod dependent_previous;
mod cooldowns;
mod previous;
mod sharing_acceptance;

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    /// C++ `Player::SatisfyQuestStatus`'s represented rewarded/active checks.
    pub fn quest_status_block(
        &self,
        quest: &QuestEligibilityRules<'_>,
    ) -> Option<QuestStatusBlock> {
        if self.rewarded_quest_ids_like_cpp().contains(&quest.id()) && !quest.is_repeatable() {
            return Some(QuestStatusBlock::AlreadyRewarded);
        }
        if self.statuses_like_cpp().contains_key(&quest.id()) {
            return Some(QuestStatusBlock::AlreadyActive);
        }
        None
    }

    /// Evaluate one peer during the caller's existing exclusive-group store walk.
    /// DF and daily membership remain independent checks, followed by weekly,
    /// seasonal, active-key presence and the repeatability/rewarded rule.
    pub fn exclusive_group_peer_blocks(
        &self,
        quest: &QuestEligibilityRules<'_>,
        peer: &QuestEligibilityRules<'_>,
    ) -> bool {
        if quest.exclusive_group() <= 0 || peer.exclusive_group() != quest.exclusive_group() {
            return false;
        }

        if peer.is_dungeon_finder()
            && self.df_quest_ids_like_cpp().contains(&peer.id())
        {
            return true;
        }
        if peer.is_daily() && self.daily_quest_ids_like_cpp().contains(&peer.id()) {
            return true;
        }
        if peer.is_weekly() && self.weekly_quest_ids_like_cpp().contains(&peer.id()) {
            return true;
        }
        if peer.is_seasonal()
            && !self.seasonal_quests_like_cpp().is_empty()
            && self
                .seasonal_quests_like_cpp()
                .get(&peer.event_id())
                .is_some_and(|bucket| !bucket.is_empty() && bucket.contains_key(&peer.id()))
        {
            return true;
        }

        if self.statuses_like_cpp().contains_key(&peer.id()) {
            return true;
        }
        !(quest.is_repeatable() && peer.is_repeatable())
            && self.rewarded_quest_ids_like_cpp().contains(&peer.id())
    }

    /// C++ `Player::SatisfyQuestPreviousQuest`, including signed-ID handling.
    pub fn previous_quest_requirement_satisfied(&self, previous_quest_id: i32) -> bool {
        Self::previous_quest_requirement_satisfied_from_membership(
            previous_quest_id,
            |id| self.rewarded_quest_ids_like_cpp().contains(&id),
            |id| self.statuses_like_cpp().get(&id).map(|status| status.status),
        )
    }

    /// Whether an ID is present in C++ `m_RewardedQuests`.
    pub fn quest_is_rewarded(&self, quest_id: u32) -> bool {
        self.rewarded_quest_ids_like_cpp().contains(&quest_id)
    }

    /// Whether an ID has the exact active `INCOMPLETE` state required for a
    /// negative previous-quest link.
    pub fn quest_is_incomplete(&self, quest_id: u32) -> bool {
        self.statuses_like_cpp()
            .get(&quest_id)
            .is_some_and(|status| status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    }

    pub fn quest_day_cooldown_block(
        &self,
        quest: &QuestEligibilityRules<'_>,
    ) -> Option<QuestDayCooldownBlock> {
        Self::quest_day_cooldown_block_from_membership(
            quest,
            || self.df_quest_ids_like_cpp().contains(&quest.id()),
            || self.daily_quest_ids_like_cpp().contains(&quest.id()),
        )
    }

    pub fn quest_weekly_cooldown_blocks(
        &self,
        quest: &QuestEligibilityRules<'_>,
    ) -> bool {
        quest.is_weekly() && self.weekly_quest_ids_like_cpp().contains(&quest.id())
    }

    pub fn quest_monthly_cooldown_blocks(
        &self,
        quest: &QuestEligibilityRules<'_>,
    ) -> bool {
        quest.is_monthly() && self.monthly_quest_ids_like_cpp().contains(&quest.id())
    }

    pub fn quest_seasonal_cooldown_blocks(
        &self,
        quest: &QuestEligibilityRules<'_>,
    ) -> bool {
        quest.is_seasonal()
            && !self.seasonal_quests_like_cpp().is_empty()
            && self
                .seasonal_quests_like_cpp()
                .get(&quest.event_id())
                .is_some_and(|bucket| !bucket.is_empty() && bucket.contains_key(&quest.id()))
    }

    /// Evaluate the shared breadcrumb membership rule against a caller-owned
    /// status lookup. The closure only reads the caller's existing snapshot.
    pub fn dependent_breadcrumb_quest_ids_block(
        dependent_breadcrumb_quest_ids: &[u32],
        mut status_by_quest_id: impl FnMut(u32) -> Option<u8>,
    ) -> bool {
        dependent_breadcrumb_quest_ids.iter().any(|quest_id| {
            matches!(
                status_by_quest_id(*quest_id),
                Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                    | Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
                    | Some(QUEST_STATUS_FAILED_LIKE_CPP)
            )
        })
    }

    pub fn dependent_breadcrumb_quests_block(
        &self,
        dependent_breadcrumb_quest_ids: &[u32],
    ) -> bool {
        Self::dependent_breadcrumb_quest_ids_block(
            dependent_breadcrumb_quest_ids,
            |quest_id| {
                self.statuses_like_cpp()
                    .get(&quest_id)
                    .map(|status| status.status)
            },
        )
    }
}
