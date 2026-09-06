//! Void-storage unlock, swap and transfer persistence contracts.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{
    LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PlayerMoneyTransactionOutcomeLikeCpp,
};

/// SQLx-free persisted shape of one `character_void_storage` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageItemWriteLikeCpp {
    pub item_id: u64,
    pub item_entry: u32,
    pub creator_guid: u64,
    pub fixed_scaling_level: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
}

/// Atomic durable half of the represented void-storage unlock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageUnlockWriteRequestLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub player_flags_after: u32,
}

impl VoidStorageUnlockWriteRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// Atomic durable half of one represented void-storage slot swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageSwapWriteRequestLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub old_slot: u8,
    pub new_slot: u8,
    pub source_item: VoidStorageItemWriteLikeCpp,
    pub destination_item: Option<VoidStorageItemWriteLikeCpp>,
}

impl VoidStorageSwapWriteRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// One inventory object destroyed as part of a void-storage deposit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageDestroyedItemWriteLikeCpp {
    pub item_db_guid: u64,
}

/// Durable half of one accepted void-storage deposit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageDepositWriteLikeCpp {
    pub destroyed_items: Vec<VoidStorageDestroyedItemWriteLikeCpp>,
    pub void_slot: u8,
    pub void_item: VoidStorageItemWriteLikeCpp,
}

/// Complete SQLx-free state needed to clone one withdrawn void item into
/// `item_instance` and link it into `character_inventory`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageNewInventoryItemWriteLikeCpp {
    pub item_db_guid: u64,
    pub item_entry: u32,
    pub creator_guid: u64,
    pub count: u32,
    pub enchantments: String,
    pub item_flags: u32,
    pub max_durability: u32,
    pub total_played_time: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
    pub container_db_guid: u64,
    pub inventory_slot: u8,
}

/// Complete SQLx-free state needed to persist a withdrawal merged into an
/// existing inventory item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageMergedInventoryItemWriteLikeCpp {
    pub item_db_guid: u64,
    pub item_entry: u32,
    pub owner_guid: u64,
    pub creator_guid: u64,
    pub gift_creator_guid: u64,
    pub count: u32,
    pub expiration: u32,
    pub charges: String,
    pub dynamic_flags: u32,
    pub enchantments: String,
    pub durability: u32,
    pub create_played_time: u32,
    pub text: String,
    pub battle_pet_species_id: u32,
    pub battle_pet_breed_data: u32,
    pub battle_pet_level: u32,
    pub battle_pet_display_id: u32,
    pub random_properties_id: i32,
    pub property_seed: i32,
    pub context: i32,
}

/// Inventory write caused by one withdrawal. A quest-bound item and an item
/// merged into an earlier withdrawal need no additional item statement here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoidStorageWithdrawalInventoryWriteLikeCpp {
    None,
    New(VoidStorageNewInventoryItemWriteLikeCpp),
    MergeExisting(VoidStorageMergedInventoryItemWriteLikeCpp),
}

/// Durable half of one accepted void-storage withdrawal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageWithdrawalWriteLikeCpp {
    pub old_void_slot: u8,
    pub inventory_write: VoidStorageWithdrawalInventoryWriteLikeCpp,
}

/// One nonzero quest-objective counter selected by the gameplay owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageQuestObjectiveWriteLikeCpp {
    pub storage_index: u8,
    pub count: i32,
}

/// Quest status changed by the inventory transfer. The gameplay owner selects
/// valid objective storage indexes; the adapter only renders their statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageQuestStatusWriteLikeCpp {
    pub quest_id: u32,
    pub status: u8,
    pub explored: bool,
    pub accept_time_secs: i64,
    pub end_time_secs: i64,
    pub objectives: Vec<VoidStorageQuestObjectiveWriteLikeCpp>,
}

/// Complete atomic durable half of one represented void-storage transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageTransferWriteRequestLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub deposits: Vec<VoidStorageDepositWriteLikeCpp>,
    pub withdrawals: Vec<VoidStorageWithdrawalWriteLikeCpp>,
    pub quest_statuses: Vec<VoidStorageQuestStatusWriteLikeCpp>,
}

impl VoidStorageTransferWriteRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// SQLx-free CharacterDB capability for the bounded void-storage workflows.
pub trait VoidStoragePersistencePortLikeCpp: Send + Sync {
    fn persist_void_storage_unlock_like_cpp<'a>(
        &'a self,
        request: VoidStorageUnlockWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp>;

    fn persist_void_storage_swap_like_cpp<'a>(
        &'a self,
        request: VoidStorageSwapWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp>;

    fn persist_void_storage_transfer_like_cpp<'a>(
        &'a self,
        request: VoidStorageTransferWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp>;
}
