//! Shared session owners for the world shell and Core.

pub use crate::player_directory as directory;

mod prelude;
pub use prelude::{
    AFLAG_SCALABLE_LIKE_CPP, PLAYER_FLAGS_AFK_LIKE_CPP, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
    PLAYER_FLAGS_DND_LIKE_CPP, PLAYER_FLAGS_GHOST_LIKE_CPP,
    PLAYER_FLAGS_RESTING_LIKE_CPP, SKILL_ENCHANTING_LIKE_CPP, SharedCanonicalMapManager,
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

mod canonical_access;
pub use canonical_access::{
    InventoryPlayerProjectionLikeCpp, OwnedCollectionsAccessLikeCpp,
    OwnedEquipmentSetsAccessLikeCpp, OwnedInventoryAccessLikeCpp,
    OwnedItemModifiersAccessLikeCpp, OwnedSpellAcquisitionAccessLikeCpp,
    OwnedPlayerCurrencyAccessLikeCpp, PlayerRegistryHydrationAccessLikeCpp, QuestRewardPlayerAccessLikeCpp,
};
mod catalogs;
mod connection;
mod construction;
mod instances;
mod money;
pub use money::{currency_max_quantity_cpp, PlayerCurrencyDelta};
mod movement;
mod runtime_policy_access;
mod world_state;
pub use movement::MovementTransportMembershipLikeCpp;
pub use movement::{PlayerRegistryControlBindingLikeCpp, PlayerRegistrySyncAccessLikeCpp};
mod condition_objects;
mod player_presentation;
mod player_stat_queries;
mod visibility;
pub use player_presentation::{LIQUID_MAP_IN_WATER_LIKE_CPP, LIQUID_MAP_UNDER_WATER_LIKE_CPP};
mod pet_dismissal;
mod pet_loading;
mod player_vitals_adapter;
pub use pet_loading::CharacterPetStableRowLikeCpp;
mod spell_state;
pub use spell_state::{
    AppliedAuraEffectLikeCpp, PlayerAuraEffectLikeCpp, player_aura_effects_all_like_cpp,
    player_aura_effects_by_spell_aura_type_like_cpp,
    player_aura_effects_full_by_spell_aura_type_like_cpp,
};
mod quest_dialog;
mod spell_pet_catalogs;
pub use quest_dialog::{
    pet_type_from_db_like_cpp, power_type_from_u8_like_cpp, react_state_from_db_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use quest_dialog::primary_power_type_for_player_class_like_cpp;
mod item_modifiers;
mod player_items;
mod quest_catalog_queries;
pub use item_modifiers::{
    RepresentedScalingStatContextLikeCpp, player_class_mask_for_talent_like_cpp,
    player_class_mask_for_transmog_like_cpp,
};
mod player_melee_application;
pub use player_melee_application::begin_combat_ref_on_map_like_cpp;
mod chat;
mod faction_reactions;
mod social;
pub use faction_reactions::{
    AttackReputationFactionSnapshotLikeCpp, RepresentedFactionReactionInputLikeCpp,
    RepresentedGetReactionInputLikeCpp, ReputationGainSourceLikeCpp,
};
mod combat;
pub use combat::{
    DAMAGE_FALL_LIKE_CPP, DAMAGE_FALL_TO_VOID_LIKE_CPP, PLAYER_FLAGS_IN_PVP_LIKE_CPP,
    SPELL_PVP_RULES_ENABLED_LIKE_CPP,
};
mod admission;
mod battleground_adapter;
mod lifecycle_ops;
mod npc_interaction;
mod player_registry_binding;
mod publication;
pub use publication::PacketPublicationAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use battleground_adapter::RepresentedBattlegroundQueueSlotLikeCpp;
pub use battleground_adapter::{
    RepresentedBattlefieldListLikeCpp, RepresentedBattlefieldPortLikeCpp,
    RepresentedBattlegroundQueueTypeIdLikeCpp, RepresentedBattlemasterHelloLikeCpp,
    RepresentedBattlemasterJoinArenaLikeCpp, RepresentedBattlemasterJoinLikeCpp,
    RepresentedBattlemasterJoinSkirmishLikeCpp, battleground_queue_type_id_from_packed_like_cpp,
};

mod social_requests;
pub use social_requests::RepresentedWargameInviteAcceptanceLikeCpp;
mod taxi;
mod taxi_contracts;
pub use taxi_contracts::{PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP, RepresentedActivateTaxiLikeCpp};

mod creature_aggro_contracts;
pub use creature_aggro_contracts::{
    DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP, LegacyCreatureAggroConfigLikeCpp,
    spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp,
};

mod creature_spell_metadata;
pub use creature_spell_metadata::creature_ai_spell_difficulty_chain_like_cpp;

mod stand_state_adapter;
#[cfg(any(test, feature = "test-fixtures"))]
pub use stand_state_adapter::RepresentedLiveApplicationLikeCpp;
pub use stand_state_adapter::{
    RepresentedLiveIntentAppliedLikeCpp, RepresentedLiveIntentApplyOutcomeLikeCpp,
    RepresentedLiveIntentLikeCpp, RepresentedStandChannelCancellationBoundary,
    RepresentedStandStateChangedLikeCpp,
};

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
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_spell_records::is_non_durable_skill_tombstone_like_cpp;
pub use player_spell_records::{RepresentedPlayerSkillLikeCpp, RepresentedPlayerSkillStateLikeCpp};
pub use player_spell_records::{
    canonical_player_skill_record_like_cpp, represented_player_skill_record_like_cpp,
    represented_skill_records_from_values_like_cpp, represented_skill_values_from_records_like_cpp,
};
mod trait_configs;

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
pub use progression::{
    FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP,
    FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP,
    FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP,
};

mod rest_progression;
#[cfg(any(test, feature = "test-fixtures"))]
pub use rest_progression::RestMgrTestFixtureLikeCpp;

mod progression_adapters;
pub use progression_adapters::WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP;
mod xp_grants;

pub use spell_click_values::RepresentedCreatureAccessLikeCpp;

mod persistence;
pub use persistence::{
    PlayerMoneyTransactionSessionAccessLikeCpp, plan_player_currency_save_for_store_like_cpp,
};
pub mod persistence_capabilities;

mod action_bar_adapter;
pub use action_bar_adapter::{
    action_button_action_like_cpp, action_button_type_like_cpp, make_action_button_like_cpp,
    set_active_player_update_bit_like_cpp,
};
mod collection_adapter;
mod collections;
pub use collection_adapter::{TOY_FLAG_FAVORITE_LIKE_CPP, TOY_FLAG_HAS_FANFARE_LIKE_CPP};

mod appearance;
mod raid_profile_values;
pub use raid_profile_values::{
    player_cuf_profile_from_packet_like_cpp, player_cuf_profile_to_packet_like_cpp,
};
mod cinematic_adapter;

mod creature_spawn_contracts;
mod world_entities;
pub use creature_spawn_contracts::{
    CreatureCreateDisplaySelectionLikeCpp, CreatureCreateModelScalarsLikeCpp,
};
mod creature_canonical_adapter;
mod loot;
pub use creature_canonical_adapter::{
    add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp,
    reconcile_creature_loot_authority_mirrors_like_cpp,
    relocate_canonical_creature_map_object_on_map_like_cpp,
    remove_canonical_creature_map_object_on_map_like_cpp,
    remove_canonical_respawn_time_on_map_like_cpp, sync_canonical_creature_entity_on_map_like_cpp,
};

pub mod player_binding;
pub use player_binding::PlayerIdentityBootstrapLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_binding::PlayerTransportLoginStateLikeCpp;
pub use player_binding::{
    PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP, PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP,
};

pub mod movement_protocol;
pub use movement_protocol::{
    creature_movement_spline_speed_opcode_like_cpp, player_movement_speed_opcodes_like_cpp,
};
pub mod pets;

pub mod time_synchronization;
pub use time_synchronization::{TimeSynchronizationStateLikeCpp, game_time_ms_like_cpp};

pub mod mailbox;
pub mod state;
pub use state::SessionWorldConfig;
pub use state::{HubMut, HubRef};
pub use state::{SessionCatalogs, SessionCore, SessionDriverPhaseLikeCpp};
pub use state::hub_support::default_display_id;

pub mod map_admission;
pub use map_admission::{MMapRuntimeConfigLikeCpp, WaypointPathResolverLikeCpp};

pub mod catalog_capabilities;
pub use catalog_capabilities::GroupInvitePolicyLikeCpp;
pub use catalog_capabilities::ObjectMgrCatalogsLikeCpp;
pub use catalog_capabilities::PlayerBootstrapCatalogsLikeCpp;
pub use catalog_capabilities::SupportFeaturePolicyLikeCpp;
pub use catalog_capabilities::{ItemValuationCatalogsLikeCpp, ProgressionCatalogsLikeCpp};

#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_support;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::test_fixtures::PlayerBootstrapCatalogTestFixtureLikeCpp;
