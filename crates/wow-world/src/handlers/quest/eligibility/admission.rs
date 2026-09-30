//! admission operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    /// Check if the player currently has an active quest with the given ID.
    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| state.statuses_like_cpp().contains_key(&quest_id))
    }

    /// Full eligibility check before accepting a quest.
    /// C++ ref: Player::CanTakeQuest (Player.cpp:14087-14096).
    /// Preserve the represented Rust gate order and its timed/breadcrumb gaps.
    pub fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if self.is_quest_disabled_like_cpp(quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: quest disabled"
            );
            return false;
        }
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let rules = quest.eligibility_rules();

        // SatisfyQuestStatus — Player.cpp:15285-15309
        // If quest is already rewarded (non-repeatable), cannot take again.
        match recurrence.quest_status_block(&rules) {
            Some(QuestStatusBlock::AlreadyRewarded) => {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: already rewarded"
                );
                return false;
            }
            Some(QuestStatusBlock::AlreadyActive) => {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: already active"
                );
                return false;
            }
            None => {}
        }

        // SatisfyQuestExclusiveGroup — Player.cpp:15342-15385 (CanTakeQuest:14087).
        // Inserted here to match C++ CanTakeQuest evaluation order: status → exclusive group.
        if !self.satisfy_quest_exclusive_group_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: exclusive group blocked"
            );
            return false;
        }

        // SatisfyQuestRace + SatisfyQuestClass + SatisfyQuestLevel
        if !quest.is_available_for(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            self.player_level_like_cpp(),
        ) {
            return false;
        }

        // SatisfyQuestSkill — Player.cpp:15009-15037.
        if !self.satisfy_quest_skill_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: skill requirement not met"
            );
            return false;
        }

        // SatisfyQuestReputation — Player.cpp:15256-15283.
        if !self.satisfy_quest_reputation_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: reputation requirement not met"
            );
            return false;
        }

        // SatisfyQuestPreviousQuest — Player.cpp:15088-15112
        // prev_quest_id > 0 → previous quest must have been rewarded
        // prev_quest_id < 0 → previous quest must be currently active (Incomplete)
        if !recurrence.previous_quest_requirement_satisfied(rules.previous_quest_id()) {
            let prev_id = rules.previous_quest_id().unsigned_abs();
            if rules.previous_quest_id() > 0 {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    prev_id,
                    "CanTakeQuest: prev quest not rewarded"
                );
            } else {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    prev_id,
                    "CanTakeQuest: negative prev quest not active"
                );
            }
            return false;
        }

        // SatisfyQuestDependentPreviousQuests — Player.cpp:15115-15171.
        // Blocks acceptance if the scalar dependent-previous list is not satisfied.
        // Per C++ CanTakeQuest (Player.cpp:14087), this cluster runs
        // after SatisfyQuestReputation, not before Race/Class/Level.
        if let Some(quest_store) = &self.quests.store {
            if represented_satisfy_quest_dependent_previous_quests_failed_with_rules(
                quest_store,
                &rules,
                |previous_quest_id| {
                    recurrence.quest_is_rewarded(previous_quest_id)
                },
            ) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: dependent previous quests not satisfied"
                );
                return false;
            }
        }

        // SatisfyQuestDependentBreadcrumbQuests — Player.cpp:15197-15216.
        // Blocks acceptance if any breadcrumb quest listed in `dependent_breadcrumb_quests` is
        // currently INCOMPLETE/COMPLETE/FAILED in the player's log.
        // Note: BreadcrumbQuest (recursive single breadcrumb, Player.cpp:15173-15195) remains
        // unimplemented here without falsing.
        if recurrence.dependent_breadcrumb_quests_block(
            rules.dependent_breadcrumb_quest_ids(),
        ) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: dependent breadcrumb in log"
            );
            return false;
        }

        // SatisfyQuestDay — Player.cpp:15387-15401.
        // DF is the outer branch: a DF+daily quest never reads daily membership.
        // Exclusive peers retain Rust's different, independent DF/daily checks.
        match recurrence.quest_day_cooldown_block(&rules) {
            Some(QuestDayCooldownBlock::DungeonFinder) => {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: DF quest already completed"
                );
                return false;
            }
            Some(QuestDayCooldownBlock::Daily) => {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: daily quest already completed"
                );
                return false;
            }
            None => {}
        }

        // SatisfyQuestWeek — Player.cpp:15403-15410.
        if recurrence.quest_weekly_cooldown_blocks(&rules) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: weekly quest on cooldown"
            );
            return false;
        }

        // SatisfyQuestMonth — Player.cpp:15439-15446.
        if recurrence.quest_monthly_cooldown_blocks(&rules) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: monthly quest on cooldown"
            );
            return false;
        }

        // SatisfyQuestSeasonal — Player.cpp:15412-15423.
        // Per C++ CanTakeQuest order (Player.cpp:14087): Day/Week/Month (above) and
        // Seasonal precede Conditions; the dependent cluster (prev_quest_id,
        // DependentPreviousQuests, DependentBreadcrumbQuests) runs before this, as part of
        // SatisfyQuestDependentQuests. SatisfyQuestTimed remains a separate gap:
        // the session has no active-timed-quest set yet (see #QUESTS.15).
        if recurrence.quest_seasonal_cooldown_blocks(&rules) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                event_id = quest.event_id_for_quest_like_cpp(),
                "CanTakeQuest: seasonal quest cooldown"
            );
            return false;
        }

        // SatisfyQuestConditions — Player.cpp:15311-15325.
        if !self.represented_quest_available_conditions_meet_like_cpp(quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: quest available conditions not met"
            );
            return false;
        }

        // SatisfyQuestExpansion — Player.cpp:15425-15437.
        if i32::from(self.expansion) < quest.expansion {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: required expansion"
            );
            return false;
        }

        true
    }
}
