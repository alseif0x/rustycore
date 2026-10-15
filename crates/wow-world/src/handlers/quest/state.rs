// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest log slots and the accept/complete/remove status lifecycle.

use super::*;

impl WorldSession {
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

    pub(super) async fn add_quest_confirm_accept_local_state_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(slot) = wow_world_quest::first_free_quest_slot_like_cpp(self) else {
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
        if wow_world_quest::insert_represented_quest_status_like_cpp(self, quest.id, status)
            == false
        {
            return false;
        }
        self.complete_represented_quest_after_add_with_generator_like_cpp(
            item_guid_generator,
            quest,
        )
        .await;
        wow_world_quest::save_represented_quest_status_like_cpp(self, quest.id).await;
        self.sync_player_registry_state_like_cpp();
        true
    }

    pub(crate) fn find_quest_slot_like_cpp(&self, quest_id: u32) -> Option<u8> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_quest::find_quest_slot_like_cpp(&owner, &self.quest_state, quest_id, cfg!(test))
    }

    pub(crate) fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_quest::quest_log_create_entries_like_cpp(
            &owner,
            &self.quest_state,
            &self.catalogs,
            cfg!(test),
        )
    }

    pub(crate) fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8) {
        let owner = self.core.quest_objective_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        wow_world_quest::send_represented_quest_log_slot_update_like_cpp(
            &owner,
            &self.quest_state,
            &self.catalogs,
            &publication,
            slot,
            cfg!(test),
        );
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/quest/state/test_only.rs"]
mod test_only;
