// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the guild invitation handlers moved to
//! `wow-world-social` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_social::GuildHandlerCxLikeCpp;

use crate::session::WorldSession;

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
}
