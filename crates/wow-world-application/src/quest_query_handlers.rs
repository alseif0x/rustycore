// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest query and world-quest update packet bodies.
//!
//! C++ source of truth: `WorldSession::HandleQueryQuestInfo`,
//! `HandleQueryQuestCompletionNPCs` and `HandleRequestWorldQuestUpdate`
//! (`QuestHandler.cpp`). The family owns the catalog reads and the response
//! assembly; the World session only builds the borrowed hub plus quest-store
//! context (#1263 F5).

use std::sync::Arc;

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_data::quest::QuestStore;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::query::{
    QueryQuestCompletionNpcs, QuestCompletionNpc, QuestCompletionNpcResponse,
};
use wow_packet::packets::quest::{
    QueryQuestInfoResponse, QuestObjectiveInfo, WorldQuestUpdateResponse,
};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one quest query handler invocation.
pub struct QuestQueryHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    quest_store: Option<Arc<QuestStore>>,
}

impl<'a> QuestQueryHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, quest_store: Option<Arc<QuestStore>>) -> Self {
        Self { hub, quest_store }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    pub async fn handle_request_world_quest_update(&mut self, _pkt: wow_packet::WorldPacket) {
        self.publication_like_cpp()
            .send_packet(&WorldQuestUpdateResponse {
                updates: Vec::new(),
            });
    }

    pub async fn handle_query_quest_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _guid = pkt.read_packed_guid(); // requester GUID (usually player)

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => {
                self.publication_like_cpp()
                    .send_packet(&QueryQuestInfoResponse {
                        quest_id,
                        allow: false,
                        ..Default::default()
                    });
                return;
            }
        };

        match quest_store.get(quest_id) {
            None => {
                self.publication_like_cpp()
                    .send_packet(&QueryQuestInfoResponse {
                        quest_id,
                        allow: false,
                        ..Default::default()
                    });
            }
            Some(quest) => {
                let objectives: Vec<QuestObjectiveInfo> = quest
                    .objectives
                    .iter()
                    .map(|obj| QuestObjectiveInfo {
                        id: obj.id,
                        obj_type: obj.obj_type,
                        storage_index: obj.storage_index,
                        object_id: obj.object_id,
                        amount: obj.amount,
                        flags: obj.flags,
                        flags2: obj.flags2,
                        progress_bar_weight: obj.progress_bar_weight,
                        description: obj.description.clone(),
                    })
                    .collect();

                self.publication_like_cpp()
                    .send_packet(&QueryQuestInfoResponse {
                        quest_id,
                        allow: true,
                        quest_type: quest.quest_type,
                        quest_level: quest.quest_level,
                        quest_max_scaling_level: quest.quest_max_scaling_level,
                        min_level: quest.min_level,
                        quest_sort_id: quest.quest_sort_id,
                        quest_info_id: quest.quest_info_id,
                        suggested_group_num: quest.suggested_group_num,
                        reward_next_quest: quest.reward_next_quest,
                        reward_xp_difficulty: quest.reward_xp_difficulty,
                        reward_money_difficulty: quest.reward_money_difficulty,
                        flags: quest.flags,
                        flags_ex: quest.flags_ex,
                        flags_ex2: quest.flags_ex2,
                        reward_items: quest.reward_items,
                        reward_amounts: quest.reward_amounts,
                        reward_display_spell: quest.reward_display_spell,
                        reward_spell: quest.reward_spell,
                        reward_faction_ids: quest.reward_faction_ids,
                        reward_faction_values: quest.reward_faction_values,
                        reward_faction_overrides: quest.reward_faction_overrides,
                        reward_faction_cap_in: quest.reward_faction_cap_in,
                        reward_faction_flags: quest.reward_faction_flags,
                        objectives,
                        log_title: quest.log_title.clone(),
                        log_description: quest.log_description.clone(),
                        quest_description: quest.quest_description.clone(),
                        area_description: quest.area_description.clone(),
                        quest_completion_log: quest.quest_completion_log.clone(),
                    });
            }
        }
    }

    pub async fn handle_query_quest_completion_npcs(&mut self, query: QueryQuestCompletionNpcs) {
        let store = self.quest_store.as_deref();
        let quests = store.map_or_else(Vec::new, |quest_store| {
            represented_quest_completion_npc_response_like_cpp(quest_store, &query.quest_ids)
        });

        self.publication_like_cpp()
            .send_packet(&QuestCompletionNpcResponse { quests });
    }
}

pub fn represented_quest_completion_npc_response_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    raw_quest_ids: &[i32],
) -> Vec<QuestCompletionNpc> {
    raw_quest_ids
        .iter()
        .filter_map(|&raw_quest_id| {
            let quest_id = u32::try_from(raw_quest_id).ok()?;
            if quest_store.get(quest_id).is_none() {
                return None;
            }

            let mut npcs = Vec::new();
            for creature_entry in quest_store.creature_ender_entries_for_quest_like_cpp(quest_id) {
                let Ok(entry) = i32::try_from(creature_entry) else {
                    debug!(
                        quest_id,
                        creature_entry,
                        "QueryQuestCompletionNPCs: creature entry exceeds signed i32 response field"
                    );
                    continue;
                };
                npcs.push(entry);
            }

            for go_entry in quest_store.gameobject_ender_entries_for_quest_like_cpp(quest_id) {
                npcs.push((go_entry | 0x8000_0000) as i32);
            }

            Some(QuestCompletionNpc {
                quest_id: raw_quest_id,
                npcs,
            })
        })
        .collect()
}

/// Builds a quest query handler context from a host's hub and quest store.
pub trait QuestQueryHandlerHostLikeCpp<C> {
    fn quest_query_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> QuestQueryHandlerCxLikeCpp<'a>;
}

fn handle_request_world_quest_update_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .quest_query_handler_cx_like_cpp(catalogs)
            .handle_request_world_quest_update(pkt)
            .await;
    })
}

fn handle_query_quest_info_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .quest_query_handler_cx_like_cpp(catalogs)
            .handle_query_quest_info(pkt)
            .await;
    })
}

fn handle_query_quest_completion_npcs_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match QueryQuestCompletionNpcs::read(&mut pkt) {
            Ok(query) => {
                session
                    .quest_query_handler_cx_like_cpp(catalogs)
                    .handle_query_quest_completion_npcs(query)
                    .await;
            }
            Err(e) => warn!("Failed to read QueryQuestCompletionNpcs: {e}"),
        }
    })
}

/// Registers the quest query handlers on the packet registry.
pub fn register_quest_query_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: QuestQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_info",
        handler: handle_query_quest_info_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestCompletionNpcs,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_completion_npcs",
        handler: handle_query_quest_completion_npcs_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestWorldQuestUpdate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_world_quest_update",
        handler: handle_request_world_quest_update_thunk::<S, C>,
    })?;
    Ok(())
}
