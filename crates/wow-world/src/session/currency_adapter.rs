// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Currency adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{CurrencyTypesEntry, HashMap, PlayerCurrency, Team, WorldSession};

pub(crate) use wow_data_model::currency::CurrencyGainDelta as PlayerCurrencyDelta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestObjectiveProgressEventLikeCpp {
    MoneyChanged {
        old_money: u64,
        new_money: u64,
    },
    #[allow(dead_code)]
    CurrencyChanged {
        currency_id: u32,
        change: i32,
    },
    #[allow(dead_code)]
    ReputationChanged {
        faction_id: u32,
        change: i32,
    },
}

pub(crate) use wow_constants::currency::CurrencyGainSourceLikeCpp;

pub(crate) fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}

pub(in crate::session) fn currency_max_quantity_cpp(
    entry: &CurrencyTypesEntry,
    currency: &PlayerCurrency,
) -> u32 {
    currency.max_quantity(entry)
}

impl WorldSession {
    pub(crate) fn set_player_currencies_like_cpp(
        &mut self,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_currencies_like_cpp(currencies.clone());
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_currencies = currencies;
            return true;
        }
        canonical
    }

    pub(crate) fn clear_player_currencies_like_cpp(&mut self) -> bool {
        self.set_player_currencies_like_cpp(HashMap::new())
    }

    pub(crate) fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().currencies.clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_currencies.clone());
        }
        canonical
    }
}
