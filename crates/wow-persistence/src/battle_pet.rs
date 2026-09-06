//! Battle.net pet account durability and the existing trainer-purchase saga contract.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

/// Largest battle-pet counter accepted by C++-shaped ObjectGuid allocation.
pub const BATTLE_PET_GUID_COUNTER_LIMIT_LIKE_CPP: u64 = 0xFF_FFFF_FFFE;

/// Process-wide Login DB journal lease used by the canonical battle-pet owner.
pub trait BattlePetProcessLeaseLikeCpp: Send {
    fn is_valid_like_cpp(&self) -> bool {
        true
    }

    fn fence_like_cpp(&self) -> u64 {
        1
    }
}

/// Durable identity of one represented battle-pet add request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BattlePetAddRequestKeyLikeCpp([u8; 16]);

impl BattlePetAddRequestKeyLikeCpp {
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Builds the durable uncage identity from a process-lifetime item GUID's
    /// raw bytes, rejecting the all-zero empty GUID without depending on the
    /// gameplay GUID type.
    pub fn from_source_guid_bytes_like_cpp(bytes: [u8; 16]) -> Option<Self> {
        (bytes != [0; 16]).then_some(Self(bytes))
    }
}

/// SQLx- and packet-free durable declined-name projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetDeclinedNamesLikeCpp {
    pub names: [String; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableBattlePetRowLikeCpp {
    pub guid_counter: u64,
    pub species: u32,
    pub breed: u16,
    pub display_id: u32,
    pub level: u16,
    pub exp: u16,
    pub health: u32,
    pub quality: u8,
    pub flags: u16,
    pub name: String,
    pub name_timestamp: i64,
    pub owner_guid_counter: Option<u64>,
    pub declined_names: Option<BattlePetDeclinedNamesLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableBattlePetSlotLikeCpp {
    pub index: u8,
    pub pet_guid_counter: Option<u64>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedBattlePetAccountLikeCpp {
    pub pets: Vec<DurableBattlePetRowLikeCpp>,
    pub slots: Vec<DurableBattlePetSlotLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableBattlePetAddLikeCpp {
    pub account_id: u32,
    pub realm_id: u16,
    pub request_key: BattlePetAddRequestKeyLikeCpp,
    pub max_per_scope: u8,
    pub fence: u64,
    pub pet: DurableBattlePetRowLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableBattlePetAddReceiptLikeCpp {
    pub account_id: u32,
    pub requested_pet: DurableBattlePetRowLikeCpp,
    pub current_pet: Option<DurableBattlePetRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistBattlePetAddOutcomeLikeCpp {
    Inserted,
    Replayed {
        pet: DurableBattlePetRowLikeCpp,
        still_present: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePetPersistenceErrorLikeCpp {
    Database(String),
    Capacity,
    GuidCollision,
    DuplicateRequest,
    StaleAuthority,
}

/// SQLx-free durability capability consumed by the account-scoped battle-pet owner.
pub trait BattlePetAccountPersistencePortLikeCpp: Send + Sync {
    fn try_acquire_process_lease<'a>(
        &'a self,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<Option<Box<dyn BattlePetProcessLeaseLikeCpp>>, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn load_account<'a>(
        &'a self,
        account_id: u32,
        realm_id: u16,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn allocate_guid_counter_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<u64, BattlePetPersistenceErrorLikeCpp>>;

    fn insert_pet_idempotently<'a>(
        &'a self,
        request: DurableBattlePetAddLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn lookup_add_request<'a>(
        &'a self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn update_pet<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        pet: DurableBattlePetRowLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;

    fn delete_pet<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        pet_guid_counter: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;

    fn replace_slots<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;
}

/// Durable states of the recoverable battle-pet trainer-purchase saga.
///
/// These numeric values are persisted in `character_battle_pet_purchase` and
/// therefore belong to the SQLx-free contract rather than to either the
/// gameplay caller or the MariaDB adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePetPurchaseStatusLikeCpp {
    /// Money charged and command durable; pet not yet confirmed durable.
    PendingApplication,
    /// Durable pet receipt confirmed and the success transition committed.
    Completed,
    /// Terminal application failure recorded; an exact-once refund is owed.
    CompensationPending,
    /// Refund and status flip committed in the same Character transaction.
    Compensated,
    /// The character row disappeared, so automatic refund cannot converge.
    TerminalFailure,
}

impl BattlePetPurchaseStatusLikeCpp {
    pub fn as_u8_like_cpp(self) -> u8 {
        match self {
            Self::PendingApplication => 0,
            Self::Completed => 1,
            Self::CompensationPending => 2,
            Self::Compensated => 3,
            Self::TerminalFailure => 4,
        }
    }

    pub fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::PendingApplication),
            1 => Some(Self::Completed),
            2 => Some(Self::CompensationPending),
            3 => Some(Self::Compensated),
            4 => Some(Self::TerminalFailure),
            _ => None,
        }
    }

    pub fn is_terminal_like_cpp(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Compensated | Self::TerminalFailure
        )
    }
}

/// Stable input and durable projection of one battle-pet trainer purchase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetPurchaseCommandLikeCpp {
    pub request_key: [u8; 16],
    pub character_guid: u64,
    pub account_id: u32,
    pub trainer_id: u32,
    pub spell_id: u32,
    pub species: u32,
    pub breed: u16,
    pub quality: u8,
    pub display_id: u32,
    pub level: u16,
    pub price: u32,
    pub money_before: u64,
    pub money_after: u64,
    pub status: BattlePetPurchaseStatusLikeCpp,
    pub published: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePetPurchaseStoreErrorLikeCpp {
    /// The attempted transition definitely did not commit and may be retried.
    Retryable(String),
    /// A durable precondition makes this transition permanently invalid.
    Terminal(String),
    /// COMMIT attribution or its required reconciliation is unknown.
    Indeterminate(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePetPurchaseChargeOutcomeLikeCpp {
    Charged,
    /// Definitely no charge and no command.
    RolledBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePetPurchaseMarkOutcomeLikeCpp {
    Applied,
    AlreadyApplied,
    /// A concurrent driver completed the purchase; compensation is forbidden.
    ConflictedCompleted,
    /// Compensation/terminal failure won while completion was requested.
    ConflictedCompensated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePetPurchaseCompensationOutcomeLikeCpp {
    Compensated {
        durable_money: u64,
    },
    /// Replay of an already committed refund; durable money is authoritative.
    AlreadyCompensated {
        durable_money: u64,
    },
    ConflictedCompleted,
    CharacterMissing,
}

/// Cancellation-sensitive Character COMMIT fence supplied by the gameplay
/// money owner. The database adapter arms it immediately before awaiting a
/// COMMIT and disarms it only after the outcome is safe to publish or retry.
pub trait BattlePetPurchaseCommitFenceLikeCpp: Send {
    fn arm_like_cpp(&mut self);
    fn disarm_like_cpp(&mut self);
}

pub fn reconcile_battle_pet_purchase_charge_like_cpp(
    row: Option<&BattlePetPurchaseCommandLikeCpp>,
    expected: &BattlePetPurchaseCommandLikeCpp,
) -> BattlePetPurchaseChargeOutcomeLikeCpp {
    match row {
        Some(row)
            if row.request_key == expected.request_key
                && row.character_guid == expected.character_guid
                && row.account_id == expected.account_id
                && row.trainer_id == expected.trainer_id
                && row.spell_id == expected.spell_id
                && row.species == expected.species
                && row.breed == expected.breed
                && row.quality == expected.quality
                && row.display_id == expected.display_id
                && row.level == expected.level
                && row.price == expected.price
                && row.money_before == expected.money_before
                && row.money_after == expected.money_after =>
        {
            BattlePetPurchaseChargeOutcomeLikeCpp::Charged
        }
        _ => BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack,
    }
}

pub fn reconcile_battle_pet_purchase_mark_like_cpp(
    row: Option<&BattlePetPurchaseCommandLikeCpp>,
    pending_source: BattlePetPurchaseStatusLikeCpp,
    target: BattlePetPurchaseStatusLikeCpp,
) -> Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp> {
    let Some(row) = row else {
        return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
            "battle-pet purchase command disappeared during a status transition".to_string(),
        ));
    };
    Ok(match row.status {
        status if status == target => BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
        BattlePetPurchaseStatusLikeCpp::Completed => {
            BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompleted
        }
        BattlePetPurchaseStatusLikeCpp::Compensated
        | BattlePetPurchaseStatusLikeCpp::TerminalFailure => {
            BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated
        }
        status if status == pending_source => {
            return Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                "battle-pet purchase status transition did not commit".to_string(),
            ));
        }
        status => {
            return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                "battle-pet purchase status transition from {status:?} is not owned here"
            )));
        }
    })
}

/// SQLx-free Character durability seam for the battle-pet purchase saga.
pub trait BattlePetPurchasePersistencePortLikeCpp: Send + Sync {
    /// T1: guarded absolute charge plus pending command in one transaction.
    fn charge_and_insert_command<'a>(
        &'a self,
        command: BattlePetPurchaseCommandLikeCpp,
        cancellation_fence: Box<dyn BattlePetPurchaseCommitFenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// Oldest unconverged commands for bounded login recovery.
    fn load_pending_commands<'a>(
        &'a self,
        character_guid: u64,
        limit: u32,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<Vec<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// Record that the success packets were queued after durable pet creation.
    fn mark_published<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T3: pending/compensation-pending to completed.
    fn mark_completed<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T4: record a terminal application decision before refunding.
    fn mark_compensation_pending<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T5: capped refund plus compensated status in one transaction.
    fn compensate<'a>(
        &'a self,
        request_key: [u8; 16],
        max_money: u64,
        cancellation_fence: Box<dyn BattlePetPurchaseCommitFenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T6: best-effort terminal marker when the character row is gone.
    fn mark_terminal_failure<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> PersistenceFutureLikeCpp<'a, Result<(), BattlePetPurchaseStoreErrorLikeCpp>>;
}
