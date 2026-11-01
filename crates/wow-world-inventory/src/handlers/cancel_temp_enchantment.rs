// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::InventoryHandlerHostLikeCpp;
use wow_handler::HandlerFuture;
use wow_packet::{ClientPacket, WorldPacket};

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
        match wow_packet::packets::item::CancelTempEnchantment::read(&mut pkt) {
            Ok(cancel) => session
                .item_enchantment_handler_cx_like_cpp(catalogs)
                .cancel_temp_enchantment_like_cpp(cancel),
            Err(e) => tracing::warn!("Failed to read CancelTempEnchantment: {e}"),
        }
    })
}
