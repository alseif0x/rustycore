// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_bank_bag_slot_prices_store(&mut self, store: Arc<BankBagSlotPricesStore>) {
        self.catalogs.set_bank_bag_slot_prices_store(store)
    }
    #[cfg(test)]
    pub(crate) fn bank_bag_slot_prices_store_for_test_like_cpp(
        &self,
    ) -> Option<&Arc<BankBagSlotPricesStore>> {
        self.catalogs.bank_bag_slot_prices_store_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_bank_item_moves_like_cpp(&self) -> &[RepresentedBankItemMoveLikeCpp] {
        self.inventory.represented_bank_item_moves_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_inventory_moves_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankInventoryMoveLikeCpp] {
        self.inventory
            .represented_guild_bank_inventory_moves_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_list_requests_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankListRequestLikeCpp] {
        self.inventory
            .represented_guild_bank_list_requests_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_money_moves_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankMoneyMoveLikeCpp] {
        self.inventory.represented_guild_bank_money_moves_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_tab_actions_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankTabActionLikeCpp] {
        self.inventory.represented_guild_bank_tab_actions_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_represented_guild_repair_bank_state_like_cpp(
        &mut self,
        state: Option<RepresentedGuildRepairBankStateLikeCpp>,
    ) {
        self.inventory
            .set_represented_guild_repair_bank_state_like_cpp(state)
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_repair_bank_withdraws_like_cpp(
        &self,
    ) -> &[RepresentedGuildRepairBankWithdrawLikeCpp] {
        self.inventory
            .represented_guild_repair_bank_withdraws_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_bank_bag_slot_count_like_cpp(&self) -> u8 {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_bank_bag_slot_count_like_cpp(hub)
    }
}
