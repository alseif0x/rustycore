// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest availability: status classification and the `SatisfyQuest*` gates.

use super::*;

impl WorldSession {
    /// Resolves CMSG_QUEST_GIVER_STATUS_QUERY through the represented equivalent of
    /// C++ `ObjectAccessor::GetObjectByTypeMask(*_player, guid, TYPEMASK_UNIT | TYPEMASK_GAMEOBJECT)`.
    /// Missing canonical objects and unsupported Player/Item/other GUID types fail closed with no packet.
    pub(crate) fn represented_quest_giver_status_query_source_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
    ) -> Option<RepresentedQuestGiverStatusSourceLikeCpp> {
        if guid.is_any_type_creature() {
            // C++ TYPEID_UNIT branch also checks Creature::IsHostileTo before computing
            // dialog status. Exact faction/hostility is not represented here yet; a
            // resolved canonical Creature is treated as non-hostile only for this
            // bounded represented status calculation.
            let access = self.canonical_creature_access_like_cpp(guid)?;
            return Some(RepresentedQuestGiverStatusSourceLikeCpp::Creature {
                entry: access.entry,
            });
        }

        if guid.is_game_object() {
            let access = self.canonical_gameobject_access_like_cpp(guid)?;
            return Some(RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                entry: access.entry,
            });
        }

        None
    }

    /// Bounded representation of C++ `Player::GetQuestDialogStatus(Object const*)`.
    /// Creature sources use Creature starter/ender relations; GameObject sources use
    /// GO starter/ender relations. Full AI dialog status, ConditionMgr, event overlays
    /// and important/covenant/journey DB2 classification remain documented migration gaps.
    pub(crate) fn get_represented_quest_giver_status_like_cpp(
        &self,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        let Some(store) = &self.quest_store else {
            return quest_giver_status::NONE;
        };

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
                QUEST_STATUS_COMPLETE_LIKE_CPP => {
                    result |= self.represented_quest_reward_complete_status_like_cpp(quest);
                }
                QUEST_STATUS_INCOMPLETE_LIKE_CPP => {
                    result |= self.represented_quest_reward_status_like_cpp(quest);
                }
                _ => {}
            }

            if quest.quest_type == 0
                && self.can_take_quest(quest)
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
            if !self.represented_quest_available_conditions_meet_like_cpp(quest.id) {
                continue;
            }

            if self.quest_status_like_cpp(quest.id) != Some(QUEST_STATUS_NONE_LIKE_CPP) {
                continue;
            }

            if !self.can_see_start_quest_represented_bounded_like_cpp(quest) {
                continue;
            }

            if self.satisfy_quest_level_represented_like_cpp(quest) {
                result |= self.represented_quest_available_status_like_cpp(
                    quest,
                    self.represented_quest_is_trivial_like_cpp(quest),
                );
            } else {
                result |= self.represented_quest_future_status_like_cpp(quest);
            }
        }

        result
    }

    fn represented_quest_available_conditions_meet_like_cpp(&self, quest_id: u32) -> bool {
        let condition_store = if let Some(store) = self.condition_store() {
            Arc::clone(store)
        } else if let Some(store) = crate::conditions::condition_mgr_store_like_cpp() {
            store
        } else {
            return true;
        };

        if !crate::conditions::has_conditions_for_not_grouped_entry_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
        ) {
            return true;
        }

        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            return false;
        };
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        let quest_statuses: Vec<_> = recurrence
            .statuses
            .iter()
            .map(
                |(&quest_id, status)| crate::conditions::ConditionQuestStatusSnapshot {
                    quest_id,
                    status: status.status,
                },
            )
            .collect();
        let quest_objective_progress: Vec<_> = self
            .quest_store
            .as_ref()
            .map(|store| {
                recurrence
                    .statuses
                    .iter()
                    .filter_map(|(&quest_id, status)| {
                        store.get(quest_id).map(|quest| {
                            quest.objectives.iter().filter_map(move |objective| {
                                let storage_index =
                                    usize::try_from(objective.storage_index).ok()?;
                                let counter = status
                                    .objective_counts
                                    .get(storage_index)
                                    .copied()
                                    .unwrap_or(0);
                                Some(crate::conditions::ConditionQuestObjectiveProgressSnapshot {
                                    quest_id,
                                    objective_id: objective.id,
                                    counter,
                                })
                            })
                        })
                    })
                    .flatten()
                    .collect()
            })
            .unwrap_or_default();
        let rewarded_quest_ids: Vec<_> = recurrence.rewarded_quest_ids.iter().copied().collect();
        let daily_quest_ids: Vec<_> = recurrence.daily_quest_ids.iter().copied().collect();
        let quest_snapshot = crate::conditions::ConditionPlayerQuestSnapshot {
            statuses: &quest_statuses,
            objective_progress: &quest_objective_progress,
            rewarded_quest_ids: &rewarded_quest_ids,
            daily_quest_ids: &daily_quest_ids,
        };
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return false;
        };
        let area_table_store = self.area_table_store().cloned();

        let mut source_info =
            crate::conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return false;
        };
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, self.condition_player_snapshot_like_cpp());
        source_info.set_player_quest_target_snapshot(0, quest_snapshot);
        if let Some(store) = self.player_condition_store() {
            source_info.set_player_condition_store(store.as_ref());
            if let Some(context) = player_condition_context.as_context(self) {
                source_info.set_player_condition_context(0, context);
            }
        }

        crate::conditions::is_object_meeting_not_grouped_conditions_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
            &mut source_info,
            |condition, source_info| {
                crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }

    fn quest_status_like_cpp(&self, quest_id: u32) -> Option<u8> {
        let state = self.player_quest_gameplay_snapshot_like_cpp()?;
        if state.rewarded_quest_ids.contains(&quest_id) {
            return Some(QUEST_STATUS_REWARDED_LIKE_CPP);
        }

        Some(
            state
                .statuses
                .get(&quest_id)
                .map(|quest| quest.status)
                .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP),
        )
    }

    // SatisfyQuestSkill — Player.cpp:14098, 15015-15037
    fn satisfy_quest_skill_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_skill_id == 0 {
            return true;
        }
        let Ok(skill_u16) = u16::try_from(quest.required_skill_id) else {
            return true;
        };
        self.resolved_player_skill_value_like_cpp(skill_u16)
            .is_some_and(|value| u32::from(value) >= quest.required_skill_points)
    }

    // SatisfyQuestReputation — Player.cpp:14098, 15262-15289
    //
    // Mirrors C++ GetReputation(fId) = base + standing.
    // faction_store None or faction not found → treat reputation as 0 (C++ GetReputation returns 0
    // for unknown faction id, Player.cpp:15265 / ReputationMgr.cpp:118-124).
    fn satisfy_quest_reputation_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_min_rep_faction != 0 {
            let rep = match self
                .faction_store()
                .and_then(|store| store.get(quest.required_min_rep_faction))
            {
                Some(faction_entry) => {
                    let Some(rep) = self.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction_entry,
                            self.player_race_like_cpp(),
                            self.player_class_like_cpp(),
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
                    let Some(rep) = self.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction_entry,
                            self.player_race_like_cpp(),
                            self.player_class_like_cpp(),
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

    // SatisfyQuestExclusiveGroup — Player.cpp:15348-15391
    //
    // Only positive exclusive_group values restrict: a positive group means "take
    // at most one quest from this set".  Non-positive (0 or negative) groups are
    // unused/unrestricted → always true (Player.cpp:15351).
    //
    // quest_store None → fail-open: without the store we cannot enumerate peers,
    // so we conservatively allow the quest rather than silently blocking it.  The
    // same fail-open pattern is used throughout can_take_quest for missing stores.
    fn satisfy_quest_exclusive_group_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // Player.cpp:15351 — non-positive exclusive_group never restricts
        if quest.exclusive_group <= 0 {
            return true;
        }

        let Some(quest_store) = &self.quest_store else {
            return true;
        };
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        for peer in quest_store
            .quests
            .values()
            .filter(|c| c.exclusive_group == quest.exclusive_group)
        {
            // Player.cpp:15360 — skip the quest being evaluated
            if peer.id == quest.id {
                continue;
            }

            // Player.cpp:15366 — SatisfyQuestDay: daily/DF cooldown blocks the group
            // Mirrors the daily/DF pattern from the push path (quest.rs:271-278).
            if peer.is_df_quest_like_cpp() && recurrence.df_quest_ids.contains(&peer.id) {
                return false;
            }
            if peer.is_daily_like_cpp() && recurrence.daily_quest_ids.contains(&peer.id) {
                return false;
            }

            // Player.cpp:15366 — SatisfyQuestWeek: weekly cooldown blocks the group
            if peer.is_weekly_like_cpp() && recurrence.weekly_quest_ids.contains(&peer.id) {
                return false;
            }

            // Player.cpp:15366 — SatisfyQuestSeasonal: seasonal cooldown blocks the group
            // Mirrors the seasonal pattern from can_take_quest (quest.rs:5948-5963).
            if peer.is_seasonal_like_cpp() && !recurrence.seasonal_quests.is_empty() {
                if let Some(bucket) = recurrence
                    .seasonal_quests
                    .get(&peer.event_id_for_quest_like_cpp())
                {
                    if !bucket.is_empty() && bucket.contains_key(&peer.id) {
                        return false;
                    }
                }
            }

            // Player.cpp:15379 — alternative quest already active or rewarded (non-repeatable pair).
            //
            // C++: GetQuestStatus(peer) != QUEST_STATUS_NONE
            //   → in C++ GetQuestStatus returns REWARDED when rewarded, so this single
            //     term would also catch rewarded quests.  We model the two cases separately
            //     to keep the Rust representation explicit:
            //   Term 1: peer is currently active in player_quests (Incomplete/Complete/Failed).
            //   Term 2: peer was rewarded AND not both quests are repeatable (matching the
            //           C++ second OR operand: GetQuestRewardStatus + !IsRepeatable pair).
            if recurrence.statuses.contains_key(&peer.id) {
                return false;
            }
            if !(quest.is_repeatable() && peer.is_repeatable())
                && recurrence.rewarded_quest_ids.contains(&peer.id)
            {
                return false;
            }
        }

        true
    }

    fn represented_quest_info_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<&QuestInfoEntry> {
        self.quest_info_store
            .as_ref()
            .and_then(|store| store.get(quest.quest_info_id as u32))
    }

    pub(crate) fn represented_quest_is_important_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        const QUEST_INFO_MODIFIER_IMPORTANT_LIKE_CPP: i32 = 0x400;
        self.represented_quest_info_like_cpp(quest)
            .is_some_and(|info| (info.modifiers & QUEST_INFO_MODIFIER_IMPORTANT_LIKE_CPP) != 0)
    }

    fn represented_quest_is_covenant_calling_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        const QUEST_TAG_TYPE_COVENANT_CALLING_LIKE_CPP: i8 = 15;
        self.represented_quest_info_like_cpp(quest)
            .is_some_and(|info| info.quest_type == QUEST_TAG_TYPE_COVENANT_CALLING_LIKE_CPP)
    }

    fn represented_quest_reward_complete_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
                quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_POI
            }
        } else if self.represented_quest_is_covenant_calling_like_cpp(quest) {
            if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI
            }
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
                quest_giver_status::LEGENDARY_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::LEGENDARY_REWARD_COMPLETE_POI
            }
        } else if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
            quest_giver_status::REWARD_COMPLETE_NO_POI
        } else {
            quest_giver_status::REWARD_COMPLETE_POI
        }
    }

    fn represented_quest_reward_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            quest_giver_status::IMPORTANT_REWARD
        } else if self.represented_quest_is_covenant_calling_like_cpp(quest) {
            quest_giver_status::COVENANT_CALLING_REWARD
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            quest_giver_status::LEGENDARY_REWARD
        } else {
            quest_giver_status::REWARD
        }
    }

    fn represented_quest_available_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        trivial: bool,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            if trivial {
                quest_giver_status::TRIVIAL_IMPORTANT_QUEST
            } else {
                quest_giver_status::IMPORTANT_QUEST
            }
        } else if self.represented_quest_is_covenant_calling_like_cpp(quest) {
            quest_giver_status::COVENANT_CALLING_QUEST
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            if trivial {
                quest_giver_status::TRIVIAL_LEGENDARY_QUEST
            } else {
                quest_giver_status::LEGENDARY_QUEST
            }
        } else if quest.is_daily_like_cpp() {
            if trivial {
                quest_giver_status::TRIVIAL_DAILY_QUEST
            } else {
                quest_giver_status::DAILY_QUEST
            }
        } else if trivial {
            quest_giver_status::TRIVIAL
        } else {
            quest_giver_status::QUEST
        }
    }

    fn represented_quest_future_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            quest_giver_status::FUTURE_IMPORTANT_QUEST
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            quest_giver_status::FUTURE_LEGENDARY_QUEST
        } else {
            quest_giver_status::FUTURE
        }
    }

    fn represented_quest_is_trivial_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.player_level_like_cpp() as i32
            > quest
                .quest_level
                .saturating_add(self.quest_low_level_hide_diff_like_cpp as i32)
    }

    fn satisfy_quest_level_represented_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let level = self.player_level_like_cpp();
        if quest.min_level > 0 && i32::from(level) < quest.min_level {
            return false;
        }

        if quest.max_level > 0 && level > quest.max_level {
            return false;
        }

        true
    }

    fn satisfy_quest_race_class_represented_like_cpp(
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

    fn can_see_start_quest_represented_bounded_like_cpp(
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
        if quest.is_seasonal_like_cpp() && !recurrence.seasonal_quests.is_empty() {
            if let Some(bucket) = recurrence
                .seasonal_quests
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
                if !recurrence.rewarded_quest_ids.contains(&prev_id) {
                    return false;
                }
            } else if !recurrence
                .statuses
                .get(&prev_id)
                .is_some_and(|qs| qs.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            {
                return false;
            }
        }

        self.satisfy_quest_race_class_represented_like_cpp(quest)
            && i32::from(self.player_level_like_cpp())
                .saturating_add(self.quest_high_level_hide_diff_like_cpp as i32)
                >= quest.min_level
    }

    /// Check if the player currently has an active quest with the given ID.
    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| state.statuses.contains_key(&quest_id))
    }

    /// Full eligibility check before accepting a quest.
    /// C++ ref: Player::CanTakeQuest (Player.cpp:14093-14102) — gate order mirrors C++ exactly.
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

        // SatisfyQuestStatus — C# lines 1624-1654
        // If quest is already rewarded (non-repeatable), cannot take again.
        if recurrence.rewarded_quest_ids.contains(&quest.id) && !quest.is_repeatable() {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: already rewarded"
            );
            return false;
        }
        // If quest is already active, cannot accept again.
        if recurrence.statuses.contains_key(&quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: already active"
            );
            return false;
        }

        // SatisfyQuestExclusiveGroup — Player.cpp:14096, Player.cpp:15348-15391
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

        // SatisfyQuestSkill — Player.cpp:14098, 15015-15037
        if !self.satisfy_quest_skill_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: skill requirement not met"
            );
            return false;
        }

        // SatisfyQuestReputation — Player.cpp:14098, 15262-15289
        if !self.satisfy_quest_reputation_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: reputation requirement not met"
            );
            return false;
        }

        // SatisfyQuestPreviousQuest — C# lines 1415-1440
        // prev_quest_id > 0 → previous quest must have been rewarded
        // prev_quest_id < 0 → previous quest must be currently active (Incomplete)
        if quest.prev_quest_id != 0 {
            let prev_id = quest.prev_quest_id.unsigned_abs();
            if quest.prev_quest_id > 0 {
                if !recurrence.rewarded_quest_ids.contains(&prev_id) {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id,
                        prev_id,
                        "CanTakeQuest: prev quest not rewarded"
                    );
                    return false;
                }
            } else {
                // negative: prev quest must be active
                let active = recurrence
                    .statuses
                    .get(&prev_id)
                    .is_some_and(|qs| qs.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP);
                if !active {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id,
                        prev_id,
                        "CanTakeQuest: negative prev quest not active"
                    );
                    return false;
                }
            }
        }

        // SatisfyQuestDependentPreviousQuests — Player.cpp:15090 / Player.cpp:15121-15177
        // Blocks acceptance if the scalar dependent-previous list is not satisfied.
        // Per C++ SatisfyQuestDependentQuests (Player.cpp:15088-15092), this cluster runs
        // after SatisfyQuestReputation, not before Race/Class/Level.
        if let Some(quest_store) = &self.quest_store {
            if represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                quest_store,
                quest,
                &recurrence.rewarded_quest_ids.iter().copied().collect(),
            ) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: dependent previous quests not satisfied"
                );
                return false;
            }
        }

        // SatisfyQuestDependentBreadcrumbQuests — Player.cpp:15203-15222
        // Blocks acceptance if any breadcrumb quest listed in `dependent_breadcrumb_quests` is
        // currently INCOMPLETE/COMPLETE/FAILED in the player's log.
        // Note: BreadcrumbQuest (recursive single breadcrumb, Player.cpp:15179-15202) remains
        // unimplemented here without falsing.
        {
            let statuses: std::collections::HashMap<u32, u8> = recurrence
                .statuses
                .iter()
                .map(|(&qid, qs)| (qid, qs.status))
                .collect();
            if represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
                quest, &statuses,
            ) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: dependent breadcrumb in log"
                );
                return false;
            }
        }

        // SatisfyQuestDay — Player.cpp:15393-15407 (CanTakeQuest term Player.cpp:14093-14102).
        // DF (dungeon-finder) quests are gated by the DFQuests set; regular dailies by
        // DailyQuestsCompleted. Mirrors the completion-push split at quest.rs:2973-2979
        // and the exclusive-group peer pattern at quest.rs:5873-5879.
        if quest.is_df_quest_like_cpp() {
            if recurrence.df_quest_ids.contains(&quest.id) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: DF quest already completed"
                );
                return false;
            }
        } else if quest.is_daily_like_cpp() && recurrence.daily_quest_ids.contains(&quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: daily quest already completed"
            );
            return false;
        }

        // SatisfyQuestWeek — Player.cpp:15409-15418 (CanTakeQuest term Player.cpp:14093-14102).
        if quest.is_weekly_like_cpp() && recurrence.weekly_quest_ids.contains(&quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: weekly quest on cooldown"
            );
            return false;
        }

        // SatisfyQuestMonth — Player.cpp:15445-15454 (CanTakeQuest term Player.cpp:14093-14102).
        if quest.is_monthly_like_cpp() && recurrence.monthly_quest_ids.contains(&quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: monthly quest on cooldown"
            );
            return false;
        }

        // SatisfyQuestSeasonal — C++ Player::SatisfyQuestSeasonal
        // Per C++ CanTakeQuest order (Player.cpp:14093-14102): Day/Week/Month (above) and
        // Seasonal precede Conditions; the dependent cluster (prev_quest_id,
        // DependentPreviousQuests, DependentBreadcrumbQuests) runs before this, as part of
        // SatisfyQuestDependentQuests. SatisfyQuestTimed remains a separate gap:
        // the session has no active-timed-quest set yet (see #QUESTS.15).
        if quest.is_seasonal_like_cpp() && !recurrence.seasonal_quests.is_empty() {
            if let Some(bucket) = recurrence
                .seasonal_quests
                .get(&quest.event_id_for_quest_like_cpp())
            {
                if !bucket.is_empty() && bucket.contains_key(&quest.id) {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id,
                        event_id = quest.event_id_for_quest_like_cpp(),
                        "CanTakeQuest: seasonal quest cooldown"
                    );
                    return false;
                }
            }
        }

        // SatisfyQuestConditions — C++ Player.cpp:14102
        if !self.represented_quest_available_conditions_meet_like_cpp(quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: quest available conditions not met"
            );
            return false;
        }

        // SatisfyQuestExpansion — Player.cpp:15431-15443 (CanTakeQuest term Player.cpp:14102)
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

    pub(crate) fn is_quest_disabled_like_cpp(&self, quest_id: u32) -> bool {
        self.disable_mgr().is_some_and(|disable_mgr| {
            disable_mgr.is_disabled_for_like_cpp(DISABLE_TYPE_QUEST, quest_id, None, 0, None)
        })
    }
}
