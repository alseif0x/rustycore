// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character query handlers that answer from the hub and the object catalogs.
//!
//! C++ source of truth: `src/server/game/Handlers/QueryHandler.cpp`. The
//! creature/gameobject templates and the realm names already live in the Core
//! hub (catalogs and connection identity), so the World session only builds
//! the borrowed context (#1263 F5). The page-text, pet-name and player-name
//! bodies also answer from the hub plus the lifecycle name-cache port. The
//! corpse and gossip bodies stay in the World shell while they need the shell
//! condition and interaction orchestration.

use std::sync::Arc;
use tracing::debug;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::query::GameObjectStats;
use wow_packet::packets::query::{
    CorpseLocation, CorpseTransportQuery, NameCacheLookupResult, PageTextInfo,
    PlayerGuidLookupData, QueryCorpseLocationFromClient, QueryCorpseTransport, QueryCreature,
    QueryCreatureResponse, QueryGameObject, QueryGameObjectResponse, QueryPageText,
    QueryPageTextResponse, QueryPetName, QueryPetNameResponse, QueryPlayerNames,
    QueryPlayerNamesResponse, QueryRealmName, RealmQueryResponse,
};
use wow_packet::packets::query::{CreatureDisplayStats, CreatureStats, CreatureXDisplay};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::directory::PlayerNameQuerySnapshotLikeCpp;
use wow_world_core::session::{HubMut, ObjectMgrCatalogsLikeCpp, PacketPublicationAccessLikeCpp};
use wow_world_lifecycle::SessionLifecycleState;

/// Borrowed inputs of one character query handler invocation.
pub struct CharacterQueryHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    /// Object-manager catalogs lent by the dispatch host.
    object_mgr: Arc<ObjectMgrCatalogsLikeCpp>,
    /// Lifecycle state lent by the dispatch host for the name-cache port.
    lifecycle: &'a SessionLifecycleState,
}

impl<'a> CharacterQueryHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        object_mgr: Arc<ObjectMgrCatalogsLikeCpp>,
        lifecycle: &'a SessionLifecycleState,
    ) -> Self {
        Self {
            hub,
            object_mgr,
            lifecycle,
        }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// CMSG_QUERY_CREATURE — answer the creature template query.
    pub async fn handle_query_creature(&mut self, query: QueryCreature) {
        let catalogs = Arc::clone(&self.object_mgr);

        let row = match catalogs
            .creature
            .resolve_like_cpp(query.creature_id, &self.hub.shared().core.locale)
        {
            Some(row) => row,
            None => {
                self.publication_like_cpp()
                    .send_packet(&QueryCreatureResponse {
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

        self.publication_like_cpp()
            .send_packet(&QueryCreatureResponse {
                creature_id: query.creature_id,
                allow: true,
                stats: Some(stats),
            });
    }

    /// CMSG_QUERY_GAME_OBJECT — answer the gameobject template query.
    pub async fn handle_query_game_object(&mut self, query: QueryGameObject) {
        let catalogs = Arc::clone(&self.object_mgr);

        let row = match catalogs
            .gameobject
            .resolve_like_cpp(query.game_object_id, &self.hub.shared().core.locale)
        {
            Some(row) => row,
            None => {
                self.publication_like_cpp()
                    .send_packet(&QueryGameObjectResponse {
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
            quest_items: catalogs
                .gameobject_quest_items
                .get_gameobject_quest_item_list_like_cpp(query.game_object_id)
                .into_iter()
                .flatten()
                .filter_map(|item| i32::try_from(*item).ok())
                .collect(),
            content_tuning_id: row.content_tuning_id,
        };

        self.publication_like_cpp()
            .send_packet(&QueryGameObjectResponse {
                game_object_id: query.game_object_id,
                guid: query.guid,
                allow: true,
                stats: Some(stats),
            });
    }

    /// CMSG_QUERY_REALM_NAME — answer the realm-name lookup.
    pub async fn handle_query_realm_name(&mut self, query: QueryRealmName) {
        let virtual_realm_address = query.virtual_realm_address;
        let core = &self.hub.shared().core;
        debug!(
            "QueryRealmName: VRA=0x{:08X}, ours=0x{:08X}, local={}",
            virtual_realm_address,
            core.virtual_realm_address(),
            virtual_realm_address == core.virtual_realm_address()
        );

        let response = realm_query_response_like_cpp(core, virtual_realm_address);
        self.publication_like_cpp().send_packet_realm(&response);
    }

    /// CMSG_QUERY_PAGE_TEXT — answer a page-text chain query.
    pub async fn handle_query_page_text(&mut self, query: QueryPageText) {
        let pages = self
            .object_mgr
            .page_text
            .resolve_chain_like_cpp(query.page_text_id, &self.hub.shared().core.locale);

        let pages = pages
            .into_iter()
            .map(|page| PageTextInfo {
                id: page.id,
                next_page_id: page.next_page_id,
                player_condition_id: page.player_condition_id,
                flags: page.flags,
                text: page.text,
            })
            .collect::<Vec<_>>();

        self.publication_like_cpp()
            .send_packet(&QueryPageTextResponse {
                page_text_id: query.page_text_id,
                allow: !pages.is_empty(),
                pages,
            });
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

        self.publication_like_cpp().send_packet(&response);
    }

    fn represented_query_canonical_pet_name_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> Option<(String, u32)> {
        let player_guid = self.hub.shared().core.player_guid()?;
        let key = self
            .hub
            .shared()
            .core
            .current_canonical_player_map_key_like_cpp()?;
        let manager = Arc::clone(self.hub.shared().core.canonical_map_manager.as_ref()?);
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

    /// CMSG_QUERY_PLAYER_NAMES — answer the client name-cache lookup.
    pub async fn handle_query_player_names(&mut self, query: QueryPlayerNames) {
        let port = match self.lifecycle.player_name_query_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                let players = query
                    .players
                    .iter()
                    .map(|guid| NameCacheLookupResult {
                        player: *guid,
                        result: 1,
                        data: None,
                    })
                    .collect();
                self.publication_like_cpp()
                    .send_packet_realm(&QueryPlayerNamesResponse { players });
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
            let hub = self.hub.shared();
            let core = hub.core;
            let connected = core
                .player_registry()
                .and_then(|registry| registry.player_name_query_snapshot_like_cpp(*guid))
                .or_else(|| {
                    (core.player_guid() == Some(*guid)).then(|| PlayerNameQuerySnapshotLikeCpp {
                        guid: *guid,
                        name: hub.player_name_like_cpp().unwrap_or_default(),
                        account_id: core.account_id,
                        battlenet_account_id: core.battlenet_account_id(),
                        race: hub.player_race_like_cpp(),
                        class: hub.player_class_like_cpp(),
                        sex: hub.player_gender_like_cpp(),
                        level: hub.player_level_like_cpp(),
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
            let account_guid = ObjectGuid::new(
                (wow_core::guid::HighGuid::WowAccount as i64) << 58,
                account_id as i64,
            );
            let bnet_guid = ObjectGuid::new(
                (wow_core::guid::HighGuid::BNetAccount as i64) << 58,
                battlenet_account_id as i64,
            );

            // Use the session VRA (region << 24 | battlegroup << 16 | realmId)
            // to match what every other packet sends. The wrong formula caused
            // "Unknown Entity" because the client rejected the mismatched VRA.
            let vra = core.virtual_realm_address();

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
            self.hub.shared().core.account_id
        );
        self.publication_like_cpp()
            .send_packet_realm(&QueryPlayerNamesResponse { players: results });
    }

    /// CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT — answer an invalid corpse
    /// location while the live corpse/raid lookup is not represented.
    pub async fn handle_query_corpse_location(&mut self, query: QueryCorpseLocationFromClient) {
        // C++ sends an invalid CorpseLocation when the queried player is missing,
        // has no corpse, or is not in the querying player's raid. Rust does not
        // yet have the live corpse/raid lookup needed for the valid branch.
        self.publication_like_cpp()
            .send_packet(&CorpseLocation::not_found_like_cpp(query.player));
    }

    /// CMSG_QUERY_CORPSE_TRANSPORT — answer an invalid corpse transport while
    /// the transport corpse lookup is not represented.
    pub async fn handle_query_corpse_transport(&mut self, query: QueryCorpseTransport) {
        // C++ always sends CorpseTransportQuery. Position/facing remain default
        // unless the queried player is in raid and has a corpse on this transport.
        self.publication_like_cpp()
            .send_packet(&CorpseTransportQuery::not_found_like_cpp(query.player));
    }
}

/// C++ `WorldSession::HandleQueryRealmName` response projection.
pub fn realm_query_response_like_cpp(
    core: &wow_world_core::session::SessionCore,
    virtual_realm_address: u32,
) -> RealmQueryResponse {
    match core.realm_names_for_address_like_cpp(virtual_realm_address) {
        Some((realm_name_actual, realm_name_normalized)) => RealmQueryResponse {
            virtual_realm_address,
            lookup_state: 0, // RESPONSE_SUCCESS
            realm_name_actual: realm_name_actual.to_string(),
            realm_name_normalized: realm_name_normalized.to_string(),
            is_local: virtual_realm_address == core.virtual_realm_address(),
        },
        None => RealmQueryResponse {
            virtual_realm_address,
            lookup_state: 1, // RESPONSE_FAILURE
            realm_name_actual: String::new(),
            realm_name_normalized: String::new(),
            is_local: false,
        },
    }
}

/// Builds a character query handler context from a host's hub.
pub trait CharacterQueryHandlerHostLikeCpp<C> {
    fn character_query_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> CharacterQueryHandlerCxLikeCpp<'a>;
}

fn handle_query_creature_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryCreature::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_creature(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryCreature: {e}"),
        }
    })
}

fn handle_query_game_object_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryGameObject::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_game_object(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryGameObject: {e}"),
        }
    })
}

fn handle_query_realm_name_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryRealmName::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_realm_name(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryRealmName: {e}"),
        }
    })
}

fn handle_query_page_text_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryPageText::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_page_text(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryPageText: {e}"),
        }
    })
}

fn handle_query_pet_name_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryPetName::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_pet_name(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryPetName: {e}"),
        }
    })
}

fn handle_query_player_names_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryPlayerNames::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_player_names(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryPlayerNames: {e}"),
        }
    })
}

fn handle_query_corpse_location_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryCorpseLocationFromClient::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_corpse_location(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryCorpseLocationFromClient: {e}"),
        }
    })
}

fn handle_query_corpse_transport_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match QueryCorpseTransport::read(&mut pkt) {
            Ok(query) => {
                session
                    .character_query_handler_cx_like_cpp(catalogs)
                    .handle_query_corpse_transport(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read QueryCorpseTransport: {e}"),
        }
    })
}

/// Registers the character query handlers on the packet registry.
pub fn register_character_query_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CharacterQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCreature,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_creature",
        handler: handle_query_creature_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryGameObject,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_game_object",
        handler: handle_query_game_object_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryRealmName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_realm_name",
        handler: handle_query_realm_name_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPageText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_page_text",
        handler: handle_query_page_text_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_pet_name",
        handler: handle_query_pet_name_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPlayerNames,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_player_names",
        handler: handle_query_player_names_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCorpseLocationFromClient,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_corpse_location",
        handler: handle_query_corpse_location_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCorpseTransport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_corpse_transport",
        handler: handle_query_corpse_transport_thunk::<S, C>,
    })?;
    Ok(())
}
