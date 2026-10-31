//! Coherent currency-row capture for the character full-save transaction.

use wow_entities::Player;
use wow_persistence::{
    PlayerCurrencySaveKindLikeCpp, PlayerCurrencySaveRequestLikeCpp, PlayerCurrencySaveRowLikeCpp,
};

pub(super) fn capture(
    player: &Player,
    currency_types_store: Option<&wow_data::CurrencyTypesStore>,
    player_guid: u64,
) -> Option<PlayerCurrencySaveRequestLikeCpp> {
    if !player.gameplay_state().currencies_loaded_like_cpp {
        return None;
    }
    let store = currency_types_store?;
    let mut rows = Vec::new();
    for (&currency_id, currency) in &player.gameplay_state().currencies {
        let kind = match currency.state {
            wow_entities::PlayerCurrencyState::New => PlayerCurrencySaveKindLikeCpp::New,
            wow_entities::PlayerCurrencyState::Changed => PlayerCurrencySaveKindLikeCpp::Changed,
            wow_entities::PlayerCurrencyState::Unchanged
            | wow_entities::PlayerCurrencyState::Removed => continue,
        };
        if !store.has_record(currency_id) {
            return None;
        }
        let Some(currency_db_id) = u16::try_from(currency_id).ok() else {
            return None;
        };
        rows.push(PlayerCurrencySaveRowLikeCpp {
            kind,
            currency_id: currency_db_id,
            quantity: currency.quantity,
            weekly_quantity: currency.weekly_quantity,
            tracked_quantity: currency.tracked_quantity,
            increased_cap_quantity: currency.increased_cap_quantity,
            earned_quantity: currency.earned_quantity,
            flags: currency.flags,
        });
    }
    rows.sort_by_key(|row| row.currency_id);
    Some(PlayerCurrencySaveRequestLikeCpp { player_guid, rows })
}
