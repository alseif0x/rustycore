// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the arena-team handlers moved to
//! `wow-world-social` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_social::ArenaTeamHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_arena_team_roster(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_roster(pkt)
            .await;
    }

    pub async fn handle_arena_team_accept(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_accept(pkt)
            .await;
    }

    pub async fn handle_arena_team_decline(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_decline(pkt)
            .await;
    }

    pub async fn handle_arena_team_leave(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_leave(pkt)
            .await;
    }

    pub async fn handle_arena_team_remove(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_remove(pkt)
            .await;
    }

    pub async fn handle_arena_team_disband(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_disband(pkt)
            .await;
    }

    pub async fn handle_arena_team_leader(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_arena_team_leader(pkt)
            .await;
    }

    pub async fn handle_query_arena_team(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
            .handle_query_arena_team(pkt)
            .await;
    }
}
