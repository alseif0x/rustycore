// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_world_core::session::{
    HubMut, HubRef, OwnedInventoryAccessLikeCpp, QuestRewardPlayerAccessLikeCpp,
};

impl crate::InventoryState {
    pub fn set_player_gold_like_cpp(&mut self, hub: &mut HubMut<'_>, gold: u64) -> bool {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.set_player_gold_with_access_like_cpp(&access, gold)
    }

    pub fn set_player_gold_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        gold: u64,
    ) -> bool {
        let canonical = access.set_player_money_like_cpp(gold);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || access.owner_handle_absent_like_cpp() {
            self.player_gold = gold;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            canonical || access.owner_handle_absent_like_cpp()
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            canonical
        }
    }

    pub fn resolved_player_money_with_quest_reward_access_like_cpp(
        &self,
        access: &QuestRewardPlayerAccessLikeCpp<'_>,
    ) -> Option<u64> {
        self.resolved_player_money_with_access_like_cpp(&access.inventory_like_cpp())
    }

    pub fn set_player_gold_with_quest_reward_access_like_cpp(
        &mut self,
        access: &QuestRewardPlayerAccessLikeCpp<'_>,
        gold: u64,
    ) -> bool {
        self.set_player_gold_with_access_like_cpp(&access.inventory_like_cpp(), gold)
    }
}

impl crate::InventoryState {
    pub fn resolved_player_money_like_cpp(&self, hub: HubRef<'_>) -> Option<u64> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_player_money_with_access_like_cpp(&access)
    }

    pub fn resolved_player_money_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<u64> {
        let canonical = access.player_money_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            return Some(self.player_gold);
        }
        canonical
    }
}
