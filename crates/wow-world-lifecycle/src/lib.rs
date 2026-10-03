//! Lifecycle-domain state shared with the World session adapter.

mod finalization;
mod character_administration;
mod rename_callbacks;
mod durable_item_loot;
mod value_contracts;
mod pet_load;

pub use finalization::{
    FinalizationDisposition, FinalizationMode, FinalizationOutcome, FinalizationReport,
    FinalizationStep,
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
pub use pet_load::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
    PetLoadQueryHolderRowsLikeCpp,
};

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
