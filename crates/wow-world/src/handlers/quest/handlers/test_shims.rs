// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the quest query handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
use wow_world_application::QuestQueryHandlerCxLikeCpp;

use crate::session::WorldSession;

async fn dispatch_registered_like_cpp(
    session: &mut WorldSession,
    opcode: ClientOpcodes,
    pkt: WorldPacket,
) {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == opcode)
        .expect("registered quest handler");
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

impl WorldSession {
    pub async fn handle_request_world_quest_update(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
            .handle_request_world_quest_update(pkt)
            .await;
    }

    pub async fn handle_query_quest_info(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
            .handle_query_quest_info(pkt)
            .await;
    }

    /// Dispatches through the registered production thunk so the two-phase
    /// owner/host orchestration (pending-share clear, registry sync, evidence)
    /// is exercised by every caller instead of re-implemented here.
    pub async fn handle_quest_push_result(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::QuestPushResult, pkt).await;
    }

    /// Dispatches through the registered production thunk so the two-phase
    /// owner/host orchestration (status removal, DB delete, registry sync,
    /// slot update) is exercised by every caller instead of re-implemented here.
    pub async fn handle_quest_log_remove_quest(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::QuestLogRemoveQuest, pkt).await;
    }

    pub async fn handle_quest_giver_close_quest(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
            .handle_quest_giver_close_quest(pkt);
    }

    pub async fn handle_query_quest_completion_npcs(
        &mut self,
        query: wow_packet::packets::query::QueryQuestCompletionNpcs,
    ) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
            .handle_query_quest_completion_npcs(query)
            .await;
    }

    /// Delegates to the moved owner body with this session as its host; the
    /// shell-only capabilities it needs stay on the host implementation.
    pub async fn handle_quest_giver_complete_quest(&mut self, mut pkt: WorldPacket) {
        wow_world_application::handle_quest_giver_complete_quest_like_cpp(self, pkt).await;
    }
}
