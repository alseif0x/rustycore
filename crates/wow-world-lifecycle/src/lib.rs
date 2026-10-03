//! Lifecycle-domain state shared with the World session adapter.

mod collection_contracts;
mod money_persistence;
mod finalization;
mod character_administration;
mod rename_callbacks;
mod durable_item_loot;
mod value_contracts;
mod save_contracts;
mod pet_load;
mod persistence_capabilities;
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixture;

pub use finalization::{
    FinalizationDisposition, FinalizationMode, FinalizationOutcome, FinalizationReport,
    FinalizationStep,
};

#[doc(hidden)]
pub use collection_contracts::{
    AccountHeirloomSaveRowLikeCpp, AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp,
};

#[doc(hidden)]
pub use money_persistence::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp, ExclusivePlayerMoneyPersistenceLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp,
    reconcile_absolute_player_money_commit_like_cpp,
};

#[doc(hidden)]
pub use finalization::SessionFinalization;

pub use character_administration::{
    prepare_rename, PreparedRename, RenameFailure, RenameOutcome, RenamePreparation, RenameRequest,
};

#[doc(hidden)]
pub use rename_callbacks::RenameCallbacks;

#[doc(hidden)]
pub use durable_item_loot::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableItemLootPersistenceTrackerLikeCpp, DurableLootItemFanoutLikeCpp,
};

#[doc(hidden)]
pub use value_contracts::{
    AccountDataLikeCpp, ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, GLOBAL_CACHE_MASK_LIKE_CPP,
    HomebindPersistenceJobLikeCpp, PER_CHARACTER_CACHE_MASK_LIKE_CPP,
    default_account_data_like_cpp,
};

#[doc(hidden)]
pub use save_contracts::PlayerSaveOutcomeLikeCpp;

#[doc(hidden)]
pub use pet_load::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
    PetLoadQueryHolderRowsLikeCpp,
};

#[doc(hidden)]
pub use persistence_capabilities::{
    CatalogPersistenceCapabilitiesLikeCpp, PlayerPersistenceCapabilitiesLikeCpp,
    SessionAdmissionPersistenceLikeCpp, SessionPersistencePortsLikeCpp,
    WorldPersistenceCapabilitiesLikeCpp,
};

#[doc(hidden)]
pub use state::{
    group_persistence_command_like_cpp, DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
    LootMoneyPersistenceErrorLikeCpp, LootTemplateRow, LootTemplateTable,
    RepresentedTalentResetStatePlanLikeCpp, SessionLifecycleState, WrappedGiftLoad, WrappedGiftRow,
};

#[cfg(any(test, feature = "test-fixtures"))]
#[doc(hidden)]
pub use fixture::LoadedPlayerFlagsTestFixtureLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
#[doc(hidden)]
pub use value_contracts::RepresentedAtLoginFlagRemovalLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
#[doc(hidden)]
pub use character_administration::test_fixture::{
    candidate as character_administration_rename_candidate_fixture_like_cpp,
    fixture as character_administration_rename_fixture_like_cpp, RenameCandidateFixtureLikeCpp,
    RenamePersistencePortFixtureLikeCpp,
};
