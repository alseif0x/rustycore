// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest log slots and the accept/complete/remove status lifecycle.

use super::*;

impl WorldSession {
    fn complete_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(old_status) = self.complete_represented_quest_status_like_cpp(
            quest.id,
        ) else {
            return false;
        };
        self.record_represented_quest_complete_status_update_like_cpp(
            RepresentedQuestCompleteStatusUpdateLikeCpp {
                quest_id: quest.id,
                old_status,
                new_status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                send_quest_update_called: true,
                quest_slot_state_complete_represented: true,
                quest_slot_state_live_update_unrepresented: true,
                visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
                spell_area_runtime_unrepresented: true,
                tracking_event_auto_reward_unrepresented: (quest.flags
                    & QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP)
                    != 0,
                quest_tracker_complete_time_unrepresented: true,
                script_status_change_unrepresented: true,
            },
        );
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.sync_player_registry_state_like_cpp();
        true
    }

    #[cfg(test)]
    pub(crate) async fn complete_represented_quest_after_add_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.complete_represented_quest_after_add_with_generator_like_cpp(generator.as_ref(), quest)
            .await
    }

    pub(crate) async fn complete_represented_quest_after_add_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.complete_represented_quest_after_objective_with_generator_like_cpp(
            item_guid_generator,
            quest,
            0,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn complete_represented_quest_after_objective_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.complete_represented_quest_after_objective_with_generator_like_cpp(
            generator.as_ref(),
            quest,
            ignored_objective_id,
        )
        .await
    }

    pub(crate) async fn complete_represented_quest_after_objective_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    ) -> bool {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        if !state.can_complete_started_quest(
            quest.id,
            ignored_objective_id,
            || quest.objective_rules_like_cpp(),
        ) {
            return false;
        }

        if !self.complete_represented_quest_like_cpp(quest) {
            return false;
        }

        if (quest.flags & QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP) != 0 {
            let quest_giver_guid = self
                .player_guid()
                .unwrap_or(wow_core::ObjectGuid::new(0, 0));
            let choice = QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 0,
                quantity: 0,
            };
            let rewarded = self
                .reward_represented_quest_with_generator_like_cpp(
                    item_guid_generator,
                    quest,
                    quest_giver_guid,
                    choice,
                )
                .await;
            if rewarded {
                if let Some(evidence) = self
                    .quest_state
                    .represented_quest_complete_status_updates_like_cpp
                    .iter_mut()
                    .rev()
                    .find(|evidence| evidence.quest_id == quest.id)
                {
                    evidence.tracking_event_auto_reward_unrepresented = false;
                }
                Box::pin(
                    self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                        item_guid_generator,
                    ),
                )
                .await;
            }
        }

        true
    }

    pub(crate) fn remove_represented_active_rewarded_duplicates_like_cpp(&mut self) -> Vec<u32> {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return Vec::new();
        };
        let duplicate_quest_ids = state.plan_rewarded_active_duplicates(|quest_id| {
            self.quests.store.as_ref()
                .and_then(|store| store.get(quest_id))
                .map(|quest| quest.is_repeatable())
        });

        if !duplicate_quest_ids.is_empty() {
            self.invalidate_player_quest_status_authority_like_cpp();
        }

        if !duplicate_quest_ids.is_empty() {
            let duplicate_ids = duplicate_quest_ids.clone();
            let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
                state.remove_rewarded_active_duplicates(&duplicate_ids);
            });
        }

        duplicate_quest_ids
    }

    pub(crate) fn acknowledge_auto_accept_quest_like_cpp(&mut self, quest_id: u32) -> bool {
        // C++ order: FindQuestSlot(QuestID), then GetQuestTemplate(QuestID), then
        // ScriptMgr::OnQuestAcknowledgeAutoAccept(player, quest).
        if self.find_quest_slot_like_cpp(quest_id).is_none() {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCloseQuest: represented active quest log miss"
            );
            return false;
        }

        let Some(quest_store) = &self.quests.store else {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCloseQuest: missing represented quest store"
            );
            return false;
        };

        if quest_store.get(quest_id).is_none() {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCloseQuest: represented quest template miss"
            );
            return false;
        }

        #[cfg(test)]
        self.quest_test_fixture_like_cpp
            .represented_auto_accept_acknowledged_quests_like_cpp
            .push(quest_id);
        true
    }

    pub(super) async fn add_quest_confirm_accept_local_state_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(slot) = self.first_free_quest_slot_like_cpp() else {
            return false;
        };

        let (accept_time_secs, end_time_secs) =
            quest.accepted_and_end_time_like_cpp(wow_core::GameTime::now().as_secs() as i64);

        self.invalidate_player_quest_status_authority_like_cpp();
        let status = wow_entities::PlayerQuestGameplayState::prepare_quest_start(
            quest.id, slot, accept_time_secs, end_time_secs, quest.objectives.len(),
        );
        if self.insert_represented_quest_status_like_cpp(quest.id, status) == false {
            return false;
        }
        self.complete_represented_quest_after_add_with_generator_like_cpp(
            item_guid_generator,
            quest,
        )
        .await;
        self.save_represented_quest_status_like_cpp(quest.id).await;
        self.sync_player_registry_state_like_cpp();
        true
    }

    pub(super) fn remove_represented_timed_quest_like_cpp(&mut self, quest_id: u32) {
        let removed = self.clear_represented_quest_end_time_like_cpp(quest_id);
        if removed {
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.quest_test_fixture_like_cpp
                    .represented_timed_quest_removals_like_cpp
                    .push(quest_id);
            }
        }
    }

    pub(crate) fn first_free_quest_slot_like_cpp(&self) -> Option<u8> {
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .find(|&slot| !self.quest_slot_has_active_entry_like_cpp(slot))
    }

    fn quest_slot_has_active_entry_like_cpp(&self, slot: u8) -> bool {
        slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
            && self.player_quest_gameplay_snapshot_like_cpp()
                .is_some_and(|state| state.slot_has_active_entry(slot))
    }

    pub(crate) fn get_quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            return None;
        }

        self.player_quest_gameplay_snapshot_like_cpp()?.quest_id_at_slot(slot)
    }

    pub(crate) fn find_quest_slot_like_cpp(&self, quest_id: u32) -> Option<u8> {
        self.player_quest_gameplay_snapshot_like_cpp()?.slot_for_quest(quest_id)
    }

    pub(crate) fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return Vec::new();
        };
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .map(|slot| {
                let Some(quest_id) = self.get_quest_slot_quest_id_like_cpp(slot) else {
                    return (0, 0, 0, [0; 24]);
                };
                state.quest_log_entry(quest_id, |id| {
                    self.quests.store.as_ref()
                        .and_then(|store| store.get(id))
                        .map(|quest| quest.objective_rules_like_cpp())
                })
            })
            .collect()
    }

    pub(crate) fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8) {
        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            return;
        }
        let Some(guid) = self.player_guid() else {
            return;
        };

        let Some((quest_id, state_flags, end_time, objective_progress)) = self
            .quest_log_create_entries_like_cpp()
            .get(slot as usize)
            .copied()
        else {
            return;
        };

        let mut data = PlayerDataValuesDeltaUpdate::default();
        data.player_data_mask[35 / 32] |= 1 << (35 % 32);
        let slot_bit = 36 + usize::from(slot);
        data.player_data_mask[slot_bit / 32] |= 1 << (slot_bit % 32);
        data.quest_log[slot as usize] = QuestLogValuesUpdate {
            // C++ Player::SetQuestSlot marks QuestID, StateFlags, EndTime,
            // and every ObjectiveProgress field changed for the slot.
            quest_log_mask: 0x1FFF_FFFF,
            end_time,
            quest_id: quest_id.min(i32::MAX as u32) as i32,
            state_flags,
            objective_progress,
        };

        self.send_packet(&UpdateObject::full_player_values_update(
            guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }
}
