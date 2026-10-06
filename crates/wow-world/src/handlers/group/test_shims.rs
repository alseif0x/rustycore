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
    /// Builds the application group context exactly as the production host
    /// does, with the test-only invite policy.
    fn group_test_cx_like_cpp<'a>(
        &'a mut self,
        policy: &'a crate::session::GroupInvitePolicyLikeCpp,
    ) -> wow_world_application::GroupHandlerCxLikeCpp<'a> {
        let (social, lifecycle, loot, instances, hub) =
            crate::session::split_group_handler_states_mut(self);
        wow_world_application::GroupHandlerCxLikeCpp::new(
            social,
            lifecycle,
            loot,
            instances,
            policy,
            hub,
            cfg!(test),
        )
    }

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
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_set_party_leader(pkt)
            .await;
    }

    pub async fn handle_set_assistant_leader(&mut self, pkt: WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_set_assistant_leader(pkt)
            .await;
    }

    pub async fn handle_set_everyone_is_assistant(&mut self, pkt: WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_set_everyone_is_assistant(pkt)
            .await;
    }

    pub async fn handle_set_party_assignment(&mut self, pkt: WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_set_party_assignment(pkt)
            .await;
    }

    pub async fn handle_change_sub_group(&mut self, pkt: WorldPacket) {
        let tail = {
            let policy = self.group_invite_policy_for_test_like_cpp();
            self.group_test_cx_like_cpp(&policy)
                .handle_change_sub_group(pkt)
                .await
        };
        self.apply_group_publication_tail_like_cpp(tail).await;
    }

    pub async fn handle_swap_sub_groups(&mut self, pkt: WorldPacket) {
        let tail = {
            let policy = self.group_invite_policy_for_test_like_cpp();
            self.group_test_cx_like_cpp(&policy)
                .handle_swap_sub_groups(pkt)
                .await
        };
        self.apply_group_publication_tail_like_cpp(tail).await;
    }

    pub async fn handle_leave_group(&mut self, pkt: WorldPacket) {
        let tail = {
            let policy = self.group_invite_policy_for_test_like_cpp();
            self.group_test_cx_like_cpp(&policy)
                .handle_leave_group(pkt)
                .await
        };
        self.apply_group_publication_tail_like_cpp(tail).await;
    }

    pub async fn handle_convert_raid(&mut self, pkt: WorldPacket) {
        let tail = {
            let policy = self.group_invite_policy_for_test_like_cpp();
            self.group_test_cx_like_cpp(&policy)
                .handle_convert_raid(pkt)
                .await
        };
        self.apply_group_publication_tail_like_cpp(tail).await;
    }

    pub async fn handle_party_invite_response(&mut self, pkt: WorldPacket) {
        let tail = {
            let policy = self.group_invite_policy_for_test_like_cpp();
            self.group_test_cx_like_cpp(&policy)
                .handle_party_invite_response(pkt)
                .await
        };
        self.apply_group_publication_tail_like_cpp(tail).await;
    }

    pub async fn handle_party_invite_with_policy_like_cpp(
        &mut self,
        pkt: WorldPacket,
        policy: &crate::session::GroupInvitePolicyLikeCpp,
    ) {
        self.group_test_cx_like_cpp(policy)
            .handle_party_invite(pkt)
            .await;
    }

    pub async fn handle_party_invite(&mut self, pkt: WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_party_invite(pkt)
            .await;
    }

    pub async fn handle_random_roll(&mut self, pkt: WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_random_roll(pkt)
            .await;
    }

    pub async fn handle_opt_out_of_loot(&mut self, pkt: WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.group_test_cx_like_cpp(&policy)
            .handle_opt_out_of_loot(pkt)
            .await;
    }

    pub async fn handle_party_uninvite(&mut self, pkt: WorldPacket) {
        let tail = {
            let policy = self.group_invite_policy_for_test_like_cpp();
            self.group_test_cx_like_cpp(&policy)
                .handle_party_uninvite(pkt)
                .await
        };
        self.apply_group_publication_tail_like_cpp(tail).await;
    }
}
