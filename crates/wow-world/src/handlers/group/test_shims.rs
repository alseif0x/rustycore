// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the group handlers moved to `wow-world-social`
//! (#1263 F5).
//!
//! The group scenario suites still build a `WorldSession` here, so these
//! bounded delegates construct the social owner's context from the session's
//! social state and hub.

use wow_packet::WorldPacket;
use wow_world_social::SocialGroupHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_set_loot_method(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_set_loot_method(pkt)
            .await;
    }

    pub async fn handle_silence_party_talker(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_silence_party_talker(pkt)
            .await;
    }

    pub async fn handle_set_role(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_set_role(pkt)
            .await;
    }

    pub async fn handle_initiate_role_poll(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_initiate_role_poll(pkt)
            .await;
    }

    pub async fn handle_update_raid_target(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_update_raid_target(pkt)
            .await;
    }

    pub async fn handle_request_party_join_updates(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_request_party_join_updates(pkt)
            .await;
    }

    pub async fn handle_request_party_member_stats(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_request_party_member_stats(pkt)
            .await;
    }

    pub async fn handle_do_ready_check(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_do_ready_check(pkt)
            .await;
    }

    pub async fn handle_ready_check_response(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_ready_check_response(pkt)
            .await;
    }

    pub async fn handle_low_level_raid1(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_low_level_raid1(pkt)
            .await;
    }

    pub async fn handle_low_level_raid2(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_low_level_raid2(pkt)
            .await;
    }

    pub async fn handle_minimap_ping(&mut self, pkt: WorldPacket) {
        let (social, hub) = crate::session::split_social_mut(self);
        SocialGroupHandlerCxLikeCpp::new(social, hub)
            .handle_minimap_ping(pkt)
            .await;
    }

    pub async fn handle_set_party_leader(&mut self, pkt: WorldPacket) {
        let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
        wow_world_application::GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
            .handle_set_party_leader(pkt)
            .await;
    }

    pub async fn handle_set_assistant_leader(&mut self, pkt: WorldPacket) {
        let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
        wow_world_application::GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
            .handle_set_assistant_leader(pkt)
            .await;
    }

    pub async fn handle_set_everyone_is_assistant(&mut self, pkt: WorldPacket) {
        let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
        wow_world_application::GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
            .handle_set_everyone_is_assistant(pkt)
            .await;
    }

    pub async fn handle_set_party_assignment(&mut self, pkt: WorldPacket) {
        let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
        wow_world_application::GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
            .handle_set_party_assignment(pkt)
            .await;
    }

    pub async fn handle_change_sub_group(&mut self, pkt: WorldPacket) {
        let changed = {
            let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
            wow_world_application::GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
                .handle_change_sub_group(pkt)
                .await
        };
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub async fn handle_swap_sub_groups(&mut self, pkt: WorldPacket) {
        let changed = {
            let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
            wow_world_application::GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
                .handle_swap_sub_groups(pkt)
                .await
        };
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }
}
