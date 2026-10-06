#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{EmotesStore, EmotesTextStore};

use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::state::SessionCore;
use wow_entities::{
    UNIT_DATA_BITS, UNIT_DATA_EMOTE_STATE_BIT, UNIT_DATA_MODS_PARENT_BIT, UnitDataUpdate,
    UnitDataValues, UpdateMask,
};

impl SessionCore {
    pub fn player_emote_state_update_packet_like_cpp(
        &self,
        emote_state: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let guid = self.player_guid()?;
        let mut mask = UpdateMask::new(UNIT_DATA_BITS);
        mask.set(UNIT_DATA_MODS_PARENT_BIT);
        mask.set(UNIT_DATA_EMOTE_STATE_BIT);
        let update = wow_entities::PlayerValuesUpdate {
            changed_object_type_mask: 0,
            object_data: None,
            unit_data: Some(UnitDataUpdate {
                mask,
                values: UnitDataValues {
                    emote_state: emote_state.min(i32::MAX as u32) as i32,
                    ..Default::default()
                },
            }),
            player_data: None,
            active_player_data: None,
        };
        player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
    }
}

impl crate::session::state::SessionWorldConfig {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_chat_fake_message_preventing_like_cpp(&mut self, enabled: bool) {
        self.chat_fake_message_preventing_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_chat_level_requirements_like_cpp(
        &mut self,
        requirements: ChatLevelRequirementsLikeCpp,
    ) {
        self.chat_level_requirements_like_cpp = requirements;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_chat_listen_ranges_like_cpp(&mut self, ranges: ChatListenRangesLikeCpp) {
        self.chat_listen_ranges_like_cpp = ranges;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_chat_flood_config_like_cpp(&mut self, config: ChatFloodConfigLikeCpp) {
        self.chat_flood_config_like_cpp = config;
    }
}

impl crate::session::state::SessionCatalogs {
    /// Set the C++ Emotes.db2 store for `Unit::HandleEmoteCommand`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_emotes_store_like_cpp(&mut self, store: Arc<EmotesStore>) {
        self.emotes_store = Some(store);
    }

    /// Set the C++ EmotesText.db2 store for `HandleTextEmoteOpcode`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_emotes_text_store_like_cpp(&mut self, store: Arc<EmotesTextStore>) {
        self.emotes_text_store = Some(store);
    }
}

impl crate::session::HubRef<'_> {
    pub fn resolved_player_emote_state_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().emote_state_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.presentation.player_emote_state_like_cpp);
        }
        canonical
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_emote_state_like_cpp(
        &mut self,
        emote_state: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        if self.shared().resolved_player_emote_state_like_cpp() == Some(emote_state) {
            return None;
        }

        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_emote_state_like_cpp(emote_state);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.presentation.player_emote_state_like_cpp = emote_state;
            if !canonical {
                let _ = self.core.mutate_canonical_player_like_cpp(|player| {
                    player.unit_mut().set_emote_state_like_cpp(emote_state);
                });
            }
        }
        if !canonical
            && !(cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none())
        {
            return None;
        }
        self.core
            .player_emote_state_update_packet_like_cpp(emote_state)
    }
}
