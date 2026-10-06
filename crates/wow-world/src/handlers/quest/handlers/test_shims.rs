// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the quest query handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::QuestQueryHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_request_world_quest_update(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle)
            .handle_request_world_quest_update(pkt)
            .await;
    }

    pub async fn handle_query_quest_info(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle)
            .handle_query_quest_info(pkt)
            .await;
    }

    pub async fn handle_quest_log_remove_quest(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let slot = {
            let (quest_state, lifecycle, hub) =
                crate::session::split_quest_state_lifecycle_mut(self);
            QuestQueryHandlerCxLikeCpp::new(hub, quest_store.clone(), quest_state, lifecycle)
                .handle_quest_log_remove_quest(pkt)
                .await
        };
        if let Some(slot) = slot {
            self.sync_player_registry_state_like_cpp();
            let (quest_state, lifecycle, hub) =
                crate::session::split_quest_state_lifecycle_mut(self);
            QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle)
                .finish_quest_log_remove_quest(slot);
        }
    }

    pub async fn handle_quest_giver_close_quest(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle)
            .handle_quest_giver_close_quest(pkt);
    }

    pub async fn handle_query_quest_completion_npcs(
        &mut self,
        query: wow_packet::packets::query::QueryQuestCompletionNpcs,
    ) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle)
            .handle_query_quest_completion_npcs(query)
            .await;
    }
}
