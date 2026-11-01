// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow selected-owner and inventory reads used by World adapters.

use super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub(super) fn plan_quest_status_for_reward_like_cpp(
        &self,
        quest_id: u32,
        status: u8,
    ) -> Option<wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp> {
        let owner = self.player.quest_objective_access_like_cpp();
        super::super::objective_progress::plan_quest_status_save_like_cpp(
            &owner,
            self.quest_state,
            self.catalogs,
            quest_id,
            status,
            self.world_test_consumer,
        )
    }

    pub(super) fn invalidate_player_quest_status_authority_like_cpp(&mut self) {
        let owner = self.player.quest_objective_access_like_cpp();
        super::super::objective_progress::invalidate_player_quest_status_authority_like_cpp(
            &owner,
            self.quest_state,
            self.world_test_consumer,
        );
    }

    pub fn account_id_like_cpp(&self) -> u32 {
        self.player.account_id_like_cpp()
    }

    pub fn plan_quest_reward_item_removal_like_cpp(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<wow_world_inventory::ExtendedCostItemTurninChange>> {
        let access = self.player.inventory_like_cpp();
        self.inventory
            .plan_quest_reward_item_removal_with_access_like_cpp(&access, item_entry, count)
    }

    pub fn represented_inventory_item_counts_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<u32, u32>> {
        let access = self.player.inventory_like_cpp();
        self.inventory
            .represented_inventory_item_counts_with_access_like_cpp(&access)
    }

    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.player.player_guid_like_cpp()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.player.player_map_id_like_cpp()
    }

    pub fn player_race_like_cpp(&self) -> u8 {
        self.player.player_race_like_cpp()
    }

    pub fn player_class_like_cpp(&self) -> u8 {
        self.player.player_class_like_cpp()
    }
}
