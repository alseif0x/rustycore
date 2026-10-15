// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the quest state, persistence, dialog and
//! publication bodies moved to `wow_world_quest::quest_session` (#1363 slice
//! 6a): one-call method forwards to the moved free functions the scenarios
//! call, with this session as the host, so no call site changes; and the two
//! `cfg(test)` quest readers nested in the files they left
//! (`session/quest/giver.rs` and `handlers/quest/persistence.rs`), moved
//! verbatim.

#![cfg(test)]

use wow_core::ObjectGuid;
use wow_data::progression_rewards::QuestV2Store;
use wow_entities::PlayerQuestStatusRecord as PlayerQuestStatus;
use wow_world_quest::{
    ItemTransferQuestPersistencePlanLikeCpp, LoadSeasonalQuestStatusOutcomeLikeCpp,
    QuestSourceItemBoundPersistencePlanLikeCpp, ResetSeasonalQuestStatusOutcomeLikeCpp,
    SeasonalQuestStatusDbRowLikeCpp,
};

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn begin_player_quest_status_authority_load_like_cpp(&mut self) {
        wow_world_quest::begin_player_quest_status_authority_load_like_cpp(self)
    }
    pub(crate) fn load_seasonal_quest_status_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = SeasonalQuestStatusDbRowLikeCpp>,
        quest_store: Option<&wow_data::quest::QuestStore>,
        quest_v2_store: Option<&QuestV2Store>,
    ) -> LoadSeasonalQuestStatusOutcomeLikeCpp {
        wow_world_quest::load_seasonal_quest_status_like_cpp(
            self,
            rows,
            quest_store,
            quest_v2_store,
        )
    }
    pub(crate) fn reset_seasonal_quest_status_like_cpp(
        &mut self,
        event_id: u16,
        event_start_time: u64,
    ) -> ResetSeasonalQuestStatusOutcomeLikeCpp {
        wow_world_quest::reset_seasonal_quest_status_like_cpp(self, event_id, event_start_time)
    }
    pub(crate) fn send_represented_quest_giver_query_quest_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest_id: u32,
    ) -> bool {
        wow_world_quest::send_represented_quest_giver_query_quest_like_cpp(
            self,
            source_guid,
            quest_id,
        )
    }
    pub(crate) fn use_represented_creature_questgiver_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        creature_entry: u32,
    ) -> bool {
        wow_world_quest::use_represented_creature_questgiver_like_cpp(
            self,
            creature_guid,
            creature_entry,
        )
    }
    pub(crate) fn use_represented_gameobject_questgiver_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::QuestgiverUseSource,
    ) -> bool {
        wow_world_quest::use_represented_gameobject_questgiver_like_cpp(
            self,
            gameobject_guid,
            player_guid,
            gameobject_entry,
            source,
        )
    }
    pub(crate) fn set_loaded_quest_completed_bit_like_cpp(&mut self, quest_bit: u32) -> bool {
        wow_world_quest::set_loaded_quest_completed_bit_like_cpp(self, quest_bit)
    }
    pub(crate) fn clear_loaded_quest_completed_bit_like_cpp(&mut self, quest_bit: u32) -> bool {
        wow_world_quest::clear_loaded_quest_completed_bit_like_cpp(self, quest_bit)
    }
    pub(crate) fn can_complete_repeatable_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        wow_world_quest::can_complete_repeatable_quest_represented_bounded_like_cpp(self, quest)
    }
    pub(crate) fn remove_represented_active_rewarded_duplicates_like_cpp(&mut self) -> Vec<u32> {
        wow_world_quest::remove_represented_active_rewarded_duplicates_like_cpp(self)
    }
    pub(crate) fn first_free_quest_slot_like_cpp(&self) -> Option<u8> {
        wow_world_quest::first_free_quest_slot_like_cpp(self)
    }
    pub(crate) async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        wow_world_quest::save_quest_to_db(self, quest_id, status).await
    }
    pub(crate) async fn load_player_quests(&mut self) {
        wow_world_quest::load_player_quests(self).await
    }
    pub(crate) fn plan_bank_item_quest_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        moving_to_bank: bool,
        post_move_non_bank_count: u32,
        added_count: u32,
    ) -> Vec<PlayerQuestStatus> {
        wow_world_quest::plan_bank_item_quest_persistence_like_cpp(
            self,
            entry_id,
            quest_log_item_id,
            moving_to_bank,
            post_move_non_bank_count,
            added_count,
        )
    }
    pub(crate) fn begin_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
    ) -> ItemTransferQuestPersistencePlanLikeCpp {
        wow_world_quest::begin_item_transfer_quest_persistence_like_cpp(
            self,
            removed_entries_in_order,
            post_removal_non_bank_counts,
        )
    }
    pub(crate) fn plan_item_transfer_withdrawal_quest_persistence_like_cpp(
        &self,
        plan: &mut ItemTransferQuestPersistencePlanLikeCpp,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> bool {
        wow_world_quest::plan_item_transfer_withdrawal_quest_persistence_like_cpp(
            self,
            plan,
            entry_id,
            quest_log_item_id,
            count,
        )
    }
    pub(crate) fn plan_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
        added_items_in_order: &[(u32, u32, u32)],
    ) -> Vec<PlayerQuestStatus> {
        wow_world_quest::plan_item_transfer_quest_persistence_like_cpp(
            self,
            removed_entries_in_order,
            post_removal_non_bank_counts,
            added_items_in_order,
        )
    }
    pub(crate) fn finish_item_transfer_quest_persistence_like_cpp(
        &self,
        plan: ItemTransferQuestPersistencePlanLikeCpp,
    ) -> Vec<PlayerQuestStatus> {
        wow_world_quest::finish_item_transfer_quest_persistence_like_cpp(self, plan)
    }
    pub(crate) fn plan_quest_source_item_bound_objective_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPersistencePlanLikeCpp> {
        wow_world_quest::plan_quest_source_item_bound_objective_persistence_like_cpp(
            self,
            entry_id,
            quest_log_item_id,
            count,
        )
    }
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
    pub(crate) fn represented_quest_statuses_for_save_like_cpp(&self) -> Vec<(u32, u8)> {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return Vec::new();
        };
        let mut quests = state
            .statuses_like_cpp()
            .iter()
            .filter_map(|(quest_id, status)| {
                let store = self.catalogs.quests.store.as_ref();
                if state.rewarded_quest_ids_like_cpp().contains(quest_id)
                    && store
                        .and_then(|store| store.get(*quest_id))
                        .is_some_and(|quest| !quest.is_repeatable())
                {
                    return None;
                }

                Some((*quest_id, status.status))
            })
            .collect::<Vec<_>>();
        quests.sort_by_key(|(quest_id, _)| *quest_id);
        quests
    }
}
