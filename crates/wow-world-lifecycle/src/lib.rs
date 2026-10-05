//! Lifecycle-domain state shared with the World session adapter.

mod character_administration;
mod collection_contracts;
mod durable_item_loot;
mod finalization;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixture;
pub mod handlers;
mod money_persistence;
mod persistence_capabilities;
mod pet_load;
mod rename_callbacks;
mod save_contracts;
mod state;
mod value_contracts;
pub use handlers::{
    AccountDataHandlerCxLikeCpp, AccountDataHandlerHostLikeCpp,
    register_account_data_handlers_like_cpp,
};
pub mod login_transport;
pub mod loot_delivery_contracts;
pub mod loot_template_rules;

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
    PlayerMoneyCommitCancellationFenceLikeCpp, reconcile_absolute_player_money_commit_like_cpp,
};

#[doc(hidden)]
pub use finalization::SessionFinalization;

pub use character_administration::{
    PreparedRename, RenameFailure, RenameOutcome, RenamePreparation, RenameRequest, prepare_rename,
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
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, AccountDataLikeCpp, GLOBAL_CACHE_MASK_LIKE_CPP,
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
    DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP, LootMoneyPersistenceErrorLikeCpp, LootTemplateRow,
    LootTemplateTable, RepresentedTalentResetStatePlanLikeCpp, SessionLifecycleState,
    WrappedGiftLoad, WrappedGiftRow, group_persistence_command_like_cpp,
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
    RenameCandidateFixtureLikeCpp, RenamePersistencePortFixtureLikeCpp,
    candidate as character_administration_rename_candidate_fixture_like_cpp,
    fixture as character_administration_rename_fixture_like_cpp,
};
