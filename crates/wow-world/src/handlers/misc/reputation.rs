// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private reputation capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::reputation::{
    RequestForcedReactions, SetFactionAtWarRequest, SetFactionInactive, SetFactionNotAtWarRequest,
    SetWatchedFaction,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestForcedReactions,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_forced_reactions",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_forced_reactions(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionAtWar,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_at_war",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_faction_at_war(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionNotAtWar,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_not_at_war",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_faction_not_at_war(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionInactive,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_inactive",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_faction_inactive(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetWatchedFaction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_watched_faction",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_watched_faction(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_request_forced_reactions(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = RequestForcedReactions::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "RequestForcedReactions parse failed: {error}"
            );
            return;
        }

        let packet = self
            .reputation_mgr_like_cpp()
            .set_forced_reactions_packet_like_cpp();
        self.send_packet(&packet);
    }

    pub async fn handle_set_faction_at_war(&mut self, pkt: wow_packet::WorldPacket) {
        self.handle_set_faction_at_war_like_cpp(pkt, true).await;
    }

    pub async fn handle_set_faction_not_at_war(&mut self, pkt: wow_packet::WorldPacket) {
        self.handle_set_faction_at_war_like_cpp(pkt, false).await;
    }

    async fn handle_set_faction_at_war_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        at_war: bool,
    ) {
        let faction_index = if at_war {
            match SetFactionAtWarRequest::read(&mut pkt) {
                Ok(request) => request.faction_index,
                Err(error) => {
                    warn!(
                        account = self.account_id,
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
                        account = self.account_id,
                        "SetFactionNotAtWar parse failed: {error}"
                    );
                    return;
                }
            }
        };

        let Some(faction_store) = self.faction_store().cloned() else {
            warn!(
                account = self.account_id,
                faction_index, "SetFactionAtWar ignored without Faction.db2 store"
            );
            return;
        };
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().cloned();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();

        self.reputation_mgr_like_cpp_mut()
            .set_at_war_by_replist_like_cpp(
                u32::from(faction_index),
                at_war,
                faction_store.as_ref(),
                friendship_rep_reaction_store.as_deref(),
                race,
                class,
            );
    }

    pub async fn handle_set_faction_inactive(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SetFactionInactive::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetFactionInactive parse failed: {error}"
                );
                return;
            }
        };

        self.reputation_mgr_like_cpp_mut()
            .set_inactive_by_replist_like_cpp(request.index, request.state);
    }

    pub async fn handle_set_watched_faction(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SetWatchedFaction::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetWatchedFaction parse failed: {error}"
                );
                return;
            }
        };

        self.set_watched_faction_index_like_cpp(request.faction_index as i32);
    }
}
