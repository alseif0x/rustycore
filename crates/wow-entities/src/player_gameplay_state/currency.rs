//! Quest gains on the canonical Player currency record.
//!
//! Reference: a5f8da2e `Player.cpp`, ModifyCurrency (6916), AddCurrency (7048),
//! GetCurrencyMaxQuantity (7182), and GetCurrencyWeeklyCap (7204).
//! The represented quest path retains its saturating arithmetic and limited
//! ignore-cap sources; application catalog and award-condition gates stay outside.

use super::{PlayerCurrency, PlayerCurrencyState};
use wow_constants::{CurrencyTypesFlags, currency::CurrencyGainSourceLikeCpp};
use wow_data_model::currency::{CurrencyGainDelta, CurrencyTypesEntry};

#[cfg(test)]
mod tests;

impl PlayerCurrency {
    pub fn apply_quest_gain(
        &mut self,
        currency_id: u32,
        amount: u32,
        entry: &CurrencyTypesEntry,
        gain_source: CurrencyGainSourceLikeCpp,
    ) -> Option<CurrencyGainDelta> {
        let ignore_caps = matches!(
            gain_source,
            CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps
                | CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps
        );
        let weekly_cap = entry.max_earnable_per_week;
        let mut applied = amount;
        if !ignore_caps {
            if weekly_cap != 0 && self.weekly_quantity.saturating_add(applied) > weekly_cap {
                applied = weekly_cap.saturating_sub(self.weekly_quantity);
            }

            let max_quantity = self.max_quantity(entry);
            if max_quantity != 0 && self.quantity.saturating_add(applied) > max_quantity {
                applied = max_quantity.saturating_sub(self.quantity);
            }
        }

        if applied == 0 {
            return None;
        }

        if self.state != PlayerCurrencyState::New {
            self.state = PlayerCurrencyState::Changed;
        }
        self.quantity = self.quantity.saturating_add(applied);
        if !ignore_caps {
            if weekly_cap != 0 {
                self.weekly_quantity = self.weekly_quantity.saturating_add(applied);
            }
            if entry.is_tracking_quantity() {
                self.tracked_quantity = self.tracked_quantity.saturating_add(applied);
            }
            if entry.has_total_earned() {
                self.earned_quantity = self.earned_quantity.saturating_add(applied);
            }
        }

        let scaler = entry.scaler().max(1) as u32;
        let max_quantity = self.max_quantity(entry);
        Some(CurrencyGainDelta {
            currency_id,
            quantity: self.quantity,
            amount: applied,
            weekly_quantity: ((self.weekly_quantity / scaler) > 0)
                .then_some(self.weekly_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity),
            total_earned: entry.has_total_earned().then_some(self.earned_quantity),
            suppress_chat_log: entry.is_suppressing_chat_log(false),
        })
    }

    pub fn max_quantity(&self, entry: &CurrencyTypesEntry) -> u32 {
        if !entry.has_max_quantity(false, false) {
            return 0;
        }

        let mut max_quantity = entry.max_qty;
        if entry.flags.contains(CurrencyTypesFlags::DYNAMIC_MAXIMUM) {
            max_quantity = max_quantity.saturating_add(self.increased_cap_quantity);
        }
        max_quantity
    }
}
