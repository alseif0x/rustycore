// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character and name queries, inspection responses.

use super::*;

impl WorldSession {
    /// Handle CMSG_DB_QUERY_BULK — client requests DB2 records.
    ///
    /// TrinityCore only sends a Valid `DBReply` when `sDB2Manager.GetStorage`
    /// returns typed storage and that storage can serialize the record through
    /// `DB2StorageBase::WriteRecord`. Rust's `HotfixBlobCache` stores raw
    /// WDC4/DB2 record bytes, which are not the same wire format. Only typed
    /// stores implemented here may answer Valid; missing typed storage follows
    /// the C++ Invalid branch and lets the client use its local DB2 cache.
    pub async fn handle_db_query_bulk(&mut self, query: wow_packet::packets::misc::DbQueryBulk) {
        info!(
            "DbQueryBulk: table=0x{:08X}, {} records {:?} for account {}",
            query.table_hash,
            query.queries.len(),
            query.queries,
            self.account_id
        );
        for record_id in &query.queries {
            if query.table_hash == TACT_KEY_TABLE_HASH_LIKE_CPP {
                let tact_key = (*record_id)
                    .try_into()
                    .ok()
                    .and_then(|id| self.tact_key_store().and_then(|store| store.get(id)));
                if let Some(entry) = tact_key {
                    debug!(
                        "DbQueryBulk: TactKey.db2 record={} -> Valid(1), 16-byte typed WriteRecord payload",
                        record_id
                    );
                    self.send_packet_realm(&DBReply::found(
                        query.table_hash,
                        *record_id,
                        entry.key.to_vec(),
                    ));
                    continue;
                }
                debug!(
                    "DbQueryBulk: NOT_FOUND TactKey.db2 record={} -> Invalid(3), client may use local DB2 cache",
                    record_id
                );
            } else {
                info!(
                    "DbQueryBulk: table=0x{:08X} record={} -> Invalid(3), no typed DB2 storage serializer",
                    query.table_hash, record_id
                );
            }
            // RecordRemoved(2) would tell the client to delete the record from its cache,
            // which is wrong for client-local DB2 rows missing from server typed storage.
            self.send_packet_realm(&DBReply::not_found(query.table_hash, *record_id));
        }
    }

    /// Handle CMSG_QUERY_CREATURE — client requests creature template data.
    ///
    /// The client sends this automatically after receiving an UpdateObject with
    /// unknown creature entries. Without a response, NPC names don't display
    /// and interaction menus don't work.
    pub async fn handle_query_creature(&mut self, query: QueryCreature) {
        // If already responded, skip — client caches locally after first response
        if self.creature_query_cache.contains(&query.creature_id) {
            return;
        }
        self.creature_query_cache.insert(query.creature_id);

        let port = match self.creature_query_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                self.send_packet(&QueryCreatureResponse {
                    creature_id: query.creature_id,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let row = match port
            .load_creature_query_catalog_like_cpp(
                wow_persistence::CreatureQueryCatalogRequestLikeCpp {
                    entry: query.creature_id,
                    locale: self.locale.clone(),
                },
            )
            .await
        {
            wow_persistence::CreatureQueryCatalogOutcomeLikeCpp::Found { row, locale_error } => {
                if let Some(error) = locale_error {
                    warn!(
                        "Failed to query creature locale for {}: {error}",
                        query.creature_id
                    );
                }
                row
            }
            wow_persistence::CreatureQueryCatalogOutcomeLikeCpp::Failed { reason } => {
                debug!(
                    "Failed to query creature template {}: {reason}",
                    query.creature_id
                );
                self.send_packet(&QueryCreatureResponse {
                    creature_id: query.creature_id,
                    allow: false,
                    stats: None,
                });
                return;
            }
            wow_persistence::CreatureQueryCatalogOutcomeLikeCpp::Missing => {
                self.send_packet(&QueryCreatureResponse {
                    creature_id: query.creature_id,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let total_probability = row.displays.iter().map(|display| display.probability).sum();
        let displays = row
            .displays
            .iter()
            .map(|display| CreatureXDisplay {
                creature_display_id: display.display_id,
                scale: display.scale,
                probability: display.probability,
            })
            .collect();

        let mut names: [String; 4] = Default::default();
        names[0] = row.name;

        let stats = CreatureStats {
            title: row.subname,
            title_alt: row.title_alt,
            cursor_name: row.icon_name,
            civilian: row.civilian,
            leader: row.racial_leader,
            names,
            name_alts: Default::default(),
            flags: row.type_flags,
            creature_type: row.creature_type,
            creature_family: row.creature_family,
            classification: row.classification,
            proxy_creature_ids: row.kill_credits,
            display: CreatureDisplayStats {
                displays,
                total_probability,
            },
            hp_multi: row.hp_multi,
            energy_multi: row.energy_multi,
            quest_items: Vec::new(),
            creature_movement_info_id: row.movement_id,
            health_scaling_expansion: 0,
            required_expansion: row.required_expansion,
            vignette_id: row.vignette_id,
            unit_class: row.unit_class,
            creature_difficulty_id: row.creature_difficulty_id,
            widget_set_id: row.widget_set_id,
            widget_set_unit_condition_id: row.widget_set_unit_condition_id,
        };

        self.send_packet(&QueryCreatureResponse {
            creature_id: query.creature_id,
            allow: true,
            stats: Some(stats),
        });
    }

    /// Handle CMSG_QUERY_GAME_OBJECT — client requests gameobject template data.
    pub async fn handle_query_game_object(
        &mut self,
        query: wow_packet::packets::query::QueryGameObject,
    ) {
        let port = match self.gameobject_query_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                self.send_packet(&QueryGameObjectResponse {
                    game_object_id: query.game_object_id,
                    guid: query.guid,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let row = match port
            .load_gameobject_query_catalog_like_cpp(
                wow_persistence::GameObjectQueryCatalogRequestLikeCpp {
                    entry: query.game_object_id,
                    locale: self.locale.clone(),
                },
            )
            .await
        {
            wow_persistence::GameObjectQueryCatalogOutcomeLikeCpp::Found {
                row,
                locale_error,
                quest_items_error,
            } => {
                if let Some(error) = locale_error {
                    debug!(
                        "Failed to query gameobject locale {} {}: {error}",
                        query.game_object_id, self.locale
                    );
                }
                if let Some(error) = quest_items_error {
                    debug!(
                        "Failed to query gameobject quest items {}: {error}",
                        query.game_object_id
                    );
                }
                row
            }
            wow_persistence::GameObjectQueryCatalogOutcomeLikeCpp::Failed { reason } => {
                debug!(
                    "Failed to query gameobject template {}: {reason}",
                    query.game_object_id
                );
                self.send_packet(&QueryGameObjectResponse {
                    game_object_id: query.game_object_id,
                    guid: query.guid,
                    allow: false,
                    stats: None,
                });
                return;
            }
            wow_persistence::GameObjectQueryCatalogOutcomeLikeCpp::Missing => {
                self.send_packet(&QueryGameObjectResponse {
                    game_object_id: query.game_object_id,
                    guid: query.guid,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let mut names: [String; 4] = Default::default();
        names[0] = row.name;

        let stats = GameObjectStats {
            names,
            icon_name: row.icon_name,
            cast_bar_caption: row.cast_bar_caption,
            unk_string: row.unk_string,
            go_type: row.go_type,
            display_id: row.display_id,
            data: row.data,
            size: row.size,
            quest_items: row.quest_items,
            content_tuning_id: row.content_tuning_id,
        };

        self.send_packet(&QueryGameObjectResponse {
            game_object_id: query.game_object_id,
            guid: query.guid,
            allow: true,
            stats: Some(stats),
        });
    }

    pub async fn handle_query_page_text(&mut self, query: QueryPageText) {
        let port = match self.page_text_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                self.send_packet(&QueryPageTextResponse {
                    page_text_id: query.page_text_id,
                    allow: false,
                    pages: Vec::new(),
                });
                return;
            }
        };

        let outcome = port
            .load_page_text_catalog_like_cpp(wow_persistence::PageTextCatalogRequestLikeCpp {
                page_text_id: query.page_text_id,
                locale: self.locale.clone(),
            })
            .await;
        for diagnostic in outcome.diagnostics {
            match diagnostic {
                wow_persistence::PageTextCatalogDiagnosticLikeCpp::PageReadFailed {
                    page_text_id,
                    reason,
                } => debug!("Failed to query page text {page_text_id}: {reason}"),
                wow_persistence::PageTextCatalogDiagnosticLikeCpp::LocaleReadFailed {
                    page_text_id,
                    locale,
                    reason,
                } => debug!("Failed to query page text locale {page_text_id} {locale}: {reason}"),
            }
        }
        let pages = outcome
            .pages
            .into_iter()
            .map(|page| PageTextInfo {
                id: page.id,
                next_page_id: page.next_page_id,
                player_condition_id: page.player_condition_id,
                flags: page.flags,
                text: page.text,
            })
            .collect::<Vec<_>>();

        self.send_packet(&QueryPageTextResponse {
            page_text_id: query.page_text_id,
            allow: !pages.is_empty(),
            pages,
        });
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

    pub(crate) fn represented_query_canonical_pet_name_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> Option<(String, u32)> {
        let player_guid = self.player_guid()?;
        let key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let managed = manager.find_map(key.map_id, key.instance_id)?;
        let pet = managed.map().map_object_record(unit_guid)?.pet()?;
        if pet.owner_guid() != player_guid {
            return None;
        }

        let name = pet.creature().unit().world().name().to_string();
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

            // Build account GUIDs (simplified — just use account_id)
            let account_id_val = self.account_id as i64;
            let account_guid = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, account_id_val);
            let bnet_guid = ObjectGuid::new((HighGuid::BNetAccount as i64) << 58, account_id_val);

            // Use the session VRA (region << 24 | battlegroup << 16 | realmId)
            // to match what every other packet sends. The wrong formula caused
            // "Unknown Entity" because the client rejected the mismatched VRA.
            let vra = self.virtual_realm_address();

            results.push(NameCacheLookupResult {
                player: *guid,
                result: 0, // Success
                data: Some(PlayerGuidLookupData {
                    name: row.name,
                    race: row.race,
                    sex: row.sex,
                    class: row.class,
                    level: row.level,
                    guid_actual: *guid,
                    account_id: account_guid,
                    bnet_account_id: bnet_guid,
                    virtual_realm_address: vra,
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
    pub async fn handle_quest_giver_status_multiple_query(&mut self) {
        trace!(
            "QuestGiverStatusMultipleQuery from account {}",
            self.account_id
        );

        let visible_guids: Vec<ObjectGuid> = self
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .collect();
        let statuses = self.collect_quest_giver_status_multiple_like_cpp(visible_guids);
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
    pub async fn handle_quest_giver_status_tracked_query(&mut self, mut pkt: WorldPacket) {
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

        let statuses = self.collect_quest_giver_status_multiple_like_cpp(quest_giver_guids);
        self.send_packet(&QuestGiverStatusMultiple { statuses });
    }
}
