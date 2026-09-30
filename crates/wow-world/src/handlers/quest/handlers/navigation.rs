//! Quest navigation and acknowledgement packet operations.
use super::*;

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

        let Some(quest_store) = self.quests.store.clone() else {
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

    #[cfg(any(test, feature = "test-fixtures"))]
    pub async fn handle_adventure_map_start_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self.adventure_map_poi_store().cloned().unwrap_or_else(|| {
            std::sync::Arc::new(wow_data::AdventureMapPoiStore::from_entries([]))
        });
        self.handle_adventure_map_start_quest_with_catalog_like_cpp(store.as_ref(), pkt)
            .await;
    }

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
        let _ = self.remove_represented_quest_status_like_cpp(qid);
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
}
