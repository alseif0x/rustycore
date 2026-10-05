// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat handler test entry points.
//!
//! The chat handlers themselves moved to `wow-world-social` under #1263 F5.
//! These cfg(test) entry points let the chat scenario module drive one moved
//! handler through the social context without composing a dispatch table.

impl crate::session::WorldSession {

    #[cfg(test)]
    pub async fn handle_chat_message(&mut self, pkt: wow_packet::WorldPacket, msg_type: ChatMsg) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_message_with_policy_like_cpp(pkt, msg_type)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_whisper(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_whisper_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_channel_message(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_channel_message_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_afk(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_afk_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_dnd(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_dnd_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_emote(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_emote_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_addon_message(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_addon_message_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_addon_message_targeted(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_addon_message_targeted_with_policy_like_cpp(pkt)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_chat_addon_message_whisper(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::ChatHandlerCxLikeCpp::new(hub, social, &chat_policy)
            .handle_chat_addon_message_whisper_with_policy_like_cpp(pkt)
            .await;
    }
}

#[cfg(test)]
#[path = "../../unit_tests/handlers/chat/tests/mod.rs"]
mod tests;
