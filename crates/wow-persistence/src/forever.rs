//! Build-70170 authenticated account and character-selection persistence.
//! SQL-free DTOs; the session owns the admitted snapshot, adapters own queries.

use crate::PersistenceFutureLikeCpp;
use std::collections::BTreeMap;

pub const ACCOUNT_DATA_TYPES: usize = 20;
pub const GLOBAL_CACHE_MASK: u32 = 0x000B_A515;

// No Debug: account-data strings are private even in a disposable QA account.
#[derive(Clone, Default)]
pub struct AccountData {
    pub time: i64,
    pub data: String,
}

pub struct AccountSnapshot {
    pub account_data: [AccountData; ACCOUNT_DATA_TYPES],
    pub tutorials: [u32; 8],
    pub instance_release_times: BTreeMap<u32, i64>,
    pub realm_character_counts: BTreeMap<u32, u8>,
}

/// Metadata-only errors: never return SQL errors, URLs or row values to logging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    Database,
    InvalidRow,
    /// Nonempty collections/characters need their target operation ported.
    /// This is an explicit admission boundary, never an empty-result fallback.
    UnsupportedState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassRequirement {
    pub class_id: u8,
    pub active_expansion: u8,
    pub account_expansion: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaceClassRequirement {
    pub race_id: u8,
    pub class: ClassRequirement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaceUnlockRequirement {
    pub race_id: u8,
    pub expansion: u8,
    pub achievement_id: u32,
}

pub struct AvailabilityRows {
    pub classes: Vec<RaceClassRequirement>,
    pub unlocks: Vec<RaceUnlockRequirement>,
}

/// One query-holder responsibility, not a pool/table-oriented capability bag.
/// Zero rows mean real empty state; query errors must remain errors.
pub trait SessionRepository: Send + Sync {
    fn load_account(
        &self,
        account: u32,
        battlenet: u32,
        realm: u32,
    ) -> PersistenceFutureLikeCpp<'_, Result<AccountSnapshot, LoadError>>;

    fn enumerate_empty(&self, account: u32) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>>;
}

pub trait AvailabilityRepository: Send + Sync {
    fn load_availability(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<AvailabilityRows, LoadError>>;
}
