// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;
use wow_data::CurrencyTypesStore;
use wow_entities::{PlayerCurrency, PlayerCurrencyState};

pub fn plan_player_currency_save_for_store_like_cpp(
    store: Option<&CurrencyTypesStore>,
    character_guid: u64,
    currencies: &mut HashMap<u32, PlayerCurrency>,
) -> wow_persistence::PlayerCurrencySaveRequestLikeCpp {
    let mut rows = Vec::new();
    let Some(store) = store else {
        return wow_persistence::PlayerCurrencySaveRequestLikeCpp {
            player_guid: character_guid,
            rows,
        };
    };
    for (&currency_id, currency) in currencies.iter_mut() {
        if !store.has_record(currency_id) {
            continue;
        }
        let Ok(currency_db_id) = u16::try_from(currency_id) else {
            continue;
        };

        match currency.state {
            PlayerCurrencyState::New => {
                rows.push(wow_persistence::PlayerCurrencySaveRowLikeCpp {
                    kind: wow_persistence::PlayerCurrencySaveKindLikeCpp::New,
                    currency_id: currency_db_id,
                    quantity: currency.quantity,
                    weekly_quantity: currency.weekly_quantity,
                    tracked_quantity: currency.tracked_quantity,
                    increased_cap_quantity: currency.increased_cap_quantity,
                    earned_quantity: currency.earned_quantity,
                    flags: currency.flags,
                });
                currency.state = PlayerCurrencyState::Unchanged;
            }
            PlayerCurrencyState::Changed => {
                rows.push(wow_persistence::PlayerCurrencySaveRowLikeCpp {
                    kind: wow_persistence::PlayerCurrencySaveKindLikeCpp::Changed,
                    currency_id: currency_db_id,
                    quantity: currency.quantity,
                    weekly_quantity: currency.weekly_quantity,
                    tracked_quantity: currency.tracked_quantity,
                    increased_cap_quantity: currency.increased_cap_quantity,
                    earned_quantity: currency.earned_quantity,
                    flags: currency.flags,
                });
                currency.state = PlayerCurrencyState::Unchanged;
            }
            PlayerCurrencyState::Unchanged | PlayerCurrencyState::Removed => {}
        }
    }
    wow_persistence::PlayerCurrencySaveRequestLikeCpp {
        player_guid: character_guid,
        rows,
    }
}

impl crate::session::state::SessionCatalogs {
    /// C++ `Player::_SaveCurrency` plan for changed/new currency rows.
    /// Gameplay owns filtering and state transitions; the persistence adapter
    /// owns statement identity, bind order, and transaction execution.
    pub fn plan_player_currency_save_like_cpp(
        &self,
        character_guid: u64,
        currencies: &mut HashMap<u32, PlayerCurrency>,
    ) -> wow_persistence::PlayerCurrencySaveRequestLikeCpp {
        plan_player_currency_save_for_store_like_cpp(
            self.currency_types_store.as_deref(),
            character_guid,
            currencies,
        )
    }
}
