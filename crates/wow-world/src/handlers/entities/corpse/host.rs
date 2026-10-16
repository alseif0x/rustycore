// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the application corpse registrar (#1263 F5 deferred
//! families).
//!
//! The four `MiscHandler.cpp` death/corpse opcodes left the legacy inventory
//! path for `wow_world_entities::register_corpse_handlers_like_cpp`. This
//! adapter lends that registrar the same `WorldSession` operations with the
//! same catalog destructuring the legacy closures used; the alive/ghost state,
//! the resurrection request, the teleport/transfer path and every publication
//! stay in those operations.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_entities::CorpseHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CorpseHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn handle_resurrect_response_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { wow_world_entities::handle_resurrect_response(self, pkt).await })
    }

    fn handle_repop_request_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { wow_world_entities::handle_repop_request(self, pkt).await })
    }

    fn handle_reclaim_corpse_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { wow_world_entities::handle_reclaim_corpse(self, pkt).await })
    }

    fn handle_request_cemetery_list_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            wow_world_entities::handle_request_cemetery_list_with_catalog_like_cpp(
                self,
                catalogs.graveyards.as_ref(),
                pkt,
            )
            .await
        })
    }
}

impl wow_world_entities::CorpseHostLikeCpp for WorldSession {
    fn graveyard_conditions_meet_like_cpp(
        &mut self,
        conditions_ref: &wow_data::ConditionsReference,
    ) -> bool {
        WorldSession::graveyard_conditions_meet_like_cpp(self, conditions_ref)
    }

    #[cfg(test)]
    fn represented_resurrection_requested_by_like_cpp(
        &self,
        resurrecter: wow_core::ObjectGuid,
    ) -> bool {
        WorldSession::represented_resurrection_requested_by_like_cpp(self, resurrecter)
    }

    fn schedule_represented_resurrection_after_teleport_like_cpp(
        &mut self,
        request: wow_entities::PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        wow_world_entities::schedule_represented_resurrection_after_teleport_like_cpp(self, request)
    }

    fn teleport_to(
        &mut self,
        new_map: u32,
        new_pos: wow_core::Position,
    ) -> impl std::future::Future<Output = ()> + Send {
        WorldSession::teleport_to(self, new_map, new_pos)
    }

    fn pending_teleport_like_cpp(&self) -> Option<(u32, wow_core::Position)> {
        self.lifecycle
            .pending_teleport_like_cpp(crate::session::hub_ref(self))
    }
}
