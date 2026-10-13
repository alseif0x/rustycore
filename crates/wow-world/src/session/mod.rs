// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession` — per-player session that receives packets from the [`WorldSocket`](wow_network::WorldSocket) and dispatches them to handlers.
mod admission;
mod appearance;
mod connection;
mod deferred_visibility;
pub use crate::player_directory as directory;
mod dispatch;
mod driver;
mod dungeon_finding_handler_contexts;
mod lifecycle;
pub use lifecycle::PlayerSaveOutcomeLikeCpp;
mod arena_team_handler_contexts;
mod battlenet_handler_contexts;
mod calendar_handler_contexts;
mod character_query_handler_contexts;
mod chat_handler_contexts;
mod client_state_handler_contexts;
mod collections_handler_contexts;
mod combat;
mod combat_handler_contexts;
mod data_service_handler_contexts;
mod gameobject_handler_contexts;
mod group_application_handler_contexts;
mod group_handler_contexts;
mod guild_handler_contexts;
mod player_handler_contexts;
mod spell_handler_contexts;
mod trade_handler_contexts;
mod travel_handler_contexts;
mod vehicle_handler_contexts;
pub(crate) use combat::CR_ARMOR_PENETRATION_LIKE_CPP;
mod account_data_handler_contexts;
mod effect_learning;
mod instance_handler_contexts;
mod instances;
mod inventory_handler_contexts;
mod reputation_handler_contexts;
mod social_contacts_handler_contexts;
mod social_handler_contexts;
mod support_handler_contexts;
// #1263 F6-8D3a-1: the aggro commit of the admitted canonical executor;
// world-server calls the same body.
pub use wow_world_creature::{
    apply_creature_attack_start_commands_on_manager_like_cpp,
    apply_creature_attack_stop_commands_on_manager_like_cpp,
    committed_creature_combat_commands_like_cpp, retain_committed_creature_combat_events_like_cpp,
};
// #1263 F6-8D2/D3b-1: the admitted creature executor; the world-server
// canonical map loop calls it.
pub use wow_world_creature::{
    AdmittedCreatureCombatPhaseInputsLikeCpp, AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    AdmittedCreatureExecutionLikeCpp, AdmittedCreatureExecutionMapLikeCpp,
    AdmittedCreatureExecutionObjectLikeCpp, CreatureExecutionAdmissionLikeCpp,
    CreatureExecutionLeaseLikeCpp, CreatureExecutionOwnerLikeCpp,
    CreatureExecutionTransitionLikeCpp, IsolatedCreatureExecutionOutcomeLikeCpp,
    SharedCreatureExecutionLeaseLikeCpp, capture_admitted_creature_execution_like_cpp,
    capture_admitted_creature_execution_on_manager_like_cpp,
    run_admitted_creature_combat_phases_isolated_like_cpp,
    run_admitted_creature_combat_phases_on_locked_manager_like_cpp,
    run_admitted_creature_execution_isolated_like_cpp,
};
// #1363 slice 3: the runtime decisions the unit tests still call; the
// single-phase entries moved with their drivers to `wow-world-creature` (3b).
#[cfg(test)]
use wow_world_creature::{
    creature_ai_can_attack_decision_like_cpp, creature_melee_spell_miss_threshold_3_3_5_like_cpp,
};
mod canonical_access;
mod catalogs;
mod chat;
mod collections;
mod lifecycle_ops;
mod loot;
pub mod mailbox;
mod money;
mod movement;
mod object_updates;
pub(crate) use object_updates::dynamic_object_create_data_from_canonical_like_cpp;
pub(crate) use wow_world_quest::represented_gameobject_dynamic_flags_update_like_cpp;
pub(crate) use wow_world_visibility::represented_dynamic_object_values_update_delivery_fingerprint_like_cpp;
mod persistence;
mod pets;
mod player_cast;
mod player_items;
mod progression;
mod publication;
mod quest;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest::quest_reward_fixture_refs_like_cpp;
pub mod registry;
mod social;
mod spell_effects;
mod spell_state;
pub(crate) use persistence::player_homebind_update_request_like_cpp;
pub(crate) use spell_state::player_aura_info_like_cpp;
mod taxi;
pub(crate) use time_synchronization::game_time_ms_like_cpp;
mod test_support;
mod trainer_acquisition;
mod trait_configs;
mod visibility;
mod world_entities;
pub(crate) use world_entities::creature_message_to_set_target_allows_like_cpp;
pub(crate) use world_entities::insert_canonical_creature_map_object_on_map_like_cpp;
mod world_state;

mod action_bar_adapter;
#[cfg(test)]
mod auction_contracts;
#[cfg(test)]
pub(crate) use auction_contracts::RepresentedAuctionPlaceBidLikeCpp;
#[cfg(test)]
pub(crate) use auction_contracts::RepresentedAuctionRemoveItemLikeCpp;
#[cfg(test)]
pub(crate) use auction_contracts::RepresentedAuctionReplicateRequestLikeCpp;
#[cfg(test)]
pub(crate) use auction_contracts::RepresentedAuctionSellItemLikeCpp;
mod aura_effect_values;
use aura_effect_values::CanonicalThreatAuraSnapshotLikeCpp;
use aura_effect_values::represented_aura_effect_amounts_like_cpp;
use aura_effect_values::unit_owned_apply_aura_effect_mask_like_cpp;
mod battle_pet_adapter;
#[cfg(test)]
pub(crate) use battle_pet_adapter::{
    NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP, RepresentedBattlePetCageOutcomeLikeCpp,
    next_represented_battle_pet_guid_like_cpp,
};
#[cfg(test)]
pub(crate) use wow_world_core::session::battle_pet_adapter::RepresentedBattlePetCageItemLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::battle_pet_adapter::RepresentedBattlePetCalculatedStatsLikeCpp;
pub(crate) use wow_world_core::session::battle_pet_adapter::RepresentedBattlePetDataLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::battle_pet_adapter::RepresentedBattlePetLevelCriteriaLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::battle_pet_adapter::RepresentedBattlePetQueryCompanionLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::battle_pet_adapter::RepresentedBattlePetSaveInfoLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_inventory::{
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedBattlePetGrantExperienceOutcomeLikeCpp,
    RepresentedBattlePetGrantLevelOutcomeLikeCpp, RepresentedBattlePetQualityOutcomeLikeCpp,
    RepresentedBattlePetXpSourceLikeCpp, apply_battle_pet_calculated_stats_like_cpp,
};
mod battleground_adapter;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlefieldListLikeCpp;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlegroundQueueTypeIdLikeCpp;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlemasterHelloLikeCpp;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlemasterJoinArenaLikeCpp;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlemasterJoinLikeCpp;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlemasterJoinSkirmishLikeCpp;
mod catalog_capabilities;
pub use catalog_capabilities::AreaTriggerCatalogsLikeCpp;
pub use catalog_capabilities::GroupInvitePolicyLikeCpp;
pub use catalog_capabilities::{CreatureSpawnCatalogsLikeCpp, ProgressionCatalogsLikeCpp};
pub use catalog_capabilities::{ItemValuationCatalogsLikeCpp, ObjectMgrCatalogsLikeCpp};
pub use catalog_capabilities::{PlayerBootstrapCatalogsLikeCpp, PlayerRestRatePolicyLikeCpp};
pub use catalog_capabilities::{SessionHandlerCatalogsLikeCpp, SessionIdGeneratorsLikeCpp};
pub use wow_world_core::session::ChatPolicyCatalogsLikeCpp;
pub use wow_world_core::session::SupportFeaturePolicyLikeCpp;
use wow_world_core::session::character_availability::default_available_classes;
mod character_customization;
pub(crate) use character_customization::RepresentedConfirmRespecWipeLikeCpp;
pub(crate) use character_customization::RepresentedTalentRespecVisualSpellCastLikeCpp;
#[cfg(test)]
pub(crate) use character_customization::{
    RepresentedAlterAppearanceLikeCpp, RepresentedAtLoginFlagRemovalLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp, RepresentedTalentResetScriptHookLikeCpp,
    RepresentedTalentRespecCriteriaEventLikeCpp,
};
mod cinematic_adapter;
mod collection_adapter;
pub(crate) use collection_adapter::AccountHeirloomSaveRowLikeCpp;
pub(crate) use collection_adapter::AccountItemAppearanceSavePlanLikeCpp;
pub(crate) use collection_adapter::AccountTransmogIllusionSavePlanLikeCpp;
use collection_adapter::DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use collection_adapter::RepresentedTransmogCriteriaEvent;
pub(crate) use collection_adapter::{AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp};
mod connection_identity;
pub(crate) use connection_identity::PER_CHARACTER_CACHE_MASK_LIKE_CPP;
#[cfg(test)]
use connection_identity::PacketSpoofPendingBanLikeCpp;
use connection_identity::PacketSpoofPendingBanTargetLikeCpp;
pub use connection_identity::SessionState;
pub(crate) use connection_identity::{AccountDataLikeCpp, GLOBAL_CACHE_MASK_LIKE_CPP};
#[cfg(test)]
pub(crate) use connection_identity::{ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp};
use connection_identity::{trinity_sprintf_like_cpp, unix_now};
mod construction;
mod creature_aggro_contracts;
pub use creature_aggro_contracts::CreatureAggroCandidateLikeCpp;
pub use creature_aggro_contracts::CreatureAggroConfigLikeCpp;
pub use creature_aggro_contracts::CreatureAggroTickOutcomeLikeCpp;
pub use creature_aggro_contracts::CreatureLifecycleTickOutcomeLikeCpp;
pub use creature_aggro_contracts::CreatureMeleeTickOutcomeLikeCpp;
pub use creature_aggro_contracts::CreatureMovementTickOutcomeLikeCpp;
pub use creature_aggro_contracts::CreatureSpellTickOutcomeLikeCpp;
mod creature_canonical_adapter;
pub(crate) use creature_canonical_adapter::relocate_canonical_creature_map_object_on_map_like_cpp;
pub(crate) use creature_canonical_adapter::remove_canonical_creature_map_object_on_map_like_cpp;
mod creature_kill_contracts;
#[cfg(test)]
pub(crate) use creature_kill_contracts::RepresentedCreatureKillEventLikeCpp;
// #1363 slice 3: only the unit tests read these names since the creature
// runtime moved to `wow-world-creature`.
#[cfg(test)]
mod creature_movement_adapter;
mod creature_spawn_contracts;
pub use creature_spawn_contracts::PendingCreatureSpawn;
mod creature_spell_metadata;
use creature_spell_metadata::CreatureAiSpellTargetLikeCpp;
use creature_spell_metadata::creature_ai_spell_difficulty_chain_like_cpp;
#[cfg(test)]
use creature_spell_metadata::creature_ai_spell_x_spell_visual_id_like_cpp;
#[cfg(test)]
mod creature_spell_planning;
mod creature_spell_publication;
use creature_spell_publication::CreatureSpellHitProfileLikeCpp;
mod currency_adapter;
pub(crate) use currency_adapter::PlayerCurrencyDelta;
pub(crate) use currency_adapter::RepresentedQuestObjectiveProgressEventLikeCpp;
use currency_adapter::currency_max_quantity_cpp;
pub(crate) use currency_adapter::{CurrencyGainSourceLikeCpp, player_team_for_race_cpp};
mod duel_contracts;
#[cfg(test)]
pub(crate) use duel_contracts::RepresentedCanDuelSpellCastLikeCpp;
#[cfg(test)]
pub(crate) use duel_contracts::RepresentedDuelCancelOutcomeLikeCpp;
#[cfg(test)]
pub(crate) use duel_contracts::RepresentedDuelCancelledLikeCpp;
#[cfg(test)]
pub(crate) use duel_contracts::{RepresentedDuelAcceptedLikeCpp, RepresentedDuelRequestedLikeCpp};
mod faction_reactions;
pub(crate) use faction_reactions::RepresentedFactionReactionInputLikeCpp;
pub(crate) use faction_reactions::RepresentedGetReactionInputLikeCpp;
pub(crate) use faction_reactions::ReputationGainSourceLikeCpp;
mod gameobject_interaction;
pub(crate) use gameobject_interaction::RepresentedCapturePointStateLikeCpp;
pub(crate) use gameobject_interaction::RepresentedGameObjectUseEffect;
pub(crate) use gameobject_interaction::RepresentedGameObjectUseState;
pub(crate) use gameobject_interaction::RepresentedNewFlagStateRequest;
#[cfg(test)]
pub(crate) use gameobject_interaction::{
    BattlegroundFlagDropClickTarget, RepresentedBattlegroundObjectUseRejection,
    RepresentedGameObjectCriteriaEvent,
};
mod guild_inventory_contracts;
pub(crate) use guild_inventory_contracts::RepresentedBankItemMoveLikeCpp;
#[cfg(test)]
pub(crate) use guild_inventory_contracts::{
    RepresentedGuildBankInventoryMoveLikeCpp, RepresentedGuildBankListRequestLikeCpp,
    RepresentedGuildBankMoneyMoveLikeCpp, RepresentedGuildBankTabActionKindLikeCpp,
    RepresentedGuildBankTabActionLikeCpp, RepresentedGuildRepairBankStateLikeCpp,
    RepresentedGuildRepairBankWithdrawLikeCpp,
};
mod instance_bind_contracts;
#[cfg(test)]
pub(crate) use instance_bind_contracts::RepresentedGameObjectSpellCaster;
pub(crate) use instance_bind_contracts::RepresentedHomebindLikeCpp;
pub(crate) use instance_bind_contracts::RepresentedPendingBind;
mod inventory_request_contracts;
pub(crate) use inventory_request_contracts::DirectInventoryStorageOverlayLikeCpp;
pub(crate) use inventory_request_contracts::MAX_EQUIPMENT_SET_INDEX_LIKE_CPP;
pub(crate) use inventory_request_contracts::RepresentedAutoUnequipOffhandReasonLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use inventory_request_contracts::VendorBuyItemTestOverrideLikeCpp;
mod item_modifiers;
use item_modifiers::ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP;
pub(crate) use item_modifiers::InitialLoadedItemModsOutcomeLikeCpp;
pub(crate) use item_modifiers::LoadedEquippedItemEnchantmentsOutcomeLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedCombatStatRecalculationLikeCpp;
pub(crate) use item_modifiers::RepresentedItemBonusActionLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedItemBonusStateLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedItemModsReapplyEventLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedItemSetAuraRefreshEventLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedItemSetEffectLikeCpp;
pub(crate) use item_modifiers::RepresentedItemSetSpellEventLikeCpp;
use item_modifiers::is_represented_bag_slot;
pub(crate) use item_modifiers::item_storage_fields_values_update_like_cpp;
use item_modifiers::player_class_by_armor_subclass_like_cpp;
use item_modifiers::player_class_mask_for_transmog_like_cpp;
use item_modifiers::represented_player_stat_changes_like_cpp;
#[cfg(test)]
pub(crate) use item_modifiers::void_withdrawal_post_store_item_values_update_like_cpp;
pub(crate) use player_items::item_push_result_from_send_new_item_plan;
mod loot_delivery_contracts;
pub(crate) use loot_delivery_contracts::DurableItemLootCompletionLikeCpp;
pub(crate) use loot_delivery_contracts::DurableItemLootPersistenceGuardLikeCpp;
pub(crate) use loot_delivery_contracts::DurableLootItemFanoutLikeCpp;
pub(crate) use loot_delivery_contracts::LootMoneyDeliveryAddressLikeCpp;
pub(crate) use loot_delivery_contracts::LootMoneyViewerFanoutLikeCpp;
pub(crate) use loot_delivery_contracts::RepresentedLootRollVote;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_loot::RepresentedLootRollCriteriaEvent;
pub(crate) use wow_world_loot::RepresentedLootRollState;
mod map_admission;
pub(crate) use map_admission::CreateMapSideEffectApplySummaryLikeCpp;
pub use map_admission::{MMapRuntimeConfigLikeCpp, WaypointPathResolverLikeCpp};
pub use map_admission::{PlayerGridLoadOutcomeLikeCpp, PlayerGridLoadResolverLikeCpp};
use map_admission::{create_map_decision_difficulty_id_like_cpp, create_map_decision_key_like_cpp};
use wow_world_instances::create_map_instance_lock_token_like_cpp;
mod money_persistence_contracts;
use money_persistence_contracts::AbsolutePlayerMoneyCommitReconciliationLikeCpp;
pub(crate) use money_persistence_contracts::CommittedRepresentedTalentResetLikeCpp;
pub(crate) use money_persistence_contracts::ExclusivePlayerMoneyPersistenceLikeCpp;
pub(crate) use money_persistence_contracts::LootMoneyPersistenceErrorLikeCpp;
pub(crate) use money_persistence_contracts::PlayerMoneyCommitCancellationFenceLikeCpp;
use money_persistence_contracts::RepresentedTalentResetStatePlanLikeCpp;
use money_persistence_contracts::reconcile_absolute_player_money_commit_like_cpp;
mod movement_protocol;
pub(crate) use movement_protocol::MoveTeleportAckActionLikeCpp;
#[cfg(test)]
pub(crate) use movement_protocol::{
    MoveSplineDoneTaxiActionLikeCpp, MoveSplineDoneTaxiEventLikeCpp, MoveTeleportAckEventLikeCpp,
    MovementAckEventLikeCpp, MovementFallDamageEvent, MovementSpeedAckActionLikeCpp,
    MovementSpeedAckEventLikeCpp, MovementUnderMapDamageEvent, RepresentedAreaZoneCriteriaLikeCpp,
    RepresentedTaxiFlightNodeLikeCpp, TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP, UnitMoveTypeLikeCpp,
};
mod npc_interaction;
mod persistence_capabilities;
pub use persistence_capabilities::CatalogPersistenceCapabilitiesLikeCpp;
#[cfg(test)]
pub(crate) use persistence_capabilities::CharacterPowerSnapshotLikeCpp;
pub use persistence_capabilities::PlayerPersistenceCapabilitiesLikeCpp;
#[cfg(test)]
pub(crate) use persistence_capabilities::PlayerSaveToDbSnapshotLikeCpp;
pub use persistence_capabilities::SessionAdmissionPersistenceLikeCpp;
pub use persistence_capabilities::SessionPersistencePortsLikeCpp;
pub use persistence_capabilities::WorldPersistenceCapabilitiesLikeCpp;
use persistence_capabilities::character_power_snapshot_values_like_cpp;
#[cfg(test)]
use persistence_capabilities::empty_character_power_snapshot_like_cpp;
#[cfg(test)]
use persistence_capabilities::loaded_character_power_snapshot_like_cpp;
mod pet_dismissal;
#[cfg(test)]
pub(crate) use wow_world_core::session::CharacterPetStableRowLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_lifecycle::adjusted_represented_pet_aura_remain_time_like_cpp;
#[cfg(test)]
pub(crate) use wow_world_lifecycle::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
};
#[cfg(test)]
pub(crate) use wow_world_spell::{CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp};
mod player_binding;
use player_binding::PlayerIdentityBootstrapLikeCpp;
pub(crate) use player_binding::SessionPlayerController;
mod player_bootstrap;
mod player_condition_values;
#[cfg(test)]
pub(crate) use player_condition_values::RepresentedItemLevelCapsLikeCpp;
pub(crate) use player_condition_values::RepresentedPlayerConditionContextLikeCpp;
mod player_load_values;
use player_load_values::{gender_from_u8, player_team_id_for_race_cpp};
// #1363 slice 3: the player melee phase moved to `wow-world-creature`;
// world-server and the unit tests keep these Session paths.
pub use wow_world_creature::PlayerMeleeAttackerSnapshotLikeCpp;
pub use wow_world_creature::PlayerMeleePhaseStateLikeCpp;
pub use wow_world_creature::PlayerMeleeTickOutcomeLikeCpp;
#[cfg(test)]
pub(in crate::session) use wow_world_creature::RepresentedMeleeDamageBonusLikeCpp;
#[cfg(test)]
use wow_world_creature::is_unit_facing_target_for_melee_like_cpp;
#[cfg(test)]
pub(in crate::session) use wow_world_creature::legacy_attack_power_multiplier_like_cpp;
#[cfg(test)]
pub(in crate::session) use wow_world_creature::take_canonical_player_attack_swings_like_cpp;
mod player_presentation;
use player_presentation::RepresentedMountSpellCheckOutcomeLikeCpp;
mod player_registry_binding;
mod player_spell_records;
pub(crate) use player_spell_records::RepresentedPlayerSkillLikeCpp;
pub(crate) use player_spell_records::RepresentedPlayerSkillStateLikeCpp;
#[cfg(test)]
pub(crate) use player_spell_records::is_non_durable_skill_tombstone_like_cpp;
#[cfg(test)]
use wow_world_spell::canonical_player_spell_runtime_like_cpp;
use wow_world_spell::represented_player_spell_runtime_like_cpp;
pub(crate) use wow_world_spell::{
    RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellRuntimeLikeCpp,
    RepresentedPlayerSpellStateLikeCpp,
};
mod player_vitals_adapter;
mod progression_adapters;
mod quest_dialog;
pub(crate) use quest_dialog::LoadSeasonalQuestStatusOutcomeLikeCpp;
pub(crate) use quest_dialog::RepresentedAdventureMapStartQuestLikeCpp;
pub(crate) use quest_dialog::RepresentedPendingQuestSharingLikeCpp;
use quest_dialog::RepresentedPreparedQuestMenuItemLikeCpp;
pub(crate) use quest_dialog::RepresentedPushQuestToPartyOutcomeLikeCpp;
pub(crate) use quest_dialog::RepresentedPushQuestToPartyOutcomeReasonLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestCompleteStatusUpdateLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestConfirmAcceptLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestPushResultResponseLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest_dialog::RepresentedQuestRewardMailLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest_dialog::RepresentedQuestRewardReputationLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestRewardReputationSourceLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest_dialog::RepresentedQuestRewardSpellCastLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest_dialog::RepresentedQuestRewardSpellKindLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest_dialog::RepresentedQuestRewardTalentPointsLikeCpp;
pub(crate) use quest_dialog::ResetSeasonalQuestStatusOutcomeLikeCpp;
pub(crate) use quest_dialog::ResetSeasonalQuestStatusReasonLikeCpp;
pub(crate) use quest_dialog::SeasonalQuestStatusDbRowLikeCpp;
use quest_dialog::quest_giver_creature_id_from_source_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use quest_dialog::{
    RepresentedForceDeselectLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
use quest_dialog::{quest_has_represented_item_objective_like_cpp, quest_rewards_block_like_cpp};
mod quest_interaction;
mod raid_profile_values;
use raid_profile_values::player_cuf_profile_from_packet_like_cpp;
mod rest_progression;
mod runtime_policy_access;
mod social_requests;
pub(crate) use social_requests::RepresentedCalendarAddEventLikeCpp;
pub(crate) use social_requests::RepresentedCalendarCommunityInviteLikeCpp;
pub(crate) use social_requests::RepresentedCalendarRemoveEventLikeCpp;
pub(crate) use social_requests::RepresentedDeclinePetitionLikeCpp;
pub(crate) use social_requests::RepresentedQueryPetitionLikeCpp;
pub(crate) use social_requests::RepresentedSignPetitionLikeCpp;
#[cfg(test)]
pub(crate) use social_requests::RepresentedSilencePartyTalkerLikeCpp;
#[cfg(test)]
pub(crate) use social_requests::RepresentedWargameInviteAcceptanceLikeCpp;
use wow_world_core::session::power_type_from_u8_like_cpp as party_member_power_kind_from_u8_like_cpp;
mod spell_click_values;
use spell_click_values::NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP;
use spell_click_values::NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP;
use spell_click_values::NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP;
pub(crate) use spell_click_values::RepresentedCanSeeSpellClickOutcomeLikeCpp;
pub(crate) use spell_click_values::RepresentedCreatureAccessLikeCpp;
pub(crate) use spell_click_values::RepresentedGameObjectAccessLikeCpp;
pub(crate) use spell_click_values::RepresentedSpellClickCastLikeCpp;
use spell_click_values::RepresentedSpellClickClickeeCasterOutcomeLikeCpp;
use spell_click_values::RepresentedSpellClickCreatureSnapshotLikeCpp;
pub(crate) use spell_click_values::RepresentedSpellClickExecutionOutcomeLikeCpp;
pub(crate) use spell_click_values::RepresentedSpellClickPlanLikeCpp;
pub(crate) use spell_click_values::RepresentedSpellClickUnitRefLikeCpp;
#[cfg(test)]
pub(crate) use spell_click_values::RepresentedVehicleBaseMovementLikeCpp;
#[cfg(test)]
pub(crate) use spell_click_values::RepresentedVehicleDismissMovementLikeCpp;
#[cfg(test)]
pub(crate) use spell_click_values::RepresentedVehicleEnterRequestLikeCpp;
#[cfg(test)]
pub(crate) use spell_click_values::RepresentedVehicleSeatChangeRequestLikeCpp;
#[cfg(test)]
pub(crate) use spell_click_values::RepresentedVehicleSeatSpellClickRequestLikeCpp;
use spell_click_values::represented_spell_click_school_damage_amount_like_cpp;
use spell_click_values::spell_effect_has_non_or_db_nearby_entry_destination_like_cpp;
use spell_click_values::spell_effect_is_represented_summon_object_slot_like_cpp;
use wow_world_spell::spell_cast_adapter::represented_spell_cast_guid_for_map_like_cpp;
mod spell_pet_catalogs;
mod stand_state_adapter;
#[cfg(test)]
pub(crate) use stand_state_adapter::RepresentedLiveApplicationLikeCpp;
pub(crate) use stand_state_adapter::RepresentedLiveIntentAppliedLikeCpp;
pub(crate) use stand_state_adapter::RepresentedLiveIntentApplyOutcomeLikeCpp;
pub(crate) use stand_state_adapter::RepresentedLiveIntentLikeCpp;
pub(crate) use stand_state_adapter::RepresentedStandChannelCancellationBoundary;
pub(crate) use stand_state_adapter::RepresentedStandStateChangedLikeCpp;
mod state;
pub use state::WorldSession;
pub(crate) use state::{
    HubMut, InventoryCxRef, LifecycleCx, LifecycleCxRef, QuestStateCx, QuestStateCxRef,
    cx_inventory_ref, cx_lifecycle, cx_lifecycle_ref, cx_quest_state, cx_quest_state_ref, hub_mut,
    hub_ref, hub_support, split_battle_pet_handler_mut, split_battleground_mut,
    split_character_handler_mut, split_group_handler_states_mut, split_guild_bank_mut,
    split_instances_mut, split_instances_ref, split_interaction, split_interaction_ref,
    split_interaction_world_entities_mut, split_inventory_lifecycle_shared_hub_mut,
    split_inventory_mut, split_inventory_ref, split_lifecycle_mut, split_lifecycle_ref,
    split_loot_mut, split_loot_ref, split_player_handler_states_mut,
    split_quest_state_lifecycle_mut, split_quest_state_mut, split_quest_state_ref,
    split_social_mut, split_social_ref, split_spell_state_mut, split_spell_state_ref,
    split_trade_mut, split_visibility_mut, split_visibility_ref, split_world_entities_mut,
    split_world_entities_ref,
};
#[cfg(test)]
pub(crate) use state::{InventoryCx, cx_inventory};
mod summon_object_contracts;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectSlotSessionStatusLikeCpp;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectWildSessionOutcomeLikeCpp;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectWildSessionStatusLikeCpp;
pub(crate) use wow_world_spell::RepresentedSpellFocusObjectLikeCpp;
#[cfg(test)]
mod support_features;
mod taxi_contracts;
pub(crate) use taxi_contracts::RepresentedActivateTaxiLikeCpp;
mod time_synchronization;
mod trade_adapter;
mod void_storage_adapter;
mod xp_grants;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::Rng;
#[cfg(test)]
use rand::{SeedableRng, rngs::StdRng};
use tracing::{debug, info, warn};

use crate::battle_pet_account::BattlePetAccountAttachmentLikeCpp;
use crate::entity_update_bridge::{
    dynamic_object_values_update_to_update_object, item_values_update_to_update_object,
    player_values_update_to_update_object, unit_values_update_to_packet,
};
use crate::loot_persistence::DurableLootMoneyCompletionLikeCpp;
#[cfg(test)]
use crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp;
use crate::pathfinder::WorldMMapPathfinderWorkerLikeCpp;
use crate::session::directory::{
    PlayerRegistry, PlayerSessionRegistrationLikeCpp, PlayerVisibilityCreateSnapshot,
};
#[cfg(test)]
use crate::session::mailbox::GameEventQuestCompleteClientOutcomeLikeCpp;
use crate::session::mailbox::{
    CreatureAttackStartLikeCppCommand, GameEventQuestCompleteCommandLikeCpp,
    SendIfVisibleLikeCppCommand, SessionCommand,
};
use crate::session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, PlayerRegenerationRatesLikeCpp,
    ReputationRatesLikeCpp,
};
use wow_ai::max_level_for_expansion_like_cpp;
// #1363 slice 3: names the creature runtime used through this root; since it
// moved to `wow-world-creature` only the unit tests still read them here.
#[cfg(test)]
use crate::runtime_plan::RuntimeEvent;
#[cfg(test)]
use creature_aggro_contracts::{
    CreatureAiCanAttackDecisionLikeCpp, CreatureSpellTargetHitResultLikeCpp,
};
#[cfg(test)]
use creature_spell_metadata::{
    creature_ai_spell_has_unrepresented_target_restrictions_like_cpp,
    creature_ai_spell_is_combat_forbidden_like_cpp, creature_ai_spell_target_like_cpp,
};
#[cfg(test)]
use creature_spell_planning::{
    CreatureAiSpellRepresentationRejectionLikeCpp,
    creature_ai_spell_requires_projectile_payload_like_cpp,
    creature_ai_spell_single_unit_topology_like_cpp,
};
#[cfg(test)]
use creature_spell_publication::{
    creature_spell_cast_log_data_like_cpp, represented_creature_spell_hit_profile_like_cpp,
    resolve_creature_spell_hit_profile_like_cpp,
};
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use wow_ai::CreatureAiKindLikeCpp;
#[cfg(test)]
use wow_constants::ItemEnchantmentType;
#[cfg(test)]
use wow_constants::creature::CreatureTypeFlags;
use wow_constants::item::{CurrencyTypes, EnchantmentSlot, ItemFieldFlags2};
use wow_constants::shared::DifficultyFlags;
#[cfg(test)]
use wow_constants::unit::UnitFlags2;
use wow_constants::unit::{
    Gender, NPCFlags1, PowerType, Team, UnitFlags, UnitPvpFlags, UnitStandStateType,
    WeaponAttackType,
};
use wow_constants::{
    BuyResult, ClientOpcodes, InventoryResult, InventoryType, ItemClass, ItemContext, ItemFlags,
    ItemFlags2, ItemFlags3, ItemModifier, ItemQuality, ItemSpelltriggerType, ItemSubClassArmor,
    ItemSubClassWeapon, ServerOpcodes, SpellCastResult, TypeId, UnitState,
};
#[cfg(test)]
use wow_constants::{SellResult, item::ItemBonusType, movement::MovementFlag};
use wow_core::{
    EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position,
    VoidStorageItemIdGeneratorLikeCpp, guid::HighGuid,
};
#[cfg(test)]
use wow_data::CinematicSequencesStore;
#[cfg(test)]
use wow_data::SpellLinkedTypeLikeCpp;
#[cfg(test)]
use wow_data::SpellProcEntryLikeCpp;
#[cfg(test)]
use wow_data::SummonPropertiesEntry;
#[cfg(test)]
use wow_data::VehicleTemplateStoreLikeCpp;
use wow_data::character_progression::{ChrClassesStore, ChrRacesStore, PowerTypeStore};
use wow_data::trait_tree::TraitDefinitionStore;
use wow_data::{
    AccessRequirementStoreLikeCpp, AdventureMapPoiStore, AreaTableStore, AreaTriggerDb2Store,
    AreaTriggerScriptStoreLikeCpp, AreaTriggerStore, BankBagSlotPricesStore, BattlemasterListStore,
    ChrSpecializationStore, CombatRatingsGameTableLikeCpp, ConditionEntriesByTypeStore,
    CreatureDisplayInfoExtraStore, CreatureDisplayInfoStore, CreatureModelDataStore,
    CreatureTemplateLifecycleStoreLikeCpp, CreatureTemplateMountStoreLikeCpp, CurrencyTypesStore,
    DifficultyStore, DisableMgrLikeCpp, DungeonEncounterStore, DurabilityCostsStore,
    DurabilityQualityStore, EmotesStore, EmotesTextStore, FishingBaseSkillStoreLikeCpp,
    GameObjectDisplayInfoStore, GameObjectTemplateLifecycleStoreLikeCpp, GemPropertiesStore,
    GlyphPropertiesStore, GraveyardStore, HeirloomStore, HotfixBlobCache, ItemAppearanceStore,
    ItemBonusDb2Store, ItemChildEquipmentStore, ItemEffectStore, ItemExtendedCostStore,
    ItemLimitCategoryConditionStore, ItemLimitCategoryStore, ItemModifiedAppearanceStore,
    ItemRandomEnchantmentTemplateStore, ItemRandomPropertiesStore, ItemRandomSuffixStore,
    ItemSearchNameStore, ItemSetSpellStore, ItemSetStore, ItemSpecOverrideStore, ItemStatsStore,
    ItemStore, LfgDungeonStoreLikeCpp, LfgDungeonsStore, LockStore, MapDifficultyStore,
    MapDifficultyXConditionStore, MapStore, MountCapabilityStore, MountDefinitionStoreLikeCpp,
    MountStore, MountTypeXCapabilityStore, MountXDisplayStore, MovieStore,
    NpcSpellClickStoreLikeCpp, PhaseGroupStore, PhaseStore, PlayerConditionCountLikeCpp,
    PlayerConditionSkillLikeCpp, PlayerConditionStore, PlayerStatsStore, PvpItemStore,
    RandPropPointsStore, RegenGameTablesLikeCpp, SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
    ScriptIdLikeCpp, ScriptNameInternerLikeCpp, ShieldBlockRegularGameTableLikeCpp, SkillLineStore,
    SkillStore, SkillTiersStoreLikeCpp, SpellAcquisitionCatalogLikeCpp, SpellAreaLikeCpp,
    SpellAreaStoreLikeCpp, SpellAuraOptionsStore, SpellAuraRestrictionsStore, SpellCategoryStore,
    SpellChainStoreLikeCpp, SpellCustomAttributeStoreLikeCpp, SpellDurationStore,
    SpellEquippedItemsEntry, SpellEquippedItemsStore, SpellGroupStackRuleStoreLikeCpp,
    SpellGroupStoreLikeCpp, SpellItemEnchantmentConditionStore, SpellItemEnchantmentStore,
    SpellLearnSkillStoreLikeCpp, SpellLearnSpellStoreLikeCpp, SpellLevelsStore,
    SpellLinkedStoreLikeCpp, SpellMiscStore, SpellPetAuraStoreLikeCpp, SpellProcStoreLikeCpp,
    SpellRadiusStore, SpellRangeStore, SpellRequiredStoreLikeCpp, SpellShapeshiftFormStore,
    SpellStore, SpellTargetPositionStoreLikeCpp, SpellTargetRestrictionsStore,
    SpellThreatEntryLikeCpp, SpellThreatStoreLikeCpp, TactKeyStore, TalentStore,
    TavernAreaTriggerStoreLikeCpp, ToyStore, TrainerStoreLikeCpp, TransmogSetItemStore,
    TrinityStringStoreLikeCpp, VehicleAccessoryStoreLikeCpp, VehicleSeatStore, VehicleStore,
    WorldSafeLocStore, is_player_meeting_condition_like_cpp,
    progression_rewards::{
        ContentTuningStore, CurvePointStore, CurveStore, FactionEntry, FactionStore,
        FactionTemplateStore, FriendshipRepReactionStore, NumTalentsAtLevelStore,
        ParagonReputationStore, QuestFactionRewardStore, QuestInfoStore, QuestMoneyRewardStore,
        QuestPackageItemStore, QuestV2Store, ScalingStatDistributionStore, ScalingStatValuesStore,
    },
    reputation::{
        CreatureOnKillReputationStoreLikeCpp, RepSpilloverTemplateStoreLikeCpp,
        ReputationRewardRateStoreLikeCpp,
    },
    spell_click::{
        SPELL_CLICK_USER_FRIEND_LIKE_CPP, SPELL_CLICK_USER_PARTY_LIKE_CPP,
        SPELL_CLICK_USER_RAID_LIKE_CPP, UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
    },
    spell_duration_ms_like_cpp, spell_effect_radius_like_cpp,
};
#[cfg(test)]
use wow_data::{
    BattlePetBreedQualityStore, BattlePetBreedStateStore, BattlePetSpeciesStateStore,
    BattlePetSpeciesStore, BattlePetXpGameTableLikeCpp,
};
#[cfg(test)]
use wow_data::{
    ExplorationBaseXpStoreLikeCpp, ImportPriceStores, ItemClassStore, ItemCurrencyCostStore,
    ItemDisenchantLootStore, ItemPriceBaseStore,
};
#[cfg(test)]
use wow_data::{
    PetDefaultSpellStoreLikeCpp, PetDefaultSpellsEntryLikeCpp, PetFamilySpellStoreLikeCpp,
    PetLevelupSpellSetLikeCpp, PetLevelupSpellStoreLikeCpp, ServersideSpellInfoLikeCpp,
    ServersideSpellStoreLikeCpp, SpellEnchantProcEntryLikeCpp, SpellEnchantProcStoreLikeCpp,
    SpellTotemModelStoreLikeCpp,
};
#[cfg(test)]
use wow_data::{
    PlayerCreateInfoCastSpellStoreLikeCpp, PlayerCreateInfoCustomSpellStoreLikeCpp,
    SpellGroupStackRuleLikeCpp,
};
#[cfg(test)]
use wow_entities::PetStable;
#[cfg(test)]
use wow_entities::TitanGripPenaltyAction;
#[cfg(test)]
use wow_entities::Unit;
use wow_entities::player_rules::is_using_two_handed_weapon_in_one_hand_template as two_handed_in_one_hand_like_cpp;
// Test-only: the session admission fixtures compare against the player slot end.
#[cfg(test)]
use wow_entities::ApplyEnchantmentTemplateRef;
#[cfg(test)]
use wow_entities::ItemLimitCategoryTemplate;
#[cfg(test)]
pub(crate) use wow_entities::PLAYER_SLOT_END;
use wow_entities::{
    AccessorObjectKind, ApplyEnchantmentArgs, ApplyEnchantmentDurationAction,
    ApplyEnchantmentEffectAction, ApplyEnchantmentEffectRef, ApplyEnchantmentPlan,
    ApplyEnchantmentRandomSuffixRef, ApplyEnchantmentResult, BANK_SLOT_BAG_END,
    BANK_SLOT_BAG_START, BUYBACK_SLOT_START, CreatureAddonLifecycleRecordLikeCpp,
    EQUIPMENT_SLOT_FEET, EQUIPMENT_SLOT_HEAD, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND,
    EQUIPMENT_SLOT_RANGED, GameObject, INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, ITEM_DATA_BITS, ITEM_DATA_DURABILITY_BIT,
    Item, ItemDataUpdate, ItemPosCount, ItemStorageTemplate, ItemValuesUpdate, MAX_MONEY_AMOUNT,
    MAX_POWERS, MAX_POWERS_PER_CLASS, NULL_BAG, NULL_SLOT, PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP,
    PhaseShift, Player, PlayerEnchantTimeUpdate, PlayerInteractionDataLikeCpp,
    PlayerInventoryRuntime, PlayerItemTimeUpdate, PlayerPetLifecycleStateLikeCpp,
    PlayerQuestGameplayState, PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp,
    PlayerTeleportStateLikeCpp, QUESTS_COMPLETED_BITS_PER_BLOCK, QUESTS_COMPLETED_BITS_SIZE,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, SendNewItemDelivery, SendNewItemDisplayText,
    SendNewItemPlan, TYPEID_ITEM, UpdateMask, Vehicle, VisibleItemValues, WorldObject,
    is_equipment_packed_pos, make_item_pos, parse_explored_zones_db_string_like_cpp,
};
#[cfg(test)]
use wow_entities::{ActiveState, PetType, explored_zones_db_string_from_blocks_like_cpp};
#[cfg(test)]
use wow_entities::{MovementGeneratorKind, PetAuraLikeCpp, PetSaveMode, PetStableInfo, ReactState};
pub(crate) use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_handler::{PacketProcessing, SessionStatus};
use wow_progression::reputation_to_rank_like_cpp;
#[cfg(test)]
use wow_progression::{ReputationMgrMutLikeCpp, ReputationMgrRefLikeCpp};

// Only the test modules mounted into this file name these directly; the
// production surface reaches them through `wow-session` since #297.
#[cfg(test)]
use wow_network::{SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp};

use wow_loot::{LootStores, OwnedLootAuthority};
// Test-only: the session fixtures still read these through this prelude.
#[cfg(test)]
pub(crate) use wow_loot::LootStoreKind;
use wow_network::SocketTimeoutsLikeCpp;
use wow_network::session_mgr::SessionManager;
#[cfg(test)]
use wow_packet::packets::chat::{ChatMsg, ChatPkt, PrintNotification};
use wow_packet::packets::gossip::ClientGossipText;
#[cfg(test)]
use wow_packet::packets::item::{
    InventoryChangeFailure, ItemInstance, ItemMod, ItemModList, ItemPushResult,
    ItemPushResultDisplayType,
};
use wow_packet::packets::item::{ItemEnchantTimeUpdate, ItemTimeUpdate};
use wow_packet::packets::misc::{
    AccountHeirloom, AccountMount, AccountToy, DungeonDifficultySet,
    MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP, MountResult, SetProficiency, SetupCurrency,
    SetupCurrencyRecord, TRADE_SLOT_COUNT_LIKE_CPP,
};
#[cfg(test)]
use wow_packet::packets::misc::{BuyFailed, SellResponse};
use wow_packet::packets::quest::{
    QuestGiverOfferReward, QuestGiverQuestDetails, QuestGiverQuestList, QuestGiverRequestItems,
    QuestGiverRequestItemsCollect, QuestGiverRequestItemsCurrency, QuestListEntry,
    QuestObjectiveSimple, QuestRewardsBlock,
};
use wow_packet::packets::spell::SpellTargetData;
use wow_social::group::{
    GroupInfo, GroupInstanceResetMethodLikeCpp, GroupInstanceResetResultLikeCpp, GroupRegistry,
    PendingInvites, group_guid_by_db_store_id_like_cpp,
};

const QUEST_OBJECTIVE_ITEM_LIKE_CPP: u8 = 1;
const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP: u8 = 4;
#[cfg(test)]
const QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP: u8 = 6;
#[cfg(test)]
const QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP: u8 = 7;
const QUEST_OBJECTIVE_MONEY_LIKE_CPP: u8 = 8;
#[cfg(test)]
const QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP: u8 = 16;
#[cfg(test)]
const QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP: u8 = 17;
#[cfg(test)]
const QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP: u8 = 18;
#[cfg(test)]
const DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP: u32 = 100;
const QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP: u32 = 0x1;
const QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP: u32 = 0x0010_0000;
const QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP: u32 = 0x0000_1000;
pub(crate) use wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP;
pub(crate) use wow_world_entities::spell_click_values::MAX_GAMEOBJECT_SLOT_LIKE_CPP;
#[cfg(test)]
pub(crate) use wow_world_spell::spell_cast_adapter::SPELL_CAST_SOURCE_NORMAL_LIKE_CPP;
const PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP: u32 = 0x0000_0001;
pub(crate) use wow_world_core::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
pub(crate) use wow_world_core::session::{
    AFLAG_SCALABLE_LIKE_CPP, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP, PLAYER_FLAGS_GHOST_LIKE_CPP,
    SKILL_ENCHANTING_LIKE_CPP,
};
#[cfg(test)]
pub(crate) use wow_world_core::session::{
    BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP, PLAYER_FLAGS_AFK_LIKE_CPP, PLAYER_FLAGS_DND_LIKE_CPP,
};
const PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP: u32 = 0x0000_0800;
pub(crate) use wow_world_core::session::PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP;
pub(crate) const PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP: u32 = 0x2000_0000;
pub(crate) use wow_constants::rest::{
    REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP, REST_STATE_RESTED_LIKE_CPP,
};

/// Live seam for C++ `ScriptMgr::OnAreaTrigger`: a ported content script receives the mutable session and returns the same consumed/not-consumed boolean that controls C++ handler continuation.
pub type AreaTriggerScriptDispatcherLikeCpp =
    Arc<dyn Fn(&mut WorldSession, ScriptIdLikeCpp, u32, bool) -> bool + Send + Sync>;

#[cfg(test)]
use wow_world_core::session::state::config::GivePlayerXpScriptDispatcherLikeCpp;

const REST_FLAG_IN_TAVERN_LIKE_CPP: u32 = 0x1;
const REST_FLAG_IN_CITY_LIKE_CPP: u32 = 0x2;
const REST_FLAG_IN_FACTION_AREA_LIKE_CPP: u32 = 0x4;
const REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP: f32 = 0.031;
const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP: f32 = 0.125;
const REST_ONLINE_INGAME_BUBBLE_LIKE_CPP: f32 = 0.125;
use wow_world_instances::{
    DIFFICULTY_10_N_LIKE_CPP, DIFFICULTY_NORMAL_LIKE_CPP, DIFFICULTY_NORMAL_RAID_LIKE_CPP,
};
#[cfg(test)]
const MAP_INSTANCE_LIKE_CPP: u8 = 1;
#[cfg(test)]
const MAP_RAID_LIKE_CPP: u8 = 2;
use wow_world_core::session::PLAYER_FLAGS_IN_PVP_LIKE_CPP;
#[cfg(test)]
const PLAYER_FLAGS_PVP_TIMER_LIKE_CPP: u32 = 0x0004_0000;
use wow_world_core::session::SPELL_PVP_RULES_ENABLED_LIKE_CPP;
const LANG_RESET_SPELLS_LIKE_CPP: u32 = 215;
const LANG_RESET_SPELLS_TEXT_LIKE_CPP: &str = "Your spells have been reset.";
#[cfg(test)]
pub(crate) const TRADE_STATUS_PLAYER_BUSY_LIKE_CPP: u8 = 0;
#[cfg(test)]
use wow_world_core::session::PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP;
const AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP: u32 = 0x0000_0010;
use wow_world_spell::AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP;
const AREA_FLAG_CONTESTED_LIKE_CPP: u32 = 0x0004_0000;
const AREA_FLAG_COMBAT_ZONE_LIKE_CPP: u32 = 0x0100_0000;
const CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP: u8 = 0x13;
pub(crate) use wow_world_core::session::movement_protocol::{
    TELE_TO_SEAMLESS_LIKE_CPP, TeleportToOptionsLikeCpp,
};
pub(crate) const TELE_TO_NONE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x00;
#[allow(dead_code)]
pub(crate) const TELE_TO_GM_MODE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x01;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_TRANSPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x02;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x04;
pub(crate) const TELE_TO_SPELL_LIKE_CPP: TeleportToOptionsLikeCpp = 0x10;
#[allow(dead_code)]
pub(crate) const TELE_TO_TRANSPORT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x20;
#[allow(dead_code)]
pub(crate) const TELE_REVIVE_AT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x40;
#[cfg(test)]
const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP: u32 = 0x0000_0010;
#[cfg(test)]
const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP: u32 = 0x0001_0000;

const QUEST_MENU_ICON_TURN_IN_LIKE_CPP: u8 = 0;
const QUEST_MENU_ICON_AVAILABLE_LIKE_CPP: u8 = 2;
const QUEST_MENU_ICON_COMPLETE_LIKE_CPP: u8 = 4;

#[cfg(test)]
const PACKET_SPOOF_BAN_REASON_LIKE_CPP: &str = "DOS (Packet Flooding/Spoofing";
#[cfg(test)]
const PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP: &str = "Server: AutoDOS";

use wow_packet::WorldPacket;

const TRANSFER_ABORT_DIFFICULTY_LIKE_CPP: u32 = 8;
const TRANSFER_ABORT_ERROR_LIKE_CPP: u32 = 1;
const TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP: u32 = 2;
const TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP: u32 = 4;
const TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP: u32 = 6;
const TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP: u32 = 7;
const TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP: u32 = 9;
const TRANSFER_ABORT_NEED_GROUP_LIKE_CPP: u32 = 11;
const TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP: u32 = 16;
const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
const DEATH_KNIGHT_START_MAP_LIKE_CPP: u16 = 609;
const DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP: i32 = 50977;
// C++ `SpellCastSource::Normal` is encoded in the six-bit Cast GUID subtype.
// The capture contract validates this field rather than treating it as a runtime counter, so keep the canonical numeric value here.
pub(crate) const CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP: u32 = 0x08000;

/// C++ `CAST_FLAG_PENDING` (`Spells/Spell.h:78`); `SendSpellStart` and `SendSpellGo` set it for a triggered cast that is not `m_fromClient`.
pub(crate) const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;

pub(crate) use wow_entities::PlayerFavoriteAppearanceStateLikeCpp as FavoriteAppearanceStateLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::battle_pet_adapter::BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP;
#[cfg(test)]
pub(crate) const BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP: u32 = 82_800;
#[allow(dead_code)]
pub(crate) const BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP: u8 = 3;
#[cfg(test)]
pub(crate) use wow_world_inventory::BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP;
pub(crate) use wow_world_inventory::MAX_BATTLE_PET_LEVEL_LIKE_CPP;

pub(crate) const SKILL_RIDING_LIKE_CPP: u16 = 762;
pub use wow_world_core::session::{LIQUID_MAP_IN_WATER_LIKE_CPP, LIQUID_MAP_UNDER_WATER_LIKE_CPP};
const DAMAGE_FIRE_LIKE_CPP: u8 = 5;
const SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP: i32 = 0x0000_0001;
const CREATURE_MODEL_DATA_FLAG_CAN_MOUNT_WHILE_TRANSFORMED_AS_THIS_LIKE_CPP: u32 = 0x0000_0080;
const CHR_RACES_FLAG_CAN_MOUNT_LIKE_CPP: i32 = 0x0000_0004;
pub use wow_world_core::session::SharedCanonicalMapManager;
pub(crate) use wow_world_social::DUEL_COUNTDOWN_MS_LIKE_CPP;
#[cfg(test)]
pub(crate) use wow_world_social::SPELL_DUEL_BEG_LIKE_CPP;
pub(crate) use wow_world_social::{SPELL_DUEL_LIKE_CPP, SPELL_MOUNTED_DUEL_LIKE_CPP};

// Compatibility paths while #578 moves cast consumers out of the Session adapter.
pub(crate) use wow_entities::PendingSpellCastRequestLikeCpp as RepresentedPendingSpellCastRequestLikeCpp;
pub use wow_entities::{SpellCastBattlePetItemModifiersLikeCpp, SpellCastMetadata, SpellCastState};

/// Compatibility name while handler modules move to the Player-owned type.
pub use wow_entities::PlayerGossipOptionLikeCpp as GossipOptionInfo;

/// Compatibility name retained while handlers move onto Player-owned inventory commands and queries; the concrete record is owned by `wow_entities::Player`.
pub use wow_entities::PlayerInventoryItem as InventoryItem;

pub(crate) use wow_entities::{
    PlayerEquipmentSetLikeCpp as RepresentedEquipmentSetLikeCpp,
    PlayerEquipmentSetTypeLikeCpp as RepresentedEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp as RepresentedEquipmentSetUpdateStateLikeCpp,
    PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp,
};

use wow_world_core::session::WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP;
#[cfg(test)]
use wow_world_core::session::{
    FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP,
    FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP,
    FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP,
};

pub use wow_entities::AuraApplicationLikeCpp as AuraApplication;
pub use wow_entities::{RepresentedAuraEffectAmountLikeCpp, RepresentedAuraEffectLikeCpp};

pub(crate) use wow_world_core::session::AFLAG_NOCASTER_LIKE_CPP;

pub(crate) const SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP: u32 = 0x0000_0800;
pub(crate) use wow_world_core::session::PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
#[cfg(test)]
pub(crate) use wow_world_entities::SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;
#[cfg(test)]
pub(crate) use wow_world_spell::SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP;

#[cfg(test)]
#[path = "../../unit_tests/session_tests.rs"]
mod tests;
