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
