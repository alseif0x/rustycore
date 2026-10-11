// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the application corpse registrar (#1263 F5 deferred
//! families).
//!
//! The four `MiscHandler.cpp` death/corpse opcodes left the legacy inventory
//! path for `wow_world_application::register_corpse_handlers_like_cpp`. This
//! adapter lends that registrar the same `WorldSession` operations with the
//! same catalog destructuring the legacy closures used; the alive/ghost state,
//! the resurrection request, the teleport/transfer path and every publication
//! stay in those operations.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::CorpseHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CorpseHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn handle_resurrect_response_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { self.handle_resurrect_response(pkt).await })
    }

    fn handle_repop_request_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { self.handle_repop_request(pkt).await })
    }

    fn handle_reclaim_corpse_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { self.handle_reclaim_corpse(pkt).await })
    }

    fn handle_request_cemetery_list_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            self.handle_request_cemetery_list_with_catalog_like_cpp(
                catalogs.graveyards.as_ref(),
                pkt,
            )
            .await
        })
    }
}
