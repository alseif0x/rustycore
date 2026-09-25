//! Quest-giver dialog and quest-query handler implementations.

use super::*;

impl WorldSession {
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

    /// CMSG_QUEST_GIVER_CLOSE_QUEST — acknowledged client close for auto-accept quest flow.
    /// C++ ref: `WorldSession::HandleQuestgiverCloseQuest`, `QuestHandler.cpp:591-601`.
    /// Represented seam only: records local `ScriptMgr::OnQuestAcknowledgeAutoAccept` evidence.

    /// CMSG_QUERY_QUEST_INFO — client asks for full quest template data by ID.
    /// Used to populate the quest log and tooltip.
    /// Legacy non-canonical note: QuestHandler.HandleQueryQuestInfo
    pub async fn handle_query_quest_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _guid = pkt.read_packed_guid(); // requester GUID (usually player)

        let quest_store = match &self.quests.store {
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
        let store = self.quests.store.as_deref();
        let quests = store.map_or_else(Vec::new, |quest_store| {
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
}
