// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Reputation command packet handlers.
//!
//! C++ source of truth: `WorldSession::HandleRequestForcedReactions`,
//! `HandleSetFactionAtWar`, `HandleSetFactionNotAtWar`, `HandleSetFactionInactive` and
//! `HandleSetWatchedFaction` (`src/server/game/Handlers/CharacterHandler.cpp`) together with
//! `ReputationMgr::SetAtWar`, `SetInactive` and `SendForceReactions`
//! (`src/server/game/Reputation/ReputationMgr.cpp`). The family owns the packet bodies and
//! the faction catalogs it reads; the World session only builds the borrowed context from
//! its disjoint owners (#1263 F5).

use std::sync::Arc;

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_data::progression_rewards::{FactionStore, FriendshipRepReactionStore};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::reputation::{
    RequestForcedReactions, SetFactionAtWarRequest, SetFactionInactive, SetFactionNotAtWarRequest,
    SetWatchedFaction,
};
use wow_world_core::session::HubMut;

/// Borrowed inputs of one reputation handler invocation.
pub struct ReputationHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    faction_store: Option<Arc<FactionStore>>,
    friendship_rep_reaction_store: Option<Arc<FriendshipRepReactionStore>>,
}

impl<'a> ReputationHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        faction_store: Option<Arc<FactionStore>>,
        friendship_rep_reaction_store: Option<Arc<FriendshipRepReactionStore>>,
    ) -> Self {
        Self {
            hub,
            faction_store,
            friendship_rep_reaction_store,
        }
    }

    fn account_id_like_cpp(&self) -> u32 {
        self.hub.shared().core.account_id
    }

    fn publication_like_cpp(&self) -> wow_world_core::session::PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    pub async fn handle_request_forced_reactions(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = RequestForcedReactions::read(&mut pkt) {
            warn!(
                account = self.account_id_like_cpp(),
                "RequestForcedReactions parse failed: {error}"
            );
            return;
        }

        let Some(packet) = self
            .hub
            .shared()
            .with_reputation_mgr_like_cpp(|mgr| mgr.set_forced_reactions_packet_like_cpp())
        else {
            return;
        };
        self.publication_like_cpp().send_packet(&packet);
    }

    pub async fn handle_set_faction_at_war(&mut self, pkt: WorldPacket) {
        self.handle_set_faction_at_war_like_cpp(pkt, true).await;
    }

    pub async fn handle_set_faction_not_at_war(&mut self, pkt: WorldPacket) {
        self.handle_set_faction_at_war_like_cpp(pkt, false).await;
    }

    async fn handle_set_faction_at_war_like_cpp(&mut self, mut pkt: WorldPacket, at_war: bool) {
        let faction_index = if at_war {
            match SetFactionAtWarRequest::read(&mut pkt) {
                Ok(request) => request.faction_index,
                Err(error) => {
                    warn!(
                        account = self.account_id_like_cpp(),
                        "SetFactionAtWar parse failed: {error}"
                    );
                    return;
                }
            }
        } else {
            match SetFactionNotAtWarRequest::read(&mut pkt) {
                Ok(request) => request.faction_index,
                Err(error) => {
                    warn!(
                        account = self.account_id_like_cpp(),
                        "SetFactionNotAtWar parse failed: {error}"
                    );
                    return;
                }
            }
        };

        let Some(faction_store) = self.faction_store.clone() else {
            warn!(
                account = self.account_id_like_cpp(),
                faction_index, "SetFactionAtWar ignored without Faction.db2 store"
            );
            return;
        };
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store.clone();
        let race = self.hub.shared().player_race_like_cpp();
        let class = self.hub.shared().player_class_like_cpp();

        let _ = self.hub.mutate_reputation_mgr_like_cpp(|mgr| {
            mgr.set_at_war_by_replist_like_cpp(
                u32::from(faction_index),
                at_war,
                faction_store.as_ref(),
                friendship_rep_reaction_store.as_deref(),
                race,
                class,
            )
        });
    }

    pub async fn handle_set_faction_inactive(&mut self, mut pkt: WorldPacket) {
        let request = match SetFactionInactive::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SetFactionInactive parse failed: {error}"
                );
                return;
            }
        };

        let _ = self.hub.mutate_reputation_mgr_like_cpp(|mgr| {
            mgr.set_inactive_by_replist_like_cpp(request.index, request.state)
        });
    }

    pub async fn handle_set_watched_faction(&mut self, mut pkt: WorldPacket) {
        let request = match SetWatchedFaction::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SetWatchedFaction parse failed: {error}"
                );
                return;
            }
        };

        self.hub
            .set_watched_faction_index_like_cpp(request.faction_index as i32);
    }
}

/// Builds a reputation handler context from a host's owners and faction catalogs.
pub trait ReputationHandlerHostLikeCpp<C> {
    fn reputation_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> ReputationHandlerCxLikeCpp<'a>;
}

fn handle_request_forced_reactions_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ReputationHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .reputation_handler_cx_like_cpp(catalogs)
            .handle_request_forced_reactions(pkt)
            .await;
    })
}

fn handle_set_faction_at_war_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ReputationHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .reputation_handler_cx_like_cpp(catalogs)
            .handle_set_faction_at_war(pkt)
            .await;
    })
}

fn handle_set_faction_not_at_war_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ReputationHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .reputation_handler_cx_like_cpp(catalogs)
            .handle_set_faction_not_at_war(pkt)
            .await;
    })
}

fn handle_set_faction_inactive_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ReputationHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .reputation_handler_cx_like_cpp(catalogs)
            .handle_set_faction_inactive(pkt)
            .await;
    })
}

fn handle_set_watched_faction_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ReputationHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .reputation_handler_cx_like_cpp(catalogs)
            .handle_set_watched_faction(pkt)
            .await;
    })
}

/// Register the reputation command entries through their application adapter.
pub fn register_reputation_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: ReputationHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestForcedReactions,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_forced_reactions",
        handler: handle_request_forced_reactions_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionAtWar,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_at_war",
        handler: handle_set_faction_at_war_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionNotAtWar,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_not_at_war",
        handler: handle_set_faction_not_at_war_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionInactive,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_inactive",
        handler: handle_set_faction_inactive_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetWatchedFaction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_watched_faction",
        handler: handle_set_watched_faction_thunk::<S, C>,
    })?;
    Ok(())
}
