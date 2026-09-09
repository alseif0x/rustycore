//! Represented chat, gossip and text operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn apply_chat_away_mode_like_cpp(
        &mut self,
        mode: PlayerAwayModeLikeCpp,
        text: String,
    ) -> bool {
        if self.resolved_in_combat_like_cpp() != Some(false) || text.len() > 511 {
            return false;
        }

        if self.player_guid().is_none() {
            return false;
        }

        let (active_flag, other_flag, default_text) = match mode {
            PlayerAwayModeLikeCpp::Afk => (
                PLAYER_FLAGS_AFK_LIKE_CPP,
                PLAYER_FLAGS_DND_LIKE_CPP,
                "Away from Keyboard",
            ),
            PlayerAwayModeLikeCpp::Dnd => (
                PLAYER_FLAGS_DND_LIKE_CPP,
                PLAYER_FLAGS_AFK_LIKE_CPP,
                "Do not Disturb",
            ),
        };

        self.mutate_canonical_player_like_cpp(move |player| {
            if player.has_player_flag(active_flag) {
                if text.is_empty() {
                    player.remove_player_flag(active_flag);
                } else {
                    player.gameplay_state_mut().social.auto_reply_msg_like_cpp = text;
                }
                return;
            }
            if player.has_player_flag(other_flag) {
                player.remove_player_flag(other_flag);
            }
            player.set_player_flag(active_flag);
            player.gameplay_state_mut().social.auto_reply_msg_like_cpp = if text.is_empty() {
                default_text.to_string()
            } else {
                text
            };
        })
        .is_some()
    }
    /// Set the C++ Emotes.db2 store for `Unit::HandleEmoteCommand`.
    #[cfg(test)]
    pub fn set_emotes_store_like_cpp(&mut self, store: Arc<EmotesStore>) {
        self.emotes_store = Some(store);
    }
    /// Set the C++ EmotesText.db2 store for `HandleTextEmoteOpcode`.
    #[cfg(test)]
    pub fn set_emotes_text_store_like_cpp(&mut self, store: Arc<EmotesTextStore>) {
        self.emotes_text_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_chat_fake_message_preventing_like_cpp(&mut self, enabled: bool) {
        self.chat_fake_message_preventing_like_cpp = enabled;
    }
    #[cfg(test)]
    pub fn set_chat_strict_link_checking_kick_like_cpp(&mut self, enabled: bool) {
        self.chat_strict_link_checking_kick_like_cpp = enabled;
    }
    #[cfg(test)]
    pub fn set_chat_level_requirements_like_cpp(
        &mut self,
        requirements: ChatLevelRequirementsLikeCpp,
    ) {
        self.chat_level_requirements_like_cpp = requirements;
    }
    #[cfg(test)]
    pub fn set_chat_listen_ranges_like_cpp(&mut self, ranges: ChatListenRangesLikeCpp) {
        self.chat_listen_ranges_like_cpp = ranges;
    }
    #[cfg(test)]
    pub fn set_chat_flood_config_like_cpp(&mut self, config: ChatFloodConfigLikeCpp) {
        self.chat_flood_config_like_cpp = config;
    }
    pub fn set_remote_address_like_cpp(&mut self, address: Option<String>) {
        self.remote_address_like_cpp = address;
    }
    #[cfg(test)]
    pub(crate) fn chat_fake_message_preventing_like_cpp(&self) -> bool {
        self.chat_fake_message_preventing_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_strict_link_checking_kick_like_cpp(&self) -> bool {
        self.chat_strict_link_checking_kick_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_level_requirements_like_cpp(&self) -> ChatLevelRequirementsLikeCpp {
        self.chat_level_requirements_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_listen_ranges_like_cpp(&self) -> ChatListenRangesLikeCpp {
        self.chat_listen_ranges_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_flood_config_like_cpp(&self) -> ChatFloodConfigLikeCpp {
        self.chat_flood_config_like_cpp
    }
    pub(crate) fn set_player_emote_state_like_cpp(
        &mut self,
        emote_state: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        if self.resolved_player_emote_state_like_cpp() == Some(emote_state) {
            return None;
        }

        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_emote_state_like_cpp(emote_state);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_emote_state_like_cpp = emote_state;
            if !canonical {
                let _ = self.mutate_canonical_player_like_cpp(|player| {
                    player.unit_mut().set_emote_state_like_cpp(emote_state);
                });
            }
        }
        if !canonical && !(cfg!(test) && self.player_handle_like_cpp.is_none()) {
            return None;
        }
        self.player_emote_state_update_packet_like_cpp(emote_state)
    }
    pub(in crate::session) fn resolved_player_emote_state_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().emote_state_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_emote_state_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_emote_state_like_cpp(&self) -> u32 {
        self.resolved_player_emote_state_like_cpp()
            .expect("test Player emote-state owner must resolve")
    }
    fn player_emote_state_update_packet_like_cpp(
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
    pub(crate) fn clear_player_gossip_options_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.clear_gossip_options_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.gossip_options.clear();
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn replace_player_gossip_options_like_cpp(
        &mut self,
        options: Vec<GossipOptionInfo>,
    ) -> bool {
        #[cfg(test)]
        let fixture_options = options.clone();
        let mut options = Some(options);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_gossip_options_like_cpp(
                    options.take().expect("gossip option mutation runs once"),
                );
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.gossip_options = fixture_options;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn player_gossip_option_like_cpp(
        &self,
        gossip_option_id: i32,
    ) -> Option<GossipOptionInfo> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gossip_options_like_cpp()
                .iter()
                .find(|option| option.gossip_option_id == gossip_option_id)
                .cloned()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self
                .gossip_options
                .iter()
                .find(|option| option.gossip_option_id == gossip_option_id)
                .cloned();
        }
        canonical.flatten()
    }
    pub(crate) fn represented_creature_gossip_text_like_cpp(
        &self,
        creature_entry: u32,
    ) -> Vec<ClientGossipText> {
        let Some(quest_store) = self.quest_store.as_ref() else {
            return Vec::new();
        };

        let starter_candidates = quest_store
            .quests_for_starter(creature_entry)
            .iter()
            .map(|quest| {
                (
                    quest.id,
                    quest.allowable_races,
                    quest.allowable_classes,
                    quest.min_level,
                    quest.max_level,
                    self.can_take_quest(quest),
                )
            })
            .collect::<Vec<_>>();
        let menu_items =
            self.represented_creature_quest_menu_items_like_cpp(quest_store, creature_entry);
        info!(
            creature_entry,
            race = self.player_race_like_cpp(),
            class = self.player_class_like_cpp(),
            level = self.player_level_like_cpp(),
            starter_candidates = ?starter_candidates,
            quests = ?menu_items.iter().map(|item| item.quest.id).collect::<Vec<_>>(),
            "Prepared creature gossip quest text like C++"
        );

        menu_items
            .iter()
            .map(|item| self.gossip_text_from_menu_item_like_cpp(item))
            .collect()
    }
    fn gossip_text_from_menu_item_like_cpp(
        &self,
        menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
    ) -> ClientGossipText {
        let quest = &menu_item.quest;
        ClientGossipText {
            quest_id: quest.id as i32,
            content_tuning_id: 0,
            quest_type: i32::from(menu_item.quest_icon),
            quest_level: quest.quest_level,
            quest_max_scaling_level: quest.quest_max_scaling_level,
            quest_flags: quest.flags,
            quest_flags_ex: quest.flags_ex,
            repeatable: quest.is_turn_in_like_cpp()
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
                && !quest.is_monthly_like_cpp(),
            important: self.represented_quest_is_important_like_cpp(quest),
            quest_title: quest.log_title.clone(),
        }
    }
}
