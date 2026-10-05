// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the chat handlers owned by `wow-world-social`.
//!
//! The chat family moved out of this crate in #1263 F5; the scenario suites
//! still build a `WorldSession` here, so these bounded delegates construct the
//! social owner's context from the session's disjoint borrows. They are the
//! only chat code left in `wow-world` besides the imports, the module mounts
//! and the emote/text-emote registrations that were not part of the move.

use wow_packet::packets::chat::ChatMsg;
use wow_world_core::session::ChatPolicyCatalogsLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    #[cfg(test)]
    pub async fn handle_chat_addon_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, chat_policy)
            .handle_chat_addon_message_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_channel_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_channel_command(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_channel_password(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_channel_password(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_channel_player_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_channel_player_command(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_join_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_join_channel(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_leave_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_leave_channel(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        msg_type: ChatMsg,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, chat_policy)
            .handle_chat_message_with_policy_like_cpp(pkt, msg_type)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_register_addon_prefixes(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_register_addon_prefixes(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_report_filtered(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_report_filtered(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_report_ignored(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_report_ignored(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_update_aadc_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_update_aadc_status(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_unregister_all_addon_prefixes(
        &mut self,
        pkt: wow_packet::WorldPacket,
    ) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_unregister_all_addon_prefixes(pkt)
            .await;
    }
}
