// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered quest-giver dialog status from selected quest and player state.

use crate::quest::QuestDialogClassificationLikeCpp;
use crate::quest::dialog_status::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::quest::visibility::quest_eligibility::QuestEligibilityCx;
use wow_data::{progression_rewards::QuestInfoStore, quest::QuestTemplate};

impl QuestEligibilityCx<'_> {
    /// Resolve a creature or GameObject's represented dialog status in the
    /// native ender-then-starter relation order.
    pub fn get_represented_quest_giver_status_with_catalog_like_cpp(
        &self,
        quest_info: Option<&QuestInfoStore>,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        let Some(store) = &self.catalogs.quests.store else {
            return wow_packet::packets::quest::quest_giver_status::NONE;
        };

        use wow_packet::packets::quest::quest_giver_status;

        let turn_in_quests = match source {
            RepresentedQuestGiverStatusSourceLikeCpp::Creature { entry } => {
                store.quests_for_ender(entry)
            }
            RepresentedQuestGiverStatusSourceLikeCpp::GameObject { entry } => {
                store.quests_for_gameobject_ender(entry)
            }
        };

        let mut result = quest_giver_status::NONE;
        for quest in turn_in_quests {
            let Some(status) = self.quest_status_like_cpp(quest.id) else {
                return quest_giver_status::NONE;
            };
            match status {
                wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP => {
                    result |= self
                        .dialog_classification_like_cpp(quest, quest_info)
                        .reward_complete();
                }
                wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP => {
                    result |= self
                        .dialog_classification_like_cpp(quest, quest_info)
                        .reward();
                }
                _ => {}
            }

            if quest.quest_type == 0
                && self.can_take_quest_like_cpp(quest)
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
                && !quest.is_monthly_like_cpp()
            {
                if self.represented_quest_is_trivial_like_cpp(quest) {
                    result |= quest_giver_status::TRIVIAL_REPEATABLE_TURNIN;
                } else {
                    result |= quest_giver_status::REPEATABLE_TURNIN;
                }
            }
        }

        let start_quests = match source {
            RepresentedQuestGiverStatusSourceLikeCpp::Creature { entry } => {
                store.quests_for_starter(entry)
            }
            RepresentedQuestGiverStatusSourceLikeCpp::GameObject { entry } => {
                store.quests_for_gameobject_starter(entry)
            }
        };

        for quest in start_quests {
            let owner = self.player.quest_objective_access_like_cpp();
            if !self
                .conditions
                .represented_quest_available_conditions_meet_like_cpp(
                    &owner,
                    self.quest_state,
                    self.catalogs,
                    quest.id,
                    self.consumer_test,
                )
            {
                continue;
            }

            if self.quest_status_like_cpp(quest.id)
                != Some(wow_conditions::QUEST_STATUS_NONE_LIKE_CPP)
            {
                continue;
            }

            if !self.can_see_start_quest_like_cpp(quest) {
                continue;
            }

            if self.satisfy_quest_level_like_cpp(quest) {
                let classification = self.dialog_classification_like_cpp(quest, quest_info);
                result |=
                    classification.available(self.represented_quest_is_trivial_like_cpp(quest));
            } else {
                result |= self
                    .dialog_classification_like_cpp(quest, quest_info)
                    .future();
            }
        }

        result
    }

    fn quest_status_like_cpp(&self, quest_id: u32) -> Option<u8> {
        let state = self.quest_gameplay_snapshot_like_cpp()?;
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

    fn dialog_classification_like_cpp(
        &self,
        quest: &QuestTemplate,
        quest_info: Option<&QuestInfoStore>,
    ) -> QuestDialogClassificationLikeCpp {
        QuestDialogClassificationLikeCpp::new(
            quest.flags,
            quest.flags_ex,
            quest_info.and_then(|store| store.get(quest.quest_info_id as u32)),
        )
    }

    /// C++ `Player::CanSeeStartQuest` (Player.cpp:14073-14085).
    ///
    /// The CompleteQuest guard asks this question before it publishes a dialog,
    /// so it is part of the public represented surface.
    pub fn can_see_start_quest_like_cpp(&self, quest: &QuestTemplate) -> bool {
        if self.catalogs.disable_mgr().is_some_and(|disable_mgr| {
            disable_mgr.is_disabled_for_like_cpp(
                wow_data::DISABLE_TYPE_QUEST,
                quest.id,
                None,
                0,
                None,
            )
        }) {
            return false;
        }

        if self.quest_status_like_cpp(quest.id) != Some(wow_conditions::QUEST_STATUS_NONE_LIKE_CPP)
        {
            return false;
        }

        let Some(recurrence) = self.quest_gameplay_snapshot_like_cpp() else {
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
            let previous_id = quest.prev_quest_id.unsigned_abs();
            if quest.prev_quest_id > 0 {
                if !recurrence
                    .rewarded_quest_ids_like_cpp()
                    .contains(&previous_id)
                {
                    return false;
                }
            } else if !recurrence
                .statuses_like_cpp()
                .get(&previous_id)
                .is_some_and(|status| {
                    status.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                })
            {
                return false;
            }
        }

        self.satisfy_quest_race_class_like_cpp(quest)
            && i32::from(self.player.player_level_like_cpp())
                .saturating_add(self.quest_state.quest_high_level_hide_diff_like_cpp() as i32)
                >= quest.min_level
    }

    fn satisfy_quest_level_like_cpp(&self, quest: &QuestTemplate) -> bool {
        let level = self.player.player_level_like_cpp();
        if quest.min_level > 0 && i32::from(level) < quest.min_level {
            return false;
        }

        if quest.max_level > 0 && level > quest.max_level {
            return false;
        }

        true
    }

    fn satisfy_quest_race_class_like_cpp(&self, quest: &QuestTemplate) -> bool {
        quest.is_available_for(
            self.player.player_race_like_cpp(),
            self.player.player_class_like_cpp(),
            self.player
                .player_level_like_cpp()
                .max(quest.min_level.max(1).min(i32::from(u8::MAX)) as u8),
        )
    }

    fn represented_quest_is_trivial_like_cpp(&self, quest: &QuestTemplate) -> bool {
        self.player.player_level_like_cpp() as i32
            > quest
                .quest_level
                .saturating_add(self.quest_state.quest_low_level_hide_diff_like_cpp() as i32)
    }
}
