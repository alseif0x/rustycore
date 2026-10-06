// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the arena-team handlers moved to
//! `wow-world-social` and the battleground handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::BattlegroundHandlerCxLikeCpp;
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

    fn battleground_test_cx_like_cpp(&mut self) -> BattlegroundHandlerCxLikeCpp<'_> {
        let (world_entities, instances, hub) = crate::session::split_battleground_mut(self);
        BattlegroundHandlerCxLikeCpp::new(hub, world_entities, instances)
    }

    pub async fn handle_battlefield_port(&mut self, pkt: WorldPacket) {
        self.battleground_test_cx_like_cpp()
            .handle_battlefield_port(pkt)
            .await;
    }

    pub async fn handle_battlefield_leave(&mut self, pkt: WorldPacket) {
        self.battleground_test_cx_like_cpp()
            .handle_battlefield_leave(pkt)
            .await;
    }

    pub async fn handle_request_rated_pvp_info(&mut self, pkt: WorldPacket) {
        self.battleground_test_cx_like_cpp()
            .handle_request_rated_pvp_info(pkt)
            .await;
    }

    pub async fn handle_request_pvp_rewards(&mut self, pkt: WorldPacket) {
        self.battleground_test_cx_like_cpp()
            .handle_request_pvp_rewards(pkt)
            .await;
    }

    pub async fn handle_toggle_pvp(&mut self, pkt: WorldPacket) {
        let changed = self
            .battleground_test_cx_like_cpp()
            .handle_toggle_pvp(pkt)
            .await;
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub async fn handle_set_pvp(&mut self, pkt: WorldPacket) {
        let changed = self
            .battleground_test_cx_like_cpp()
            .handle_set_pvp(pkt)
            .await;
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub fn apply_toggle_pvp_like_cpp(&mut self) {
        let changed = self
            .battleground_test_cx_like_cpp()
            .apply_toggle_pvp_like_cpp();
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub fn apply_set_pvp_like_cpp(&mut self, enable_pvp: bool) {
        let changed = self
            .battleground_test_cx_like_cpp()
            .apply_set_pvp_like_cpp(enable_pvp);
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub async fn handle_area_spirit_healer_query(&mut self, pkt: WorldPacket) {
        self.battleground_test_cx_like_cpp()
            .handle_area_spirit_healer_query(pkt)
            .await;
    }

    pub async fn handle_area_spirit_healer_queue(&mut self, pkt: WorldPacket) {
        self.battleground_test_cx_like_cpp()
            .handle_area_spirit_healer_queue(pkt)
            .await;
    }
}
