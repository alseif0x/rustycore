// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Closing one quest-reward operation: the commit and everything that may
//! only happen once its outcome is known.
//!
//! C++ reaches the database once here, through `SaveToDB(false)`
//! (Player.cpp:14867). The planning half lives with the reward handler that
//! decides the grants; this module owns the durable boundary itself.

use std::collections::HashMap;

use tracing::warn;
use wow_entities::PlayerCurrency;

use crate::session::WorldSession;

use super::QuestRewardDurablePlanLikeCpp;

impl WorldSession {
    /// Commit the operation's single character transaction.
    ///
    /// Returns `None` when the reward must not be completed, and otherwise the
    /// money the operation made durable. C++ reaches the database once here,
    /// through `SaveToDB(false)` (Player.cpp:14867).
    ///
    /// Departure with an explicit contract: C++ publishes the reward before
    /// that save, and applies money to memory before it. This server keeps the
    /// established exclusive-money fence instead, so player money becomes
    /// visible only after a known COMMIT. The fence is stricter than Classic,
    /// never weaker, and the money row doubles as the transaction's commit
    /// witness.
    pub(crate) async fn commit_quest_reward_plan_like_cpp(
        &mut self,
        mut plan: QuestRewardDurablePlanLikeCpp,
        quest_id: u32,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        let owner_guid = plan.owner_guid();

        // `_SaveCurrency` (Player.cpp:19654) writes the player's complete
        // currency state once. Project it from a copy so the change flags are
        // only cleared once the batch is known to have committed.
        let mut currencies = self.player_currencies_like_cpp()?;
        let currency_save = self.plan_player_currency_save_like_cpp(owner_guid, &mut currencies);
        let currency_rows_present = !currency_save.rows.is_empty();
        if currency_rows_present {
            plan.set_currencies(currency_save);
        }

        let money = plan.money();
        let request = plan.into_request()?;

        let port = self.player_quest_reward_persistence_port_like_cpp();

        // Narrow unit fixtures that install no durable owner keep the explicit
        // no-I/O money seam, so assertions unrelated to durability still run
        // the operation end to end. A fixture that does install the owner is
        // exercising the transaction itself and must reach it.
        #[cfg(test)]
        if port.is_none()
            && let Some(success) = self.loot_money_persistence_test_result_like_cpp
        {
            if !success {
                return None;
            }
            return self.apply_committed_quest_reward_state_like_cpp(
                money,
                currency_rows_present.then_some(currencies),
            );
        }

        let Some(port) = port else {
            warn!(
                account = self.account_id,
                quest_id,
                "Quest reward has no durable owner installed; the operation completed in memory only"
            );
            return Some(None);
        };

        match money {
            Some(money) => {
                let fence = self
                    .begin_exclusive_player_money_persistence_like_cpp()
                    .await?;
                let commit = async move {
                    quest_reward_money_outcome_like_cpp(
                        port.persist_quest_reward_like_cpp(request).await,
                    )
                };
                let Some(fence) = self
                    .await_exclusive_player_money_transaction_outcome_like_cpp(
                        fence,
                        commit,
                        money.money_before,
                        money.money_after,
                        "quest reward transaction",
                    )
                    .await
                else {
                    self.fail_quest_reward_after_rollback_like_cpp(
                        quest_id,
                        "the money-fenced reward transaction did not commit",
                    );
                    return None;
                };
                let applied = self.apply_committed_quest_reward_state_like_cpp(
                    Some(money),
                    currency_rows_present.then_some(currencies),
                );
                drop(fence);
                applied
            }
            None => match port.persist_quest_reward_like_cpp(request).await {
                wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::Committed => self
                    .apply_committed_quest_reward_state_like_cpp(
                        None,
                        currency_rows_present.then_some(currencies),
                    ),
                wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack {
                    reason,
                } => {
                    self.fail_quest_reward_after_rollback_like_cpp(quest_id, &reason);
                    None
                }
                wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::CommitOutcomeUnknown {
                    reason,
                    witness,
                } => self.resolve_unknown_quest_reward_commit_like_cpp(
                    quest_id,
                    reason,
                    witness,
                    currency_rows_present.then_some(currencies),
                ),
            },
        }
    }

    /// Reset a session whose reward transaction did not commit.
    ///
    /// Nothing durable changed, so the quest stays retryable and no grant is
    /// duplicated. The operation did mutate the in-memory inventory while it
    /// planned — C++ `StoreNewItem` does the same before its save — so the
    /// session is quarantined rather than left showing items the database does
    /// not have. Classic leaves that mismatch standing until relog; this ends
    /// it immediately. Reaching this after the money fence already quarantined
    /// an indeterminate COMMIT is harmless: the session is disconnecting
    /// either way.
    fn fail_quest_reward_after_rollback_like_cpp(&mut self, quest_id: u32, reason: &str) {
        warn!(
            account = self.account_id,
            quest_id,
            error = %reason,
            "Quest reward transaction did not commit; nothing was granted and the session is quarantined"
        );
        self.kick("quest reward transaction rolled back; relog required to resync durable state");
    }

    /// Apply the memory half the operation deferred until its COMMIT was known.
    fn apply_committed_quest_reward_state_like_cpp(
        &mut self,
        money: Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>,
        currencies: Option<HashMap<u32, PlayerCurrency>>,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        if let Some(money) = money
            && !self.set_player_gold_like_cpp(money.money_after)
        {
            self.kick("canonical Player money owner became unavailable after durable COMMIT");
            return None;
        }
        if let Some(currencies) = currencies
            && !self.set_player_currencies_like_cpp(currencies)
        {
            self.kick("canonical Player currency owner became unavailable after durable COMMIT");
            return None;
        }
        Some(money)
    }

    /// Decide a quest reward whose COMMIT reply was lost.
    ///
    /// Only reached when the reward grants no money, so the witness is the
    /// rewarded quest's active status row: both durable shapes of the
    /// operation delete it, and it is written by the same transaction.
    fn resolve_unknown_quest_reward_commit_like_cpp(
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
                    account = self.account_id,
                    quest_id,
                    error = %reason,
                    "Quest reward COMMIT reply was lost but the durable quest status proves the transaction committed"
                );
                self.apply_committed_quest_reward_state_like_cpp(None, currencies)
            }
            Some(false) => {
                warn!(
                    account = self.account_id,
                    quest_id,
                    error = %reason,
                    "Quest reward COMMIT reply was lost but the durable quest status proves the transaction rolled back"
                );
                None
            }
            None => {
                self.kick(
                    "quest reward COMMIT outcome is unknown; relog required before another reward",
                );
                warn!(
                    account = self.account_id,
                    quest_id,
                    error = %reason,
                    "Quest reward COMMIT outcome remains indeterminate; quarantined the session"
                );
                None
            }
        }
    }
}

/// Read one quest-reward commit as the money contract reads it.
///
/// Only used when the reward changes money, so the witness is the durable
/// money value the exclusive-money reconciliation already understands.
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
