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
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
        ) else {
            return false;
        };
        self.quest_state
            .record_represented_quest_complete_status_update_like_cpp(
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
        let Some(status) = state.statuses_like_cpp().get(&quest.id) else {
            return false;
        };
        let quest_already_rewarded = state.rewarded_quest_ids_like_cpp().contains(&quest.id);
        if !wow_entities::represented_can_complete_quest_after_objective_like_cpp(
            status,
            &quest.objective_rules_like_cpp(),
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
                .reward_represented_quest_with_generator_like_cpp(
                    item_guid_generator,
                    quest,
                    quest_giver_guid,
                    choice,
                )
                .await;
            if rewarded {
                self.quest_state
                    .mark_latest_tracking_event_auto_reward_like_cpp(quest.id);
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
        let mut duplicate_quest_ids = state
            .statuses_like_cpp()
            .keys()
            .filter(|quest_id| {
                let store = self.catalogs.quests.store.as_ref();
                state.rewarded_quest_ids_like_cpp().contains(quest_id)
                    && store
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

        if !duplicate_quest_ids.is_empty() {
            let duplicate_ids = duplicate_quest_ids.clone();
            let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
                for quest_id in &duplicate_ids {
                    state.remove_status_like_cpp(*quest_id);
                }
                let mut remaining_slots = state
                    .statuses_like_cpp()
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.slot))
                    .collect::<Vec<_>>();
                remaining_slots.sort_by_key(|(_, slot)| *slot);
                for (slot, (quest_id, _)) in remaining_slots.into_iter().enumerate() {
                    if let Some(status) = state.status_mut_like_cpp(quest_id) {
                        status.slot = u8::try_from(slot)
                            .unwrap_or(MAX_QUEST_LOG_SIZE_LIKE_CPP.saturating_sub(1));
                    }
                }
            });
        }

        duplicate_quest_ids
    }

    pub(crate) fn acknowledge_auto_accept_quest_like_cpp(&mut self, quest_id: u32) -> bool {
        // C++ order: FindQuestSlot(QuestID), then GetQuestTemplate(QuestID), then
        // ScriptMgr::OnQuestAcknowledgeAutoAccept(player, quest).
        if self.find_quest_slot_like_cpp(quest_id).is_none() {
            debug!(
                account = self.core.account_id,
                quest_id, "QuestGiverCloseQuest: represented active quest log miss"
            );
            return false;
        }

        let Some(quest_store) = &self.catalogs.quests.store else {
            debug!(
                account = self.core.account_id,
                quest_id, "QuestGiverCloseQuest: missing represented quest store"
            );
            return false;
        };

        if quest_store.get(quest_id).is_none() {
            debug!(
                account = self.core.account_id,
                quest_id, "QuestGiverCloseQuest: represented quest template miss"
            );
            return false;
        }

        #[cfg(test)]
        self.quest_state
            .fixture_record_auto_accept_acknowledged_quest_like_cpp(quest_id);
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
        let status = PlayerQuestStatus {
            quest_id: quest.id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs,
            end_time_secs,
            objective_counts: vec![0; quest.objectives.len()],
            slot,
        };
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
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut player = self.core.quest_reward_player_access_like_cpp();
        wow_world_application::QuestRewardCx::remove_represented_timed_quest_like_cpp(
            &mut self.quest_state,
            &mut player,
            quest_id,
            cfg!(test),
        );
    }

    pub(crate) fn first_free_quest_slot_like_cpp(&self) -> Option<u8> {
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .find(|&slot| !self.quest_slot_has_active_entry_like_cpp(slot))
    }

    fn quest_slot_has_active_entry_like_cpp(&self, slot: u8) -> bool {
        // C++ `QuestSlotOffset` stores the quest id independently from the status fields;
        // represented active slots are INCOMPLETE, COMPLETE, or FAILED.
        slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
            && self
                .player_quest_gameplay_snapshot_like_cpp()
                .is_some_and(|state| {
                    state.statuses_like_cpp().values().any(|status| {
                        status.slot == slot
                            && matches!(
                                status.status,
                                QUEST_STATUS_INCOMPLETE_LIKE_CPP
                                    | QUEST_STATUS_COMPLETE_LIKE_CPP
                                    | QUEST_STATUS_FAILED_LIKE_CPP
                            )
                    })
                })
    }

    pub(crate) fn get_quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_application::get_quest_slot_quest_id_like_cpp(
            &owner, &self.quest_state, slot, cfg!(test),
        )
    }

    pub(crate) fn find_quest_slot_like_cpp(&self, quest_id: u32) -> Option<u8> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_application::find_quest_slot_like_cpp(
            &owner,
            &self.quest_state,
            quest_id,
            cfg!(test),
        )
    }

    pub(crate) fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_application::quest_log_create_entries_like_cpp(
            &owner, &self.quest_state, &self.catalogs, cfg!(test),
        )
    }

    pub(crate) fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8) {
        let owner = self.core.quest_objective_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        wow_world_application::send_represented_quest_log_slot_update_like_cpp(
            &owner, &self.quest_state, &self.catalogs, &publication, slot, cfg!(test),
        );
    }
}
