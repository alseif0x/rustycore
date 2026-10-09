// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the travel handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session
//! splits its lifecycle state from the hub, so no session reference crosses
//! into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends the three travel and
//! transfer entries (`CMSG_ACTIVATE_TAXI`, `CMSG_AREA_TRIGGER`,
//! `CMSG_WORLD_PORT_RESPONSE`) that moved off the legacy inventory path in
//! `crates/wow-world/src/handlers/travel/travel.rs`, delegating to the same
//! `WorldSession` operations with the same catalog destructuring the legacy
//! closures used.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{TravelHandlerCxLikeCpp, TravelHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl TravelHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn travel_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> TravelHandlerCxLikeCpp<'a> {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        TravelHandlerCxLikeCpp::new(hub, lifecycle)
    }

    fn sync_player_registry_state_after_taxi_benchmark_change_like_cpp(&mut self) {
        self.sync_player_registry_state_like_cpp();
    }

    fn handle_activate_taxi<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_activate_taxi(self, pkt).await })
    }

    fn handle_area_trigger_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_area_trigger_with_catalogs_like_cpp(
                self,
                catalogs.area_triggers.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_world_port_response_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_world_port_response_with_catalogs_like_cpp(
                self,
                catalogs.creature_spawns.as_ref(),
                catalogs.player_bootstrap.trait_node_entries.as_ref(),
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }
}
