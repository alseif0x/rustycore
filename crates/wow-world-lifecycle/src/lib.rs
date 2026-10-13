//! Lifecycle-domain state shared with the World session adapter.

mod battlenet_handlers;
mod character_administration;
mod client_state;
mod collection_contracts;
mod data_service_handlers;
mod durable_item_loot;
mod finalization;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixture;
pub mod handlers;
mod money_persistence;
mod persistence_capabilities;
mod pet_load;
mod pet_session;
mod player_save;
mod rename_callbacks;
mod save_contracts;
mod state;
mod support;
mod value_contracts;
pub use battlenet_handlers::{
    BattlenetHandlerCxLikeCpp, BattlenetHandlerHostLikeCpp, register_battlenet_handlers_like_cpp,
};
pub use client_state::{
    ClientStateHandlerCxLikeCpp, ClientStateHandlerHostLikeCpp,
    register_client_state_handlers_like_cpp,
};
pub use data_service_handlers::{
    DataServiceHandlerCxLikeCpp, DataServiceHandlerHostLikeCpp,
    register_data_service_handlers_like_cpp,
};
pub use handlers::{
    AccountDataHandlerCxLikeCpp, AccountDataHandlerHostLikeCpp,
    register_account_data_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_save::{OwnerlessPlayerSaveCaptureLikeCpp, OwnerlessPlayerSaveReceiptLikeCpp};
pub use player_save::{
    PlayerSaveDiagnosticsLikeCpp, PlayerSaveHostLikeCpp, PlayerSaveParticipantsLikeCpp,
    PlayerSavePersistenceResultLikeCpp, apply_player_save_acknowledgement_like_cpp,
    capture_player_save_request_like_cpp, persist_player_save_request_like_cpp,
    save_canonical_player_like_cpp, save_current_player_to_db_with_generator_like_cpp,
};
pub mod login_transport;
pub mod loot_delivery_contracts;
pub mod loot_money_persistence;
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

pub use pet_load::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
    PetLoadQueryHolderRowsLikeCpp,
};
#[doc(hidden)]
pub use pet_session::{
    PetSessionHostLikeCpp, adjusted_represented_pet_aura_remain_time_like_cpp,
    apply_represented_login_pet_talent_reset_like_cpp,
    begin_represented_character_pet_authority_load_like_cpp,
    load_represented_login_pet_state_like_cpp, load_represented_pet_aura_effect_rows_like_cpp,
    load_represented_pet_aura_rows_like_cpp, load_represented_pet_aura_rows_with_timediff_like_cpp,
    load_represented_pet_declined_names_like_cpp, load_represented_pet_spell_charge_rows_like_cpp,
    load_represented_pet_spell_cooldown_rows_like_cpp, load_represented_pet_spell_rows_like_cpp,
    resummon_pet_temporary_unsummoned_like_cpp, set_represented_pet_mode_state_with_spell_like_cpp,
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
pub use support::{
    SupportHandlerCxLikeCpp, SupportHandlerHostLikeCpp, register_support_handlers_like_cpp,
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
