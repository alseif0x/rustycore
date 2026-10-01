// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_emotes_store_like_cpp(&mut self, store: Arc<EmotesStore>) {
        self.catalogs.set_emotes_store_like_cpp(store)
    }
    #[cfg(test)]
    pub fn set_emotes_text_store_like_cpp(&mut self, store: Arc<EmotesTextStore>) {
        self.catalogs.set_emotes_text_store_like_cpp(store)
    }
    #[cfg(test)]
    pub fn set_chat_fake_message_preventing_like_cpp(&mut self, enabled: bool) {
        self.config
            .set_chat_fake_message_preventing_like_cpp(enabled)
    }
    #[cfg(test)]
    pub fn set_chat_level_requirements_like_cpp(
        &mut self,
        requirements: ChatLevelRequirementsLikeCpp,
    ) {
        self.config
            .set_chat_level_requirements_like_cpp(requirements)
    }
    #[cfg(test)]
    pub fn set_chat_listen_ranges_like_cpp(&mut self, ranges: ChatListenRangesLikeCpp) {
        self.config.set_chat_listen_ranges_like_cpp(ranges)
    }
    #[cfg(test)]
    pub fn set_chat_flood_config_like_cpp(&mut self, config: ChatFloodConfigLikeCpp) {
        self.config.set_chat_flood_config_like_cpp(config)
    }
    #[cfg(test)]
    pub(crate) fn player_emote_state_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_social_ref(self);
        state.player_emote_state_like_cpp(hub)
    }
    pub(crate) fn set_player_emote_state_like_cpp(
        &mut self,
        emote_state: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        crate::session::hub_mut(self).set_player_emote_state_like_cpp(emote_state)
    }
}
