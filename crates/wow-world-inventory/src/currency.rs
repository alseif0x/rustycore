// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;

use wow_entities::PlayerCurrency;
use wow_world_core::session::{
    HubMut, HubRef, OwnedPlayerCurrencyAccessLikeCpp, QuestRewardPlayerAccessLikeCpp,
};

impl crate::InventoryState {
    pub fn player_currency_quantity_with_access_like_cpp(
        &self,
        access: &OwnedPlayerCurrencyAccessLikeCpp<'_>,
        currency_id: u32,
    ) -> Option<u32> {
        self.player_currencies_with_access_like_cpp(access).map(|currencies| {
            currencies
                .get(&currency_id)
                .map(|currency| currency.quantity)
                .unwrap_or(0)
        })
    }

    pub fn remove_currency_with_access_like_cpp(
        &mut self,
        access: &OwnedPlayerCurrencyAccessLikeCpp<'_>,
        currency_id: u32,
        amount: u32,
    ) -> bool {
        let Some(mut currencies) = self.player_currencies_with_access_like_cpp(access) else {
            return false;
        };
        if !wow_entities::plan_remove_currency_like_cpp(&mut currencies, currency_id, amount) {
            return false;
        }
        self.set_player_currencies_with_access_like_cpp(access, currencies)
    }

    pub fn set_player_currencies_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let access = hub.core.owned_player_currency_access_like_cpp();
        self.set_player_currencies_with_access_like_cpp(&access, currencies)
    }

    pub fn set_player_currencies_with_access_like_cpp(
        &mut self,
        access: &OwnedPlayerCurrencyAccessLikeCpp<'_>,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let canonical = access.set_player_currencies_like_cpp(&currencies);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || access.owner_handle_absent_like_cpp() {
            self.player_currencies = currencies;
            return true;
        }
        canonical
    }

    pub fn player_currencies_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<u32, PlayerCurrency>> {
        let access = hub.core.owned_player_currency_access_like_cpp();
        self.player_currencies_with_access_like_cpp(&access)
    }

    pub fn player_currencies_with_access_like_cpp(
        &self,
        access: &OwnedPlayerCurrencyAccessLikeCpp<'_>,
    ) -> Option<HashMap<u32, PlayerCurrency>> {
        let canonical = access.player_currencies_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            return Some(self.player_currencies.clone());
        }
        canonical
    }

    pub fn player_currencies_with_quest_reward_access_like_cpp(
        &self,
        access: &QuestRewardPlayerAccessLikeCpp<'_>,
    ) -> Option<HashMap<u32, PlayerCurrency>> {
        self.player_currencies_with_access_like_cpp(&access.currency_like_cpp())
    }

    pub fn set_player_currencies_with_quest_reward_access_like_cpp(
        &mut self,
        access: &QuestRewardPlayerAccessLikeCpp<'_>,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        self.set_player_currencies_with_access_like_cpp(&access.currency_like_cpp(), currencies)
    }
}
