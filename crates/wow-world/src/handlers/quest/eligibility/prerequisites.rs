//! prerequisites operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    pub(super) fn quest_status_like_cpp(&self, quest_id: u32) -> Option<u8> {
        let state = self.player_quest_gameplay_snapshot_like_cpp()?;
        if state.rewarded_quest_ids_like_cpp().contains(&quest_id) {
            return Some(QUEST_STATUS_REWARDED_LIKE_CPP);
        }

        Some(
            state
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|quest| quest.status)
                .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP),
        )
    }

    // SatisfyQuestSkill — Player.cpp:15009-15037 (CanTakeQuest:14087).
    pub(super) fn satisfy_quest_skill_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_skill_id == 0 {
            return true;
        }
        let Ok(skill_u16) = u16::try_from(quest.required_skill_id) else {
            return true;
        };
        self.resolved_player_skill_value_like_cpp(skill_u16)
            .is_some_and(|value| u32::from(value) >= quest.required_skill_points)
    }

    // SatisfyQuestReputation — Player.cpp:15256-15283 (CanTakeQuest:14087).
    //
    // Mirrors C++ GetReputation(fId) = base + standing.
    // faction_store None or faction not found → treat reputation as 0 (C++ GetReputation returns 0
    // for unknown faction id, Player.cpp:15256 / ReputationMgr.cpp:118-124).
    pub(super) fn satisfy_quest_reputation_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_min_rep_faction != 0 {
            let rep = match self
                .faction_store()
                .and_then(|store| store.get(quest.required_min_rep_faction))
            {
                Some(faction_entry) => {
                    let player_race = self.player_race_like_cpp();
                    let player_class = self.player_class_like_cpp();
                    let Some(rep) = self.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction_entry,
                            player_race,
                            player_class,
                        )
                    }) else {
                        return false;
                    };
                    rep
                }
                None => 0,
            };
            if rep < quest.required_min_rep_value {
                return false;
            }
        }

        if quest.required_max_rep_faction != 0 {
            let rep = match self
                .faction_store()
                .and_then(|store| store.get(quest.required_max_rep_faction))
            {
                Some(faction_entry) => {
                    let player_race = self.player_race_like_cpp();
                    let player_class = self.player_class_like_cpp();
                    let Some(rep) = self.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction_entry,
                            player_race,
                            player_class,
                        )
                    }) else {
                        return false;
                    };
                    rep
                }
                None => 0,
            };
            if rep >= quest.required_max_rep_value {
                return false;
            }
        }

        true
    }

    // SatisfyQuestExclusiveGroup — Player.cpp:15342-15385.
    //
    // Only positive exclusive_group values restrict: a positive group means "take
    // at most one quest from this set".  Non-positive (0 or negative) groups are
    // unused/unrestricted → always true (Player.cpp:15342).
    //
    // quest_store None → fail-open: without the store we cannot enumerate peers,
    // so we conservatively allow the quest rather than silently blocking it.  The
    // same fail-open pattern is used throughout can_take_quest for missing stores.
    pub(super) fn satisfy_quest_exclusive_group_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // Player.cpp:15342 — non-positive exclusive_group never restricts.
        if quest.exclusive_group <= 0 {
            return true;
        }

        let Some(quest_store) = &self.quests.store else {
            return true;
        };
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let rules = quest.eligibility_rules();

        for peer in quest_store
            .quests
            .values()
            .filter(|candidate| candidate.exclusive_group == quest.exclusive_group)
        {
            // SatisfyQuestExclusiveGroup (15342): skip the quest being evaluated.
            if peer.id == quest.id {
                continue;
            }
            let peer_rules = peer.eligibility_rules();
            // Preserve Rust's independent DF/daily checks and active-key rule;
            // C++ instead calls SatisfyQuestDay and GetQuestStatus here.
            if recurrence.exclusive_group_peer_blocks(&rules, &peer_rules) {
                return false;
            }
        }

        true
    }

    pub(crate) fn represented_quest_is_important_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        WorldSession::represented_quest_dialog_classification_like_cpp(
            quest,
            self.quests.info_store.as_deref(),
        )
        .is_important()
    }

    pub(super) fn represented_quest_is_trivial_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.player_level_like_cpp() as i32
            > quest
                .quest_level
                .saturating_add(self.quest_state.quest_low_level_hide_diff_like_cpp as i32)
    }

    pub(super) fn satisfy_quest_level_represented_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let level = self.player_level_like_cpp();
        quest.meets_min_level(level) && quest.meets_max_level(level)
    }

    pub(super) fn satisfy_quest_race_class_represented_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        quest.is_available_for(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            self.player_level_like_cpp()
                .max(quest.min_level.max(1).min(i32::from(u8::MAX)) as u8),
        )
    }

    pub(super) fn can_see_start_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        if self.is_quest_disabled_like_cpp(quest.id) {
            return false;
        }

        if self.quest_status_like_cpp(quest.id) != Some(QUEST_STATUS_NONE_LIKE_CPP) {
            return false;
        }

        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let rules = quest.eligibility_rules();
        if recurrence.quest_seasonal_cooldown_blocks(&rules) {
            return false;
        }

        if !recurrence.previous_quest_requirement_satisfied(rules.previous_quest_id()) {
            return false;
        }

        self.satisfy_quest_race_class_represented_like_cpp(quest)
            && i32::from(self.player_level_like_cpp())
                .saturating_add(self.quest_state.quest_high_level_hide_diff_like_cpp as i32)
                >= quest.min_level
    }

    pub(crate) fn is_quest_disabled_like_cpp(&self, quest_id: u32) -> bool {
        self.disable_mgr().is_some_and(|disable_mgr| {
            disable_mgr.is_disabled_for_like_cpp(DISABLE_TYPE_QUEST, quest_id, None, 0, None)
        })
    }
}

pub(in crate::handlers::quest) fn player_race_or_class_mask_like_cpp(id: u8) -> u32 {
    if id == 0 {
        return 0;
    }

    1_u32
        .checked_shl(u32::from(id.saturating_sub(1)))
        .unwrap_or(0)
}


pub(crate) fn represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    quest: &wow_data::quest::QuestTemplate,
    receiver_rewarded_quests: &std::collections::HashSet<u32>,
) -> bool {
    let rules = quest.eligibility_rules();
    represented_satisfy_quest_dependent_previous_quests_failed_with_rules(
        quest_store,
        &rules,
        |quest_id| receiver_rewarded_quests.contains(&quest_id),
    )
}


// Player::SatisfyQuestDependentPreviousQuests, Player.cpp:15115-15171 at a5f8da2e.
// Catalog reads remain lazy and in application; the predicate reads one snapshot.
pub(in crate::handlers::quest) fn represented_satisfy_quest_dependent_previous_quests_failed_with_rules(
    quest_store: &wow_data::quest::QuestStore,
    quest: &wow_data::quest::QuestEligibilityRules<'_>,
    is_rewarded: impl FnMut(u32) -> bool,
) -> bool {
    PlayerQuestGameplayState::dependent_previous_quest_ids_block(
        quest.dependent_previous_quest_ids(),
        |prev_id| quest_store.get(prev_id).map(|previous| previous.exclusive_group),
        |exclusive_group| {
            quest_store
                .quests
                .values()
                .filter(move |candidate| candidate.exclusive_group == exclusive_group)
                .map(|candidate| candidate.id)
        },
        is_rewarded,
    )
}


// Player::SatisfyQuestDependentBreadcrumbQuests, Player.cpp:15197-15216 at a5f8da2e.
pub(in crate::handlers::quest) fn represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
    receiver_active_quest_statuses: &std::collections::HashMap<u32, u8>,
) -> bool {
    PlayerQuestGameplayState::dependent_breadcrumb_quest_ids_block(
        &quest.dependent_breadcrumb_quests,
        |quest_id| receiver_active_quest_statuses.get(&quest_id).copied(),
    )
}


pub(in crate::handlers::quest) fn represented_can_take_quest_after_expansion_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    quest: &wow_data::quest::QuestTemplate,
    receiver: &crate::session::directory::PlayerQuestSharingSnapshot,
) -> bool {
    // C++ anchor: `Player::CanTakeQuest`, Player.cpp:14093-14102, after the
    // push handler has already emitted dedicated messages for class/race/level,
    // reputation, prerequisite, daily/DF, and expansion gates. This bounded
    // helper keeps the remaining represented `false` cases that TrinityCore
    // groups under `QuestPushReason::Invalid` at the final CanTakeQuest gate.
    // Explicitly out of scope for this represented-partial slice: DisableMgr,
    // skill, timed, weekly/monthly, ConditionMgr, and full seasonal runtime.
    PlayerQuestGameplayState::sharing_acceptance_after_expansion(
        &quest.eligibility_rules(),
        || quest_store.quests.values().map(|peer| peer.eligibility_rules()),
        |id| receiver.active_quest_statuses.get(&id).copied(),
        |id| receiver.rewarded_quests.contains(&id),
        |id| receiver.df_quests.contains(&id),
        |id| receiver.daily_quests_completed.contains(&id),
    )
}
