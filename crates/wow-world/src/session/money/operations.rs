//! Represented money and currency operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the currency types store for this session.
    pub fn set_currency_types_store(&mut self, store: Arc<CurrencyTypesStore>) {
        self.catalogs.currency_types_store = Some(store);
    }
    pub fn currency_types_store(&self) -> Option<&Arc<CurrencyTypesStore>> {
        self.catalogs.currency_types_store()
    }
    /// C++ `Player::GetCurrencyQuantity`.
    pub(crate) fn player_currency_quantity(&self, currency_id: u32) -> Option<u32> {
        let access = self.core.owned_player_currency_access_like_cpp();
        self.inventory
            .player_currency_quantity_with_access_like_cpp(&access, currency_id)
    }
    /// C++ `Player::HasCurrency`.
    pub(crate) fn has_currency(&self, currency_id: u32, amount: u32) -> bool {
        self.player_currency_quantity(currency_id)
            .is_some_and(|quantity| quantity >= amount)
    }
    /// C++ `Player::SetCurrencyFlags` + `Player::SendCurrencies`.
    pub(crate) fn represented_set_currency_flags_like_cpp(
        &mut self,
        currency_id: u32,
        flags: u8,
    ) -> bool {
        let Some(store) = self.catalogs.currency_types_store.as_ref() else {
            return false;
        };
        if !store.has_record(currency_id) {
            return false;
        }

        let Some(mut currencies) = self.player_currencies_like_cpp() else {
            return false;
        };
        if let Some(currency) = currencies.get_mut(&currency_id) {
            if currency.flags != flags {
                currency.flags = flags;
                if currency.state != PlayerCurrencyState::New {
                    currency.state = PlayerCurrencyState::Changed;
                }
            }
            if !self.set_player_currencies_like_cpp(currencies) {
                return false;
            }
        }

        let Some(packet) = self.setup_currencies_packet_like_cpp() else {
            return false;
        };
        self.send_packet(&packet);
        true
    }
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
    /// C++ `Player::AddCurrency(..., CurrencyGainSource::ItemRefund)`.
    pub(crate) fn add_currency_item_refund(
        &mut self,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        if amount == 0 {
            return Ok(None);
        }

        let Some(entry) = self
            .catalogs
            .currency_types_store
            .as_ref()
            .and_then(|store| store.get(currency_id))
            .copied()
        else {
            return Err(());
        };

        let player_team =
            player_team_for_race_cpp(crate::session::hub_ref(self).player_race_like_cpp());
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

        if currency.state != PlayerCurrencyState::New {
            currency.state = PlayerCurrencyState::Changed;
        }
        currency.quantity = currency.quantity.saturating_add(amount);

        let scaler = entry.scaler().max(1) as u32;
        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        let delta = PlayerCurrencyDelta {
            currency_id,
            quantity: currency.quantity,
            amount,
            weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                .then_some(currency.weekly_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity),
            total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
            suppress_chat_log: entry.is_suppressing_chat_log(false),
        };
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(());
        }
        Ok(Some(delta))
    }
    #[cfg(test)]
    pub(crate) async fn money_changed_like_cpp(&mut self, new_money: u64) {
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        self.quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
    #[cfg(test)]
    pub(crate) async fn apply_player_money_change_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    ) {
        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            return;
        }
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
    /// Publish an already-durable absolute money mutation without awaiting.
    /// Transactional callers use this while their exclusive money guard is
    /// still held, then drop the guard before draining criteria (which can
    /// re-enter money persistence through a quest reward).
    pub(crate) fn stage_player_money_change_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    ) -> bool {
        if !self.set_player_gold_like_cpp(new_money) {
            return false;
        }
        if old_money != new_money {
            self.quest_state
                .enqueue_represented_quest_objective_progress_like_cpp(
                    RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                        old_money,
                        new_money,
                    },
                );
        }
        true
    }
    #[cfg(test)]
    pub(crate) async fn currency_changed_like_cpp(&mut self, currency_id: u32, change: i32) {
        self.quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                    currency_id,
                    change,
                },
            );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
    pub(crate) fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_player_money_like_cpp(hub)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/money/operations/f3_shims.rs"]
mod f3_shims;
