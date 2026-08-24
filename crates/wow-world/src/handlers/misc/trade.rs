// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private trade capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    AcceptTrade, BeginTrade, BusyTrade, CanDuel, ClearTradeItem, DeclinePetition, DuelResponse,
    IgnoreTrade, QueryPetition, QueryPetitionResponse, SetTradeGold, SetTradeItem, SetTradeSpell,
    SignPetition, TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP,
    UnacceptTrade,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelTrade,
        status: SessionStatus::LoggedInOrRecentlyLogout,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_cancel_trade",
        handler: |session, pkt| Box::pin(async move { session.handle_cancel_trade(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_trade",
        handler: |session, pkt| Box::pin(async move { session.handle_accept_trade(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ClearTradeItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_clear_trade_item",
        handler: |session, pkt| Box::pin(async move { session.handle_clear_trade_item(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_item",
        handler: |session, pkt| Box::pin(async move { session.handle_set_trade_item(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeGold,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_gold",
        handler: |session, pkt| Box::pin(async move { session.handle_set_trade_gold(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_spell",
        handler: |session, pkt| Box::pin(async move { session.handle_set_trade_spell(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SignPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_sign_petition",
        handler: |session, pkt| Box::pin(async move { session.handle_sign_petition(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeclinePetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_petition",
        handler: |session, pkt| Box::pin(async move { session.handle_decline_petition(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_petition",
        handler: |session, pkt| Box::pin(async move { session.handle_query_petition(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UnacceptTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_unaccept_trade",
        handler: |session, pkt| Box::pin(async move { session.handle_unaccept_trade(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BusyTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_busy_trade",
        handler: |session, pkt| Box::pin(async move { session.handle_busy_trade(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BeginTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_begin_trade",
        handler: |session, pkt| Box::pin(async move { session.handle_begin_trade(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CanDuel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_can_duel",
        handler: |session, pkt| Box::pin(async move { session.handle_can_duel(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DuelResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_duel_response",
        handler: |session, pkt| Box::pin(async move { session.handle_duel_response(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::IgnoreTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_ignore_trade",
        handler: |session, pkt| Box::pin(async move { session.handle_ignore_trade(pkt).await }),
    }
}

impl crate::session::WorldSession {
    pub async fn handle_cancel_trade(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ calls Player::TradeCancel(true) for a present player; TradeCancel
        // itself is a no-op when no active TradeData exists.
        self.cancel_represented_trade_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP, true);
    }

    pub async fn handle_accept_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AcceptTrade::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AcceptTrade parse failed: {error}"
                );
                return;
            }
        };

        self.accept_represented_trade_like_cpp(packet.state_index);
    }

    pub async fn handle_clear_trade_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ClearTradeItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ClearTradeItem parse failed: {error}"
                );
                return;
            }
        };

        self.clear_represented_trade_item_like_cpp(packet.trade_slot);
    }

    pub async fn handle_set_trade_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTradeItem parse failed: {error}"
                );
                return;
            }
        };

        self.set_represented_trade_item_like_cpp(
            packet.trade_slot,
            packet.pack_slot,
            packet.item_slot_in_pack,
        );
    }

    pub async fn handle_set_trade_gold(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeGold::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTradeGold parse failed: {error}"
                );
                return;
            }
        };

        self.set_represented_trade_gold_like_cpp(packet.coinage);
    }

    pub async fn handle_set_trade_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeSpell::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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
                    account = self.account_id,
                    "SignPetition parse failed: {error}"
                );
                return;
            }
        };

        self.record_represented_sign_petition_like_cpp(packet.petition_guid, packet.choice);
    }

    pub async fn handle_decline_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match DeclinePetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DeclinePetition parse failed: {error}"
                );
                return;
            }
        };

        self.record_represented_decline_petition_like_cpp(packet.petition_guid);
    }

    pub async fn handle_query_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match QueryPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "QueryPetition parse failed: {error}"
                );
                return;
            }
        };

        self.record_represented_query_petition_like_cpp(packet.petition_id, packet.item_guid);
        self.send_packet(&QueryPetitionResponse::not_found_like_cpp(packet.item_guid));
    }

    pub async fn handle_unaccept_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = UnacceptTrade::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "UnacceptTrade parse failed: {error}"
            );
            return;
        }

        self.unaccept_represented_trade_like_cpp();
    }

    pub async fn handle_busy_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BusyTrade::read(&mut pkt) {
            warn!(account = self.account_id, "BusyTrade parse failed: {error}");
            return;
        }

        self.cancel_represented_trade_like_cpp(
            crate::session::TRADE_STATUS_PLAYER_BUSY_LIKE_CPP,
            true,
        );
    }

    pub async fn handle_begin_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BeginTrade::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "BeginTrade parse failed: {error}"
            );
            return;
        }

        self.begin_represented_trade_like_cpp();
    }

    pub async fn handle_can_duel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match CanDuel::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "CanDuel parse failed: {error}");
                return;
            }
        };

        self.handle_can_duel_like_cpp(packet.target_guid, packet.to_the_death);
    }

    pub async fn handle_duel_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match DuelResponse::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DuelResponse parse failed: {error}"
                );
                return;
            }
        };

        self.handle_duel_response_like_cpp(packet.arbiter_guid, packet.accepted, packet.forfeited);
    }

    pub async fn handle_ignore_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = IgnoreTrade::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "IgnoreTrade parse failed: {error}"
            );
            return;
        }

        self.cancel_represented_trade_like_cpp(TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP, true);
    }
}
