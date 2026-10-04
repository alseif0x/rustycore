// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Currency-reward publication operation.

use super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub async fn grant_quest_reward_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice_item_id: u32,
        choice_loot_item_type: u8,
        currency_choice_type: u8,
    ) -> bool {
        let publication = self.player.packet_publication_access_like_cpp();
        super::super::currencies::grant_quest_reward_currencies_like_cpp(
            self.inventory,
            &self.player,
            &publication,
            self.currency_types,
            quest,
            choice_item_id,
            choice_loot_item_type,
            currency_choice_type,
        )
        .await
    }
}
