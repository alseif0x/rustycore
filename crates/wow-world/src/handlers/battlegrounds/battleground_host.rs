// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the battleground/PvP handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the host also lends `CMSG_BATTLEMASTER_JOIN_SKIRMISH`
//! and `CMSG_ACCEPT_WARGAME_INVITE`, whose bodies stay in the World shell.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{BattlegroundHandlerCxLikeCpp, BattlegroundHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl BattlegroundHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn battleground_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> BattlegroundHandlerCxLikeCpp<'a> {
        let (world_entities, instances, social, hub) = crate::session::split_battleground_mut(self);
        BattlegroundHandlerCxLikeCpp::new(hub, world_entities, instances, social, cfg!(test))
    }

    fn sync_player_registry_state_after_pvp_change_like_cpp(&mut self) {
        self.sync_player_registry_state_like_cpp();
    }

    fn battlemaster_lists_like_cpp(
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> &wow_data::BattlemasterListStore {
        catalogs.battlemaster_lists.as_ref()
    }

    fn handle_battlemaster_join_skirmish_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_battlemaster_join_skirmish_with_catalog_like_cpp(
                self,
                Self::battlemaster_lists_like_cpp(catalogs),
                pkt,
            )
            .await
        })
    }

    fn handle_accept_wargame_invite<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_accept_wargame_invite(self, pkt).await })
    }
}
