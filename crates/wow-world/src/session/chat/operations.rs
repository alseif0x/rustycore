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
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.apply_chat_away_mode_like_cpp(&mut hub, mode, text)
    }
    #[cfg(test)]
    pub fn set_chat_strict_link_checking_kick_like_cpp(&mut self, enabled: bool) {
        self.config.chat_strict_link_checking_kick_like_cpp = enabled;
    }
    pub fn set_remote_address_like_cpp(&mut self, address: Option<String>) {
        self.core.transport.remote_address_like_cpp = address;
    }
    #[cfg(test)]
    pub(crate) fn chat_fake_message_preventing_like_cpp(&self) -> bool {
        self.config.chat_fake_message_preventing_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_strict_link_checking_kick_like_cpp(&self) -> bool {
        self.config.chat_strict_link_checking_kick_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_level_requirements_like_cpp(&self) -> ChatLevelRequirementsLikeCpp {
        self.config.chat_level_requirements_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_listen_ranges_like_cpp(&self) -> ChatListenRangesLikeCpp {
        self.config.chat_listen_ranges_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn chat_flood_config_like_cpp(&self) -> ChatFloodConfigLikeCpp {
        self.config.chat_flood_config_like_cpp
    }
    pub(crate) fn clear_player_gossip_options_like_cpp(&mut self) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_gossip_options_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.interaction.clear_gossip_options_for_test_like_cpp();
        }
        canonical || cfg!(test) && self.core.player_handle_like_cpp.is_none()
    }
    pub(crate) fn replace_player_gossip_options_like_cpp(
        &mut self,
        options: Vec<GossipOptionInfo>,
    ) -> bool {
        #[cfg(test)]
        let fixture_options = options.clone();
        let mut options = Some(options);
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_gossip_options_like_cpp(
                    options.take().expect("gossip option mutation runs once"),
                );
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.interaction
                .replace_gossip_options_for_test_like_cpp(fixture_options);
        }
        canonical || cfg!(test) && self.core.player_handle_like_cpp.is_none()
    }
    pub(crate) fn player_gossip_option_like_cpp(
        &self,
        gossip_option_id: i32,
    ) -> Option<GossipOptionInfo> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .gossip_options_like_cpp()
                .iter()
                .find(|option| option.gossip_option_id == gossip_option_id)
                .cloned()
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .interaction
                .gossip_options_for_test_like_cpp()
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
        let Some(quest_store) = self.catalogs.quests.store.as_ref() else {
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
            race = crate::session::hub_ref(self).player_race_like_cpp(),
            class = crate::session::hub_ref(self).player_class_like_cpp(),
            level = crate::session::hub_ref(self).player_level_like_cpp(),
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

#[cfg(test)]
#[path = "../../../unit_tests/session/chat/operations/f3_shims.rs"]
mod f3_shims;
