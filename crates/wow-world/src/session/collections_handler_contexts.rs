// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the collections handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends `CMSG_ADD_TOY`
//! (`ToyHandler.cpp:28`) and `CMSG_USE_TOY` (`ToyHandler.cpp:54`), whose
//! represented bodies stay in the World shell.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{CollectionsHandlerCxLikeCpp, CollectionsHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CollectionsHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn collections_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> CollectionsHandlerCxLikeCpp<'a> {
        let (inventory, hub) = crate::session::split_inventory_mut(self);
        CollectionsHandlerCxLikeCpp::new(hub, inventory)
    }

    fn handle_add_toy<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_add_toy(self, pkt).await })
    }

    fn handle_use_toy_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_use_toy_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                pkt,
            )
            .await
        })
    }
}
