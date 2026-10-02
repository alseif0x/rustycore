//! Shared session owners for the world shell and Core.

pub use crate::player_directory as directory;

mod prelude;
pub use prelude::{
    AFLAG_SCALABLE_LIKE_CPP, PLAYER_FLAGS_AFK_LIKE_CPP, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
    PLAYER_FLAGS_DND_LIKE_CPP, PLAYER_FLAGS_GHOST_LIKE_CPP, SKILL_ENCHANTING_LIKE_CPP,
    SharedCanonicalMapManager,
};

pub mod battle_pet_adapter;
#[cfg(any(test, feature = "test-fixtures"))]
pub use battle_pet_adapter::RepresentedBattlePetCageItemLikeCpp;
pub use battle_pet_adapter::{
    BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP, BATTLE_PET_SLOT_COUNT_LIKE_CPP,
    DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP, RepresentedBattlePetCalculatedStatsLikeCpp,
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetLevelCriteriaLikeCpp,
    RepresentedBattlePetQueryCompanionLikeCpp, RepresentedBattlePetSaveInfoLikeCpp,
    RepresentedBattlePetSlotLikeCpp,
};

pub mod connection_identity;
pub use connection_identity::{
    PacketCounterLikeCpp, PacketSpoofPendingBanLikeCpp, PacketSpoofPendingBanTargetLikeCpp,
    SessionState,
};

mod connection;
mod canonical_access;
mod instances;
mod catalogs;
mod runtime_policy_access;
mod construction;
mod movement;
mod visibility;
mod spell_state;
mod player_items;
mod social;
mod chat;
mod combat;
pub use combat::{DAMAGE_FALL_LIKE_CPP, DAMAGE_FALL_TO_VOID_LIKE_CPP};
mod admission;
mod player_registry_binding;
mod lifecycle_ops;
mod publication;
mod battleground_adapter;
pub use battleground_adapter::{
    RepresentedBattlefieldListLikeCpp, RepresentedBattlefieldPortLikeCpp,
    RepresentedBattlemasterHelloLikeCpp, RepresentedBattlemasterJoinArenaLikeCpp,
    RepresentedBattlemasterJoinLikeCpp, RepresentedBattlemasterJoinSkirmishLikeCpp,
    RepresentedBattlegroundQueueTypeIdLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use battleground_adapter::RepresentedBattlegroundQueueSlotLikeCpp;

mod social_requests;
pub use social_requests::RepresentedWargameInviteAcceptanceLikeCpp;
mod taxi_contracts;
pub use taxi_contracts::RepresentedActivateTaxiLikeCpp;

mod creature_aggro_contracts;
pub use creature_aggro_contracts::{
    DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP, LegacyCreatureAggroConfigLikeCpp,
    spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp,
};

mod creature_spell_metadata;
pub use creature_spell_metadata::creature_ai_spell_difficulty_chain_like_cpp;

mod stand_state_adapter;
pub use stand_state_adapter::{
    RepresentedLiveIntentAppliedLikeCpp, RepresentedLiveIntentApplyOutcomeLikeCpp,
    RepresentedLiveIntentLikeCpp, RepresentedStandChannelCancellationBoundary,
    RepresentedStandStateChangedLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use stand_state_adapter::RepresentedLiveApplicationLikeCpp;

mod character_customization;
pub use character_customization::{
    RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp,
    RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use character_customization::{
    RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
};

mod player_spell_records;
pub use player_spell_records::{
    RepresentedPlayerSkillLikeCpp, RepresentedPlayerSkillStateLikeCpp,
};

mod spell_click_values;
pub use spell_click_values::RepresentedGameObjectAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use spell_click_values::{
    RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp,
    RepresentedVehicleEnterRequestLikeCpp, RepresentedVehicleSeatChangeRequestLikeCpp,
    RepresentedVehicleSeatSpellClickRequestLikeCpp,
};

mod progression;
pub use progression::MAX_SPECIALIZATIONS_LIKE_CPP;
#[cfg(any(test, feature = "test-fixtures"))]
pub use progression::PlayerSkillTestFixtureLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
mod rest_progression;
#[cfg(any(test, feature = "test-fixtures"))]
pub use rest_progression::RestMgrTestFixtureLikeCpp;

pub mod persistence_capabilities;

mod action_bar_adapter;
pub use action_bar_adapter::{
    action_button_action_like_cpp, action_button_type_like_cpp, make_action_button_like_cpp,
    set_active_player_update_bit_like_cpp,
};
mod collections;
mod collection_adapter;
pub use collection_adapter::{TOY_FLAG_FAVORITE_LIKE_CPP, TOY_FLAG_HAS_FANFARE_LIKE_CPP};

mod appearance;
mod raid_profile_values;
pub use raid_profile_values::{
    player_cuf_profile_from_packet_like_cpp, player_cuf_profile_to_packet_like_cpp,
};
mod cinematic_adapter;

mod world_entities;
mod loot;
mod creature_canonical_adapter;
pub use creature_canonical_adapter::{
    add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp,
    reconcile_creature_loot_authority_mirrors_like_cpp,
    relocate_canonical_creature_map_object_on_map_like_cpp,
    remove_canonical_creature_map_object_on_map_like_cpp,
    remove_canonical_respawn_time_on_map_like_cpp,
    sync_canonical_creature_entity_on_map_like_cpp,
};

pub mod player_binding;
pub use player_binding::{
    PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_binding::PlayerTransportLoginStateLikeCpp;
pub use player_binding::PlayerIdentityBootstrapLikeCpp;

pub mod movement_protocol;
pub mod pets;

pub mod time_synchronization;
pub use time_synchronization::{game_time_ms_like_cpp, TimeSynchronizationStateLikeCpp};

pub mod mailbox;
pub mod state;
pub use state::{SessionCatalogs, SessionCore, SessionDriverPhaseLikeCpp};
pub use state::{HubMut, HubRef};
pub use state::SessionWorldConfig;

pub mod map_admission;
pub use map_admission::{MMapRuntimeConfigLikeCpp, WaypointPathResolverLikeCpp};

pub mod catalog_capabilities;
pub use catalog_capabilities::ObjectMgrCatalogsLikeCpp;
pub use catalog_capabilities::SupportFeaturePolicyLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_support;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::test_fixtures::PlayerBootstrapCatalogTestFixtureLikeCpp;
