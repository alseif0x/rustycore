// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest log slots and the accept/complete/remove status lifecycle.

use super::*;

impl WorldSession {
    pub(super) fn bind_player_quest_status_load_guid_like_cpp(
        stmt: &mut PreparedStatement,
        player_guid: ObjectGuid,
    ) {
        stmt.set_u64(0, player_guid.counter() as u64);
    }

    pub(super) fn represented_accept_and_end_time_for_new_quest_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
    ) -> (i64, i64) {
        let accept_time = GameTime::now().as_secs() as i64;
        let end_time = if quest.limit_time_secs > 0 {
            accept_time.saturating_add(quest.limit_time_secs)
        } else {
            0
        };
        (accept_time, end_time)
    }

    fn complete_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.invalidate_player_quest_status_authority_like_cpp();
        let old_status = {
            let Some(status) = self.player_quests.get_mut(&quest.id) else {
                return false;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }

            let old_status = status.status;
            status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            old_status
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

    pub(crate) async fn complete_represented_quest_after_add_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.complete_represented_quest_after_objective_if_ready_like_cpp(quest, 0)
            .await
    }

    pub(crate) async fn complete_represented_quest_after_objective_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    ) -> bool {
        let Some(status) = self.player_quests.get(&quest.id) else {
            return false;
        };
        let quest_already_rewarded = self.rewarded_quests.contains(&quest.id);
        if !Self::represented_can_complete_quest_after_objective_like_cpp(
            status,
            quest,
            ignored_objective_id,
            quest_already_rewarded,
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
                .reward_represented_quest_like_cpp(quest, quest_giver_guid, choice)
                .await;
            if rewarded {
                if let Some(evidence) = self
                    .represented_quest_complete_status_updates_like_cpp
                    .iter_mut()
                    .rev()
                    .find(|evidence| evidence.quest_id == quest.id)
                {
                    evidence.tracking_event_auto_reward_unrepresented = false;
                }
                Box::pin(self.drain_represented_quest_objective_progress_like_cpp()).await;
            }
        }

        true
    }

    pub(crate) fn remove_represented_active_rewarded_duplicates_like_cpp(&mut self) -> Vec<u32> {
        let mut duplicate_quest_ids = self
            .player_quests
            .keys()
            .filter(|quest_id| {
                self.rewarded_quests.contains(quest_id)
                    && self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(**quest_id))
                        .is_some_and(|quest| !quest.is_repeatable())
            })
            .copied()
            .collect::<Vec<_>>();
        duplicate_quest_ids.sort_unstable();
        duplicate_quest_ids.dedup();

        if !duplicate_quest_ids.is_empty() {
            self.invalidate_player_quest_status_authority_like_cpp();
        }

        for quest_id in &duplicate_quest_ids {
            self.player_quests.remove(quest_id);
        }

        if !duplicate_quest_ids.is_empty() {
            let mut remaining_slots = self
                .player_quests
                .iter()
                .map(|(quest_id, status)| (*quest_id, status.slot))
                .collect::<Vec<_>>();
            remaining_slots.sort_by_key(|(_, slot)| *slot);
            for (slot, (quest_id, _)) in remaining_slots.into_iter().enumerate() {
                if let Some(status) = self.player_quests.get_mut(&quest_id) {
                    status.slot =
                        u8::try_from(slot).unwrap_or(MAX_QUEST_LOG_SIZE_LIKE_CPP.saturating_sub(1));
                }
            }
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

        let Some(quest_store) = &self.quest_store else {
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

        self.represented_auto_accept_acknowledged_quests_like_cpp
            .push(quest_id);
        true
    }

    pub(super) async fn add_quest_confirm_accept_local_state_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(slot) = self.first_free_quest_slot_like_cpp() else {
            return false;
        };

        let (accept_time_secs, end_time_secs) =
            Self::represented_accept_and_end_time_for_new_quest_like_cpp(quest);

        self.invalidate_player_quest_status_authority_like_cpp();
        self.player_quests.insert(
            quest.id,
            PlayerQuestStatus {
                quest_id: quest.id,
                status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs,
                end_time_secs,
                objective_counts: vec![0; quest.objectives.len()],
                slot,
            },
        );
        self.complete_represented_quest_after_add_if_ready_like_cpp(quest)
            .await;
        self.save_represented_quest_status_like_cpp(quest.id).await;
        self.sync_player_registry_state_like_cpp();
        true
    }

    pub(super) fn remove_represented_timed_quest_like_cpp(&mut self, quest_id: u32) {
        if let Some(status) = self.player_quests.get_mut(&quest_id)
            && status.end_time_secs > 0
        {
            status.end_time_secs = 0;
            self.represented_timed_quest_removals_like_cpp
                .push(quest_id);
        }
    }

    pub(crate) fn first_free_quest_slot_like_cpp(&self) -> Option<u8> {
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .find(|&slot| !self.quest_slot_has_active_entry_like_cpp(slot))
    }

    fn quest_slot_has_active_entry_like_cpp(&self, slot: u8) -> bool {
        // C++ `QuestSlotOffset` stores the quest id independently from the status fields;
        // represented active slots are INCOMPLETE, COMPLETE, or FAILED.
        slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
            && self.player_quests.values().any(|status| {
                status.slot == slot
                    && matches!(
                        status.status,
                        QUEST_STATUS_INCOMPLETE_LIKE_CPP
                            | QUEST_STATUS_COMPLETE_LIKE_CPP
                            | QUEST_STATUS_FAILED_LIKE_CPP
                    )
            })
    }

    pub(crate) fn get_quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            return None;
        }

        let mut matching_quest_id = None;
        for status in self.player_quests.values().filter(|status| {
            status.slot == slot
                && matches!(
                    status.status,
                    QUEST_STATUS_INCOMPLETE_LIKE_CPP
                        | QUEST_STATUS_COMPLETE_LIKE_CPP
                        | QUEST_STATUS_FAILED_LIKE_CPP
                )
        }) {
            if matching_quest_id.is_some() {
                return None;
            }

            matching_quest_id = Some(status.quest_id);
        }

        matching_quest_id
    }

    pub(crate) fn find_quest_slot_like_cpp(&self, quest_id: u32) -> Option<u8> {
        self.player_quests.get(&quest_id).and_then(|status| {
            (status.slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
                && matches!(
                    status.status,
                    QUEST_STATUS_INCOMPLETE_LIKE_CPP
                        | QUEST_STATUS_COMPLETE_LIKE_CPP
                        | QUEST_STATUS_FAILED_LIKE_CPP
                ))
            .then_some(status.slot)
        })
    }

    pub(crate) fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .map(|slot| {
                let Some(quest_id) = self.get_quest_slot_quest_id_like_cpp(slot) else {
                    return (0, 0, 0, [0; 24]);
                };
                let Some(qs) = self.player_quests.get(&quest_id) else {
                    return (0, 0, 0, [0; 24]);
                };

                let quest = self
                    .quest_store
                    .as_ref()
                    .and_then(|store| store.get(qs.quest_id));
                let mut state_flags: u32 = match qs.status {
                    QUEST_STATUS_COMPLETE_LIKE_CPP => QUEST_STATE_COMPLETE_LIKE_CPP,
                    QUEST_STATUS_FAILED_LIKE_CPP => QUEST_STATE_FAIL_LIKE_CPP,
                    _ => 0,
                };
                let mut obj_progress = [0u16; 24];
                for (i, slot_progress) in obj_progress.iter_mut().enumerate() {
                    let count = qs.objective_counts.get(i).copied().unwrap_or(0);
                    let stores_flag = quest.is_some_and(|quest| {
                        quest.objectives.iter().any(|objective| {
                            objective.storage_index == i as i8
                                && objective.is_storing_flag_like_cpp()
                        })
                    });
                    if stores_flag {
                        if count != 0 {
                            state_flags |= QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP << i;
                        }
                        continue;
                    }
                    *slot_progress = count.min(u16::MAX as i32) as u16;
                }
                (qs.quest_id, state_flags, qs.end_time_secs, obj_progress)
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
