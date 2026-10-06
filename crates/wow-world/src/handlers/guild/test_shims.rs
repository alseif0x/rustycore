// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the guild invitation handlers moved to
//! `wow-world-social` and the guild-bank handlers moved to
//! `wow-world-application` (#1263 F5). The guild-bank entry points dispatch
//! through the registered production thunks.

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
use wow_world_social::GuildHandlerCxLikeCpp;

use crate::session::WorldSession;

async fn dispatch_registered_like_cpp(
    session: &mut WorldSession,
    opcode: ClientOpcodes,
    pkt: WorldPacket,
) {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == opcode)
        .expect("registered guild-bank handler");
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

impl WorldSession {
    pub async fn handle_guild_set_achievement_tracking(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        GuildHandlerCxLikeCpp::new(social, hub)
            .handle_guild_set_achievement_tracking(pkt)
            .await;
    }

    pub async fn handle_guild_bank_remaining_withdraw_money_query(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        GuildHandlerCxLikeCpp::new(social, hub)
            .handle_guild_bank_remaining_withdraw_money_query(pkt)
            .await;
    }

    pub async fn handle_guild_decline_invitation(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        GuildHandlerCxLikeCpp::new(social, hub)
            .handle_guild_decline_invitation(pkt)
            .await;
    }

    pub async fn handle_accept_guild_invite(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        GuildHandlerCxLikeCpp::new(social, hub)
            .handle_accept_guild_invite(pkt)
            .await;
    }

    pub async fn handle_guild_bank_activate(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankActivate, pkt).await;
    }

    pub async fn handle_guild_bank_query_tab(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankQueryTab, pkt).await;
    }

    pub async fn handle_guild_bank_buy_tab(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankBuyTab, pkt).await;
    }

    pub async fn handle_guild_bank_update_tab(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankUpdateTab, pkt).await;
    }

    pub async fn handle_guild_bank_deposit_money(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankDepositMoney, pkt).await;
    }

    pub async fn handle_guild_bank_withdraw_money(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankWithdrawMoney, pkt).await;
    }

    pub async fn handle_guild_bank_log_query(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankLogQuery, pkt).await;
    }

    pub async fn handle_guild_bank_text_query(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankTextQuery, pkt).await;
    }

    pub async fn handle_guild_bank_set_tab_text(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::GuildBankSetTabText, pkt).await;
    }

    pub async fn handle_auto_guild_bank_item(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::AutoGuildBankItem, pkt).await;
    }

    pub async fn handle_auto_store_guild_bank_item(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::AutoStoreGuildBankItem, pkt).await;
    }

    pub async fn handle_decline_guild_invites(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::DeclineGuildInvites, pkt).await;
    }
}
