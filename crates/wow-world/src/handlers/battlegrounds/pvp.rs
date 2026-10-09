// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private pvp capability handlers extracted from the legacy misc owner.
//!
//! `#1263 F5 remaining families`: the `CMSG_BATTLEMASTER_JOIN_SKIRMISH` and
//! `CMSG_ACCEPT_WARGAME_INVITE` registrations moved to the
//! `ApplicationBattleground` area registrar; the bodies stay here.

use tracing::warn;

use wow_packet::ClientPacket;
use wow_packet::packets::misc::{AcceptWargameInvite, BattlemasterJoinSkirmish};

impl crate::session::WorldSession {
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
