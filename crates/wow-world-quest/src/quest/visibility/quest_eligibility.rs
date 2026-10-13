// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected quest eligibility predicates used by quest-dialog visibility.

use crate::PlayerConditionProjectionCxLikeCpp;
use crate::quest::{QuestObjectiveProgressCx, QuestRewardCx, SessionQuestState};
use wow_data::quest::QuestTemplate;
use wow_world_core::session::{QuestEligibilityAccessLikeCpp, SessionCatalogs};

/// Read-only participants for the complete quest-admission predicate.
pub struct QuestEligibilityCx<'a> {
    pub(super) player: QuestEligibilityAccessLikeCpp<'a>,
    pub(super) quest_state: &'a SessionQuestState,
    pub(super) catalogs: &'a SessionCatalogs,
    pub(super) conditions: &'a PlayerConditionProjectionCxLikeCpp<'a>,
    pub(super) consumer_test: bool,
}

impl<'a> QuestEligibilityCx<'a> {
    pub fn new(
        player: QuestEligibilityAccessLikeCpp<'a>,
        quest_state: &'a SessionQuestState,
        catalogs: &'a SessionCatalogs,
        conditions: &'a PlayerConditionProjectionCxLikeCpp<'a>,
        consumer_test: bool,
    ) -> Self {
        Self {
            player,
            quest_state,
            catalogs,
            conditions,
            consumer_test,
        }
    }

    pub(super) fn quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        let canonical = self.player.player_quest_gameplay_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.consumer_test && self.player.owner_handle_absent_like_cpp() {
            return Some(self.quest_state.player_quest_gameplay_fixture_like_cpp());
        }
        canonical
    }
}

impl QuestObjectiveProgressCx<'_, '_> {
    pub(super) fn satisfy_quest_level_represented_like_cpp(&self, quest: &QuestTemplate) -> bool {
        satisfy_quest_level_like_cpp(&self.reward, quest)
    }

    pub(super) fn satisfy_quest_race_class_represented_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool {
        satisfy_quest_race_class_like_cpp(&self.reward, quest)
    }

    pub(super) fn can_see_start_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool {
        can_see_start_quest_like_cpp(&self.reward, quest)
    }
}

fn satisfy_quest_level_like_cpp(reward: &QuestRewardCx<'_>, quest: &QuestTemplate) -> bool {
    let level = reward.player_level_like_cpp();
    if quest.min_level > 0 && i32::from(level) < quest.min_level {
        return false;
    }

    if quest.max_level > 0 && level > quest.max_level {
        return false;
    }

    true
}

fn satisfy_quest_race_class_like_cpp(reward: &QuestRewardCx<'_>, quest: &QuestTemplate) -> bool {
    quest.is_available_for(
        reward.player.player_race_like_cpp(),
        reward.player.player_class_like_cpp(),
        reward
            .player_level_like_cpp()
            .max(quest.min_level.max(1).min(i32::from(u8::MAX)) as u8),
    )
}

fn quest_status_like_cpp(reward: &QuestRewardCx<'_>, quest_id: u32) -> Option<u8> {
    let state = reward.quest_gameplay_snapshot_like_cpp()?;
    if state.rewarded_quest_ids_like_cpp().contains(&quest_id) {
        return Some(wow_conditions::QUEST_STATUS_REWARDED_LIKE_CPP);
    }

    Some(
        state
            .statuses_like_cpp()
            .get(&quest_id)
            .map(|quest| quest.status)
            .unwrap_or(wow_conditions::QUEST_STATUS_NONE_LIKE_CPP),
    )
}

fn can_see_start_quest_like_cpp(reward: &QuestRewardCx<'_>, quest: &QuestTemplate) -> bool {
    if reward.catalogs.disable_mgr().is_some_and(|disable_mgr| {
        disable_mgr.is_disabled_for_like_cpp(wow_data::DISABLE_TYPE_QUEST, quest.id, None, 0, None)
    }) {
        return false;
    }

    if quest_status_like_cpp(reward, quest.id) != Some(wow_conditions::QUEST_STATUS_NONE_LIKE_CPP) {
        return false;
    }

    let Some(recurrence) = reward.quest_gameplay_snapshot_like_cpp() else {
        return false;
    };
    if quest.is_seasonal_like_cpp() && !recurrence.seasonal_quests_like_cpp().is_empty() {
        if let Some(bucket) = recurrence
            .seasonal_quests_like_cpp()
            .get(&quest.event_id_for_quest_like_cpp())
        {
            if !bucket.is_empty() && bucket.contains_key(&quest.id) {
                return false;
            }
        }
    }

    if quest.prev_quest_id != 0 {
        let prev_id = quest.prev_quest_id.unsigned_abs();
        if quest.prev_quest_id > 0 {
            if !recurrence.rewarded_quest_ids_like_cpp().contains(&prev_id) {
                return false;
            }
        } else if !recurrence
            .statuses_like_cpp()
            .get(&prev_id)
            .is_some_and(|status| status.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        {
            return false;
        }
    }

    satisfy_quest_race_class_like_cpp(reward, quest)
        && i32::from(reward.player_level_like_cpp())
            .saturating_add(reward.quest_state.quest_high_level_hide_diff_like_cpp() as i32)
            >= quest.min_level
}

pub(super) fn satisfy_quest_exclusive_group_like_cpp(
    eligibility: &QuestEligibilityCx<'_>,
    quest: &QuestTemplate,
) -> bool {
    if quest.exclusive_group <= 0 {
        return true;
    }

    let Some(quest_store) = &eligibility.catalogs.quests.store else {
        return true;
    };
    let Some(recurrence) = eligibility.quest_gameplay_snapshot_like_cpp() else {
        return false;
    };

    for peer in quest_store
        .quests
        .values()
        .filter(|candidate| candidate.exclusive_group == quest.exclusive_group)
    {
        if peer.id == quest.id {
            continue;
        }

        if peer.is_df_quest_like_cpp() && recurrence.df_quest_ids_like_cpp().contains(&peer.id) {
            return false;
        }
        if peer.is_daily_like_cpp() && recurrence.daily_quest_ids_like_cpp().contains(&peer.id) {
            return false;
        }
        if peer.is_weekly_like_cpp() && recurrence.weekly_quest_ids_like_cpp().contains(&peer.id) {
            return false;
        }
        if peer.is_seasonal_like_cpp() && !recurrence.seasonal_quests_like_cpp().is_empty() {
            if let Some(bucket) = recurrence
                .seasonal_quests_like_cpp()
                .get(&peer.event_id_for_quest_like_cpp())
            {
                if !bucket.is_empty() && bucket.contains_key(&peer.id) {
                    return false;
                }
            }
        }

        if recurrence.statuses_like_cpp().contains_key(&peer.id) {
            return false;
        }
        if !(quest.is_repeatable() && peer.is_repeatable())
            && recurrence.rewarded_quest_ids_like_cpp().contains(&peer.id)
        {
            return false;
        }
    }

    true
}

pub(super) fn satisfy_quest_skill_like_cpp(
    player: &QuestEligibilityAccessLikeCpp<'_>,
    quest: &QuestTemplate,
) -> bool {
    if quest.required_skill_id == 0 {
        return true;
    }
    let Ok(skill_id) = u16::try_from(quest.required_skill_id) else {
        return true;
    };
    player
        .resolved_quest_skill_value_like_cpp(skill_id)
        .is_some_and(|value| u32::from(value) >= quest.required_skill_points)
}

pub(super) fn satisfy_quest_reputation_like_cpp(
    player: &QuestEligibilityAccessLikeCpp<'_>,
    catalogs: &SessionCatalogs,
    quest: &QuestTemplate,
) -> bool {
    if quest.required_min_rep_faction != 0 {
        let reputation = match catalogs
            .faction_store()
            .and_then(|store| store.get(quest.required_min_rep_faction))
        {
            Some(faction_entry) => {
                let Some(reputation) = player.quest_reputation_for_faction_like_cpp(faction_entry)
                else {
                    return false;
                };
                reputation
            }
            None => 0,
        };
        if reputation < quest.required_min_rep_value {
            return false;
        }
    }

    if quest.required_max_rep_faction != 0 {
        let reputation = match catalogs
            .faction_store()
            .and_then(|store| store.get(quest.required_max_rep_faction))
        {
            Some(faction_entry) => {
                let Some(reputation) = player.quest_reputation_for_faction_like_cpp(faction_entry)
                else {
                    return false;
                };
                reputation
            }
            None => 0,
        };
        if reputation >= quest.required_max_rep_value {
            return false;
        }
    }

    true
}

impl QuestEligibilityCx<'_> {
    pub fn can_take_quest_like_cpp(&self, quest: &QuestTemplate) -> bool {
        if self.catalogs.disable_mgr().is_some_and(|disable_mgr| {
            disable_mgr.is_disabled_for_like_cpp(
                wow_data::DISABLE_TYPE_QUEST,
                quest.id,
                None,
                0,
                None,
            )
        }) {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: quest disabled"
            );
            return false;
        }
        let Some(recurrence) = self.quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        // Retain the current Rust rewarded/status interpretation. Target source anchor:
        // Player.cpp::SatisfyQuestStatus (a5f8da2, 15285); CanTakeQuest begins at 14090.
        if recurrence.rewarded_quest_ids_like_cpp().contains(&quest.id) && !quest.is_repeatable() {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: already rewarded"
            );
            return false;
        }
        if recurrence.statuses_like_cpp().contains_key(&quest.id) {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: already active"
            );
            return false;
        }

        if !satisfy_quest_exclusive_group_like_cpp(self, quest) {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: exclusive group blocked"
            );
            return false;
        }

        if !quest.is_available_for(
            self.player.player_race_like_cpp(),
            self.player.player_class_like_cpp(),
            self.player.player_level_like_cpp(),
        ) {
            return false;
        }

        if !satisfy_quest_skill_like_cpp(&self.player, quest) {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: skill requirement not met"
            );
            return false;
        }

        if !satisfy_quest_reputation_like_cpp(&self.player, self.catalogs, quest) {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: reputation requirement not met"
            );
            return false;
        }

        if quest.prev_quest_id != 0 {
            let prev_id = quest.prev_quest_id.unsigned_abs();
            if quest.prev_quest_id > 0 {
                if !recurrence.rewarded_quest_ids_like_cpp().contains(&prev_id) {
                    tracing::debug!(
                        account = self.player.account_id_like_cpp(),
                        quest_id = quest.id,
                        prev_id,
                        "CanTakeQuest: prev quest not rewarded"
                    );
                    return false;
                }
            } else {
                let active = recurrence
                    .statuses_like_cpp()
                    .get(&prev_id)
                    .is_some_and(|status| {
                        status.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    });
                if !active {
                    tracing::debug!(
                        account = self.player.account_id_like_cpp(),
                        quest_id = quest.id,
                        prev_id,
                        "CanTakeQuest: negative prev quest not active"
                    );
                    return false;
                }
            }
        }

        if let Some(quest_store) = &self.catalogs.quests.store {
            if Self::represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                quest_store,
                quest,
                &recurrence
                    .rewarded_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
            ) {
                tracing::debug!(
                    account = self.player.account_id_like_cpp(),
                    quest_id = quest.id,
                    "CanTakeQuest: dependent previous quests not satisfied"
                );
                return false;
            }
        }

        let statuses: std::collections::HashMap<u32, u8> = recurrence
            .statuses_like_cpp()
            .iter()
            .map(|(&quest_id, status)| (quest_id, status.status))
            .collect();
        if Self::represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
            quest, &statuses,
        ) {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: dependent breadcrumb in log"
            );
            return false;
        }

        if quest.is_df_quest_like_cpp() {
            if recurrence.df_quest_ids_like_cpp().contains(&quest.id) {
                tracing::debug!(
                    account = self.player.account_id_like_cpp(),
                    quest_id = quest.id,
                    "CanTakeQuest: DF quest already completed"
                );
                return false;
            }
        } else if quest.is_daily_like_cpp()
            && recurrence.daily_quest_ids_like_cpp().contains(&quest.id)
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: daily quest already completed"
            );
            return false;
        }

        if quest.is_weekly_like_cpp() && recurrence.weekly_quest_ids_like_cpp().contains(&quest.id)
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: weekly quest on cooldown"
            );
            return false;
        }

        if quest.is_monthly_like_cpp()
            && recurrence.monthly_quest_ids_like_cpp().contains(&quest.id)
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: monthly quest on cooldown"
            );
            return false;
        }

        if quest.is_seasonal_like_cpp() && !recurrence.seasonal_quests_like_cpp().is_empty() {
            if let Some(bucket) = recurrence
                .seasonal_quests_like_cpp()
                .get(&quest.event_id_for_quest_like_cpp())
            {
                if !bucket.is_empty() && bucket.contains_key(&quest.id) {
                    tracing::debug!(
                        account = self.player.account_id_like_cpp(),
                        quest_id = quest.id,
                        event_id = quest.event_id_for_quest_like_cpp(),
                        "CanTakeQuest: seasonal quest cooldown"
                    );
                    return false;
                }
            }
        }

        let quest_owner = self.player.quest_objective_access_like_cpp();
        if !self
            .conditions
            .represented_quest_available_conditions_meet_like_cpp(
                &quest_owner,
                self.quest_state,
                self.catalogs,
                quest.id,
                self.consumer_test,
            )
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: quest available conditions not met"
            );
            return false;
        }

        if i32::from(self.player.expansion_like_cpp()) < quest.expansion {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id = quest.id,
                "CanTakeQuest: required expansion"
            );
            return false;
        }

        true
    }

    pub fn represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
        quest_store: &wow_data::quest::QuestStore,
        quest: &QuestTemplate,
        receiver_rewarded_quests: &std::collections::HashSet<u32>,
    ) -> bool {
        if quest.dependent_previous_quests.is_empty() {
            return false;
        }

        for &prev_id in &quest.dependent_previous_quests {
            let Some(previous_quest) = quest_store.get(prev_id) else {
                // C++ asserts because ObjectMgr validates this at startup. Rust fails closed
                // as the prerequisite branch rather than panicking in the sender loop.
                return true;
            };

            if receiver_rewarded_quests.contains(&prev_id) {
                if previous_quest.exclusive_group >= 0 {
                    return false;
                }

                for exclusive_quest_id in quest_store
                    .quests
                    .values()
                    .filter(|candidate| candidate.exclusive_group == previous_quest.exclusive_group)
                    .map(|candidate| candidate.id)
                {
                    if exclusive_quest_id != prev_id
                        && !receiver_rewarded_quests.contains(&exclusive_quest_id)
                    {
                        return true;
                    }
                }

                return false;
            }
        }

        true
    }

    pub fn represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
        quest: &QuestTemplate,
        receiver_active_quest_statuses: &std::collections::HashMap<u32, u8>,
    ) -> bool {
        quest
            .dependent_breadcrumb_quests
            .iter()
            .any(|breadcrumb_quest_id| {
                matches!(
                    receiver_active_quest_statuses
                        .get(breadcrumb_quest_id)
                        .copied(),
                    Some(
                        wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                            | wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                            | wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP
                    )
                )
            })
    }
}
