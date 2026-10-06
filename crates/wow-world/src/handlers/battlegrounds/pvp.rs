// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private pvp capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    AcceptWargameInvite, BattlemasterJoinArena, BattlemasterJoinSkirmish,
};

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinArena,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_arena",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_battlemaster_join_arena_with_catalog_like_cpp(
                        catalogs.battlemaster_lists.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinSkirmish,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_skirmish",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_battlemaster_join_skirmish_with_catalog_like_cpp(
                        catalogs.battlemaster_lists.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptWargameInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_wargame_invite",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_accept_wargame_invite(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    /// CMSG_BATTLEMASTER_JOIN_ARENA — player asks to join a rated arena queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinArena`.

    pub(crate) async fn handle_battlemaster_join_arena_with_catalog_like_cpp(
        &mut self,
        battlemaster_lists: &wow_data::BattlemasterListStore,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let join = match BattlemasterJoinArena::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "BattlemasterJoinArena parse failed: {error}"
                );
                return;
            }
        };

        // C++ gates on already-in-BG, the all-arenas template, disabled arena,
        // group and leader before entering ArenaTeamMgr/queue code. Rust records
        // the bounded queue intent after those representable gates until the
        // live rated-arena manager is ported.
        let _accepted = self.battlemaster_join_arena_like_cpp(
            battlemaster_lists,
            join.team_size_index,
            join.roles,
        );
    }

    /// CMSG_BATTLEMASTER_JOIN_SKIRMISH — player asks to join an arena skirmish queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinSkirmish`.

    pub(crate) async fn handle_battlemaster_join_skirmish_with_catalog_like_cpp(
        &mut self,
        battlemaster_lists: &wow_data::BattlemasterListStore,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let join = match BattlemasterJoinSkirmish::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "BattlemasterJoinSkirmish parse failed: {error}"
                );
                return;
            }
        };

        // C++ ignores IsRated here, derives 2v2/3v3/5v5 from BgTypeId/BracketId,
        // and only applies group/leader gates when AsGroup is set. Queue add and
        // status fanout remain represented until live BattlegroundQueue is ported.
        let _accepted = self.battlemaster_join_skirmish_like_cpp(
            battlemaster_lists,
            join.bg_type_id,
            join.bracket_id,
            join.as_group,
            join.is_rated,
        );
    }

    #[cfg(test)]
    fn battlemaster_list_catalog_for_test_like_cpp(
        &self,
    ) -> std::sync::Arc<wow_data::BattlemasterListStore> {
        self.battlemaster_list_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| {
                std::sync::Arc::new(wow_data::BattlemasterListStore::from_entries([]))
            })
    }

    #[cfg(test)]
    pub async fn handle_battlemaster_join_arena(&mut self, pkt: wow_packet::WorldPacket) {
        let catalog = self.battlemaster_list_catalog_for_test_like_cpp();
        self.handle_battlemaster_join_arena_with_catalog_like_cpp(catalog.as_ref(), pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_battlemaster_join_skirmish(&mut self, pkt: wow_packet::WorldPacket) {
        let catalog = self.battlemaster_list_catalog_for_test_like_cpp();
        self.handle_battlemaster_join_skirmish_with_catalog_like_cpp(catalog.as_ref(), pkt)
            .await;
    }

    pub async fn handle_accept_wargame_invite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AcceptWargameInvite::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "AcceptWargameInvite parse failed: {error}"
                );
                return;
            }
        };

        self.accept_represented_wargame_invite_like_cpp(&packet.inviter_name);
    }
}
