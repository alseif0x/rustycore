// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free durable contract for one complete quest-reward operation.
//!
//! C++ `Player::RewardQuest` (Player.cpp:14625) applies every grant in memory
//! and closes the operation with a single `SaveToDB(false)` (Player.cpp:14867),
//! whose character half is one transaction (Player.cpp:19312). The reward's
//! durable participants inside that transaction are `_SaveInventory`,
//! `_SaveQuestStatus`, the daily/weekly/seasonal/monthly quest-status groups
//! and `_SaveCurrency` (Player.cpp:19630..19655), plus the money carried by the
//! character row.
//!
//! This contract expresses exactly those participants as one request so the
//! operation commits or rolls back as a unit. Reward mail keeps its own
//! transaction in C++ (Player.cpp:14794) and is deliberately not part of it.
//!
//! Groups the reward does not modify are not rewritten here. The durable
//! outcome for the operation is the same as the full C++ save, and the
//! divergence is recorded rather than silently widened into a global save.

use crate::{
    LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PlayerCurrencySaveRequestLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp, PlayerQuestLockoutPersistenceRequestLikeCpp,
    PlayerQuestStatusPersistenceRequestLikeCpp,
};

/// Absolute player money produced by the operation, written inside the same
/// transaction as `SaveGoldToDB` does inside the C++ character save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestRewardMoneyLikeCpp {
    /// Money observed before the operation planned its grants. Retained so an
    /// ambiguous COMMIT can be reconciled against the durable value.
    pub money_before: u64,
    pub money_after: u64,
}

/// One complete quest-reward operation expressed as durable data.
///
/// The vectors keep the operation's own order: objective and item-drop
/// removals precede reward grants, exactly as `Player::RewardQuest` sequences
/// them, so the committed batch replays the gameplay order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestRewardDurableRequestLikeCpp {
    pub owner_guid: u64,
    pub quest_id: u32,
    /// `_SaveInventory`: removals and grants of this operation, in order.
    pub inventory_mutations: Vec<PlayerInventoryPersistenceRequestLikeCpp>,
    /// `_SaveCurrency`: the complete currency state after every grant, or
    /// `None` when the operation changed no currency.
    pub currencies: Option<PlayerCurrencySaveRequestLikeCpp>,
    /// The character row's money, or `None` when the reward grants none.
    pub money: Option<PlayerQuestRewardMoneyLikeCpp>,
    /// `_SaveQuestStatus` for the rewarded quest: a rewarded row for a
    /// non-repeatable quest, a delete for a repeatable one.
    pub quest_status: PlayerQuestStatusPersistenceRequestLikeCpp,
    /// `_SaveDailyQuestStatus` and its weekly/monthly/seasonal siblings.
    pub lockouts: Vec<PlayerQuestLockoutPersistenceRequestLikeCpp>,
}

impl PlayerQuestRewardDurableRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// What the adapter could still observe after an ambiguous COMMIT.
///
/// Both witnesses are written by the same transaction, so either one proves
/// the whole operation's fate. Money is preferred when the reward changes it,
/// because its absolute value is already the reconciliation subject of the
/// established exclusive-money contract; the quest-status row answers for
/// rewards that grant no money.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerQuestRewardCommitWitnessLikeCpp {
    /// Durable money read back after the lost reply.
    Money { observed_money: Option<u64> },
    /// Whether the rewarded quest's durable status row matches the request.
    QuestStatus {
        observed_matches_request: Option<bool>,
    },
    /// Nothing could be observed; the operation stays indeterminate.
    None,
}

/// Durable fate of one quest-reward operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerQuestRewardCommitOutcomeLikeCpp {
    Committed,
    DefinitelyRolledBack {
        reason: String,
    },
    CommitOutcomeUnknown {
        reason: String,
        witness: PlayerQuestRewardCommitWitnessLikeCpp,
    },
}

/// Persists one quest-reward operation as a single character transaction.
pub trait PlayerQuestRewardPersistencePortLikeCpp: Send + Sync {
    fn persist_quest_reward_like_cpp(
        &self,
        request: PlayerQuestRewardDurableRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerQuestRewardCommitOutcomeLikeCpp>;
}
