//! currency operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    /// C++ `Player::AddCurrency(..., CurrencyGainSource::*QuestReward*)`.
    ///
    /// This represented seam intentionally fails closed for award conditions and reputation
    /// conversion currencies until those runtime systems are available to quest rewards.
    pub(crate) fn add_currency_quest_reward_like_cpp(
        &mut self,
        currency_id: u32,
        amount: u32,
        gain_source: CurrencyGainSourceLikeCpp,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        if amount == 0 {
            return Ok(None);
        }

        let Some(entry) = self
            .currency_types_store
            .as_ref()
            .and_then(|store| store.get(currency_id))
            .copied()
        else {
            return Err(());
        };

        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        if (entry.is_alliance() && player_team != Team::Alliance)
            || (entry.is_horde() && player_team != Team::Horde)
        {
            return Ok(None);
        }

        if entry.award_condition_id != 0 {
            return Err(());
        }
        if entry.faction_id != 0 || currency_id == CurrencyTypes::Azerite as u32 {
            return Ok(None);
        }

        let mut currencies = self.player_currencies_like_cpp().ok_or(())?;
        let currency = currencies.entry(currency_id).or_insert(PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 0,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        });

        let Some(delta) = currency.apply_quest_gain(currency_id, amount, &entry, gain_source) else {
            return Ok(None);
        };
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(());
        }
        Ok(Some(delta))
    }
}
