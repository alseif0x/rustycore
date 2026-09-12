//! Quest giver menus, status and offer details seen from the Session.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn begin_player_quest_status_authority_load_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_status_authority_complete_like_cpp(false);
            state.clear_rewarded_quest_rows_like_cpp();
        });
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    pub(crate) fn invalidate_player_quest_status_authority_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_status_authority_complete_like_cpp(false);
        });
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    pub(crate) fn represented_player_quest_status_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<Option<u8>> {
        let state = self.player_quest_gameplay_snapshot_like_cpp()?;
        Some(
            state
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|status| status.status),
        )
    }
    pub(in crate::session) fn represented_spell_area_quest_status_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<u8> {
        let state = self.player_quest_gameplay_snapshot_like_cpp()?;
        if !state.status_authority_complete_like_cpp() {
            return None;
        }

        if state.rewarded_quest_ids_like_cpp().contains(&quest_id)
            && self.represented_quest_can_increase_rewarded_counters_like_cpp(quest_id)?
        {
            return Some(crate::conditions::QUEST_STATUS_REWARDED_LIKE_CPP);
        }

        Some(
            state
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|quest| quest.status)
                .unwrap_or(crate::conditions::QUEST_STATUS_NONE_LIKE_CPP),
        )
    }
    pub(crate) fn load_seasonal_quest_status_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = SeasonalQuestStatusDbRowLikeCpp>,
        quest_store: Option<&wow_data::quest::QuestStore>,
        quest_v2_store: Option<&QuestV2Store>,
    ) -> LoadSeasonalQuestStatusOutcomeLikeCpp {
        // C++ Player::_LoadSeasonalQuestStatus clears first and always resets
        // m_SeasonalQuestChanged at the end. Rust deliberately skips rows when
        // no quest store is available, because C++ requires a real QuestTemplate.
        let mut seasonal_quests = BTreeMap::<u16, BTreeMap<u32, u64>>::new();
        let mut outcome = LoadSeasonalQuestStatusOutcomeLikeCpp::default();
        for row in rows {
            outcome.rows_seen += 1;

            let Some(quest_store) = quest_store else {
                outcome.skipped_no_quest_store += 1;
                continue;
            };

            if quest_store.get(row.quest_id).is_none() {
                outcome.skipped_missing_quest += 1;
                continue;
            }

            let Ok(event_id) = u16::try_from(row.event_id) else {
                outcome.skipped_event_out_of_range += 1;
                continue;
            };

            let Ok(completed_time) = u64::try_from(row.completed_time) else {
                // Bounded Rust safety divergence: C++ reads int64 then assigns
                // to uint32, but Rust avoids turning negative DB data into a
                // huge cooldown timestamp.
                outcome.skipped_negative_completed_time += 1;
                continue;
            };

            if seasonal_quests
                .entry(event_id)
                .or_default()
                .insert(row.quest_id, completed_time)
                .is_some()
            {
                outcome.replaced += 1;
            } else {
                outcome.inserted += 1;
            }

            let Some(quest_v2_store) = quest_v2_store else {
                outcome.completed_bit_skipped_no_quest_v2_store += 1;
                continue;
            };

            let quest_bit = quest_v2_store.get_quest_unique_bit_flag_like_cpp(row.quest_id);
            if quest_bit == 0 {
                outcome.completed_bit_skipped_zero_unique_bit += 1;
                continue;
            }

            if self.set_loaded_quest_completed_bit_like_cpp(quest_bit) {
                outcome.completed_bit_set += 1;
            } else {
                outcome.completed_bit_no_change_or_noop += 1;
            }
        }

        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_seasonal_quests_like_cpp(seasonal_quests, false);
        });
        outcome.seasonal_quest_changed = false;
        outcome
    }
    pub(crate) fn reset_seasonal_quest_status_like_cpp(
        &mut self,
        event_id: u16,
        event_start_time: u64,
    ) -> ResetSeasonalQuestStatusOutcomeLikeCpp {
        // C++ Player::ResetSeasonalQuestStatus: DB data deleted in caller.
        if self
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.set_seasonal_quest_changed_like_cpp(false);
            })
            .is_none()
        {
            return ResetSeasonalQuestStatusOutcomeLikeCpp {
                event_id,
                event_start_time,
                reason: ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent,
                removed_quest_ids: Vec::new(),
                completed_bit_cleared: 0,
                completed_bit_skipped_no_quest_v2_store: 0,
                completed_bit_skipped_zero_unique_bit: 0,
                completed_bit_no_change_or_noop: 0,
                completed_bit_clear_unrepresented: 0,
                event_bucket_erased: false,
                seasonal_quest_changed: false,
            };
        }
        let Some(mut recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return ResetSeasonalQuestStatusOutcomeLikeCpp {
                event_id,
                event_start_time,
                reason: ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent,
                removed_quest_ids: Vec::new(),
                completed_bit_cleared: 0,
                completed_bit_skipped_no_quest_v2_store: 0,
                completed_bit_skipped_zero_unique_bit: 0,
                completed_bit_no_change_or_noop: 0,
                completed_bit_clear_unrepresented: 0,
                event_bucket_erased: false,
                seasonal_quest_changed: false,
            };
        };
        let Some(bucket) = recurrence.seasonal_event_quests_like_cpp(event_id) else {
            return ResetSeasonalQuestStatusOutcomeLikeCpp {
                event_id,
                event_start_time,
                reason: ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent,
                removed_quest_ids: Vec::new(),
                completed_bit_cleared: 0,
                completed_bit_skipped_no_quest_v2_store: 0,
                completed_bit_skipped_zero_unique_bit: 0,
                completed_bit_no_change_or_noop: 0,
                completed_bit_clear_unrepresented: 0,
                event_bucket_erased: false,
                seasonal_quest_changed: false,
            };
        };

        if bucket.is_empty() {
            return ResetSeasonalQuestStatusOutcomeLikeCpp {
                event_id,
                event_start_time,
                reason: ResetSeasonalQuestStatusReasonLikeCpp::EmptyEvent,
                removed_quest_ids: Vec::new(),
                completed_bit_cleared: 0,
                completed_bit_skipped_no_quest_v2_store: 0,
                completed_bit_skipped_zero_unique_bit: 0,
                completed_bit_no_change_or_noop: 0,
                completed_bit_clear_unrepresented: 0,
                event_bucket_erased: false,
                seasonal_quest_changed: false,
            };
        }

        let reset = recurrence.reset_seasonal_event_like_cpp(event_id, event_start_time);
        let removed_quest_ids = reset.removed_quest_ids;
        let event_bucket_erased = reset.event_bucket_erased;
        let seasonal_quests = recurrence.seasonal_quests_snapshot_like_cpp();
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_seasonal_quests_like_cpp(seasonal_quests, false);
        });

        let mut completed_bit_cleared = 0;
        let mut completed_bit_skipped_no_quest_v2_store = 0;
        let mut completed_bit_skipped_zero_unique_bit = 0;
        let mut completed_bit_no_change_or_noop = 0;
        for quest_id in &removed_quest_ids {
            let Some(quest_v2_store) = self.quests.v2_store.as_ref().map(Arc::clone) else {
                completed_bit_skipped_no_quest_v2_store += 1;
                continue;
            };

            let quest_bit = quest_v2_store.get_quest_unique_bit_flag_like_cpp(*quest_id);
            if quest_bit == 0 {
                completed_bit_skipped_zero_unique_bit += 1;
                continue;
            }

            if self.clear_loaded_quest_completed_bit_like_cpp(quest_bit) {
                completed_bit_cleared += 1;
            } else {
                completed_bit_no_change_or_noop += 1;
            }
        }

        ResetSeasonalQuestStatusOutcomeLikeCpp {
            event_id,
            event_start_time,
            reason: if removed_quest_ids.is_empty() {
                ResetSeasonalQuestStatusReasonLikeCpp::NoOlderCompletions
            } else {
                ResetSeasonalQuestStatusReasonLikeCpp::RemovedOlderCompletions
            },
            removed_quest_ids,
            completed_bit_cleared,
            completed_bit_skipped_no_quest_v2_store,
            completed_bit_skipped_zero_unique_bit,
            completed_bit_no_change_or_noop,
            completed_bit_clear_unrepresented: 0,
            event_bucket_erased,
            seasonal_quest_changed: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn seed_seasonal_quest_status_like_cpp(
        &mut self,
        event_id: u16,
        quest_id: u32,
        completed_time: u64,
    ) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.seed_seasonal_quest_like_cpp(event_id, quest_id, completed_time);
        });
    }
    pub(crate) fn represented_quest_giver_involved_source_allows_quest_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        if source_guid.is_any_type_creature() {
            let Some(access) = self.represented_npc_can_interact_with_like_cpp(
                source_guid,
                NPCFlags1::QUEST_GIVER.bits(),
                0,
            ) else {
                debug!(
                    account = self.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverCompleteReward: represented Creature source missing or not interactable"
                );
                return false;
            };

            return quest_store.creature_has_ender_relation_like_cpp(access.entry, quest_id);
        }

        if source_guid.is_game_object() {
            let Some(access) =
                self.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
            else {
                debug!(
                    account = self.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverCompleteReward: represented GameObject source missing or not interactable"
                );
                return false;
            };

            return quest_store.gameobject_has_ender_relation_like_cpp(access.entry, quest_id);
        }

        // Player and Item questgiver branches are not involved-quest sources in this represented slice.
        // Match the C++ early-return shape by failing closed with no packet and no mutation.
        debug!(
            account = self.account_id,
            ?source_guid,
            quest_id,
            "QuestGiverCompleteReward: unsupported represented source type"
        );
        false
    }
    pub(crate) fn represented_quest_giver_accept_source_allows_quest_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        if source_guid.is_any_type_creature() {
            let Some(access) = self.represented_npc_can_interact_with_like_cpp(
                source_guid,
                NPCFlags1::QUEST_GIVER.bits(),
                0,
            ) else {
                debug!(
                    account = self.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverAcceptQuest: represented Creature source missing or not interactable"
                );
                return false;
            };

            return quest_store.creature_has_starter_relation_like_cpp(access.entry, quest_id);
        }

        if source_guid.is_game_object() {
            let Some(access) =
                self.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
            else {
                debug!(
                    account = self.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverAcceptQuest: represented GameObject source missing or not interactable"
                );
                return false;
            };

            return quest_store.gameobject_has_starter_relation_like_cpp(access.entry, quest_id);
        }

        // Player quest sharing and Item questgiver branches are not represented in this slice.
        // Match the C++ early-return shape by failing closed with no packet and no mutation.
        debug!(
            account = self.account_id,
            ?source_guid,
            quest_id,
            "QuestGiverAcceptQuest: unsupported represented source type"
        );
        false
    }
    pub(crate) fn send_represented_quest_giver_query_quest_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest_id: u32,
    ) -> bool {
        let Some(quest_store) = self.quests.store.as_ref().map(Arc::clone) else {
            debug!(
                account = self.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverQueryQuest: missing quest store"
            );
            return false;
        };
        let Some(quest) = quest_store.get(quest_id).cloned() else {
            warn!(
                account = self.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverQueryQuest: unknown quest"
            );
            return false;
        };

        let menu_item = if source_guid.is_any_type_creature() {
            let Some(access) = self.represented_npc_can_interact_with_like_cpp(
                source_guid,
                NPCFlags1::QUEST_GIVER.bits(),
                0,
            ) else {
                debug!(
                    account = self.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverQueryQuest: represented Creature source missing or not interactable"
                );
                return false;
            };

            self.represented_quest_giver_query_creature_menu_item_like_cpp(
                &quest_store,
                access.entry,
                &quest,
            )
        } else if source_guid.is_game_object() {
            let Some(access) =
                self.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
            else {
                debug!(
                    account = self.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverQueryQuest: represented GameObject source missing or not interactable"
                );
                return false;
            };

            self.represented_quest_giver_query_gameobject_menu_item_like_cpp(
                &quest_store,
                access.entry,
                &quest,
            )
        } else {
            debug!(
                account = self.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverQueryQuest: unsupported represented source type"
            );
            return false;
        };

        let Some(menu_item) = menu_item else {
            debug!(
                account = self.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverQueryQuest: source has no represented starter/involved relation for quest"
            );
            return false;
        };

        self.send_represented_prepared_single_quest_like_cpp(source_guid, &menu_item, false);
        true
    }
    pub(in crate::session) fn represented_player_quest_status_is_none_like_cpp(
        &self,
        quest_id: u32,
    ) -> bool {
        self.represented_player_quest_status_like_cpp(quest_id) == Some(None)
    }
    pub(crate) fn send_represented_quest_giver_request_items_with_completion_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest: &wow_data::quest::QuestTemplate,
        can_complete: bool,
        auto_launched: bool,
    ) {
        let collect = quest
            .objectives
            .iter()
            .filter(|objective| objective.obj_type == QUEST_OBJECTIVE_ITEM_LIKE_CPP)
            .map(|objective| QuestGiverRequestItemsCollect {
                object_id: objective.object_id,
                amount: objective.amount,
                flags: objective.flags,
            })
            .collect::<Vec<_>>();
        let currency = quest
            .objectives
            .iter()
            .filter(|objective| objective.obj_type == QUEST_OBJECTIVE_CURRENCY_LIKE_CPP)
            .map(|objective| QuestGiverRequestItemsCurrency {
                currency_id: objective.object_id,
                amount: objective.amount,
            })
            .collect::<Vec<_>>();
        let money_to_get = quest
            .objectives
            .iter()
            .filter(|objective| objective.obj_type == QUEST_OBJECTIVE_MONEY_LIKE_CPP)
            .map(|objective| objective.amount)
            .sum::<i32>();
        self.send_packet(&QuestGiverRequestItems {
            giver_guid: source_guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(source_guid),
            quest_id: quest.id,
            comp_emote_delay: 0,
            comp_emote_type: 0,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            money_to_get,
            collect,
            currency,
            status_flags: if can_complete { 0xFF } else { 0xFD },
            title: quest.log_title.clone(),
            completion_text: quest.area_description.clone(),
            auto_launched,
        });
    }
    pub(crate) fn send_represented_quest_giver_quest_details_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest: &wow_data::quest::QuestTemplate,
        auto_launched: bool,
    ) {
        let objectives: Vec<QuestObjectiveSimple> = quest
            .objectives
            .iter()
            .map(|obj| QuestObjectiveSimple {
                id: obj.id,
                object_id: obj.object_id,
                amount: obj.amount,
                obj_type: obj.obj_type,
            })
            .collect();

        self.send_packet(&QuestGiverQuestDetails {
            giver_guid: source_guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(source_guid),
            quest_id: quest.id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            objectives,
            rewards: quest_rewards_block_like_cpp(quest),
            title: quest.log_title.clone(),
            description: quest.quest_description.clone(),
            log_description: quest.log_description.clone(),
            auto_launched,
        });
    }
}
