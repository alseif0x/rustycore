// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the trade handlers moved to
//! `wow-world-application::trade_handlers` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::TradeHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn trade_test_cx_like_cpp(&mut self) -> TradeHandlerCxLikeCpp<'_> {
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
    }

    pub async fn handle_cancel_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_cancel_trade(pkt).await;
    }

    pub async fn handle_accept_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_accept_trade(pkt).await;
    }

    pub async fn handle_clear_trade_item(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_clear_trade_item(pkt)
            .await;
    }

    pub async fn handle_set_trade_item(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_set_trade_item(pkt)
            .await;
    }

    pub async fn handle_set_trade_gold(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_set_trade_gold(pkt)
            .await;
    }

    pub async fn handle_unaccept_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp()
            .handle_unaccept_trade(pkt)
            .await;
    }

    pub async fn handle_busy_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_busy_trade(pkt).await;
    }

    pub async fn handle_begin_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_begin_trade(pkt).await;
    }

    pub async fn handle_set_trade_spell(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_set_trade_spell(pkt);
    }

    pub async fn handle_sign_petition(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_sign_petition(pkt);
    }

    pub async fn handle_decline_petition(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_decline_petition(pkt);
    }

    pub async fn handle_query_petition(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_query_petition(pkt);
    }

    pub async fn handle_can_duel(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_can_duel(pkt);
    }

    pub async fn handle_duel_response(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_duel_response(pkt);
    }

    pub async fn handle_ignore_trade(&mut self, pkt: WorldPacket) {
        self.trade_test_cx_like_cpp().handle_ignore_trade(pkt).await;
    }
}
