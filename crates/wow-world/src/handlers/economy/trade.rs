// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private trade/duel/petition capability handlers extracted from the legacy misc owner.
//!
//! Under #1263 F5 the trade bodies moved to
//! `wow-world-application::trade_handlers`; this module keeps the spell-trade
//! and petition handlers while they need the shell spell store and pet state,
//! plus the cfg(test) delegates for the moved bodies.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::SetTradeSpell;

#[cfg(test)]
mod test_shims;

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_spell",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_set_trade_spell(pkt).await }),
    }
}

impl crate::session::WorldSession {
    pub async fn handle_set_trade_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeSpell::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "SetTradeSpell parse failed: {error}"
                );
                return;
            }
        };

        self.set_represented_trade_spell_like_cpp(
            packet.spell_id,
            packet.pack_slot,
            packet.item_slot_in_pack,
        );
    }
}
