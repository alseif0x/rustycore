// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical Player roles used by the quest-reward application.

use std::collections::HashMap;

use wow_entities::PlayerCurrency;

use super::inventory::OwnedInventoryAccessLikeCpp;
use crate::session::{PlayerMoneyTransactionSessionAccessLikeCpp, SessionCore};

/// Borrowed, operation-specific access to the canonical quest-reward owner.
pub struct QuestRewardPlayerAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
}

/// Shared-borrow canonical currency capability; unlike the larger mutable
/// reward owner, the existing Inventory getter can construct it from HubRef.
pub struct OwnedPlayerCurrencyAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build the selected Player capability without resolving or copying the Player.
    pub fn quest_reward_player_access_like_cpp(&mut self) -> QuestRewardPlayerAccessLikeCpp<'_> {
        QuestRewardPlayerAccessLikeCpp { core: self }
    }

    pub fn owned_player_currency_access_like_cpp(&self) -> OwnedPlayerCurrencyAccessLikeCpp<'_> {
        OwnedPlayerCurrencyAccessLikeCpp { core: self }
    }
}

impl OwnedPlayerCurrencyAccessLikeCpp<'_> {
    pub fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().currencies.clone())
    }

    pub fn set_player_currencies_like_cpp(
        &self,
        currencies: &HashMap<u32, PlayerCurrency>,
    ) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.install_currencies_like_cpp(currencies.clone());
            })
            .is_some()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}

impl QuestRewardPlayerAccessLikeCpp<'_> {
    pub fn inventory_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    pub fn currency_like_cpp(&self) -> OwnedPlayerCurrencyAccessLikeCpp<'_> {
        self.core.owned_player_currency_access_like_cpp()
    }

    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    pub fn money_transaction_access_like_cpp(
        &mut self,
    ) -> PlayerMoneyTransactionSessionAccessLikeCpp<'_> {
        self.core.player_money_transaction_access_like_cpp()
    }

    pub fn quarantine_like_cpp(&mut self, reason: &str) {
        self.core.kick(reason);
    }
}
