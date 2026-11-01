// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the trade handlers moved to
//! `wow-world-social::trade_handlers` (#1263 F5; #1363 slice A2).
#![cfg(test)]
use wow_packet::WorldPacket;
use wow_world_social::TradeHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn trade_test_cx_like_cpp(&mut self) -> TradeHandlerCxLikeCpp<'_> {
        let active_loot_guid = self.loot.active_loot_guid_like_cpp();
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
            .with_active_loot_guid_like_cpp(active_loot_guid)
    }

    pub(crate) async fn handle_initiate_trade_with_policy(
        &mut self,
        pkt: WorldPacket,
        policy: wow_world_social::TradePolicyLikeCpp,
    ) {
        let logging_out = self.lifecycle.is_logging_out_like_cpp();
        self.trade_test_cx_like_cpp()
            .with_initiate_context_like_cpp(policy, logging_out)
            .handle_initiate_trade(pkt)
            .await;
    }

    pub(crate) async fn handle_cancel_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_cancel_trade(pkt).await;
    }

    pub(crate) async fn handle_accept_trade(&mut self, pkt: WorldPacket) {
        use wow_packet::ClientPacket;
        let mut pkt = pkt;
        let Ok(packet) = wow_packet::packets::misc::AcceptTrade::read(&mut pkt) else {
            return;
        };
        let generator = self.id_generators_for_test_like_cpp().item;
        wow_world_social::accept_trade_like_cpp(self, packet.state_index, generator).await;
    }

    /// The driver's head-of-pass step for a coordinator's pending exchange.
    pub(crate) async fn continue_pending_trade_commit_for_test_like_cpp(&mut self) {
        let generator = self.id_generators_for_test_like_cpp().item;
        wow_world_social::continue_pending_trade_commit_like_cpp(self, generator).await;
    }

    pub(crate) async fn handle_clear_trade_item(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_clear_trade_item(pkt)
            .await;
    }

    pub(crate) async fn handle_set_trade_item(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_set_trade_item(pkt)
            .await;
    }

    pub(crate) async fn handle_set_trade_gold(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_set_trade_gold(pkt)
            .await;
    }

    pub(crate) async fn handle_unaccept_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_unaccept_trade(pkt)
            .await;
    }

    pub(crate) async fn handle_busy_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_busy_trade(pkt).await;
    }

    pub(crate) async fn handle_begin_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_begin_trade(pkt).await;
    }

    pub(crate) async fn handle_set_trade_spell(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_set_trade_spell(pkt);
    }

    pub(crate) async fn handle_sign_petition(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_sign_petition(pkt);
    }

    pub(crate) async fn handle_decline_petition(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_decline_petition(pkt);
    }

    pub(crate) async fn handle_query_petition(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_query_petition(pkt);
    }

    pub(crate) async fn handle_can_duel(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_can_duel(pkt);
    }

    pub(crate) async fn handle_duel_response(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_duel_response(pkt);
    }

    pub(crate) async fn handle_ignore_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_ignore_trade(pkt).await;
    }
}
