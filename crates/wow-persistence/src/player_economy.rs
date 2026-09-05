//! Existing Player money, currency, bank-slot and item-state write projections.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::LogicalDatabaseLikeCpp;

/// Commit classification for a transaction protected by the Session-owned
/// player-money exclusion fence.
///
/// The concrete adapter observes the durable money row after an ambiguous
/// COMMIT, but the Session remains the owner of reconciliation, quarantine and
/// runtime publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerMoneyTransactionOutcomeLikeCpp {
    Committed,
    DefinitelyRolledBack {
        reason: String,
    },
    CommitOutcomeUnknown {
        reason: String,
        observed_money: Option<u64>,
    },
}

/// One durable item-durability replacement, either standalone or included in
/// a player-money transaction. Gameplay selects the item and target
/// durability; the adapter owns statement identity and bind order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDurabilityRepairSaveLikeCpp {
    pub item_db_guid: u64,
    pub durability: u32,
}

impl PlayerDurabilityRepairSaveLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// The SQLx-free durable half of one absolute player-money mutation.
///
/// When repairs are present, MariaDB writes money first and every durability
/// row afterward in the same Characters transaction. Session retains the
/// exclusion fence, unknown-COMMIT reconciliation, and runtime publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMoneyTransactionRequestLikeCpp {
    pub player_guid: u64,
    pub money_after: u64,
    pub durability_repairs: Vec<PlayerDurabilityRepairSaveLikeCpp>,
}

impl PlayerMoneyTransactionRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// One non-transactional absolute money write. This preserves the existing
/// checked loot-money path, whose caller requires a definite execution result
/// before publishing the payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerMoneyWriteRequestLikeCpp {
    pub player_guid: u64,
    pub money: u64,
}

impl PlayerMoneyWriteRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// One represented personal-bank slot purchase persisted atomically with the
/// resulting absolute player-money value.
///
/// Gameplay owns banker/price validation and runtime publication. The
/// concrete adapter owns the MariaDB statement, expected-row contract and
/// ambiguous-COMMIT observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBankSlotPurchaseRequestLikeCpp {
    pub player_guid: u64,
    pub money_after: u64,
    pub bank_slot_count: u8,
}

impl PlayerBankSlotPurchaseRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// SQLx-free recovery read for the Characters-database half of one uncaged
/// battle-pet item. Rust uses this after the independent Login-database pet
/// commit so item destruction can be retried without deleting another
/// character's item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerUncageItemStateRequestLikeCpp {
    pub player_guid: u64,
    pub item_guid: u64,
}

impl PlayerUncageItemStateRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerUncageItemStateLikeCpp {
    pub owner_guid: Option<u64>,
    pub inventory_linked: bool,
}

/// A failed read stays distinct from a durably absent item. Callers must not
/// turn adapter failure into the idempotent `(None, false)` postcondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerUncageItemStateLoadOutcomeLikeCpp {
    Loaded(PlayerUncageItemStateLikeCpp),
    Failed { reason: String },
}

/// C++ `PlayerCurrencyState` rows that `_SaveCurrency` writes durably.
/// Unchanged/removed rows never cross the persistence boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCurrencySaveKindLikeCpp {
    New,
    Changed,
}

/// One SQLx-free `_SaveCurrency` row. Gameplay owns state selection and the
/// adapter owns the REPLACE/UPDATE statement identity and bind order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCurrencySaveRowLikeCpp {
    pub kind: PlayerCurrencySaveKindLikeCpp,
    pub currency_id: u16,
    pub quantity: u32,
    pub weekly_quantity: u32,
    pub tracked_quantity: u32,
    pub increased_cap_quantity: u32,
    pub earned_quantity: u32,
    pub flags: u8,
}

/// Ordered Characters-database half of one standalone Player currency save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCurrencySaveRequestLikeCpp {
    pub player_guid: u64,
    pub rows: Vec<PlayerCurrencySaveRowLikeCpp>,
}

impl PlayerCurrencySaveRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}
