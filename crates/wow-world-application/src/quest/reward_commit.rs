// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;
use tracing::warn;
use wow_data::CurrencyTypesStore;
use wow_entities::PlayerCurrency;
use wow_world_core::session::QuestRewardPlayerAccessLikeCpp;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;

use crate::QuestRewardDurablePlanLikeCpp;

use super::SessionQuestState;
use super::money_persistence::begin_exclusive_player_money_persistence_like_cpp;

/// Concrete participants for committing one already-planned quest reward.
pub struct QuestRewardCommitCx<'a, 'player> {
    inventory: &'a mut InventoryState,
    lifecycle: &'a mut SessionLifecycleState,
    quest_state: &'a mut SessionQuestState,
    player: &'a mut QuestRewardPlayerAccessLikeCpp<'player>,
    currency_types: Option<&'a CurrencyTypesStore>,
    _world_test_consumer: bool,
}

impl<'a, 'player> QuestRewardCommitCx<'a, 'player> {
    pub fn new(
        inventory: &'a mut InventoryState,
        lifecycle: &'a mut SessionLifecycleState,
        quest_state: &'a mut SessionQuestState,
        player: &'a mut QuestRewardPlayerAccessLikeCpp<'player>,
        currency_types: Option<&'a CurrencyTypesStore>,
        world_test_consumer: bool,
    ) -> Self {
        Self {
            inventory,
            lifecycle,
            quest_state,
            player,
            currency_types,
            _world_test_consumer: world_test_consumer,
        }
    }

    pub fn player_mut(&mut self) -> &mut QuestRewardPlayerAccessLikeCpp<'player> {
        self.player
    }

    /// Commit the operation's single character transaction.
    pub async fn commit_like_cpp(
        &mut self,
        mut plan: QuestRewardDurablePlanLikeCpp,
        quest_id: u32,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        let owner_guid = plan.owner_guid();

        // Project the complete currency state from a copy. The canonical
        // flags are only cleared after a known commit.
        let mut currencies = self
            .inventory
            .player_currencies_with_quest_reward_access_like_cpp(&self.player)?;
        let currency_save = wow_world_core::session::plan_player_currency_save_for_store_like_cpp(
            self.currency_types,
            owner_guid,
            &mut currencies,
        );
        let currency_rows_present = !currency_save.rows.is_empty();
        if currency_rows_present {
            plan.set_currencies(currency_save);
        }

        let money = plan.money();
        let request = plan.into_request()?;
        let port = self
            .lifecycle
            .player_quest_reward_persistence_port_like_cpp();

        // Preserve World’s cfg(test)-only no-I/O seam. App's own cfg(test)
        // does not identify the original consumer's build mode.
        #[cfg(any(test, feature = "test-fixtures"))]
        if port.is_none()
            && self._world_test_consumer
            && let Some(success) = self.lifecycle.loot_money_persistence_test_result_like_cpp()
        {
            if !success {
                return None;
            }
            return self.apply_committed_state_like_cpp(
                money,
                currency_rows_present.then_some(currencies),
            );
        }

        let Some(port) = port else {
            warn!(
                account = self.player.account_id_like_cpp(),
                quest_id,
                "Quest reward has no durable owner installed; the operation completed in memory only"
            );
            return Some(None);
        };

        match money {
            Some(money) => {
                let fence = begin_exclusive_player_money_persistence_like_cpp(
                    self.lifecycle,
                    self.inventory,
                    self.quest_state,
                    &mut self.player,
                )
                .await?;
                let commit = async move {
                    quest_reward_money_outcome_like_cpp(
                        port.persist_quest_reward_like_cpp(request).await,
                    )
                };
                let outcome = {
                    let mut access = self.player.money_transaction_access_like_cpp();
                    self.lifecycle
                        .await_exclusive_player_money_transaction_outcome_with_access_like_cpp(
                            &mut access,
                            fence,
                            commit,
                            money.money_before,
                            money.money_after,
                            "quest reward transaction",
                        )
                        .await
                };
                let Some(fence) = outcome else {
                    self.fail_after_rollback_like_cpp(
                        quest_id,
                        "the money-fenced reward transaction did not commit",
                    );
                    return None;
                };
                let applied = self.apply_committed_state_like_cpp(
                    Some(money),
                    currency_rows_present.then_some(currencies),
                );
                drop(fence);
                applied
            }
            None => match port.persist_quest_reward_like_cpp(request).await {
                wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::Committed => self
                    .apply_committed_state_like_cpp(
                        None,
                        currency_rows_present.then_some(currencies),
                    ),
                wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack {
                    reason,
                } => {
                    self.fail_after_rollback_like_cpp(quest_id, &reason);
                    None
                }
                wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::CommitOutcomeUnknown {
                    reason,
                    witness,
                } => self.resolve_unknown_no_money_commit_like_cpp(
                    quest_id,
                    reason,
                    witness,
                    currency_rows_present.then_some(currencies),
                ),
            },
        }
    }

    fn fail_after_rollback_like_cpp(&mut self, quest_id: u32, reason: &str) {
        warn!(
            account = self.player.account_id_like_cpp(),
            quest_id,
            error = %reason,
            "Quest reward transaction did not commit; nothing was granted and the session is quarantined"
        );
        self.player.quarantine_like_cpp(
            "quest reward transaction rolled back; relog required to resync durable state",
        );
    }

    fn apply_committed_state_like_cpp(
        &mut self,
        money: Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>,
        currencies: Option<HashMap<u32, PlayerCurrency>>,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        if let Some(money) = money
            && !self
                .inventory
                .set_player_gold_with_quest_reward_access_like_cpp(
                    &self.player,
                    money.money_after,
                )
        {
            self.player.quarantine_like_cpp(
                "canonical Player money owner became unavailable after durable COMMIT",
            );
            return None;
        }
        if let Some(currencies) = currencies
            && !self
                .inventory
                .set_player_currencies_with_quest_reward_access_like_cpp(
                    &self.player,
                    currencies,
                )
        {
            self.player.quarantine_like_cpp(
                "canonical Player currency owner became unavailable after durable COMMIT",
            );
            return None;
        }
        Some(money)
    }

    /// This branch is reachable only for rewards without a money mutation.
    fn resolve_unknown_no_money_commit_like_cpp(
        &mut self,
        quest_id: u32,
        reason: String,
        witness: wow_persistence::PlayerQuestRewardCommitWitnessLikeCpp,
        currencies: Option<HashMap<u32, PlayerCurrency>>,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        let observed = match witness {
            wow_persistence::PlayerQuestRewardCommitWitnessLikeCpp::QuestStatus {
                observed_matches_request,
            } => observed_matches_request,
            _ => None,
        };
        match observed {
            Some(true) => {
                warn!(
                    account = self.player.account_id_like_cpp(),
                    quest_id,
                    error = %reason,
                    "Quest reward COMMIT reply was lost but the durable quest status proves the transaction committed"
                );
                self.apply_committed_state_like_cpp(None, currencies)
            }
            Some(false) => {
                warn!(
                    account = self.player.account_id_like_cpp(),
                    quest_id,
                    error = %reason,
                    "Quest reward COMMIT reply was lost but the durable quest status proves the transaction rolled back"
                );
                None
            }
            None => {
                self.player.quarantine_like_cpp(
                    "quest reward COMMIT outcome is unknown; relog required before another reward",
                );
                warn!(
                    account = self.player.account_id_like_cpp(),
                    quest_id,
                    error = %reason,
                    "Quest reward COMMIT outcome remains indeterminate; quarantined the session"
                );
                None
            }
        }
    }
}

/// Adapt the Quest port result to Lifecycle’s existing money classifier.
fn quest_reward_money_outcome_like_cpp(
    outcome: wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp,
) -> wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp {
    match outcome {
        wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::Committed => {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed
        }
        wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack { reason } => {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack { reason }
        }
        wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::CommitOutcomeUnknown {
            reason,
            witness,
        } => wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
            reason,
            observed_money: match witness {
                wow_persistence::PlayerQuestRewardCommitWitnessLikeCpp::Money {
                    observed_money,
                } => observed_money,
                _ => None,
            },
        },
    }
}
