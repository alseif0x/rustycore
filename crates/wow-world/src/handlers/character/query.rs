// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character and name queries, inspection responses.

use super::*;

#[cfg(test)]
mod test_shims;

impl WorldSession {
    /// CMSG_SPIRIT_HEALER_ACTIVATE — ghost uses spirit healer.
    /// C++ ref: `WorldSession::HandleSpiritHealerActivate`.
    pub async fn handle_spirit_healer_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SpiritHealerActivate::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "SpiritHealerActivate parse failed: {error}"
                );
                return;
            }
        };

        let Some(_healer) = crate::session::hub_ref(self)
            .represented_npc_can_interact_with_like_cpp(
                request.healer,
                NPCFlags1::SPIRIT_HEALER.bits(),
                0,
            )
        else {
            debug!(
                account = self.core.account_id,
                healer = ?request.healer,
                "SpiritHealerActivate ignored without represented spirit healer"
            );
            return;
        };

        // C++ continues into SendSpiritResurrect here: resurrect 50%, durability
        // loss, corpse-bones spawn, and possible graveyard teleport. That player
        // corpse/death runtime is not represented in this handler yet.
        debug!(
            account = self.core.account_id,
            healer = ?request.healer,
            "SpiritHealerActivate validated; resurrection runtime pending"
        );
    }

    pub(super) fn collect_quest_giver_status_multiple_like_cpp(
        &self,
        quest_info: &wow_data::progression_rewards::QuestInfoStore,
        guids: impl IntoIterator<Item = ObjectGuid>,
    ) -> Vec<(ObjectGuid, u64)> {
        let mut statuses = Vec::new();

        for guid in guids {
            if guid.is_any_type_creature() {
                let Some(access) = self.canonical_creature_access_like_cpp(guid) else {
                    continue;
                };
                if (access.npc_flags & NPCFlags1::QUEST_GIVER.bits()) == 0 {
                    continue;
                }

                let status = self.get_represented_quest_giver_status_with_catalog_like_cpp(
                    Some(quest_info),
                    RepresentedQuestGiverStatusSourceLikeCpp::Creature {
                        entry: access.entry,
                    },
                );
                statuses.push((guid, status));
                continue;
            }

            if guid.is_game_object() {
                let Some(access) = self.core.canonical_gameobject_access_like_cpp(guid) else {
                    continue;
                };
                let Some(state) = self
                    .world_entities
                    .represented_gameobject_use_state_like_cpp(guid)
                else {
                    continue;
                };
                if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_QUESTGIVER) {
                    continue;
                }

                let status = self.get_represented_quest_giver_status_with_catalog_like_cpp(
                    Some(quest_info),
                    RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                        entry: access.entry,
                    },
                );
                statuses.push((guid, status));
            }
        }

        statuses
    }

    /// Send SMSG_QUEST_GIVER_STATUS for a single NPC.
    #[allow(dead_code)]
    fn send_quest_giver_status(&self, guid: ObjectGuid, status: u32) {
        use wow_constants::ServerOpcodes;
        let mut pkt = wow_packet::WorldPacket::new_server(ServerOpcodes::QuestGiverStatus);
        pkt.write_packed_guid(&guid);
        pkt.write_uint32(status);
        self.send_raw_packet(&pkt.into_data());
    }

    /// Handle CMSG_DB_QUERY_BULK — client requests DB2 records.
    ///
    /// TrinityCore only sends a Valid `DBReply` when `sDB2Manager.GetStorage`
    /// returns typed storage and that storage can serialize the record through
    /// `DB2StorageBase::WriteRecord`. Rust's `HotfixBlobCache` stores raw
    /// WDC4/DB2 record bytes, which are not the same wire format. Only typed
    /// stores implemented here may answer Valid; missing typed storage follows
    /// the C++ Invalid branch and lets the client use its local DB2 cache.

    /// Test-only entry point for the DB2 bulk-query handler owned by
    /// `wow-world-application` (#1263 F5).
    #[cfg(test)]
    pub async fn handle_db_query_bulk(&mut self, query: wow_packet::packets::misc::DbQueryBulk) {
        let tact_keys = self
            .tact_key_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| Arc::new(wow_data::TactKeyStore::from_entries([])));
        let hotfixes = wow_data::HotfixBlobCache::new();
        wow_world_application::DataServiceHandlerCxLikeCpp::new(
            crate::session::hub_mut(self),
            &hotfixes,
            tact_keys.as_ref(),
        )
        .handle_db_query_bulk(query)
        .await;
    }

    pub async fn handle_item_text_query(&mut self, query: ItemTextQuery) {
        self.build_item_text_query_handler_cx_like_cpp()
            .handle_item_text_query(query);
    }

    #[cfg(test)]
    pub async fn handle_quest_giver_status_multiple_query(&mut self) {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        self.handle_quest_giver_status_multiple_query_with_catalog_like_cpp(
            catalogs.quest_info.as_ref(),
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_quest_giver_status_tracked_query(&mut self, pkt: WorldPacket) {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        self.handle_quest_giver_status_tracked_query_with_catalog_like_cpp(
            catalogs.quest_info.as_ref(),
            pkt,
        )
        .await;
    }

    /// Handle CMSG_QUEST_GIVER_STATUS_MULTIPLE_QUERY — client asks quest status for visible questgivers.
    ///
    /// C++ anchors:
    /// - `Player::SendQuestGiverStatusMultiple`, `Player.cpp:16804-16837`.
    /// - `QuestGiverStatusMultiple::Write`, `QuestPackets.cpp:64-74`.
    ///
    /// Ownership/sync: represented `client_visible_guids_like_cpp` + canonical map access + read-only
    /// `QuestStore` relations -> one outbound packet only. This handler must not mutate map,
    /// QuestStore, ObjectAccessor/GameEvent, or player state. Exact Creature hostility/faction remains
    /// a documented gap; represented Creature NPC QUEST_GIVER flag is enforced when available.
    pub async fn handle_quest_giver_status_multiple_query_with_catalog_like_cpp(
        &mut self,
        quest_info: &wow_data::progression_rewards::QuestInfoStore,
    ) {
        trace!(
            "QuestGiverStatusMultipleQuery from account {}",
            self.core.account_id
        );

        let visible_guids: Vec<ObjectGuid> = self
            .core
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .collect();
        let statuses = self.collect_quest_giver_status_multiple_like_cpp(quest_info, visible_guids);
        self.send_packet(&QuestGiverStatusMultiple { statuses });
    }

    /// Handle CMSG_QUEST_GIVER_STATUS_TRACKED_QUERY — client supplies questgiver GUIDs to query.
    ///
    /// C++ anchors:
    /// - `QuestGiverStatusTrackedQuery::Read`, `QuestPackets.cpp:40-54`.
    /// - `WorldSession::HandleQuestgiverStatusTrackedQueryOpcode`, `QuestHandler.cpp:775-778`.
    /// - `Player::SendQuestGiverStatusMultiple`, `Player.cpp:16809-16837`.
    ///
    /// Ownership/sync: client packet GUID set -> represented canonical Creature/GameObject access +
    /// read-only `QuestStore` status -> one outbound packet only. This must not read the visible GUID
    /// cache and must not mutate map, QuestStore, ObjectAccessor/GameEvent, player quest state, or
    /// represented visibility state.
    pub async fn handle_quest_giver_status_tracked_query_with_catalog_like_cpp(
        &mut self,
        quest_info: &wow_data::progression_rewards::QuestInfoStore,
        mut pkt: WorldPacket,
    ) {
        trace!(
            "QuestGiverStatusTrackedQuery from account {}",
            self.core.account_id
        );

        let guid_count = match pkt.read_uint32() {
            Ok(guid_count) => guid_count,
            Err(e) => {
                warn!("Malformed QuestGiverStatusTrackedQuery count: {e}");
                return;
            }
        };

        if guid_count > QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP {
            warn!(
                guid_count,
                max = QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP,
                "QuestGiverStatusTrackedQuery exceeds C++ max capacity"
            );
            return;
        }

        let mut quest_giver_guids = HashSet::with_capacity(guid_count as usize);
        for _ in 0..guid_count {
            match pkt.read_packed_guid() {
                Ok(guid) => {
                    quest_giver_guids.insert(guid);
                }
                Err(e) => {
                    warn!("Malformed QuestGiverStatusTrackedQuery packed GUID: {e}");
                    return;
                }
            }
        }

        let statuses =
            self.collect_quest_giver_status_multiple_like_cpp(quest_info, quest_giver_guids);
        self.send_packet(&QuestGiverStatusMultiple { statuses });
    }
}
