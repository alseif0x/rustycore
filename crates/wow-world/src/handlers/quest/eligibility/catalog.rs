//! catalog operations at the existing Quest application boundary.

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
    /// GO starter/ender relations. AI status, ConditionMgr, events and journey remain gaps;
    /// important/covenant presentation uses the optional QuestInfo catalog below.
    pub(crate) fn get_represented_quest_giver_status_like_cpp(
        &self,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        self.get_represented_quest_giver_status_with_catalog_like_cpp(
            self.quests.info_store.as_deref(),
            source,
        )
    }

    pub(crate) fn get_represented_quest_giver_status_with_catalog_like_cpp(
        &self,
        quest_info: Option<&wow_data::progression_rewards::QuestInfoStore>,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        let Some(store) = &self.quests.store else {
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
                    result |= WorldSession::represented_quest_dialog_classification_like_cpp(
                        quest, quest_info,
                    )
                    .reward_complete();
                }
                QUEST_STATUS_INCOMPLETE_LIKE_CPP => {
                    result |= WorldSession::represented_quest_dialog_classification_like_cpp(
                        quest, quest_info,
                    )
                    .reward();
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
                result |= WorldSession::represented_quest_dialog_classification_like_cpp(
                    quest, quest_info,
                )
                .available(self.represented_quest_is_trivial_like_cpp(quest));
            } else {
                result |= WorldSession::represented_quest_dialog_classification_like_cpp(
                    quest, quest_info,
                )
                .future();
            }
        }

        result
    }

    pub(super) fn represented_quest_available_conditions_meet_like_cpp(&self, quest_id: u32) -> bool {
        let condition_store = if let Some(store) = self.condition_store() {
            Arc::clone(store)
        } else if let Some(store) = wow_conditions::condition_mgr_store_like_cpp() {
            store
        } else {
            return true;
        };

        if !wow_conditions::has_conditions_for_not_grouped_entry_like_cpp(
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
            .statuses_like_cpp()
            .iter()
            .map(
                |(&quest_id, status)| wow_conditions::ConditionQuestStatusSnapshot {
                    quest_id,
                    status: status.status,
                },
            )
            .collect();
        let store = self.quests.store.as_ref();
        let quest_objective_progress: Vec<_> = store
            .map(|store| {
                recurrence
                    .statuses_like_cpp()
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
                                Some(wow_conditions::ConditionQuestObjectiveProgressSnapshot {
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
        let rewarded_quest_ids: Vec<_> = recurrence
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        let daily_quest_ids: Vec<_> = recurrence
            .daily_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        let quest_snapshot = wow_conditions::ConditionPlayerQuestSnapshot {
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
            wow_conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
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

        wow_conditions::is_object_meeting_not_grouped_conditions_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
            &mut source_info,
            |condition, source_info| {
                wow_conditions::condition_meets_basic_like_cpp(
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
}
