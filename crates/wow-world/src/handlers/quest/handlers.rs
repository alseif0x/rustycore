// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest packet entry points and their handler registrations.

use super::*;
use wow_packet::ClientPacket;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AdventureMapStartQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_adventure_map_start_quest",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_adventure_map_start_quest_with_catalog_like_cpp(
                        catalogs.adventure_map_pois.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_query",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_status_query_with_catalog_like_cpp(catalogs.quest_info.as_ref(), pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_hello",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_hello(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverQueryQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_query_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_query_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverAcceptQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_accept_quest",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_giver_accept_quest_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestLogRemoveQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_log_remove_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_log_remove_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_info",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_query_quest_info(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestCompletionNpcs,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_completion_npcs",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryQuestCompletionNpcs::read(&mut pkt) {
                    Ok(query) => session.handle_query_quest_completion_npcs(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryQuestCompletionNpcs: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPoiQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_poi_query",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QuestPoiQuery::read(&mut pkt) {
                    Ok(query) => session.handle_quest_poi_query(query).await,
                    Err(e) => tracing::warn!("Failed to read QuestPoiQuery: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverRequestReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_request_reward",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_giver_request_reward_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCompleteQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_complete_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_complete_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverChooseReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_choose_reward",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_giver_choose_reward_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCloseQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_close_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_close_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestWorldQuestUpdate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_world_quest_update",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_world_quest_update(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestConfirmAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_confirm_accept",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_confirm_accept_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPushResult,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_push_result",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_push_result(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PushQuestToParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_push_quest_to_party",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_push_quest_to_party(pkt).await })
        },
    }
}

// ── Handler implementations ──────────────────────────────────────────────────

/// TrinityCore `MAX_QUEST_LOG_SIZE`; explicit quest-log slots are 0..24.

impl WorldSession {
    /// CMSG_ADVENTURE_MAP_START_QUEST.
    ///
    /// C++ `HandleAdventureMapStartQuest`:
    /// `QuestTemplate` lookup -> `sAdventureMapPOIStore` QuestID + PlayerCondition gate ->
    /// `Player::CanTakeQuest(quest, true)` -> `AddQuestAndCheckCompletion(quest, player)`.
    ///
    /// Rust keeps the same silent-return gates and records the accepted request until
    /// Adventure Map quest starts can call the same live AddQuestAndCheckCompletion path.
    pub(crate) async fn handle_adventure_map_start_quest_with_catalog_like_cpp(
        &mut self,
        adventure_map_poi_store: &wow_data::AdventureMapPoiStore,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match AdventureMapStartQuest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("AdventureMapStartQuest: bad packet: {error}");
                return;
            }
        };
        let Ok(quest_id) = u32::try_from(request.quest_id) else {
            return;
        };

        let Some(quest_store) = self.quest_store.clone() else {
            return;
        };
        let Some(quest) = quest_store.get(quest_id) else {
            return;
        };
        let Some(poi) = adventure_map_poi_store.find_start_quest_poi_like_cpp(quest_id, |id| {
            self.represented_meets_player_condition_id_like_cpp(id)
        }) else {
            return;
        };

        if !self.can_take_quest(quest) {
            return;
        }

        self.record_represented_adventure_map_start_quest_like_cpp(
            RepresentedAdventureMapStartQuestLikeCpp {
                quest_id,
                adventure_map_poi_id: poi.id,
                player_condition_id: poi.player_condition_id,
            },
        );
    }

    #[cfg(test)]
    pub async fn handle_adventure_map_start_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self.adventure_map_poi_store().cloned().unwrap_or_else(|| {
            std::sync::Arc::new(wow_data::AdventureMapPoiStore::from_entries([]))
        });
        self.handle_adventure_map_start_quest_with_catalog_like_cpp(store.as_ref(), pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_quest_giver_status_query(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        self.handle_quest_giver_status_query_with_catalog_like_cpp(
            catalogs.quest_info.as_ref(),
            pkt,
        )
        .await;
    }

    /// CMSG_QUEST_GIVER_STATUS_QUERY — returns the quest status icon for an NPC.
    /// C++ QuestHandler.cpp: HandleQuestgiverStatusQueryOpcode -> Player::GetQuestDialogStatus.
    pub async fn handle_quest_giver_status_query_with_catalog_like_cpp(
        &mut self,
        quest_info: &wow_data::progression_rewards::QuestInfoStore,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverStatusQuery: failed to read GUID");
                return;
            }
        };

        let Some(source) = self.represented_quest_giver_status_query_source_like_cpp(guid) else {
            debug!(
                account = self.account_id,
                ?guid,
                "QuestGiverStatusQuery: represented ObjectAccessor mask UNIT|GAMEOBJECT miss"
            );
            return;
        };
        let status =
            self.get_represented_quest_giver_status_with_catalog_like_cpp(Some(quest_info), source);

        debug!(
            account = self.account_id,
            ?guid,
            source_entry = source.entry(),
            source_kind = source.kind_name(),
            status = status,
            "QuestGiverStatus represented source resolved"
        );

        self.send_packet(&QuestGiverStatus { guid, status });
    }

    /// CMSG_QUEST_GIVER_HELLO — player right-clicks a quest NPC.
    /// Opens the represented quest list dialog for an interactable questgiver Creature.
    /// C++ refs:
    /// - `WorldSession::HandleQuestgiverHelloOpcode`, `QuestHandler.cpp:76-103`.
    /// - `Player::PrepareQuestMenu`, `Player.cpp:13947-14004`.
    /// Remaining represented gaps: fake-death aura removal, `AI()->OnGossipHello`,
    /// full PlayerTalkClass ownership.
    pub async fn handle_quest_giver_hello(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverHello: failed to read GUID");
                return;
            }
        };

        let Some(access) =
            self.represented_npc_can_interact_with_like_cpp(guid, NPCFlags1::QUEST_GIVER.bits(), 0)
        else {
            debug!(
                account = self.account_id,
                ?guid,
                "QuestGiverHello: NPC not found or not interactable as questgiver"
            );
            return;
        };

        self.pause_interacted_creature_movement_like_cpp(guid);

        if (access.npc_flags & NPCFlags1::GOSSIP.bits()) != 0
            && let Some(msg) = self
                .build_gossip_menu(access.entry, access.npc_flags, guid)
                .await
        {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello sent catalog-backed prepared gossip menu like C++"
            );
            self.send_packet(&msg);
            return;
        }

        if self.send_represented_creature_trainer_gossip_menu_like_cpp(
            guid,
            access.entry,
            access.npc_flags,
        ) {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello sent trainer fallback prepared gossip menu like C++"
            );
            return;
        }

        if self.use_represented_creature_questgiver_like_cpp(guid, access.entry) {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello represented Creature questgiver seam consumed"
            );
        }
    }

    /// CMSG_QUEST_GIVER_QUERY_QUEST — player clicks a quest name in the list.
    /// Shows full quest details (objectives, rewards) before acceptkng.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverQueryQuest
    pub async fn handle_quest_giver_query_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let (guid, quest_id, respond_to_giver) =
            match read_quest_giver_query_quest_like_cpp(&mut pkt) {
                Ok(packet) => packet,
                Err(_) => {
                    warn!("QuestGiverQueryQuest: failed to read packet");
                    return;
                }
            };

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            respond_to_giver,
            "Received QuestGiverQueryQuest like C++"
        );
        if !self.send_represented_quest_giver_query_quest_like_cpp(guid, quest_id) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "QuestGiverQueryQuest produced no represented response"
            );
        }
    }

    /// CMSG_QUEST_GIVER_ACCEPT_QUEST — player clicks "Accept" in the quest details dialog.
    /// Saves quest to characters DB and confirms to the client.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverAcceptQuest
    #[cfg(test)]
    pub async fn handle_quest_giver_accept_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_giver_accept_quest_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    pub async fn handle_quest_giver_accept_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let (guid, quest_id, start_cheat) = match read_quest_giver_accept_quest_like_cpp(&mut pkt) {
            Ok(packet) => packet,
            Err(_) => {
                warn!("QuestGiverAcceptQuest: failed to read packet");
                return;
            }
        };

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            start_cheat,
            "Received QuestGiverAcceptQuest like C++"
        );

        // Validate represented C++ source/relation before any quest-log mutation or DB save.
        // C++ HandleQuestgiverAcceptQuestOpcode closes gossip and clears sharing info on
        // failure; this represented slice intentionally models that as no packet/no mutation.
        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        if !self.represented_quest_giver_accept_source_allows_quest_like_cpp(
            guid,
            quest_id,
            &quest_store,
        ) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "AcceptQuest: represented source/relation guard rejected quest"
            );
            return;
        }
        let Some(quest) = quest_store.get(quest_id) else {
            warn!(
                account = self.account_id,
                quest_id, "AcceptQuest: unknown quest"
            );
            return;
        };

        // Full eligibility check: SatisfyQuestStatus + PrevQuestId + race/class/level
        // Legacy non-canonical note: Player.CanTakeQuest(quest, true)
        if !self.can_take_quest(quest) {
            warn!(
                account = self.account_id,
                quest_id,
                race = self.player_race_like_cpp(),
                class = self.player_class_like_cpp(),
                level = self.player_level_like_cpp(),
                "AcceptQuest: player does not meet requirements (CanTakeQuest failed)"
            );
            return;
        }

        // C++ Player::AddQuest uses FindQuestSlot(0) over explicit QuestLog slots.
        let Some(slot) = self.first_free_quest_slot_like_cpp() else {
            warn!(account = self.account_id, "Quest log full");
            return;
        };

        // Build objective counts (one slot per objective)
        let obj_count = quest.objectives.len();

        let (accept_time_secs, end_time_secs) =
            Self::represented_accept_and_end_time_for_new_quest_like_cpp(&quest);

        // Add to local state
        self.invalidate_player_quest_status_authority_like_cpp();
        let status = PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs,
            end_time_secs,
            objective_counts: vec![0; obj_count],
            slot,
        };
        if self
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.statuses.insert(quest_id, status);
            })
            .is_none()
        {
            return;
        }

        self.complete_represented_quest_after_add_with_generator_like_cpp(
            item_guid_generator,
            quest,
        )
        .await;

        // Save to DB after AddQuestAndCheckCompletion-style completion, unless
        // RewardQuest already removed/rewarded the quest.
        if let Some(status) = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses.get(&quest_id).map(|status| status.status))
        {
            self.save_quest_to_db(quest_id, status).await;
        }
        self.sync_player_registry_state_like_cpp();
        self.send_represented_quest_log_slot_update_like_cpp(slot);

        info!(account = self.account_id, quest_id, "Quest accepted");

        // Notify client — quest added popup
        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp: 0,
            money: 0,
            skill_line_id: 0,
            skill_points: 0,
            use_quest_reward_currency: false,
        });
    }

    /// CMSG_QUEST_GIVER_CLOSE_QUEST — acknowledged client close for auto-accept quest flow.
    /// C++ ref: `WorldSession::HandleQuestgiverCloseQuest`, `QuestHandler.cpp:591-601`.
    /// Represented seam only: records local `ScriptMgr::OnQuestAcknowledgeAutoAccept` evidence.
    pub async fn handle_quest_giver_close_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id = match pkt.read_uint32() {
            Ok(quest_id) => quest_id,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestGiverCloseQuest: failed to read QuestID"
                );
                return;
            }
        };

        let _ = self.acknowledge_auto_accept_quest_like_cpp(quest_id);
    }

    /// CMSG_REQUEST_WORLD_QUEST_UPDATE — current Trinity 3.4.3 handler sends an empty response.
    /// C++ refs: `WorldSession::HandleRequestWorldQuestUpdate`, `QuestHandler.cpp:780-788`;
    /// `RequestWorldQuestUpdate::Read`, `QuestPackets.h:655-661` (`Read() { }`, no payload consumption).
    pub async fn handle_request_world_quest_update(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_packet(&WorldQuestUpdateResponse {
            updates: Vec::new(),
        });
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
    #[cfg(test)]
    pub async fn handle_quest_confirm_accept(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_confirm_accept_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    pub async fn handle_quest_confirm_accept_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let packet = match QuestConfirmAccept::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestConfirmAccept: failed to read signed QuestID"
                );
                return;
            }
        };

        let parsed_quest_id = packet.quest_id as u32;
        let Some(pending) = self.represented_pending_quest_sharing_like_cpp() else {
            debug!(
                account = self.account_id,
                raw_quest_id = packet.quest_id,
                parsed_quest_id,
                "QuestConfirmAccept: no represented pending shared quest"
            );
            return;
        };

        if pending.quest_id != parsed_quest_id {
            debug!(
                account = self.account_id,
                pending_quest_id = pending.quest_id,
                raw_quest_id = packet.quest_id,
                parsed_quest_id,
                "QuestConfirmAccept: represented pending quest id mismatch; pending state preserved"
            );
            return;
        }

        self.clear_represented_pending_quest_sharing_like_cpp();

        let Some(quest_store) = &self.quest_store else {
            debug!(
                account = self.account_id,
                parsed_quest_id,
                "QuestConfirmAccept: pending cleared before missing quest store like C++ order"
            );
            return;
        };

        let Some(quest) = quest_store.get(parsed_quest_id).cloned() else {
            debug!(
                account = self.account_id,
                parsed_quest_id,
                "QuestConfirmAccept: pending cleared before missing quest template like C++ order"
            );
            return;
        };

        let receiver_guid = self.player_guid();
        let record = |session: &mut WorldSession,
                      reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
                      can_add_source_item_unrepresented: bool,
                      can_add_source_item_result: Option<InventoryResult>,
                      add_quest_runtime_unrepresented: bool,
                      source_spell_unrepresented: bool,
                      represented_source_spell_id: Option<u32>,
                      represented_source_spell_self_casts: u8| {
            session.record_represented_quest_confirm_accept_like_cpp(
                RepresentedQuestConfirmAcceptLikeCpp {
                    receiver_guid,
                    sender_guid_before_clear: pending.sender_guid,
                    quest_id: parsed_quest_id,
                    raw_quest_id: packet.quest_id,
                    reason,
                    object_accessor_unrepresented: true,
                    party_runtime_unrepresented: true,
                    can_add_source_item_unrepresented,
                    can_add_source_item_result,
                    add_quest_runtime_unrepresented,
                    source_spell_unrepresented,
                    represented_source_spell_id,
                    represented_source_spell_self_casts,
                },
            );
        };

        let Some(player_registry) = self.player_registry().map(Arc::clone) else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(sender_active_status) =
            player_registry.quest_active_status(pending.sender_guid, parsed_quest_id)
        else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(receiver_guid) = receiver_guid else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(group_registry) = self.group_registry().map(Arc::clone) else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let same_represented_group = group_registry.snapshots().into_iter().any(|group| {
            let members = &group.members;
            members.contains(&receiver_guid) && members.contains(&pending.sender_guid)
        });
        if !same_represented_group {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if !matches!(
            sender_active_status,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP | QUEST_STATUS_COMPLETE_LIKE_CPP)
        ) {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerNotActiveQuest,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if !self.can_take_quest(&quest) {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanTakeQuestFailed,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if self.first_free_quest_slot_like_cpp().is_none() {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if quest.source_item_id > 0 {
            let Some(source_item_template) = self.item_storage_template(quest.source_item_id)
            else {
                let source_item_result = InventoryResult::ItemNotFound;
                self.send_equip_error(source_item_result, None, None, 0, 0);
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    None,
                    0,
                );
                return;
            };

            let source_item_count = quest.source_item_count.max(1);
            let (source_item_result, source_item_dest, _) = self
                .plan_store_new_direct_inventory_item(quest.source_item_id, source_item_count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));

            if !matches!(
                source_item_result,
                InventoryResult::Ok | InventoryResult::ItemMaxCount
            ) {
                self.send_equip_error(
                    source_item_result,
                    None,
                    None,
                    0,
                    u32::from(source_item_template.item_limit_category),
                );
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    None,
                    0,
                );
                return;
            }

            let source_item_no_grant_reason = if self
                .item_template_start_quest_id(quest.source_item_id)
                .is_some_and(|start_quest_id| start_quest_id == quest.id as i32)
            {
                Some(RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStartQuestNoGrant)
            } else if source_item_result == InventoryResult::ItemMaxCount {
                Some(RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemMaxCountNoGrant)
            } else {
                None
            };

            if let Some(source_item_no_grant_reason) = source_item_no_grant_reason {
                if !self
                    .add_quest_confirm_accept_local_state_like_cpp(item_guid_generator, &quest)
                    .await
                {
                    record(
                        self,
                        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                        false,
                        None,
                        false,
                        false,
                        None,
                        0,
                    );
                    return;
                }

                let represented_source_spell_id =
                    (quest.source_spell_id > 0).then_some(quest.source_spell_id);
                let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
                record(
                    self,
                    source_item_no_grant_reason,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    represented_source_spell_id,
                    represented_source_spell_self_casts,
                );
                return;
            }

            if !self
                .add_quest_confirm_accept_local_state_like_cpp(item_guid_generator, &quest)
                .await
            {
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                    false,
                    None,
                    false,
                    false,
                    None,
                    0,
                );
                return;
            }

            let Some(source_item_store_outcome) = self
                .store_quest_source_item_with_generator_like_cpp(
                    item_guid_generator,
                    quest.source_item_id,
                    source_item_count,
                    &source_item_dest,
                )
                .await
            else {
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::GiveQuestSourceItemStoreNewItemUnrepresented,
                    false,
                    Some(source_item_result),
                    true,
                    quest.source_spell_id > 0,
                    None,
                    0,
                );
                return;
            };

            let represented_source_spell_id =
                (quest.source_spell_id > 0).then_some(quest.source_spell_id);
            let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
            let source_item_store_reason = match source_item_store_outcome {
                QuestSourceItemStoreOutcomeLikeCpp::StoredNewItem => {
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem
                }
                QuestSourceItemStoreOutcomeLikeCpp::BoundObjectiveNoGrant => {
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant
                }
            };
            record(
                self,
                source_item_store_reason,
                false,
                Some(source_item_result),
                false,
                false,
                represented_source_spell_id,
                represented_source_spell_self_casts,
            );
            return;
        }

        if !self
            .add_quest_confirm_accept_local_state_like_cpp(item_guid_generator, &quest)
            .await
        {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        let represented_source_spell_id =
            (quest.source_spell_id > 0).then_some(quest.source_spell_id);
        let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
        record(
            self,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
            false,
            None,
            false,
            false,
            represented_source_spell_id,
            represented_source_spell_self_casts,
        );
    }

    /// CMSG_PUSH_QUEST_TO_PARTY — sender-side bounded quest share preflight.
    ///
    /// C++ anchors:
    /// - `Opcodes.cpp:746`: `STATUS_LOGGEDIN`, `PROCESS_THREADUNSAFE`, `HandlePushQuestToParty`.
    /// - `QuestPackets.cpp:658-661`: packet reads one `uint32 QuestID`.
    /// - `QuestHandler.cpp:603-756`: template lookup, `CanShareQuest`, quest-pool active,
    ///   group presence, then receiver iteration.
    ///
    /// Represented-partial: this records sender-local evidence only. It never mutates DB/maps,
    /// never sets receiver pending sharing, and never fans out packets to other sessions.
    /// If the session has no real `player_guid`, Rust records the existing evidence only and
    /// does not fabricate an empty sender GUID for `SMSG_QUEST_PUSH_RESULT`.
    pub async fn handle_push_quest_to_party(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match PushQuestToParty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "PushQuestToParty: failed to read QuestID"
                );
                return;
            }
        };

        let Some(quest_store) = self.quest_store.as_ref().map(Arc::clone) else {
            debug!(
                account = self.account_id,
                quest_id = packet.quest_id,
                "PushQuestToParty: missing QuestStore, silent return like missing ObjectMgr template path"
            );
            return;
        };

        let Some(quest) = quest_store.get(packet.quest_id) else {
            debug!(
                account = self.account_id,
                quest_id = packet.quest_id,
                "PushQuestToParty: missing quest template, silent return like C++"
            );
            return;
        };

        let sender_guid = self.player_guid();
        if !self.represented_can_share_quest_like_cpp(quest) {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_ALLOWED,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotAllowed,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        let Some(quest_pool_store) = self.quest_pool_store.as_ref().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::QuestPoolActiveCheckUnrepresented,
                    quest_pool_active_check_unrepresented: true,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        };

        if !quest_pool_store.is_quest_active_like_cpp(packet.quest_id) {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_DAILY,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotDaily,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        if self.resolved_group_guid_like_cpp().is_none() {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_IN_PARTY,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotInParty,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
            return;
        };

        let Some(group_registry) = self.group_registry().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let Some(player_registry) = self.player_registry().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let Some(group_info) = group_registry.get(&group_guid).map(|entry| entry.clone()) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let canonical_map_manager = self.canonical_map_manager.clone();
        let receiver_snapshots = group_info
            .members
            .iter()
            .copied()
            .filter(|member_guid| Some(*member_guid) != sender_guid)
            .filter_map(|member_guid| {
                player_registry
                    .quest_sharing_snapshot(member_guid, canonical_map_manager.as_ref())
                    .map(|receiver| (member_guid, receiver))
            })
            .collect::<Vec<_>>();

        if receiver_snapshots.is_empty() {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        }

        let mut blocked_by_unsupported_success_path = false;
        for (receiver_guid, receiver) in receiver_snapshots {
            if receiver.pending_quest_sharing.is_some() {
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_BUSY_LIKE_CPP,
                    String::new(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverBusy,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if !receiver.is_alive {
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_DEAD_LIKE_CPP,
                    String::new(),
                );
                if let Some(sender_guid) = sender_guid {
                    let _ = player_registry.send_current_packet(
                        receiver.registration,
                        QuestPushResultResponse {
                            sender_guid,
                            result: QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
                            quest_title: quest.log_title.clone(),
                        }
                        .to_bytes(),
                    );
                }
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverDead,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if receiver.rewarded_quests.contains(&packet.quest_id) {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason:
                            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverAlreadyDone,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if let Some(status) = receiver
                .active_quest_statuses
                .get(&packet.quest_id)
                .copied()
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                if status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || status == QUEST_STATUS_COMPLETE_LIKE_CPP
                {
                    self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                        receiver_guid,
                        QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
                        String::new(),
                    );
                    let _ = player_registry.send_current_packet(
                        receiver.registration,
                        QuestPushResultResponse {
                            sender_guid: sender_guid_for_receiver_packet,
                            result: QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP,
                            quest_title: quest.log_title.clone(),
                        }
                        .to_bytes(),
                    );
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason:
                                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverOnQuest,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: false,
                        },
                    );
                    continue;
                }
            }

            // C++ `Player::SatisfyQuestLog(false)` checks `FindQuestSlot(0) <
            // MAX_QUEST_LOG_SIZE`; this represented cross-session seam uses
            // the receiver snapshot derived from the canonical Player quest
            // slots via `sync_player_registry_state_like_cpp()`.
            if receiver.active_quest_statuses.len() >= MAX_QUEST_LOG_SIZE_LIKE_CPP as usize {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ `Player::SatisfyQuestDay(quest, false)` immediately follows
            // `SatisfyQuestLog(false)` in `WorldSession::HandlePushQuestToParty`.
            // Non-daily/non-DF quests pass this gate; already-completed daily
            // quests and represented DF quests send the same AlreadyDone pair
            // as the earlier rewarded/onquest branch.
            let already_satisfied_quest_day_like_cpp = if quest.is_df_quest_like_cpp() {
                receiver.df_quests.contains(&packet.quest_id)
            } else if quest.is_daily_like_cpp() {
                receiver.daily_quests_completed.contains(&packet.quest_id)
            } else {
                false
            };

            if already_satisfied_quest_day_like_cpp {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ then evaluates `Player::SatisfyQuestMinLevel(quest, false)`
            // followed by `SatisfyQuestMaxLevel(quest, false)`.  Receiver
            // `level` is a derived cross-session snapshot synchronized from
            // the receiver `WorldSession`, never source-of-truth in reverse.
            if quest.min_level > 0 && i32::from(receiver.level) < quest.min_level {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if quest.max_level > 0 && receiver.level > quest.max_level {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ order then evaluates `Player::SatisfyQuestClass(quest, false)`
            // followed by `SatisfyQuestRace(quest, false)`. Receiver class/race
            // are read-only `PlayerRegistry` snapshots derived from the receiver
            // `WorldSession`; never sync registry state back into the session.
            let receiver_class_mask = player_race_or_class_mask_like_cpp(receiver.class);
            if quest.allowable_classes != 0 && (quest.allowable_classes & receiver_class_mask) == 0
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_CLASS_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let receiver_race_mask = u64::from(player_race_or_class_mask_like_cpp(receiver.race));
            if quest.allowable_races != 0 && (quest.allowable_races & receiver_race_mask) == 0 {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_RACE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // The receiver's standings come off its canonical `Player` since #252.
            // `None` means unknown, not zero: mid far-teleport the owner has left
            // the old map and has not reached the destination. Evaluating a
            // standing gate against an empty set would tell the sender the
            // receiver's reputation is too low when it may well qualify, so report
            // the eligibility as unrepresented instead.
            let Some(receiver_reputation_standings) = receiver.reputation_standings.as_ref() else {
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            };

            let receiver_reputation_standing_like_cpp = |faction_id: u32| -> i32 {
                receiver_reputation_standings
                    .iter()
                    .find_map(|(stored_faction_id, standing)| {
                        (*stored_faction_id == faction_id).then_some(*standing)
                    })
                    .unwrap_or(0)
            };

            let reputation_failure_reason = if quest.required_min_rep_faction != 0
                && receiver_reputation_standing_like_cpp(quest.required_min_rep_faction)
                    < quest.required_min_rep_value
            {
                Some(RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)
            } else if quest.required_max_rep_faction != 0
                && receiver_reputation_standing_like_cpp(quest.required_max_rep_faction)
                    >= quest.required_max_rep_value
            {
                Some(RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)
            } else {
                None
            };

            if let Some(reason) = reputation_failure_reason {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ `Player::SatisfyQuestDependentQuests(quest, false)` preserves this
            // sub-gate order: PreviousQuest, DependentPreviousQuests,
            // BreadcrumbQuest, DependentBreadcrumbQuests. Expansion, CanTakeQuest,
            // success fanout, SetQuestSharingInfo, details, and auto-accept stay
            // deliberately unsupported after these represented prerequisites.
            let previous_quest_prerequisite_failed = if quest.prev_quest_id > 0 {
                !receiver
                    .rewarded_quests
                    .contains(&quest.prev_quest_id.unsigned_abs())
            } else if quest.prev_quest_id < 0 {
                receiver
                    .active_quest_statuses
                    .get(&quest.prev_quest_id.unsigned_abs())
                    .copied()
                    != Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            } else {
                false
            };

            let mut prerequisite_failure_reason =
                previous_quest_prerequisite_failed.then_some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite,
                );

            if prerequisite_failure_reason.is_none()
                && represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                    &quest_store,
                    quest,
                    &receiver.rewarded_quests,
                )
            {
                prerequisite_failure_reason = Some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite,
                );
            }

            if prerequisite_failure_reason.is_none() && quest.breadcrumb_for_quest_id != 0 {
                // C++ `SatisfyQuestBreadcrumbQuest` depends on
                // `CanTakeQuest(target,false)`. Do not fake it here; keep the
                // success path blocked until real/represented CanTakeQuest is available.
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            }

            if prerequisite_failure_reason.is_none()
                && represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
                    quest,
                    &receiver.active_quest_statuses,
                )
            {
                prerequisite_failure_reason = Some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite,
                );
            }

            if let Some(reason) = prerequisite_failure_reason {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if i32::from(receiver.active_expansion) < quest.expansion {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if !represented_can_take_quest_after_expansion_like_cpp(&quest_store, quest, &receiver)
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_INVALID_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if quest.is_turn_in_like_cpp()
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
            {
                let Some(sender_guid_for_receiver_command) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: true,
                        },
                    );
                    continue;
                };

                let command = SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(
                    SendRepeatableTurnInRequestItemsLikeCppCommand {
                        sender_guid: sender_guid_for_receiver_command,
                        quest: quest.clone(),
                    },
                );

                if player_registry
                    .try_send_current_command(receiver.registration, command)
                    .is_err()
                {
                    blocked_by_unsupported_success_path = true;
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: true,
                        },
                    );
                    continue;
                }

                // C++ `HandlePushQuestToParty` sends Success to the sender before the
                // repeatable turn-in `SendQuestGiverRequestItems` receiver side effect.
                // Rust has an extra fallible queue hop, so emit represented Success only
                // after the receiver command has been accepted.
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                    String::new(),
                );

                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let Some(sender_guid_for_receiver_command) = sender_guid else {
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverQuestDetailsPromptCommandFailed,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            };

            let command = SessionCommand::SetQuestSharingInfoAndSendDetails(
                SetQuestSharingInfoAndSendDetailsCommand {
                    sender_guid: sender_guid_for_receiver_command,
                    quest: quest.clone(),
                },
            );

            if player_registry
                .try_send_current_command(receiver.registration, command)
                .is_err()
            {
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverQuestDetailsPromptCommandFailed,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            }

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                String::new(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
        }

        if blocked_by_unsupported_success_path {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
        }
    }

    /// CMSG_QUEST_PUSH_RESULT — response to a shared quest prompt.
    ///
    /// C++ anchor: `WorldSession::HandleQuestPushResult`, `QuestHandler.cpp:758-767`.
    /// Represented-partial: session-local pending sharing state is cleared like C++;
    /// matching sender responses are recorded as evidence because full `ObjectAccessor::FindPlayer`
    /// and party sender packet fanout are not represented in this bounded slice.
    pub async fn handle_quest_push_result(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match QuestPushResult::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestPushResult: failed to read SenderGUID/QuestID/Result"
                );
                return;
            }
        };

        let Some(pending) = self.represented_pending_quest_sharing_like_cpp() else {
            debug!(
                account = self.account_id,
                sender_guid = ?packet.sender_guid,
                quest_id = packet.quest_id,
                result = packet.result,
                "QuestPushResult: no represented pending shared quest"
            );
            return;
        };

        self.clear_represented_pending_quest_sharing_like_cpp();

        if pending.sender_guid != packet.sender_guid {
            self.record_represented_quest_push_result_sender_mismatch_like_cpp();
            debug!(
                account = self.account_id,
                pending_sender_guid = ?pending.sender_guid,
                packet_sender_guid = ?packet.sender_guid,
                "QuestPushResult: represented sender mismatch, pending state cleared"
            );
            return;
        }

        let Some(receiver_guid) = self.player_guid() else {
            debug!(
                account = self.account_id,
                sender_guid = ?packet.sender_guid,
                "QuestPushResult: represented sender matched but no local receiver guid is available"
            );
            return;
        };

        self.record_represented_quest_push_result_response_like_cpp(
            RepresentedQuestPushResultResponseLikeCpp {
                receiver_guid,
                sender_guid: packet.sender_guid,
                parsed_quest_id: packet.quest_id,
                pending_quest_id: pending.quest_id,
                result: packet.result,
            },
        );
    }

    /// CMSG_QUEST_LOG_REMOVE_QUEST — abandon quest-log slot.

    /// Represented-partial seam: explicit QuestLog slot lookup + local active quest removal/DB delete.
    /// Remaining gaps: source-item gates/cleanup, no-abandon-once-begun, timed/PvP state,
    /// personal summons, quest tracker DB, ScriptMgr callbacks, and criteria update evidence.
    pub async fn handle_quest_log_remove_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let slot = match pkt.read_uint8() {
            Ok(slot) => slot,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestLogRemoveQuest: failed to read Entry"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            slot, "QuestLogRemoveQuest: represented slot-backed abandon request"
        );

        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            debug!(
                account = self.account_id,
                slot, "QuestLogRemoveQuest: slot outside MAX_QUEST_LOG_SIZE"
            );
            return;
        }

        let Some(qid) = self.get_quest_slot_quest_id_like_cpp(slot) else {
            debug!(
                account = self.account_id,
                slot,
                "QuestLogRemoveQuest: valid slot empty; criteria update remains an explicit gap"
            );
            return;
        };

        self.invalidate_player_quest_status_authority_like_cpp();
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.statuses.remove(&qid);
        });
        self.delete_quest_from_db(qid).await;
        self.sync_player_registry_state_like_cpp();
        self.send_represented_quest_log_slot_update_like_cpp(slot);
        info!(
            account = self.account_id,
            quest_id = qid,
            slot,
            "Quest abandoned via represented explicit quest-log slot"
        );
    }

    /// CMSG_QUERY_QUEST_INFO — client asks for full quest template data by ID.
    /// Used to populate the quest log and tooltip.
    /// Legacy non-canonical note: QuestHandler.HandleQueryQuestInfo
    pub async fn handle_query_quest_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _guid = pkt.read_packed_guid(); // requester GUID (usually player)

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => {
                self.send_packet(&QueryQuestInfoResponse {
                    quest_id,
                    allow: false,
                    ..Default::default()
                });
                return;
            }
        };

        match quest_store.get(quest_id) {
            None => {
                self.send_packet(&QueryQuestInfoResponse {
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

                self.send_packet(&QueryQuestInfoResponse {
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

    /// CMSG_QUERY_QUEST_COMPLETION_NPCS — client asks for Creature/GO quest enders.
    /// C++ refs:
    /// - `WorldSession::HandleQueryQuestCompletionNPCs`, QueryHandler.cpp:252-278.
    /// - `QuestCompletionNPCResponse::Write`, QueryPackets.cpp:451-462.
    pub async fn handle_query_quest_completion_npcs(&mut self, query: QueryQuestCompletionNpcs) {
        let quests = self
            .quest_store
            .as_deref()
            .map_or_else(Vec::new, |quest_store| {
                represented_quest_completion_npc_response_like_cpp(quest_store, &query.quest_ids)
            });

        self.send_packet(&QuestCompletionNpcResponse { quests });
    }

    /// CMSG_QUEST_POI_QUERY — client asks for tracker POI blobs.
    ///
    /// C++ refs:
    /// - `QuestPOIQuery::Read`, QueryPackets.cpp:418-423.
    /// - `WorldSession::HandleQuestPOIQuery`, QueryHandler.cpp:280-298.
    /// - `ObjectMgr::LoadQuestPOI`, ObjectMgr.cpp:8337-8415.
    pub async fn handle_quest_poi_query(&mut self, query: QuestPoiQuery) {
        if query.missing_quest_count > i32::from(MAX_QUEST_LOG_SIZE_LIKE_CPP) {
            return;
        }

        let requested_count = query.missing_quest_count.max(0) as usize;
        let requested_count = requested_count.min(query.missing_quest_pois.len());
        let mut quest_ids = std::collections::HashSet::new();
        for quest_id in query.missing_quest_pois.iter().take(requested_count) {
            quest_ids.insert(*quest_id);
        }

        let poi_store = self.quest_poi_store_like_cpp().await;
        let mut quest_poi_data_stats = Vec::new();
        for quest_id in quest_ids {
            if quest_id <= 0 {
                continue;
            }

            let quest_id_u32 = quest_id as u32;
            if self.find_quest_slot_like_cpp(quest_id_u32).is_none() {
                continue;
            }

            if let Some(poi_data) = poi_store.get(&quest_id) {
                quest_poi_data_stats.push(poi_data.clone());
            }
        }

        self.send_packet_realm(&QuestPoiQueryResponse {
            quest_poi_data_stats,
        });
    }

    /// CMSG_QUEST_GIVER_REQUEST_REWARD — player talks to NPC to turn in a completed quest.
    /// Legacy non-canonical note: QuestHandler.HandleQuestgiverRequestReward
    /// Sent when player right-clicks a quest-ender NPC and has the quest in Complete status.
    /// Server responds with SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE (reward selection dialog).
    #[cfg(test)]
    pub async fn handle_quest_giver_request_reward(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_giver_request_reward_with_generator_like_cpp(
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    pub async fn handle_quest_giver_request_reward_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverRequestReward: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            "Received QuestGiverRequestReward like C++"
        );

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let quest = match quest_store.get(quest_id) {
            Some(q) => q.clone(),
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "RequestReward: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "RequestReward: quest disabled"
            );
            return;
        }

        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
            && !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "RequestReward: represented involved source rejected"
            );
            return;
        }

        // C++: if (_player->CanCompleteQuest(questID)) _player->CompleteQuest(questID)
        let can_complete_now = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| {
                let rewarded = state.rewarded_quest_ids.contains(&quest_id);
                state.statuses.get(&quest_id).map(|status| {
                    Self::represented_can_complete_quest_after_objective_like_cpp(
                        status, &quest, 0, rewarded,
                    )
                })
            })
            .unwrap_or(false);
        if can_complete_now {
            let completion_evidence_start = self
                .represented_quest_complete_status_updates_like_cpp
                .len();
            self.complete_represented_quest_after_add_with_generator_like_cpp(
                item_guid_generator,
                &quest,
            )
            .await;
            self.save_represented_quest_statuses_completed_after_like_cpp(
                completion_evidence_start,
            )
            .await;
        }

        let is_complete = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses.get(&quest_id).map(|qs| qs.status))
            == Some(QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Objectives not finished — silently ignore
            // (C# would send SMSG_QUEST_GIVER_REQUEST_ITEMS instead)
            warn!(
                account = self.account_id,
                quest_id, "RequestReward: quest not complete"
            );
            return;
        }

        // Build rewards block for the offer-reward dialog
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        // Populate choice items for the dialog
        for i in 0..6 {
            rewards.choice_items[i] = (
                quest.reward_choice_items[i].0,
                quest.reward_choice_items[i].1,
            );
        }
        rewards.choice_item_types = quest.reward_choice_item_types;

        // C#: SendQuestGiverOfferReward(quest, questGiverGUID, true)
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_COMPLETE_QUEST — player talks to quest-ender NPC.
    /// If objectives are done: show reward dialog. Else: show "still need X" dialog.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverCompleteQuest
    pub async fn handle_quest_giver_complete_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverCompleteQuest: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let from_script: bool = pkt.read_bit().unwrap_or(false);

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            from_script,
            "Received QuestGiverCompleteQuest like C++"
        );

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let quest = match quest_store.get(quest_id) {
            Some(q) => q,
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "QuestGiverCompleteQuest: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCompleteQuest: quest disabled"
            );
            return;
        }

        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0 {
            if from_script
                || !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                    guid,
                    quest_id,
                    &quest_store,
                )
            {
                warn!(
                    account = self.account_id,
                    ?guid,
                    quest_id,
                    from_script,
                    "QuestGiverCompleteQuest: represented involved source rejected"
                );
                return;
            }
        } else if !from_script || self.player_guid() != Some(guid) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                from_script,
                "QuestGiverCompleteQuest: auto-complete source is not script/player"
            );
            return;
        }

        // Check if player has the quest active
        if !self.has_quest(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "Player doesn't have quest"
            );
            return;
        }

        // Build rewards block
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        for i in 0..6 {
            rewards.choice_items[i] = (
                quest.reward_choice_items[i].0,
                quest.reward_choice_items[i].1,
            );
        }
        rewards.choice_item_types = quest.reward_choice_item_types;

        // Check if all objectives are done — C++ GetQuestStatus == QUEST_STATUS_COMPLETE.
        let is_complete = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses.get(&quest_id).map(|qs| qs.status))
            == Some(QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Not all objectives done — send "you still need X" dialog
            // Legacy non-canonical note: SendQuestGiverRequestItems(quest, guid, canComplete=false, false)
            self.send_packet(&QuestGiverRequestItems {
                giver_guid: guid,
                giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
                quest_id,
                comp_emote_delay: 0,
                comp_emote_type: 0,
                quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
                suggested_party_members: quest.suggested_group_num,
                money_to_get: 0,
                collect: Vec::new(),
                currency: Vec::new(),
                status_flags: 0xFD,
                title: quest.log_title.clone(),
                completion_text: quest.area_description.clone(),
                auto_launched: false,
            });
            return;
        }

        // All objectives done — show offer reward dialog
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_CHOOSE_REWARD — player clicks "Complete Quest" in reward dialog.
    /// Gives XP, gold, items. Removes quest from active log.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverChooseReward
    #[cfg(test)]
    pub async fn handle_quest_giver_choose_reward(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_giver_choose_reward_with_generator_like_cpp(
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    pub async fn handle_quest_giver_choose_reward_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("ChooseReward: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let choice = match Self::read_quest_choice_item_like_cpp(&mut pkt) {
            Ok(choice) => choice,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "ChooseReward: failed to read C++ QuestChoiceItem"
                );
                return;
            }
        };
        let choice_item_id = choice.item_id;

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            choice_item_id,
            choice_loot_type = choice.loot_item_type,
            "Received QuestGiverChooseReward like C++"
        );

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let quest = match quest_store.get(quest_id) {
            Some(q) => q.clone(),
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "ChooseReward: quest disabled"
            );
            return;
        }

        // C++ `Player::CanRewardQuest`: player must have the quest active and COMPLETE.
        let quest_status = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses.get(&quest_id).map(|qs| qs.status));
        match quest_status {
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => {}
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: quest not complete yet"
                );
                return;
            }
            _ => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: player doesn't have quest"
                );
                return;
            }
        }

        // Validate choice item — C# HandleQuestgiverChooseReward lines 255-310
        // If client sends a non-zero choice item, it must be in reward_choice_items.
        if choice_item_id != 0 {
            if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                && choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
            {
                warn!(
                    account = self.account_id,
                    quest_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: unsupported C++ LootItemType"
                );
                return;
            }
            if !self.represented_reward_choice_template_exists_like_cpp(choice) {
                warn!(
                    account = self.account_id,
                    quest_id,
                    choice_item_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: selected reward item/currency template does not exist"
                );
                return;
            }
            let valid =
                Self::represented_reward_choice_matches_loaded_type_like_cpp(&quest, choice)
                    || self.represented_quest_package_choice_matches_like_cpp(&quest, choice);
            if !valid {
                warn!(
                    account = self.account_id,
                    quest_id,
                    choice_item_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: choice item not valid for this quest (possible exploit)"
                );
                return;
            }
        }

        // C++ HandleQuestgiverChooseRewardOpcode keeps `object = _player` for auto-complete,
        // but non-auto-complete quests must resolve the packet source as an involved
        // Unit/GameObject and pass CanInteractWithQuestGiver before RewardQuest mutates state.
        // This represented-partial slice intentionally keeps bounded choice/package validation
        // only; full CanRewardQuest/RewardQuest side effects remain open.
        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
            && !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "ChooseReward: represented involved source rejected"
            );
            return;
        }

        if !self.represented_can_reward_quest_inventory_like_cpp(&quest, choice) {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "ChooseReward: represented reward inventory validation rejected like C++"
            );
            return;
        }

        let rewarded = self
            .reward_represented_quest_with_generator_like_cpp(
                item_guid_generator,
                &quest,
                guid,
                choice,
            )
            .await;
        if rewarded {
            Box::pin(
                self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                ),
            )
            .await;
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────
}
