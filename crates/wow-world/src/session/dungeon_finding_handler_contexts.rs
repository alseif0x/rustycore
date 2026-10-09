// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the dungeon-finder handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends `CMSG_DF_GET_SYSTEM_INFO`,
//! whose body stays in the World shell.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{DungeonFindingHandlerCxLikeCpp, DungeonFindingHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl DungeonFindingHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn dungeon_finding_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> DungeonFindingHandlerCxLikeCpp<'a> {
        DungeonFindingHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    fn handle_df_get_system_info_with_catalog_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_df_get_system_info_with_catalog_like_cpp(
                self,
                catalogs.lfg_dungeons.as_ref(),
                pkt,
            )
            .await
        })
    }
}
