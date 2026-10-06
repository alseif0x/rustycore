// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private trade/duel/petition capability handlers extracted from the legacy misc owner.
//!
//! Under #1263 F5 the trade bodies moved to
//! `wow-world-application::trade_handlers`; this module keeps the spell-trade,
//! petition and duel handlers while they need the shell spell store and pet
//! state, plus the cfg(test) delegates for the moved bodies.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    CanDuel, DeclinePetition, DuelResponse, QueryPetition, QueryPetitionResponse, SetTradeSpell,
    SignPetition,
};

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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SignPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_sign_petition",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_sign_petition(pkt).await }),
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeclinePetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_petition",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_decline_petition(pkt).await }),
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_petition",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_query_petition(pkt).await }),
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CanDuel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_can_duel",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_can_duel(pkt).await }),
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DuelResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_duel_response",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_duel_response(pkt).await }),
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

    pub async fn handle_sign_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SignPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "SignPetition parse failed: {error}"
                );
                return;
            }
        };

        crate::session::cx_pets(self)
            .record_represented_sign_petition_like_cpp(packet.petition_guid, packet.choice);
    }

    pub async fn handle_decline_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match DeclinePetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "DeclinePetition parse failed: {error}"
                );
                return;
            }
        };

        crate::session::cx_pets(self)
            .record_represented_decline_petition_like_cpp(packet.petition_guid);
    }

    pub async fn handle_query_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match QueryPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "QueryPetition parse failed: {error}"
                );
                return;
            }
        };

        crate::session::cx_pets(self)
            .record_represented_query_petition_like_cpp(packet.petition_id, packet.item_guid);
        self.send_packet(&QueryPetitionResponse::not_found_like_cpp(packet.item_guid));
    }

    pub async fn handle_can_duel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match CanDuel::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "CanDuel parse failed: {error}"
                );
                return;
            }
        };

        {
            let (s, mut h) = crate::session::split_social_mut(self);
            s.handle_can_duel_like_cpp(&mut h, packet.target_guid, packet.to_the_death)
        };
    }

    pub async fn handle_duel_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match DuelResponse::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "DuelResponse parse failed: {error}"
                );
                return;
            }
        };

        self.handle_duel_response_like_cpp(packet.arbiter_guid, packet.accepted, packet.forfeited);
    }
}
