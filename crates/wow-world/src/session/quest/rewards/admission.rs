//! admission operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_player_quest_status_is_complete_or_incomplete_like_cpp(
        &self,
        quest_id: u32,
    ) -> bool {
        self.represented_player_quest_status_like_cpp(quest_id)
            .is_some_and(|status| {
                matches!(
                    status,
                    Some(
                        wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                            | wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    )
                )
            })
    }
    pub(crate) fn can_complete_repeatable_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // C++ Player::CanCompleteRepeatableQuest is CanTakeQuest(false) && CanRewardQuest(false).
        // The full CanRewardQuest blocker set includes disable checks, quest status, day/week/month/
        // seasonal gates, level/skill/reputation, reward status, item/currency/money checks, etc.
        // This bounded #611 seam must not overclaim completion while those blockers remain open.
        if !self.can_take_quest(quest) {
            return false;
        }

        let Some(inventory_item_counts) = self.represented_inventory_item_counts_like_cpp() else {
            return false;
        };
        let mut saw_non_bound_item_objective = false;
        for objective in &quest.objectives {
            if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP {
                return false;
            }

            if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP) != 0 {
                continue;
            }

            saw_non_bound_item_objective = true;
            let Ok(item_id) = u32::try_from(objective.object_id) else {
                return false;
            };
            let Ok(required_count) = u32::try_from(objective.amount) else {
                return false;
            };
            if inventory_item_counts.get(&item_id).copied().unwrap_or(0) < required_count {
                return false;
            }
        }

        // Item counts are represented only as partial evidence. Do not return true until the
        // remaining C++ CanRewardQuest blockers are represented in this runtime path.
        let _ = saw_non_bound_item_objective;
        false
    }
    pub(crate) fn can_reward_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // C++ `Player::CanRewardQuest(quest, false)` requires the quest to be complete
        // unless it is a DF/turn-in-only quest, then rejects already rewarded
        // non-repeatable quests. This represented seam covers the status/reward gates
        // needed by `SendQuestGiverRequestItems`; inventory-capacity and long-tail
        // reward blockers remain in the reward handler path.
        if self.is_quest_disabled_like_cpp(quest.id) {
            return false;
        }
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        if !quest.is_df_quest_like_cpp()
            && !quest.is_turn_in_like_cpp()
            && !quests
                .statuses_like_cpp()
                .get(&quest.id)
                .is_some_and(|status| {
                    status.status == wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                })
        {
            return false;
        }

        if quests.rewarded_quest_ids_like_cpp().contains(&quest.id) && !quest.is_repeatable() {
            return false;
        }

        true
    }
}
