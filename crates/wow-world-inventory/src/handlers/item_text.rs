// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_handler::HandlerFuture;
use wow_packet::packets::query::{ItemTextQuery, QueryItemTextResponse};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{OwnedInventoryAccessLikeCpp, PacketPublicationAccessLikeCpp};

use super::InventoryHandlerHostLikeCpp;
use crate::InventoryState;

/// Borrowed participants for one complete runtime-item text query.
pub struct ItemTextQueryHandlerCxLikeCpp<'a> {
    state: &'a InventoryState,
    inventory: OwnedInventoryAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
}

impl<'a> ItemTextQueryHandlerCxLikeCpp<'a> {
    pub fn new(
        state: &'a InventoryState,
        inventory: OwnedInventoryAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            state,
            inventory,
            publication,
        }
    }

    /// QueryHandler.cpp:305–317; preserve the full Rust runtime snapshot before Item/text cloning.
    pub fn handle_item_text_query(&self, query: ItemTextQuery) {
        let response = self
            .state
            .resolved_player_inventory_item_object_with_access_like_cpp(&self.inventory, query.id)
            .map(|item| QueryItemTextResponse::valid_like_cpp(query.id, item.text().to_string()))
            .unwrap_or_else(|| QueryItemTextResponse::invalid_like_cpp(query.id));
        self.publication.send_packet(&response);
    }
}

pub(super) fn thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match ItemTextQuery::read(&mut pkt) {
            Ok(query) => session
                .item_text_query_handler_cx_like_cpp(catalogs)
                .handle_item_text_query(query),
            Err(e) => tracing::warn!("Failed to read ItemTextQuery: {e}"),
        }
    })
}
