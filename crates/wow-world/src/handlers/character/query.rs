// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character and name queries, inspection responses.

use super::*;

mod catalog_queries;

impl WorldSession {
    /// CMSG_AREA_SPIRIT_HEALER_QUEUE — select an area spirit healer for resurrection.
    /// C++ ref: `WorldSession::HandleAreaSpiritHealerQueueOpcode`.
    pub async fn handle_area_spirit_healer_queue(&mut self, mut pkt: wow_packet::WorldPacket) {
        let queue = match AreaSpiritHealerQueue::read(&mut pkt) {
            Ok(queue) => queue,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AreaSpiritHealerQueue parse failed: {error}"
                );
                return;
            }
        };

        if self
            .represented_area_spirit_healer_access_like_cpp(queue.healer_guid)
            .is_none()
        {
            debug!(
                account = self.account_id,
                healer = ?queue.healer_guid,
                "AreaSpiritHealerQueue ignored without represented area spirit healer"
            );
            return;
        }

        // C++ also casts SPELL_WAITING_FOR_RESURRECT; deferred until the
        // player spell/aura runtime owns battleground spirit resurrection.
        self.set_area_spirit_healer_guid_like_cpp(queue.healer_guid);
    }

    /// CMSG_SPIRIT_HEALER_ACTIVATE — ghost uses spirit healer.
    /// C++ ref: `WorldSession::HandleSpiritHealerActivate`.
    pub async fn handle_spirit_healer_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SpiritHealerActivate::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SpiritHealerActivate parse failed: {error}"
                );
                return;
            }
        };

        let Some(_healer) = self.represented_npc_can_interact_with_like_cpp(
            request.healer,
            NPCFlags1::SPIRIT_HEALER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                healer = ?request.healer,
                "SpiritHealerActivate ignored without represented spirit healer"
            );
            return;
        };

        // C++ continues into SendSpiritResurrect here: resurrect 50%, durability
        // loss, corpse-bones spawn, and possible graveyard teleport. That player
        // corpse/death runtime is not represented in this handler yet.
        debug!(
            account = self.account_id,
            healer = ?request.healer,
            "SpiritHealerActivate validated; resurrection runtime pending"
        );
    }

    /// Shared C++ area-spirit-healer checks: creature exists, has the area
    /// spirit-healer flag, and is within MAX_AREA_SPIRIT_HEALER_RANGE.
    pub(super) fn represented_area_spirit_healer_access_like_cpp(
        &self,
        healer_guid: ObjectGuid,
    ) -> Option<crate::session::RepresentedCreatureAccessLikeCpp> {
        let access = self.canonical_creature_access_like_cpp(healer_guid)?;
        if (access.npc_flags & NPCFlags1::AREA_SPIRIT_HEALER.bits()) == 0 {
            return None;
        }

        let player_position = self.player_position_like_cpp()?;
        access
            .position
            .is_within_dist(&player_position, MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP)
            .then_some(access)
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
                let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
                    continue;
                };
                let Some(state) = self.represented_gameobject_use_states.get(&guid) else {
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


    pub async fn handle_item_text_query(&mut self, query: ItemTextQuery) {
        let response = self
            .resolved_inventory_item_object_like_cpp(query.id)
            .map(|item| QueryItemTextResponse::valid_like_cpp(query.id, item.text().to_string()))
            .unwrap_or_else(|| QueryItemTextResponse::invalid_like_cpp(query.id));

        self.send_packet(&response);
    }

    /// CMSG_QUERY_PET_NAME — resolve an in-world pet name.
    ///
    /// C++ `SendQueryPetNameResponse` uses `ObjectAccessor::GetCreatureOrPetOrVehicle`
    /// and fills the response only when that lookup succeeds. This bounded path
    /// represents the canonical normal-pet branch; creature/vehicle names and
    /// declined-name runtime are left explicit until those object-accessor paths
    /// are unified.
    pub async fn handle_query_pet_name(&mut self, query: QueryPetName) {
        let mut response = QueryPetNameResponse::not_allowed(query.unit_guid);

        if let Some((name, timestamp)) =
            self.represented_query_canonical_pet_name_like_cpp(query.unit_guid)
        {
            response.allow = true;
            response.name = name;
            response.timestamp = timestamp;
        }

        self.send_packet(&response);
    }

    fn represented_query_canonical_pet_name_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> Option<(String, u32)> {
        let player_guid = self.player_guid()?;
        let key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let managed = manager.find_map(key.map_id, key.instance_id)?;
        let name = managed.map().with_pet_like_cpp(unit_guid, |pet| {
            (pet.owner_guid() == player_guid)
                .then(|| pet.creature().unit().world().name().to_string())
        })??;
        // C++ reads UnitData::PetNameTimestamp. The canonical entity model has
        // not exposed normal-pet rename/load timestamps yet, so this bounded
        // branch preserves the default timestamp until that runtime lands.
        let timestamp = 0;
        Some((name, timestamp))
    }

    /// Handle CMSG_GOSSIP_HELLO / TalkToGossip — player right-clicks an NPC.
    ///
    /// For now, we send an empty gossip message with a default NPC text.
    /// This allows the client to show the gossip window.
    /// Handle CMSG_QUERY_PLAYER_NAMES — client requests player name data.
    ///
    /// The client sends this after receiving UpdateObject for a player whose
    /// name isn't cached. Without a response, the player's nameplate is blank.
    pub async fn handle_query_player_names(&mut self, query: QueryPlayerNames) {
        let port = match self.player_name_query_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                // Send failure response for all queried players
                let players = query
                    .players
                    .iter()
                    .map(|guid| NameCacheLookupResult {
                        player: *guid,
                        result: 1, // Failure
                        data: None,
                    })
                    .collect();
                self.send_packet_realm(&QueryPlayerNamesResponse { players });
                return;
            }
        };

        let mut results = Vec::new();

        for guid in &query.players {
            let row = match port
                .load_player_name_like_cpp(wow_persistence::PlayerNameQueryRequestLikeCpp {
                    player_guid_counter: guid.counter() as u64,
                })
                .await
            {
                wow_persistence::PlayerNameQueryOutcomeLikeCpp::Found(row) => row,
                wow_persistence::PlayerNameQueryOutcomeLikeCpp::Missing => {
                    results.push(NameCacheLookupResult {
                        player: *guid,
                        result: 1,
                        data: None,
                    });
                    continue;
                }
                wow_persistence::PlayerNameQueryOutcomeLikeCpp::Failed { .. } => {
                    results.push(NameCacheLookupResult {
                        player: *guid,
                        result: 1,
                        data: None,
                    });
                    continue;
                }
            };

            // C++ first requires CharacterCache presence, then overlays live
            // PlayerGuidLookupData for a connected target. The target's
            // account relation comes from the cache, never from the querying
            // session.
            let connected = self
                .player_registry()
                .and_then(|registry| registry.player_name_query_snapshot_like_cpp(*guid))
                .or_else(|| {
                    (self.player_guid() == Some(*guid)).then(|| {
                        crate::session::directory::PlayerNameQuerySnapshotLikeCpp {
                            guid: *guid,
                            name: self.player_name_like_cpp().unwrap_or_default(),
                            account_id: self.account_id,
                            battlenet_account_id: self.battlenet_account_id(),
                            race: self.player_race_like_cpp(),
                            class: self.player_class_like_cpp(),
                            sex: self.player_gender_like_cpp(),
                            level: self.player_level_like_cpp(),
                        }
                    })
                });
            let (name, race, sex, class, level, account_id, battlenet_account_id, is_deleted) =
                if let Some(connected) = connected {
                    (
                        connected.name,
                        connected.race,
                        connected.sex,
                        connected.class,
                        connected.level,
                        connected.account_id,
                        connected.battlenet_account_id,
                        false,
                    )
                } else {
                    (
                        row.name,
                        row.race,
                        row.sex,
                        row.class,
                        row.level,
                        row.account_id,
                        row.battlenet_account_id,
                        row.is_deleted,
                    )
                };
            let account_guid =
                ObjectGuid::new((HighGuid::WowAccount as i64) << 58, account_id as i64);
            let bnet_guid = ObjectGuid::new(
                (HighGuid::BNetAccount as i64) << 58,
                battlenet_account_id as i64,
            );

            // Use the session VRA (region << 24 | battlegroup << 16 | realmId)
            // to match what every other packet sends. The wrong formula caused
            // "Unknown Entity" because the client rejected the mismatched VRA.
            let vra = self.virtual_realm_address();

            results.push(NameCacheLookupResult {
                player: *guid,
                result: 0, // Success
                data: Some(PlayerGuidLookupData {
                    name,
                    race,
                    sex,
                    class,
                    level,
                    guid_actual: *guid,
                    account_id: account_guid,
                    bnet_account_id: bnet_guid,
                    virtual_realm_address: vra,
                    is_deleted,
                    ..Default::default()
                }),
            });
        }

        debug!(
            "QueryPlayerNames: {} queries, {} found for account {}",
            query.players.len(),
            results.iter().filter(|r| r.result == 0).count(),
            self.account_id
        );
        self.send_packet_realm(&QueryPlayerNamesResponse { players: results });
    }

    pub fn handle_query_realm_name(&mut self, query: QueryRealmName) {
        debug!(
            "QueryRealmName: VRA=0x{:08X}, ours=0x{:08X}, local={}",
            query.virtual_realm_address,
            self.virtual_realm_address(),
            query.virtual_realm_address == self.virtual_realm_address()
        );

        let resp = self.realm_query_response_like_cpp(query.virtual_realm_address);
        self.send_packet_realm(&resp);
    }

    pub(crate) fn realm_query_response_like_cpp(
        &self,
        virtual_realm_address: u32,
    ) -> RealmQueryResponse {
        if let Some((realm_name_actual, realm_name_normalized)) =
            self.realm_names_for_address_like_cpp(virtual_realm_address)
        {
            RealmQueryResponse {
                virtual_realm_address,
                lookup_state: 0, // RESPONSE_SUCCESS
                realm_name_actual: realm_name_actual.to_string(),
                realm_name_normalized: realm_name_normalized.to_string(),
                is_local: virtual_realm_address == self.virtual_realm_address(),
            }
        } else {
            RealmQueryResponse {
                virtual_realm_address,
                lookup_state: 1, // RESPONSE_FAILURE
                realm_name_actual: String::new(),
                realm_name_normalized: String::new(),
                is_local: false,
            }
        }
    }

    /// CMSG_AREA_SPIRIT_HEALER_QUERY — ask an area spirit healer for resurrection timer.
    /// C++ ref: `WorldSession::HandleAreaSpiritHealerQueryOpcode`.
    pub async fn handle_area_spirit_healer_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let query = match AreaSpiritHealerQuery::read(&mut pkt) {
            Ok(query) => query,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AreaSpiritHealerQuery parse failed: {error}"
                );
                return;
            }
        };

        let Some(access) = self.represented_area_spirit_healer_access_like_cpp(query.healer_guid)
        else {
            debug!(
                account = self.account_id,
                healer = ?query.healer_guid,
                "AreaSpiritHealerQuery ignored without represented area spirit healer"
            );
            return;
        };

        // C++ sends the current shared channel timer or the individual aura
        // duration after casting SPELL_SPIRIT_HEAL_PLAYER_AURA. Spell/aura/channel
        // runtime is still outside this represented handler, so the packet shape
        // and validation are ported and the timer remains zero for now.
        if (access.npc_flags2
            & wow_constants::unit::NPCFlags2::AREA_SPIRIT_HEALER_INDIVIDUAL.bits())
            != 0
        {
            debug!(
                account = self.account_id,
                healer = ?query.healer_guid,
                "AreaSpiritHealerQuery individual aura/channel timer is not represented yet"
            );
        }

        self.send_packet(&AreaSpiritHealerTime {
            healer_guid: query.healer_guid,
            time_left_ms: 0,
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
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
            self.account_id
        );

        let visible_guids: Vec<ObjectGuid> = self
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
            self.account_id
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

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "query_quest_status_tests.rs"]
mod quest_status_tests;

#[cfg(test)]
#[path = "query_area_healer_tests.rs"]
mod area_healer_tests;
