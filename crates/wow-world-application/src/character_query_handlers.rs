// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character query handlers that answer from the hub and the object catalogs.
//!
//! C++ source of truth: `src/server/game/Handlers/QueryHandler.cpp`. The
//! creature/gameobject templates and the realm names already live in the Core
//! hub (catalogs and connection identity), so the World session only builds
//! the borrowed context (#1263 F5). The page-text, pet-name, player-name,
//! corpse and gossip bodies stay in the World shell while they need the shell
//! condition, pet, persistence and visibility orchestration.

use std::sync::Arc;
use tracing::debug;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::query::GameObjectStats;
use wow_packet::packets::query::{CreatureDisplayStats, CreatureStats, CreatureXDisplay};
use wow_packet::packets::query::{
    QueryCreature, QueryCreatureResponse, QueryGameObject, QueryGameObjectResponse, QueryRealmName,
    RealmQueryResponse,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{HubMut, ObjectMgrCatalogsLikeCpp, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one character query handler invocation.
pub struct CharacterQueryHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    /// Object-manager catalogs lent by the dispatch host.
    object_mgr: Arc<ObjectMgrCatalogsLikeCpp>,
}

impl<'a> CharacterQueryHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, object_mgr: Arc<ObjectMgrCatalogsLikeCpp>) -> Self {
        Self { hub, object_mgr }
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
    Ok(())
}
