// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the quest packet handlers moved to
//! `wow-world-quest` (#1263 F5, #1363 A6 and slice 6c): the former
//! `handlers/quest/handlers/test_shims.rs`, moved verbatim; the six `cfg(test)`
//! wrappers that were nested in the production impls of
//! `handlers/quest/handlers{.rs,/queries.rs,/acceptance.rs,/reward_flow.rs}`,
//! moved verbatim except that their one call names the moved free function;
//! the two pending-sharing readers of `session/quest/state.rs`, whose only
//! production caller (the confirm-accept handler) moved; and one-call method
//! forwards for the moved functions the scenarios call, with this session as
//! the host, so no call site changes. Mounted from `handlers/mod.rs` outside
//! the session and quest hotspot trees.

#![cfg(test)]

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
use wow_world_quest::QuestQueryHandlerCxLikeCpp;
use wow_world_quest::RepresentedPendingQuestSharingLikeCpp;

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
    pub(crate) async fn handle_request_world_quest_update(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
            .handle_request_world_quest_update(pkt)
            .await;
    }

    /// Dispatches through the registered production thunk so the two-phase
    /// owner/host orchestration (pending-share clear, registry sync, evidence)
    /// is exercised by every caller instead of re-implemented here.
    pub(crate) async fn handle_quest_push_result(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::QuestPushResult, pkt).await;
    }

    /// Dispatches through the registered production thunk so the two-phase
    /// owner/host orchestration (status removal, DB delete, registry sync,
    /// slot update) is exercised by every caller instead of re-implemented here.
    pub(crate) async fn handle_quest_log_remove_quest(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::QuestLogRemoveQuest, pkt).await;
    }

    pub(crate) async fn handle_quest_giver_close_quest(&mut self, pkt: WorldPacket) {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
            .handle_quest_giver_close_quest(pkt);
    }

    /// Delegates to the moved owner body with this session as its host; the
    /// shell-only capabilities it needs stay on the host implementation.
    pub(crate) async fn handle_quest_giver_complete_quest(&mut self, mut pkt: WorldPacket) {
        wow_world_quest::handle_quest_giver_complete_quest_like_cpp(self, pkt).await;
    }

    pub(crate) async fn handle_adventure_map_start_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self.adventure_map_poi_store().cloned().unwrap_or_else(|| {
            std::sync::Arc::new(wow_data::AdventureMapPoiStore::from_entries([]))
        });
        wow_world_quest::handle_adventure_map_start_quest_with_catalog_like_cpp(
            self,
            store.as_ref(),
            pkt,
        )
        .await;
    }

    pub(crate) async fn handle_quest_giver_status_query(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        wow_world_quest::handle_quest_giver_status_query_with_catalog_like_cpp(
            self,
            catalogs.quest_info.as_ref(),
            pkt,
        )
        .await;
    }

    /// CMSG_QUEST_GIVER_ACCEPT_QUEST — player clicks "Accept" in the quest details dialog.
    /// Saves quest to characters DB and confirms to the client.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverAcceptQuest
    pub(crate) async fn handle_quest_giver_accept_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::handle_quest_giver_accept_quest_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    /// CMSG_QUEST_CONFIRM_ACCEPT — confirm accepting a shared quest.
    ///
    /// C++ anchor: `WorldSession::HandleQuestConfirmAccept`, `QuestHandler.cpp:499-531`.
    /// Represented-partial: validates against session-local pending sharing state, clears before
    /// quest-template lookup like C++, then records safe represented post-template gates.
    /// No-source-item quests and source-item no-grant branches consume only local quest-log insertion
    /// + Character DB status save + PlayerRegistry snapshot sync from `Player::AddQuest`. Real
    /// `StoreNewItem`/`SendNewItem`, criteria/completion, timed/PvP, scripts, and `SendQuestUpdate`
    /// packet fanout remain explicit no-mutation boundaries.
    pub(crate) async fn handle_quest_confirm_accept(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::handle_quest_confirm_accept_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    /// CMSG_QUEST_GIVER_REQUEST_REWARD — player talks to NPC to turn in a completed quest.
    /// Legacy non-canonical note: QuestHandler.HandleQuestgiverRequestReward
    /// Sent when player right-clicks a quest-ender NPC and has the quest in Complete status.
    /// Server responds with SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE (reward selection dialog).
    pub(crate) async fn handle_quest_giver_request_reward(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::handle_quest_giver_request_reward_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    /// CMSG_QUEST_GIVER_CHOOSE_REWARD — player clicks "Complete Quest" in reward dialog.
    /// Gives XP, gold, items. Removes quest from active log.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverChooseReward
    pub(crate) async fn handle_quest_giver_choose_reward(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::handle_quest_giver_choose_reward_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    pub(crate) fn clear_represented_pending_quest_sharing_like_cpp(&mut self) {
        let (state, hub) = crate::session::split_quest_state_mut(self);
        wow_world_quest::clear_represented_pending_quest_sharing_like_cpp(hub, state);
        self.sync_player_registry_state_like_cpp();
    }

    pub(crate) fn represented_pending_quest_sharing_like_cpp(
        &self,
    ) -> Option<RepresentedPendingQuestSharingLikeCpp> {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        wow_world_quest::represented_pending_quest_sharing_like_cpp(hub, state)
    }

    pub(crate) async fn handle_quest_poi_query(
        &mut self,
        query: wow_packet::packets::query::QuestPoiQuery,
    ) {
        wow_world_quest::handle_quest_poi_query(self, query).await
    }

    pub(crate) async fn handle_push_quest_to_party(&mut self, pkt: WorldPacket) {
        wow_world_quest::handle_push_quest_to_party(self, pkt).await
    }

    pub(crate) fn set_represented_pending_quest_sharing_like_cpp(
        &mut self,
        sender_guid: wow_core::ObjectGuid,
        quest_id: u32,
    ) {
        wow_world_quest::set_represented_pending_quest_sharing_like_cpp(self, sender_guid, quest_id)
    }

    pub(crate) fn read_quest_choice_item_like_cpp(
        pkt: &mut WorldPacket,
    ) -> Result<wow_world_quest::QuestChoiceItemLikeCpp, wow_packet::PacketError> {
        wow_world_quest::read_quest_choice_item_like_cpp(pkt)
    }

    pub(crate) fn represented_reward_choice_matches_loaded_type_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
        choice: wow_world_quest::QuestChoiceItemLikeCpp,
    ) -> bool {
        wow_world_quest::represented_reward_choice_matches_loaded_type_like_cpp(quest, choice)
    }
}

/// The moved quest constants, packet readers and POI store builder the quest
/// scenarios name through `handlers/quest`'s glob import.
pub(crate) mod quest_handler_test_reexports {
    pub(crate) use crate::session::mailbox::{
        SessionCommand, SetQuestSharingInfoAndSendDetailsCommand,
    };
    pub(crate) use std::{collections::HashMap, sync::Arc};
    pub(crate) use wow_conditions::{
        QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    };
    pub(crate) use wow_constants::ItemFlags2;
    pub(crate) use wow_constants::currency::CurrencyGainSourceLikeCpp;
    pub(crate) use wow_constants::unit::NPCFlags1;
    pub(crate) use wow_data::progression_rewards::QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP;
    pub(crate) use wow_packet::ServerPacket;
    pub(crate) use wow_packet::packets::quest::{QuestPushResult, quest_push_reason};
    pub(crate) use wow_world_quest::{
        MAX_QUEST_LOG_SIZE_LIKE_CPP, quest_giver_creature_id_from_source_like_cpp,
        represented_quest_completion_npc_response_like_cpp,
    };
    pub(crate) use wow_world_quest::{
        QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP, QUEST_FLAGS_SHARABLE_LIKE_CPP,
        QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
        QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_BUSY_LIKE_CPP,
        QUEST_PUSH_REASON_CLASS_LIKE_CPP, QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
        QUEST_PUSH_REASON_DEAD_LIKE_CPP, QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
        QUEST_PUSH_REASON_EXPANSION_LIKE_CPP, QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
        QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP, QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
        QUEST_PUSH_REASON_INVALID_LIKE_CPP, QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
        QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP, QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
        QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
        QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
        QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
        QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
        QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_RACE_LIKE_CPP,
        QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
        build_quest_poi_store_like_cpp, read_quest_giver_accept_quest_like_cpp,
        read_quest_giver_query_quest_like_cpp,
    };
    pub(crate) use wow_world_quest::{
        RepresentedPushQuestToPartyOutcomeLikeCpp, RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
        RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
        RepresentedQuestPushResultResponseLikeCpp, RepresentedQuestRewardReputationSourceLikeCpp,
    };
}
