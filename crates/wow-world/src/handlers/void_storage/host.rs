// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the inventory void-storage registrar (#1263 F5
//! deferred families).
//!
//! The four `VoidStorageHandler.cpp` opcodes left the legacy inventory path for
//! `wow_world_inventory::register_void_storage_handlers_like_cpp`. This adapter
//! lends that registrar the same `WorldSession` operations with the same
//! catalog destructuring the legacy closures used; NPC admission, the
//! exclusive money-persistence fence, the void-storage persistence port and
//! every publication stay in those operations.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_inventory::VoidStorageHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl VoidStorageHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn handle_void_storage_unlock_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            self.handle_void_storage_unlock_with_generator_like_cpp(
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_void_storage_query_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { self.handle_void_storage_query(pkt).await })
    }

    fn handle_void_storage_transfer_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            self.handle_void_storage_transfer_with_generators_like_cpp(
                catalogs.id_generators.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_void_storage_swap_item_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { self.handle_void_storage_swap_item(pkt).await })
    }
}
