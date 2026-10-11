// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_item_currency_cost_store(&mut self, store: Arc<ItemCurrencyCostStore>) {
        self.catalogs.set_item_currency_cost_store(store)
    }
}

impl WorldSession {
    /// Publish the C++ vendor gain immediately for callers that do not own a
    /// wider durable transaction. Persistence-sensitive vendor handlers use
    /// [`Self::plan_add_currency_vendor_like_cpp`] and publish only after
    /// their combined item/currency transaction commits.
    pub(crate) fn add_currency_vendor(
        &mut self,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        let mut currencies = self.player_currencies_like_cpp().ok_or(())?;
        let delta = self.plan_add_currency_vendor_like_cpp(&mut currencies, currency_id, amount)?;
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(());
        }
        Ok(delta)
    }
}
