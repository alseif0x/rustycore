// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject interaction handlers that only need domain state and the hub.
//!
//! C++ source of truth: `SpellHandler.cpp:200` `HandleGameObjectUseOpcode`,
//! `SpellHandler.cpp:213` `HandleGameobjectReportUse` and `MiscHandler.cpp`
//! `HandleCloseInteraction`. The interaction state and
//! the world-entity registry are domain owners, so the World session only
//! builds the borrowed context (#1263 F5). `#1263 F5 remaining families` moved
//! the `GameObjUse` registration here too: its body stays in the World shell
//! while it needs the shell loot/spell/quest orchestration, and the host lends
//! that operation with the same catalog view the legacy closure used.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::CloseInteraction;
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{HubMut, RepresentedGameObjectAccessLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_entities::RepresentedGameObjectCriteriaEvent;
use wow_world_entities::WorldEntitiesState;
use wow_world_interaction::InteractionState;

use wow_world_entities::represented_gameobject_interaction_distance_like_cpp;

/// Borrowed inputs of one gameobject interaction handler invocation.
pub struct GameObjectHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    interaction: &'a mut InteractionState,
    world_entities: &'a mut WorldEntitiesState,
}

impl<'a> GameObjectHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        interaction: &'a mut InteractionState,
        world_entities: &'a mut WorldEntitiesState,
    ) -> Self {
        Self {
            hub,
            interaction,
            world_entities,
        }
    }

    /// CMSG_CLOSE_INTERACTION — clears the represented interaction source.
    pub async fn handle_close_interaction(&mut self, mut pkt: WorldPacket) {
        let request = match CloseInteraction::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "CloseInteraction parse failed: {error}"
                );
                return;
            }
        };

        self.interaction
            .reset_player_interaction_if_source_like_cpp(self.hub.shared(), request.source_guid);

        // C++ also clears Player::StableMaster when it matches SourceGuid. Rust
        // does not expose represented stable-master state yet.
    }

    /// CMSG_GAMEOBJ_REPORT_USE — records the client-side use of a gameobject.
    pub async fn handle_game_obj_report_use(&mut self, mut pkt: WorldPacket) {
        let gameobject_guid = match pkt.read_packed_guid() {
            Ok(guid) => guid,
            Err(e) => {
                warn!("GameObjReportUse: failed to read gameobject guid: {e}");
                return;
            }
        };

        if !gameobject_guid.is_game_object() {
            return;
        }

        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return;
        };
        if self.hub.shared().player_moved_unit_guid_like_cpp() != Some(player_guid) {
            return;
        }

        let state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(gameobject_guid);
        let interaction_distance = represented_gameobject_interaction_distance_like_cpp(
            state.and_then(|state| state.go_type),
            state.and_then(|state| state.interact_radius_override),
        );

        let gameobject_access = if self.hub.shared().core.canonical_map_manager.is_some() {
            match self
                .world_entities
                .represented_gameobject_can_interact_with_like_cpp(
                    self.hub.shared(),
                    gameobject_guid,
                    interaction_distance,
                ) {
                Some(access) => access,
                None => return,
            }
        } else {
            if !self
                .hub
                .shared()
                .core
                .client_visible_guids_like_cpp
                .contains(&gameobject_guid)
            {
                return;
            }
            let Some(position) = state.and_then(|state| state.position) else {
                return;
            };
            let Some(player_position) = self.hub.shared().player_position_like_cpp() else {
                return;
            };
            if !position.is_within_dist(&player_position, interaction_distance) {
                return;
            }
            RepresentedGameObjectAccessLikeCpp {
                entry: gameobject_guid.entry(),
                position,
            }
        };
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = gameobject_access;

        if self
            .world_entities
            .record_represented_gameobject_report_use_ai_like_cpp(gameobject_guid, player_guid)
        {
            return;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.world_entities
                .record_represented_gameobject_criteria_event_like_cpp(
                    RepresentedGameObjectCriteriaEvent::UseGameobject {
                        player_guid,
                        gameobject_entry: gameobject_access.entry,
                    },
                );
        }
    }
}

/// Builds a gameobject handler context from a host's domain state and hub.
pub trait GameObjectHandlerHostLikeCpp<C> {
    fn gameobject_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> GameObjectHandlerCxLikeCpp<'a>;

    /// C++ `GameObjectHandler.cpp` `WorldSession::HandleGameObjectUseOpcode`.
    ///
    /// `#1263 F5 remaining families`: the legacy registration closure
    /// destructured the session catalog view (`object_mgr`,
    /// `id_generators.item`, `item_valuation`), so the host receives that view
    /// here. The body stays in the World session.
    fn handle_game_obj_use_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_close_interaction_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GameObjectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .gameobject_handler_cx_like_cpp(catalogs)
            .handle_close_interaction(pkt)
            .await;
    })
}

fn handle_game_obj_report_use_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GameObjectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .gameobject_handler_cx_like_cpp(catalogs)
            .handle_game_obj_report_use(pkt)
            .await;
    })
}

/// `#1263 F5 remaining families`: the gameobject-use entry that lived in the
/// World shell's `handlers/entities/gameobject.rs`.
fn handle_game_obj_use_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GameObjectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_game_obj_use_with_catalogs_like_cpp(catalogs, pkt)
            .await
    })
}

/// Registers the gameobject interaction handlers on the packet registry.
pub fn register_gameobject_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: GameObjectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CloseInteraction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_close_interaction",
        handler: handle_close_interaction_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GameObjReportUse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_game_obj_report_use",
        handler: handle_game_obj_report_use_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GameObjUse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_game_obj_use",
        handler: handle_game_obj_use_thunk::<S, C>,
    })?;
    Ok(())
}
