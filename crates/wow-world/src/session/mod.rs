// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession` — per-player session that receives packets from the
//! [`WorldSocket`](wow_network::WorldSocket) and dispatches them to handlers.

mod admission;
mod appearance;
mod connection;
mod deferred_visibility;
pub use crate::player_directory as directory;
mod dispatch;
mod driver;
mod lifecycle;
pub use lifecycle::PlayerSaveOutcomeLikeCpp;
mod combat;
pub(crate) use combat::{CR_ARMOR_PENETRATION_LIKE_CPP, CR_HIT_MELEE_LIKE_CPP};
mod effect_learning;
mod instances;
mod legacy_runtime;
use legacy_runtime::*;
// The legacy tick entry points are called from world-server as
// `wow_world::session::run_legacy_*`. `legacy_runtime` is private, so the
// original external path is preserved by re-exporting them here.
pub use legacy_runtime::{
    run_legacy_creature_aggro_tick_once_like_cpp,
    run_legacy_creature_aggro_tick_once_with_config_like_cpp,
    run_legacy_creature_lifecycle_tick_once_like_cpp, run_legacy_creature_melee_tick_once_like_cpp,
    run_legacy_creature_movement_tick_once_like_cpp, run_legacy_creature_spell_tick_once_like_cpp,
    run_legacy_player_melee_tick_once_like_cpp,
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
pub(crate) use movement::state::MovementTransportMembershipLikeCpp;
mod object_updates;
pub(crate) use object_updates::dynamic_object_create_data_from_canonical_like_cpp;
pub(crate) use object_updates::represented_dynamic_object_values_update_delivery_fingerprint_like_cpp;
pub(crate) use object_updates::represented_gameobject_dynamic_flags_update_like_cpp;
mod persistence;
mod pets;
#[cfg(test)]
pub(crate) use pets::test_fixtures::BattlePetTestFixtureLikeCpp;
mod player_cast;
mod player_items;
mod progression;
mod publication;
mod quest;
pub(crate) use quest::state::RepresentedQuestRecurrenceLikeCpp;
pub mod registry;
mod social;
mod spell_effects;
mod spell_state;
pub(crate) use persistence::player_homebind_update_request_like_cpp;
pub(crate) use spell_state::RepresentedShapeshiftMutationLikeCpp;
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
use action_bar_adapter::make_action_button_like_cpp;
use action_bar_adapter::{action_button_action_like_cpp, action_button_type_like_cpp};
use action_bar_adapter::{rounded_median_u32, set_active_player_update_bit_like_cpp};
mod auction_contracts;
pub(crate) use auction_contracts::RepresentedAuctionPlaceBidLikeCpp;
pub(crate) use auction_contracts::RepresentedAuctionRemoveItemLikeCpp;
pub(crate) use auction_contracts::RepresentedAuctionReplicateRequestLikeCpp;
pub(crate) use auction_contracts::RepresentedAuctionSellItemLikeCpp;
mod aura_effect_values;
use aura_effect_values::CanonicalThreatAuraSnapshotLikeCpp;
use aura_effect_values::represented_aura_effect_amounts_like_cpp;
use aura_effect_values::unit_owned_apply_aura_effect_mask_like_cpp;
mod battle_pet_adapter;
#[cfg(test)]
pub(crate) use battle_pet_adapter::NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP;
#[cfg(test)]
pub(crate) use battle_pet_adapter::RepresentedBattlePetCageItemLikeCpp;
#[cfg(test)]
pub(crate) use battle_pet_adapter::RepresentedBattlePetCageOutcomeLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetCalculatedStatsLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetDataLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetGrantExperienceOutcomeLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetGrantLevelOutcomeLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetLevelCriteriaLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetQualityOutcomeLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetQueryCompanionLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetSaveInfoLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetSlotLikeCpp;
pub(crate) use battle_pet_adapter::RepresentedBattlePetXpSourceLikeCpp;
pub(crate) use battle_pet_adapter::apply_battle_pet_calculated_stats_like_cpp;
#[cfg(test)]
pub(crate) use battle_pet_adapter::next_represented_battle_pet_guid_like_cpp;
mod battleground_adapter;
pub(crate) use battleground_adapter::RepresentedBattlefieldListLikeCpp;
pub(crate) use battleground_adapter::RepresentedBattlefieldPortLikeCpp;
#[cfg(test)]
pub(crate) use battleground_adapter::RepresentedBattlegroundQueueSlotLikeCpp;
pub(crate) use battleground_adapter::RepresentedBattlegroundQueueTypeIdLikeCpp;
pub(crate) use battleground_adapter::RepresentedBattlemasterHelloLikeCpp;
pub(crate) use battleground_adapter::RepresentedBattlemasterJoinArenaLikeCpp;
pub(crate) use battleground_adapter::RepresentedBattlemasterJoinLikeCpp;
pub(crate) use battleground_adapter::RepresentedBattlemasterJoinSkirmishLikeCpp;
pub(crate) use battleground_adapter::battleground_queue_type_id_from_packed_like_cpp;
use battleground_adapter::{arena_skirmish_type_like_cpp, arena_team_type_by_slot_like_cpp};
mod buyback_adapter;
mod catalog_capabilities;
pub use catalog_capabilities::{AreaTriggerCatalogsLikeCpp, SupportFeaturePolicyLikeCpp};
pub use catalog_capabilities::{ChatPolicyCatalogsLikeCpp, GroupInvitePolicyLikeCpp};
pub use catalog_capabilities::{CreatureSpawnCatalogsLikeCpp, ProgressionCatalogsLikeCpp};
pub use catalog_capabilities::{ItemValuationCatalogsLikeCpp, ObjectMgrCatalogsLikeCpp};
pub use catalog_capabilities::{PlayerBootstrapCatalogsLikeCpp, PlayerRestRatePolicyLikeCpp};
pub use catalog_capabilities::{SessionHandlerCatalogsLikeCpp, SessionIdGeneratorsLikeCpp};
mod character_availability;
use character_availability::default_available_classes;
mod character_customization;
pub(crate) use character_customization::RepresentedAlterAppearanceLikeCpp;
#[cfg(test)]
pub(crate) use character_customization::RepresentedAtLoginFlagRemovalLikeCpp;
pub(crate) use character_customization::RepresentedConfirmBarbersChoiceLikeCpp;
pub(crate) use character_customization::RepresentedConfirmRespecWipeLikeCpp;
#[cfg(test)]
pub(crate) use character_customization::RepresentedTalentResetScriptHookLikeCpp;
#[cfg(test)]
pub(crate) use character_customization::RepresentedTalentRespecCriteriaEventLikeCpp;
pub(crate) use character_customization::RepresentedTalentRespecVisualSpellCastLikeCpp;
mod cinematic_adapter;
mod collection_adapter;
pub(crate) use collection_adapter::AccountHeirloomSaveRowLikeCpp;
pub(crate) use collection_adapter::AccountItemAppearanceSavePlanLikeCpp;
pub(crate) use collection_adapter::AccountTransmogIllusionSavePlanLikeCpp;
#[cfg(test)]
pub(crate) use collection_adapter::RepresentedTransmogCriteriaEvent;
pub(crate) use collection_adapter::{AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp};
use collection_adapter::{DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP, heirloom_bonus_for_flags_like_cpp};
mod connection_identity;
pub(crate) use connection_identity::GLOBAL_CACHE_MASK_LIKE_CPP;
pub(crate) use connection_identity::PER_CHARACTER_CACHE_MASK_LIKE_CPP;
pub use connection_identity::SessionState;
pub(crate) use connection_identity::{ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, AccountDataLikeCpp};
use connection_identity::{ChatFloodThrottleDataLikeCpp, PacketSpoofPendingBanLikeCpp};
pub(crate) use connection_identity::{ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp};
use connection_identity::{PacketCounterLikeCpp, PacketSpoofPendingBanTargetLikeCpp};
use connection_identity::{default_account_data_like_cpp, trinity_sprintf_like_cpp, unix_now};
mod construction;
mod creature_aggro_contracts;
use creature_aggro_contracts::CreatureSpellTargetHitResultLikeCpp;
use creature_aggro_contracts::DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP;
pub use creature_aggro_contracts::LegacyCreatureAggroCandidateLikeCpp;
pub use creature_aggro_contracts::LegacyCreatureAggroConfigLikeCpp;
use creature_aggro_contracts::LegacyCreatureAggroOwnerSnapshotLikeCpp;
pub use creature_aggro_contracts::LegacyCreatureAggroTickOutcomeLikeCpp;
use creature_aggro_contracts::LegacyCreatureAggroVisibilityDecisionLikeCpp;
use creature_aggro_contracts::LegacyCreatureAiCanAttackDecisionLikeCpp;
use creature_aggro_contracts::LegacyCreatureAiSelectionDecisionLikeCpp;
use creature_aggro_contracts::LegacyCreatureCanAttackLeashDecisionLikeCpp;
pub use creature_aggro_contracts::LegacyCreatureLifecycleTickOutcomeLikeCpp;
pub use creature_aggro_contracts::LegacyCreatureMeleeTickOutcomeLikeCpp;
pub use creature_aggro_contracts::LegacyCreatureMovementTickOutcomeLikeCpp;
pub use creature_aggro_contracts::LegacyCreatureSpellTickOutcomeLikeCpp;
use creature_aggro_contracts::LegacyCreatureThreatUpdateLikeCpp;
use creature_aggro_contracts::check_no_gray_aggro_config_like_cpp;
use creature_aggro_contracts::creature_ai_spell_disable_decision_like_cpp;
use creature_aggro_contracts::spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp;
mod creature_canonical_adapter;
pub(crate) use creature_canonical_adapter::add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp;
pub(crate) use creature_canonical_adapter::reconcile_creature_loot_authority_mirrors_like_cpp;
pub(crate) use creature_canonical_adapter::relocate_canonical_creature_map_object_on_map_like_cpp;
pub(crate) use creature_canonical_adapter::remove_canonical_creature_map_object_on_map_like_cpp;
pub(crate) use creature_canonical_adapter::remove_canonical_respawn_time_on_map_like_cpp;
pub(crate) use creature_canonical_adapter::sync_canonical_creature_entity_on_map_like_cpp;
mod creature_kill_contracts;
use creature_kill_contracts::PendingCreatureKillRewardLikeCpp;
#[cfg(test)]
pub(crate) use creature_kill_contracts::RepresentedCreatureKillEventLikeCpp;
mod creature_movement_adapter;
use creature_movement_adapter::creature_path_request_like_cpp;
use creature_movement_adapter::resolve_creature_detour_path_like_cpp;
use creature_movement_adapter::trace_monster_move_packet_like_cpp;
mod creature_spawn_contracts;
pub(crate) use creature_spawn_contracts::CreatureCreateDisplaySelectionLikeCpp;
pub(crate) use creature_spawn_contracts::CreatureCreateModelScalarsLikeCpp;
pub(crate) use creature_spawn_contracts::CreatureCreateStatsLikeCpp;
pub use creature_spawn_contracts::PendingCreatureSpawn;
mod creature_spell_admission;
use creature_spell_admission::CreatureSpellCastValidationResultLikeCpp;
use creature_spell_admission::TurretRejectedCastAttemptLikeCpp;
use creature_spell_admission::apply_turret_rejected_cast_attempt_like_cpp;
use creature_spell_admission::creature_ai_effective_spell_range_like_cpp;
#[cfg(test)]
use creature_spell_admission::creature_spell_target_accepts_npc_attack_like_cpp;
use creature_spell_admission::creature_spell_target_is_valid_attack_target_like_cpp;
mod creature_spell_metadata;
use creature_spell_metadata::CreatureAiSpellConditionLikeCpp;
use creature_spell_metadata::CreatureSpellCooldownProfileLikeCpp;
use creature_spell_metadata::creature_ai_effective_spell_info_like_cpp;
use creature_spell_metadata::creature_ai_has_temporally_unrepresented_noninstant_spell_like_cpp;
use creature_spell_metadata::creature_ai_spell_condition_like_cpp;
use creature_spell_metadata::creature_ai_spell_cooldown_profile_like_cpp;
use creature_spell_metadata::creature_ai_spell_cooldowns_entry_like_cpp;
use creature_spell_metadata::creature_ai_spell_difficulty_chain_like_cpp;
use creature_spell_metadata::creature_ai_spell_go_cast_flags_like_cpp;
use creature_spell_metadata::creature_ai_spell_has_represented_cooldown_semantics_like_cpp;
use creature_spell_metadata::creature_ai_spell_has_unrepresented_target_restrictions_like_cpp;
use creature_spell_metadata::creature_ai_spell_initial_cooldown_like_cpp;
use creature_spell_metadata::creature_ai_spell_is_combat_forbidden_like_cpp;
use creature_spell_metadata::creature_ai_spell_repeat_cooldown_like_cpp;
use creature_spell_metadata::creature_ai_spell_x_spell_visual_id_like_cpp;
use creature_spell_metadata::{CreatureAiSpellTargetLikeCpp, creature_ai_spell_target_like_cpp};
mod creature_spell_planning;
use creature_spell_planning::CreatureAiSpellRepresentationRejectionLikeCpp;
use creature_spell_planning::creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp;
use creature_spell_planning::creature_ai_spell_plan_like_cpp;
use creature_spell_planning::creature_ai_spell_requires_projectile_payload_like_cpp;
use creature_spell_planning::creature_ai_spell_single_unit_topology_like_cpp;
use creature_spell_planning::creature_ai_successful_untriggered_spell_resets_combat_timers_like_cpp;
use creature_spell_planning::creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp;
use creature_spell_planning::{
    CreatureSpellCastPlanLikeCpp, CreatureSpellCasterIncarnationLikeCpp,
};
mod creature_spell_publication;
use creature_spell_publication::CreatureSpellHitProfileLikeCpp;
use creature_spell_publication::append_committed_creature_spell_packets_like_cpp;
use creature_spell_publication::creature_spell_cast_log_data_like_cpp;
use creature_spell_publication::represented_creature_spell_hit_profile_like_cpp;
use creature_spell_publication::resolve_creature_spell_hit_profile_like_cpp;
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
use faction_reactions::AttackReputationFactionSnapshotLikeCpp;
pub(crate) use faction_reactions::RepresentedFactionReactionInputLikeCpp;
pub(crate) use faction_reactions::RepresentedGetReactionInputLikeCpp;
pub(crate) use faction_reactions::ReputationGainSourceLikeCpp;
mod gameobject_interaction;
pub(crate) use gameobject_interaction::BattlegroundFlagDropClickTarget;
pub(crate) use gameobject_interaction::RepresentedBattlegroundObjectUseRejection;
pub(crate) use gameobject_interaction::RepresentedCapturePointStateLikeCpp;
#[cfg(test)]
pub(crate) use gameobject_interaction::RepresentedGameObjectCriteriaEvent;
pub(crate) use gameobject_interaction::RepresentedGameObjectUseEffect;
pub(crate) use gameobject_interaction::RepresentedGameObjectUseState;
pub(crate) use gameobject_interaction::RepresentedNewFlagStateRequest;
mod guild_inventory_contracts;
pub(crate) use guild_inventory_contracts::RepresentedBankItemMoveLikeCpp;
#[cfg(test)]
pub(crate) use guild_inventory_contracts::RepresentedGuildBankInventoryMoveLikeCpp;
#[cfg(test)]
pub(crate) use guild_inventory_contracts::RepresentedGuildBankListRequestLikeCpp;
#[cfg(test)]
pub(crate) use guild_inventory_contracts::RepresentedGuildBankMoneyMoveLikeCpp;
pub(crate) use guild_inventory_contracts::RepresentedGuildBankTabActionKindLikeCpp;
#[cfg(test)]
pub(crate) use guild_inventory_contracts::RepresentedGuildBankTabActionLikeCpp;
pub(crate) use guild_inventory_contracts::RepresentedGuildRepairBankStateLikeCpp;
#[cfg(test)]
pub(crate) use guild_inventory_contracts::RepresentedGuildRepairBankWithdrawLikeCpp;
mod instance_bind_contracts;
use instance_bind_contracts::HomebindPersistenceJobLikeCpp;
pub(crate) use instance_bind_contracts::RepresentedHomebindLikeCpp;
pub(crate) use instance_bind_contracts::{
    RepresentedGameObjectSpellCaster, RepresentedPendingBind,
};
mod inventory_request_contracts;
pub(crate) use inventory_request_contracts::DirectInventoryStorageOverlayLikeCpp;
pub(crate) use inventory_request_contracts::MAX_EQUIPMENT_SET_INDEX_LIKE_CPP;
pub(crate) use inventory_request_contracts::RepresentedAutoUnequipOffhandLikeCpp;
pub(crate) use inventory_request_contracts::RepresentedAutoUnequipOffhandReasonLikeCpp;
#[cfg(test)]
pub(crate) use inventory_request_contracts::VendorBuyItemTestOverrideLikeCpp;
use inventory_request_contracts::represented_equipment_set_from_packet_like_cpp;
pub(crate) use inventory_request_contracts::{
    RepresentedEquipmentSetSavedLikeCpp, VendorItemCount,
};
mod item_modifiers;
use item_modifiers::ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP;
pub(crate) use item_modifiers::InitialLoadedItemModsOutcomeLikeCpp;
pub(crate) use item_modifiers::LoadedEquippedItemEnchantmentsOutcomeLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedCombatStatRecalculationLikeCpp;
pub(crate) use item_modifiers::RepresentedItemBonusActionLikeCpp;
pub(crate) use item_modifiers::RepresentedItemBonusStateLikeCpp;
#[cfg(test)]
pub(crate) use item_modifiers::RepresentedItemModsReapplyEventLikeCpp;
pub(crate) use item_modifiers::RepresentedItemSetAuraRefreshEventLikeCpp;
pub(crate) use item_modifiers::RepresentedItemSetEffectLikeCpp;
pub(crate) use item_modifiers::RepresentedItemSetSpellEventLikeCpp;
pub(crate) use item_modifiers::item_storage_fields_values_update_like_cpp;
use item_modifiers::player_class_by_armor_subclass_like_cpp;
use item_modifiers::player_class_mask_for_talent_like_cpp;
use item_modifiers::player_class_mask_for_transmog_like_cpp;
use item_modifiers::represented_player_stat_changes_like_cpp;
pub(crate) use item_modifiers::void_withdrawal_post_store_item_values_update_like_cpp;
use item_modifiers::{RepresentedScalingStatContextLikeCpp, is_represented_bag_slot};
pub(crate) use player_items::item_push_result_from_send_new_item_plan;
mod loot_delivery_contracts;
pub(crate) use loot_delivery_contracts::DurableItemLootCompletionLikeCpp;
pub(crate) use loot_delivery_contracts::DurableItemLootPersistenceGuardLikeCpp;
use loot_delivery_contracts::DurableItemLootPersistenceStateLikeCpp;
pub(crate) use loot_delivery_contracts::DurableItemLootPersistenceTrackerLikeCpp;
pub(crate) use loot_delivery_contracts::DurableLootItemFanoutLikeCpp;
pub(crate) use loot_delivery_contracts::LootMoneyDeliveryAddressLikeCpp;
pub(crate) use loot_delivery_contracts::LootMoneyViewerFanoutLikeCpp;
#[cfg(test)]
pub(crate) use loot_delivery_contracts::RepresentedLootRollCriteriaEvent;
pub(crate) use loot_delivery_contracts::loot_money_durable_outcome_like_cpp;
pub(crate) use loot_delivery_contracts::{RepresentedLootRollState, RepresentedLootRollVote};
mod map_admission;
pub(crate) use map_admission::CreateMapSideEffectApplySummaryLikeCpp;
use map_admission::create_map_instance_lock_token_like_cpp;
pub use map_admission::{MMapRuntimeConfigLikeCpp, WaypointPathResolverLikeCpp};
pub use map_admission::{PlayerGridLoadOutcomeLikeCpp, PlayerGridLoadResolverLikeCpp};
use map_admission::{create_map_decision_difficulty_id_like_cpp, create_map_decision_key_like_cpp};
mod money_persistence_contracts;
use money_persistence_contracts::AbsolutePlayerMoneyCommitReconciliationLikeCpp;
pub(crate) use money_persistence_contracts::CommittedRepresentedTalentResetLikeCpp;
pub(crate) use money_persistence_contracts::ExclusivePlayerMoneyPersistenceLikeCpp;
pub(crate) use money_persistence_contracts::LootMoneyPersistenceErrorLikeCpp;
pub(crate) use money_persistence_contracts::PlayerMoneyCommitCancellationFenceLikeCpp;
use money_persistence_contracts::RepresentedTalentResetStatePlanLikeCpp;
use money_persistence_contracts::reconcile_absolute_player_money_commit_like_cpp;
mod movement_protocol;
use movement_protocol::PLAYER_BASE_MOVE_SPEED_LIKE_CPP;
#[cfg(test)]
pub(crate) use movement_protocol::RepresentedAreaZoneCriteriaLikeCpp;
#[cfg(test)]
pub(crate) use movement_protocol::RepresentedTaxiFlightNodeLikeCpp;
#[cfg(test)]
use movement_protocol::canonical_taxi_flight_state_like_cpp;
#[cfg(test)]
use movement_protocol::represented_taxi_flight_node_like_cpp;
#[cfg(test)]
use movement_protocol::represented_taxi_flight_state_like_cpp;
pub(crate) use movement_protocol::{MoveSplineDoneTaxiActionLikeCpp, MovementAckEventLikeCpp};
#[cfg(test)]
pub(crate) use movement_protocol::{MoveSplineDoneTaxiEventLikeCpp, MoveTeleportAckEventLikeCpp};
pub(crate) use movement_protocol::{MoveTeleportAckActionLikeCpp, MovementSpeedAckActionLikeCpp};
pub(crate) use movement_protocol::{MovementFallDamageEvent, MovementUnderMapDamageEvent};
pub(crate) use movement_protocol::{
    MovementSpeedAckEventLikeCpp, UnitMoveTypeLikeCpp,
    creature_movement_spline_speed_opcode_like_cpp, movement_speed_ack_move_type_like_cpp,
    player_movement_speed_opcodes_like_cpp,
};
#[cfg(test)]
use movement_protocol::{RepresentedTaxiFlightStateLikeCpp, canonical_taxi_flight_node_like_cpp};
mod npc_interaction;
mod persistence_capabilities;
pub use persistence_capabilities::CatalogPersistenceCapabilitiesLikeCpp;
pub(crate) use persistence_capabilities::CharacterPowerSnapshotLikeCpp;
pub use persistence_capabilities::PlayerPersistenceCapabilitiesLikeCpp;
pub(crate) use persistence_capabilities::PlayerSaveToDbSnapshotLikeCpp;
pub use persistence_capabilities::SessionAdmissionPersistenceLikeCpp;
pub use persistence_capabilities::SessionPersistencePortsLikeCpp;
pub use persistence_capabilities::WorldPersistenceCapabilitiesLikeCpp;
use persistence_capabilities::character_power_snapshot_values_like_cpp;
#[cfg(test)]
use persistence_capabilities::empty_character_power_snapshot_like_cpp;
use persistence_capabilities::loaded_character_power_snapshot_like_cpp;
mod pet_dismissal;
mod pet_loading;
pub(crate) use pet_loading::CharacterAuraEffectRowLikeCpp;
pub(crate) use pet_loading::CharacterPetDeclinedNamesRowLikeCpp;
pub(crate) use pet_loading::CharacterPetSpellCooldownRowLikeCpp;
pub(crate) use pet_loading::adjusted_represented_pet_aura_remain_time_like_cpp;
pub(crate) use pet_loading::{CharacterAuraRowLikeCpp, CharacterPetAuraEffectRowLikeCpp};
pub(crate) use pet_loading::{CharacterPetAuraRowLikeCpp, CharacterPetSpellChargeRowLikeCpp};
pub(crate) use pet_loading::{CharacterPetSpellRowLikeCpp, CharacterPetStableRowLikeCpp};
mod player_binding;
use player_binding::PlayerIdentityBootstrapLikeCpp;
#[cfg(test)]
use player_binding::PlayerTransportLoginStateLikeCpp;
pub(crate) use player_binding::SessionPlayerController;
mod player_bootstrap;
mod player_condition_values;
pub(crate) use player_condition_values::RepresentedItemLevelCapsLikeCpp;
pub(crate) use player_condition_values::RepresentedPlayerConditionContextLikeCpp;
mod player_load_values;
use player_load_values::{gender_from_u8, unix_secs_to_ms_like_cpp};
use player_load_values::{player_team_id_for_race_cpp, represented_pet_aura_slot_like_cpp};
mod player_melee_application;
pub use player_melee_application::LegacyPlayerMeleeTickOutcomeLikeCpp;
use player_melee_application::PLAYER_MELEE_COMBAT_REF_REVALIDATE_INTERVAL_MS;
use player_melee_application::PendingPlayerSwingLikeCpp;
pub(crate) use player_melee_application::PlayerAttackStartLikeCppResult;
pub use player_melee_application::PlayerMeleeAttackerSnapshotLikeCpp;
pub(crate) use player_melee_application::PlayerMeleeCreatureHitLikeCpp;
pub use player_melee_application::PlayerMeleePhaseStateLikeCpp;
pub(in crate::session) use player_melee_application::RepresentedMeleeDamageBonusLikeCpp;
use player_melee_application::apply_player_melee_to_canonical_player_like_cpp;
use player_melee_application::begin_combat_ref_on_map_like_cpp;
use player_melee_application::is_unit_facing_target_for_melee_like_cpp;
use player_melee_application::is_within_melee_range_like_cpp;
use player_melee_application::is_within_target_boundary_radius_like_cpp;
pub(in crate::session) use player_melee_application::legacy_attack_power_multiplier_like_cpp;
use player_melee_application::represented_white_swing_damage_like_cpp;
pub(in crate::session) use player_melee_application::take_canonical_player_attack_swings_like_cpp;
mod player_presentation;
use player_presentation::RepresentedMountSpellCheckOutcomeLikeCpp;
mod player_registry_binding;
mod player_spell_records;
pub(crate) use player_spell_records::RepresentedCharacterSpellChargeLikeCpp;
pub(crate) use player_spell_records::RepresentedCharacterSpellCooldownLikeCpp;
pub(crate) use player_spell_records::RepresentedPlayerSkillLikeCpp;
pub(crate) use player_spell_records::RepresentedPlayerSkillStateLikeCpp;
pub(crate) use player_spell_records::RepresentedPlayerSpellLikeCpp;
use player_spell_records::RepresentedPlayerSpellRuntimeLikeCpp;
pub(crate) use player_spell_records::RepresentedPlayerSpellStateLikeCpp;
use player_spell_records::canonical_player_skill_record_like_cpp;
use player_spell_records::canonical_player_spell_record_like_cpp;
#[cfg(test)]
use player_spell_records::canonical_player_spell_runtime_like_cpp;
#[cfg(test)]
pub(crate) use player_spell_records::is_non_durable_skill_tombstone_like_cpp;
use player_spell_records::represented_player_skill_record_like_cpp;
use player_spell_records::represented_player_spell_record_like_cpp;
use player_spell_records::represented_player_spell_runtime_like_cpp;
use player_spell_records::represented_skill_records_from_values_like_cpp;
use player_spell_records::represented_skill_values_from_records_like_cpp;
mod player_vitals_adapter;
mod progression_adapters;
mod quest_dialog;
pub(crate) use quest_dialog::LoadSeasonalQuestStatusOutcomeLikeCpp;
pub(crate) use quest_dialog::RepresentedAdventureMapStartQuestLikeCpp;
pub(crate) use quest_dialog::RepresentedPendingQuestSharingLikeCpp;
pub(crate) use quest_dialog::RepresentedPushQuestToPartyOutcomeLikeCpp;
pub(crate) use quest_dialog::RepresentedPushQuestToPartyOutcomeReasonLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestCompleteStatusUpdateLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestConfirmAcceptLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestPushResultResponseLikeCpp;
#[cfg(test)]
pub(crate) use quest_dialog::RepresentedQuestRewardMailLikeCpp;
#[cfg(test)]
pub(crate) use quest_dialog::RepresentedQuestRewardReputationLikeCpp;
pub(crate) use quest_dialog::RepresentedQuestRewardReputationSourceLikeCpp;
#[cfg(test)]
pub(crate) use quest_dialog::RepresentedQuestRewardSpellCastLikeCpp;
#[cfg(test)]
pub(crate) use quest_dialog::RepresentedQuestRewardSpellKindLikeCpp;
#[cfg(test)]
pub(crate) use quest_dialog::RepresentedQuestRewardTalentPointsLikeCpp;
pub(crate) use quest_dialog::ResetSeasonalQuestStatusOutcomeLikeCpp;
pub(crate) use quest_dialog::ResetSeasonalQuestStatusReasonLikeCpp;
pub(crate) use quest_dialog::SeasonalQuestStatusDbRowLikeCpp;
#[cfg(test)]
use quest_dialog::primary_power_type_for_player_class_like_cpp;
#[cfg(test)]
pub(crate) use quest_dialog::{
    RepresentedForceDeselectLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
use quest_dialog::{RepresentedPreparedQuestMenuItemLikeCpp, sheath_state_from_u8_like_cpp};
use quest_dialog::{active_state_from_db_like_cpp, pet_type_from_db_like_cpp};
use quest_dialog::{power_type_from_u8_like_cpp, unit_stand_state_from_u8_like_cpp};
use quest_dialog::{quest_giver_creature_id_from_source_like_cpp, react_state_from_db_like_cpp};
use quest_dialog::{quest_has_represented_item_objective_like_cpp, quest_rewards_block_like_cpp};
mod quest_interaction;
mod raid_profile_values;
use raid_profile_values::player_cuf_profile_from_packet_like_cpp;
use raid_profile_values::player_cuf_profile_to_packet_like_cpp;
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
pub(crate) use social_requests::RepresentedWargameInviteAcceptanceLikeCpp;
use social_requests::party_member_power_kind_from_u8_like_cpp;
use social_requests::party_member_power_to_u16_like_cpp;
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
use spell_click_values::represented_spell_cast_guid_for_map_like_cpp;
use spell_click_values::represented_spell_click_school_damage_amount_like_cpp;
use spell_click_values::spell_effect_has_non_or_db_nearby_entry_destination_like_cpp;
use spell_click_values::spell_effect_is_represented_summon_object_slot_like_cpp;
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
mod summon_object_contracts;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectSlotSessionStatusLikeCpp;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectWildSessionOutcomeLikeCpp;
pub(crate) use summon_object_contracts::ApplyEffectSummonObjectWildSessionStatusLikeCpp;
pub(crate) use summon_object_contracts::RepresentedSpellFocusObjectLikeCpp;
mod support_features;
mod taxi_contracts;
pub(crate) use taxi_contracts::RepresentedActivateTaxiLikeCpp;
mod time_synchronization;
mod trade_adapter;
mod void_storage_adapter;
mod xp_grants;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::{
    Arc, Mutex, OnceLock, Weak,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::{Rng, RngCore, SeedableRng, rngs::StdRng, seq::SliceRandom};
use tracing::{debug, info, trace, warn};

use crate::battle_pet_account::{
    BattlePetAccountAttachmentLikeCpp, BattlePetAddFailureLikeCpp, BattlePetAddOutcomeLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetAddRequestLikeCpp, BattlePetMutationFailureLikeCpp,
};
use crate::entity_update_bridge::{
    bag_values_update_to_update_object, dynamic_object_values_update_to_update_object,
    game_object_values_update_to_update_object, item_values_update_to_update_object,
    player_values_update_to_update_object, unit_values_update_to_packet,
    unit_values_update_to_update_object,
};
use crate::loot_persistence::{
    DurableLootMoneyCompletionLikeCpp, DurableLootMoneyPersistenceGuardLikeCpp,
    DurableLootMoneyPersistenceTrackerLikeCpp, DurableLootMoneySaveFenceLikeCpp,
};
use crate::map_manager::{
    PendingRespawn, RecipientRule, RuntimeEvent, RuntimeOutput, RuntimePlan, RuntimeTickOwner,
    WorldMMapPathfinderWorkerLikeCpp,
};
use crate::phasing::{init_db_phase_shift_like_cpp, init_db_visible_map_id_like_cpp};
use crate::reputation::{
    ReputationMgrLikeCpp, ReputationMgrMutLikeCpp, ReputationMgrRefLikeCpp,
    reputation_to_rank_like_cpp,
};
use crate::session::directory::{
    PlayerRegistry, PlayerSessionRegistrationLikeCpp, PlayerVisibilityCreateSnapshot,
};
use crate::session::mailbox::{
    CreatureAttackStartLikeCppCommand, GameEventQuestCompleteClientOutcomeLikeCpp,
    GameEventQuestCompleteCommandLikeCpp, KickLikeCppCommand, LootRollCommandIdentityLikeCpp,
    NotifyLootMoneyRemovedLikeCppCommand, SendIfVisibleLikeCppCommand, SessionCommand,
    SharedClientVisibleGuidsLikeCpp,
};
use crate::session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, PlayerRegenerationRatesLikeCpp,
    ReputationRatesLikeCpp,
};
use wow_ai::{
    CURRENT_EXPANSION_LIKE_CPP, CreatureAiCanAttackInputLikeCpp, CreatureAiKindLikeCpp,
    CreatureAiSelectionInputLikeCpp, CreatureAttackDistanceInputLikeCpp,
    creature_ai_can_attack_like_cpp, creature_ai_uses_base_move_in_line_of_sight_like_cpp,
    creature_attack_distance_like_cpp, max_level_for_expansion_like_cpp,
    select_creature_ai_like_cpp,
};
use wow_constants::creature::{CreatureFlagsExtra, CreatureType, CreatureTypeFlags};
use wow_constants::item::{
    CurrencyTypes, CurrencyTypesFlags, EnchantmentSlot, ItemBonusType, ItemFieldFlags,
    ItemFieldFlags2,
};
use wow_constants::movement::MovementFlag;
use wow_constants::shared::DifficultyFlags;
use wow_constants::unit::{
    Gender, NPCFlags1, PowerType, SheathState, Team, UnitFlags, UnitFlags2, UnitPvpFlags,
    UnitStandStateType, WeaponAttackType,
};
use wow_constants::{
    BagFamilyMask, BuyResult, ClientOpcodes, InventoryResult, InventoryType, ItemBondingType,
    ItemClass, ItemContext, ItemEnchantmentType, ItemFlags, ItemFlags2, ItemFlags3, ItemModifier,
    ItemQuality, ItemSpelltriggerType, ItemSubClassArmor, ItemSubClassWeapon, SellResult,
    ServerOpcodes, SpellCastResult, SpellItemEnchantmentFlags, Stats, TypeId, UnitState,
};
use wow_core::{
    EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position,
    VoidStorageItemIdGeneratorLikeCpp, guid::HighGuid,
};
use wow_data::character_progression::{ChrClassesStore, ChrRacesStore, PowerTypeStore};
use wow_data::trait_tree::{TraitDefinitionStore, TraitNodeEntryStore};
use wow_data::{
    AccessRequirementStoreLikeCpp, AdventureMapPoiStore, AreaTableStore, AreaTriggerDb2Store,
    AreaTriggerScriptStoreLikeCpp, AreaTriggerStore, BankBagSlotPricesStore, BattlemasterListStore,
    ChrSpecializationStore, CinematicSequencesStore, CombatRatingsGameTableLikeCpp,
    ConditionEntriesByTypeStore, CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp,
    CreatureClassificationHealthRatesLikeCpp, CreatureDifficultyStoreLikeCpp,
    CreatureDisplayInfoExtraStore, CreatureDisplayInfoStore, CreatureEquipmentStoreLikeCpp,
    CreatureModelDataStore, CreatureSpellDisableDecisionLikeCpp,
    CreatureTemplateLifecycleStoreLikeCpp, CreatureTemplateMountStoreLikeCpp, CurrencyTypesEntry,
    CurrencyTypesStore, DISABLE_TYPE_BATTLEGROUND, DISABLE_TYPE_MAP, DifficultyStore,
    DisableMgrLikeCpp, DisableWorldObjectRefLikeCpp, DungeonEncounterStore, DurabilityCostsStore,
    DurabilityQualityStore, EmotesStore, EmotesTextStore, ExplorationBaseXpStoreLikeCpp,
    FishingBaseSkillStoreLikeCpp, GameObjectDisplayInfoStore,
    GameObjectTemplateLifecycleStoreLikeCpp, GemPropertiesStore, GlyphPropertiesStore,
    GraveyardStore, HeirloomEntry, HeirloomStore, HotfixBlobCache, ImportPriceStores,
    ItemAppearanceStore, ItemBonusDb2Store, ItemChildEquipmentEntry, ItemChildEquipmentStore,
    ItemClassStore, ItemCurrencyCostStore, ItemDisenchantLootStore, ItemEffectStore,
    ItemExtendedCostStore, ItemLimitCategoryConditionStore, ItemLimitCategoryStore,
    ItemModifiedAppearanceStore, ItemPriceBaseStore, ItemRandomEnchantmentTemplateStore,
    ItemRandomPropertiesStore, ItemRandomPropertyTemplateEntry, ItemRandomSuffixStore,
    ItemSearchNameStore, ItemSetSpellStore, ItemSetStore, ItemSpecOverrideStore, ItemStatsStore,
    ItemStore, LfgDungeonStoreLikeCpp, LfgDungeonsStore, LockStore, MapDifficultyStore,
    MapDifficultyXConditionStore, MapStore, MountCapabilityStore, MountDefinitionStoreLikeCpp,
    MountStore, MountTypeXCapabilityStore, MountXDisplayStore, MovieStore,
    NpcSpellClickStoreLikeCpp, PhaseGroupStore, PhaseStore, PlayerConditionAuraLikeCpp,
    PlayerConditionContextLikeCpp, PlayerConditionCountLikeCpp, PlayerConditionPartyStatusLikeCpp,
    PlayerConditionQuestKillLikeCpp, PlayerConditionReputationLikeCpp, PlayerConditionSkillLikeCpp,
    PlayerConditionStore, PlayerCreateInfoCastSpellStoreLikeCpp,
    PlayerCreateInfoCustomSpellStoreLikeCpp, PlayerCreateInfoStoreLikeCpp, PlayerStatsStore,
    PvpItemStore, RandPropPointsStore, RegenGameTablesLikeCpp, SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
    ScriptIdLikeCpp, ScriptNameInternerLikeCpp, ShieldBlockRegularGameTableLikeCpp, SkillLineStore,
    SkillRangeTypeLikeCpp, SkillStore, SkillTiersStoreLikeCpp, SpellAcquisitionCatalogLikeCpp,
    SpellAreaLikeCpp, SpellAreaStoreLikeCpp, SpellAuraOptionsStore, SpellAuraRestrictionsStore,
    SpellCategoryStore, SpellChainStoreLikeCpp, SpellCustomAttributeStoreLikeCpp,
    SpellDurationStore, SpellEquippedItemsEntry, SpellEquippedItemsStore,
    SpellGroupStackRuleLikeCpp, SpellGroupStackRuleStoreLikeCpp, SpellGroupStoreLikeCpp,
    SpellItemEnchantmentConditionStore, SpellItemEnchantmentStore, SpellLearnSkillLookupLikeCpp,
    SpellLearnSkillNodeLikeCpp, SpellLearnSkillStoreLikeCpp, SpellLearnSpellNodeLikeCpp,
    SpellLearnSpellStoreLikeCpp, SpellLevelsStore, SpellLinkedStoreLikeCpp, SpellLinkedTypeLikeCpp,
    SpellMiscStore, SpellPetAuraStoreLikeCpp, SpellProcEntryLikeCpp, SpellProcStoreLikeCpp,
    SpellRadiusStore, SpellRangeStore, SpellRequiredStoreLikeCpp, SpellShapeshiftFormStore,
    SpellStore, SpellTargetPositionStoreLikeCpp, SpellTargetRestrictionsStore,
    SpellThreatEntryLikeCpp, SpellThreatStoreLikeCpp, SummonPropertiesEntry, TactKeyStore,
    TalentStore, TalentTabStore, TavernAreaTriggerStoreLikeCpp, ToyStore, TrainerStoreLikeCpp,
    TransmogSetEntry, TransmogSetItemStore, TrinityStringStoreLikeCpp,
    VEHICLE_SEAT_FLAG_CAN_ATTACK, VehicleAccessoryStoreLikeCpp, VehicleSeatStore, VehicleStore,
    WorldSafeLocStore, is_player_meeting_condition_like_cpp,
    progression_rewards::{
        ContentTuningStore, CurvePointStore, CurveStore, FactionEntry, FactionStore,
        FactionTemplateStore, FriendshipRepReactionStore, NumTalentsAtLevelStore,
        ParagonReputationStore, QuestFactionRewardStore, QuestInfoStore, QuestMoneyRewardStore,
        QuestPackageItemStore, QuestV2Store, ScalingStatDistributionEntry,
        ScalingStatDistributionStore, ScalingStatValuesStore,
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
    BattlePetSpeciesStore, BattlePetXpGameTableLikeCpp, calculate_battle_pet_stats_like_cpp,
};
#[cfg(test)]
use wow_data::{
    PetDefaultSpellStoreLikeCpp, PetDefaultSpellsEntryLikeCpp, PetFamilySpellStoreLikeCpp,
    PetLevelupSpellSetLikeCpp, PetLevelupSpellStoreLikeCpp, ServersideSpellInfoLikeCpp,
    ServersideSpellStoreLikeCpp, SpellEnchantProcEntryLikeCpp, SpellEnchantProcStoreLikeCpp,
    SpellTotemModelStoreLikeCpp, VehicleTemplateStoreLikeCpp,
};
#[cfg(test)]
use wow_entities::TitanGripPenaltyAction;
use wow_entities::player_rules::is_using_two_handed_weapon_in_one_hand_template as two_handed_in_one_hand_like_cpp;
use wow_entities::{
    AccessorObjectKind, ActiveState, ApplyEnchantmentArgs, ApplyEnchantmentDurationAction,
    ApplyEnchantmentEffectAction, ApplyEnchantmentEffectRef, ApplyEnchantmentGemRequirementRef,
    ApplyEnchantmentPlan, ApplyEnchantmentRandomSuffixRef, ApplyEnchantmentResult,
    ApplyEnchantmentSocketContext, ApplyEnchantmentTemplateRef, BANK_SLOT_BAG_END,
    BANK_SLOT_BAG_START, BUYBACK_SLOT_COUNT, BUYBACK_SLOT_END, BUYBACK_SLOT_START, BagTemplateRef,
    CanBankItemArgs, CanEquipItemArgs, CanEquipItemOutcome, CanEquipUniqueItemArgs,
    CanStoreItemArgs, CanUnequipItemArgs, CanUseItemArgs, CanUseItemTemplateArgs,
    CreatureAddonLifecycleRecordLikeCpp, EQUIPMENT_SLOT_BACK, EQUIPMENT_SLOT_BODY,
    EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_END, EQUIPMENT_SLOT_FEET, EQUIPMENT_SLOT_FINGER1,
    EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD, EQUIPMENT_SLOT_LEGS,
    EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_NECK, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_RANGED,
    EQUIPMENT_SLOT_SHOULDERS, EQUIPMENT_SLOT_TABARD, EQUIPMENT_SLOT_TRINKET1,
    EQUIPMENT_SLOT_TRINKET2, EQUIPMENT_SLOT_WAIST, EQUIPMENT_SLOT_WRISTS, EquippedGemRef,
    GAMEOBJECT_TYPE_GUILD_BANK, GameObject, INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, ITEM_DATA_BITS, ITEM_DATA_CONTAINED_IN_BIT, ITEM_DATA_CREATOR_BIT,
    ITEM_DATA_DURABILITY_BIT, ITEM_DATA_DYNAMIC_FLAGS_BIT, ITEM_DATA_DYNAMIC_FLAGS2_BIT,
    ITEM_DATA_ENCHANTMENT_FIRST_BIT, ITEM_DATA_ENCHANTMENT_PARENT_BIT, ITEM_DATA_PARENT_BIT,
    ITEM_DATA_PROPERTY_SEED_BIT, ITEM_DATA_RANDOM_PROPERTIES_ID_BIT, Item, ItemCreateInfo,
    ItemDataUpdate, ItemLimitCategoryTemplate, ItemPosCount, ItemSlotRef, ItemStorageRef,
    ItemStorageTemplate, ItemValuesUpdate, MAX_BAG_SIZE, MAX_ITEM_SPELLS, MAX_MONEY_AMOUNT,
    MAX_POWERS, MAX_POWERS_PER_CLASS, MovementGeneratorKind, MovementSlot, NULL_BAG, NULL_SLOT,
    PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, PLAYER_SLOT_END, PROFESSION_SLOT_END, Pet, PetAuraLikeCpp,
    PetDeclinedNamesLikeCpp, PetSaveMode, PetSpellState, PetSpellType, PetStable, PetStableInfo,
    PetType, PhaseShift, Player, PlayerEnchantTimeUpdate, PlayerInteractionDataLikeCpp,
    PlayerInventoryRuntime, PlayerItemTimeUpdate, PlayerPetLifecycleStateLikeCpp,
    PlayerQuestGameplayState, PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp,
    PlayerTeleportStateLikeCpp, QUESTS_COMPLETED_BITS_PER_BLOCK, QUESTS_COMPLETED_BITS_SIZE,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, ReactState, SendNewItemDelivery,
    SendNewItemDisplayText, SendNewItemPlan, SocketedGemUniqueRef, SwapItemPreflightItem,
    SwapItemPreflightPlan, TYPEID_CONTAINER, TYPEID_ITEM, UNIT_DATA_BITS,
    UNIT_DATA_EMOTE_STATE_BIT, UNIT_DATA_HEALTH_BIT, UNIT_DATA_MODS_PARENT_BIT, Unit,
    UnitDataUpdate, UnitDataValues, UnitVisibilityDetectionStateLikeCpp, UpdateMask, Vehicle,
    VehicleAccessory, VisibleItemValues, WorldObject,
    explored_zones_db_string_from_blocks_like_cpp, is_bag_pos, is_equipment_packed_pos,
    is_inventory_pos, item_resistance_bonus_actions_like_cpp,
    item_scaling_stat_bonus_actions_like_cpp, item_shield_block_bonus_action_like_cpp,
    item_stat_bonus_actions_like_cpp, item_weapon_damage_actions_like_cpp, make_item_pos,
    parse_explored_zones_db_string_like_cpp,
};
use wow_entities::{
    BagValuesUpdate, CONTAINER_DATA_BITS, CONTAINER_DATA_SLOTS_FIRST_BIT,
    CONTAINER_DATA_SLOTS_PARENT_BIT, ContainerDataUpdate, ContainerDataValues,
};
pub(crate) use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_handler::{PacketProcessing, SessionStatus};

// Only the test modules mounted into this file name these directly; the
// production surface reaches them through `wow-session` since #297.
#[cfg(test)]
use wow_network::{SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp};

use registry::{PacketHandlerEntry, build_dispatch_table};
use wow_loot::{
    LootClaimLease, LootStoreKind, LootStores, OwnedLootAuthority, OwnedLootAuthorityLifecycle,
    OwnedLootAuthorityStamp, OwnedLootScope, OwnedLootSnapshot,
};
use wow_map::coords::SIZE_OF_GRID_CELL;
use wow_network::SocketTimeoutsLikeCpp;
use wow_network::session_mgr::SessionManager;
use wow_packet::packets::chat::{ChatMsg, ChatPkt, PrintNotification};
use wow_packet::packets::gossip::ClientGossipText;
use wow_packet::packets::item::{
    InventoryChangeFailure, ItemEnchantTimeUpdate, ItemInstance, ItemMod, ItemModList,
    ItemPushResult, ItemPushResultDisplayType, ItemTimeUpdate,
};
use wow_packet::packets::misc::{
    AccountHeirloom, AccountHeirloomUpdate, AccountMount, AccountMountUpdate, AccountToy,
    AccountToyUpdate, BuyFailed, DungeonDifficultySet, EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP,
    FeatureSystemConfigLikeCpp, FeatureSystemStatus, FeatureSystemStatusGlueScreen,
    MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP, MountResult, NUM_ACCOUNT_DATA_TYPES, RaidDifficultySet,
    SellResponse, SetProficiency, SetupCurrency, SetupCurrencyRecord, SpellChargeEntry,
    SpellHistoryEntry, TRADE_SLOT_COUNT_LIKE_CPP, TRADE_STATUS_ACCEPTED_LIKE_CPP,
    TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
    TRADE_STATUS_UNACCEPTED_LIKE_CPP, TradeStatus,
};
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
const QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP: u8 = 6;
const QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP: u8 = 7;
const QUEST_OBJECTIVE_MONEY_LIKE_CPP: u8 = 8;
const DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP: u32 = 15 * 60 * 1000;
const QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP: u8 = 9;
const QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP: u8 = 16;
const QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP: u8 = 17;
const QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP: u8 = 18;
#[cfg(test)]
const DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP: u32 = 100;
const QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP: u32 = 0x0080;
const QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP: u32 = 0x1;
const QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP: u32 = 0x0010_0000;
const QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP: u32 = 0x0000_1000;
const MAX_GAMEOBJECT_SLOT_LIKE_CPP: usize = 4;
pub(crate) const MAX_SPECIALIZATIONS_LIKE_CPP: usize = 4;
const NEEDED_TALENT_POINT_PER_TIER_LIKE_CPP: u32 = 5;
const PLAYER_FLAGS_UBER_LIKE_CPP: u32 = 0x0008_0000;
const PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP: u32 = 0x0000_0001;
pub(crate) const PLAYER_FLAGS_AFK_LIKE_CPP: u32 = 0x0000_0002;
pub(crate) const PLAYER_FLAGS_DND_LIKE_CPP: u32 = 0x0000_0004;
pub(crate) const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = 0x0000_0010;
const PLAYER_FLAGS_RESTING_LIKE_CPP: u32 = 0x0000_0020;
const PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP: u32 = 0x0000_0800;
const PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP: u32 = 0x0200_0000;
pub(crate) const PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP: u32 = 0x2000_0000;
pub(crate) use wow_constants::rest::{
    REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP, REST_STATE_RESTED_LIKE_CPP,
};

/// Process-wide ownership of a character's live `Player` runtime.
///
/// C++ has one `Player*` per GUID in `ObjectAccessor`; accepting a second
/// session would create two independent save authorities for the same rows.
/// Reserve the GUID before the asynchronous login pipeline starts. Weak
/// values make an abandoned session claim recoverable without a global sweep.
static ACTIVE_CHARACTER_LOGIN_CLAIMS_LIKE_CPP: OnceLock<dashmap::DashMap<ObjectGuid, Weak<()>>> =
    OnceLock::new();

/// Live seam for C++ `ScriptMgr::OnAreaTrigger`.
///
/// A ported content script receives the mutable session and returns the same
/// consumed/not-consumed boolean that controls C++ handler continuation.
pub type AreaTriggerScriptDispatcherLikeCpp =
    Arc<dyn Fn(&mut WorldSession, ScriptIdLikeCpp, u32, bool) -> bool + Send + Sync>;

#[cfg(test)]
type GivePlayerXpScriptDispatcherLikeCpp =
    Arc<dyn Fn(wow_script::player::GivePlayerXpContextLikeCpp, &mut u32) + Send + Sync>;

const REST_FLAG_IN_TAVERN_LIKE_CPP: u32 = 0x1;
const REST_FLAG_IN_CITY_LIKE_CPP: u32 = 0x2;
const REST_FLAG_IN_FACTION_AREA_LIKE_CPP: u32 = 0x4;
// C++ `RestMgr::SetRestBonus`: `float(next_level_xp) * 1.5f / 2`.
#[cfg(test)]
const REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP: f32 = 1.5 / 2.0;
const REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP: f32 = 0.031;
const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP: f32 = 0.125;
const REST_ONLINE_INGAME_BUBBLE_LIKE_CPP: f32 = 0.125;
const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;
const MAP_INSTANCE_LIKE_CPP: u8 = 1;
const MAP_RAID_LIKE_CPP: u8 = 2;
pub(crate) const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0x0000_0100;
const PLAYER_FLAGS_IN_PVP_LIKE_CPP: u32 = 0x0000_0200;
const PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP: u32 = 0x0002_0000;
const PLAYER_FLAGS_PVP_TIMER_LIKE_CPP: u32 = 0x0004_0000;
const PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP: u32 = 0x0800_0000;
const SPELL_PVP_RULES_ENABLED_LIKE_CPP: i32 = 134_735;
const LANG_RESET_SPELLS_LIKE_CPP: u32 = 215;
const LANG_RESET_TALENTS_LIKE_CPP: u32 = 216;
const LANG_RESET_SPELLS_TEXT_LIKE_CPP: &str = "Your spells have been reset.";
const LANG_RESET_TALENTS_TEXT_LIKE_CPP: &str = "Your talents have been reset.";
pub(crate) const TRADE_STATUS_PLAYER_BUSY_LIKE_CPP: u8 = 0;
const PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP: u32 = 0x0000_0800;
const AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP: u32 = 0x0000_0010;
const AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP: u32 = 0x0000_0080;
const AREA_FLAG_CONTESTED_LIKE_CPP: u32 = 0x0004_0000;
const AREA_FLAG_COMBAT_ZONE_LIKE_CPP: u32 = 0x0100_0000;
const CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP: u8 = 0x13;
pub(crate) type TeleportToOptionsLikeCpp = u32;
pub(crate) const TELE_TO_NONE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x00;
#[allow(dead_code)]
pub(crate) const TELE_TO_GM_MODE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x01;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_TRANSPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x02;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x04;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP: TeleportToOptionsLikeCpp = 0x08;
pub(crate) const TELE_TO_SPELL_LIKE_CPP: TeleportToOptionsLikeCpp = 0x10;
#[allow(dead_code)]
pub(crate) const TELE_TO_TRANSPORT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x20;
#[allow(dead_code)]
pub(crate) const TELE_REVIVE_AT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x40;
pub(crate) const TELE_TO_SEAMLESS_LIKE_CPP: TeleportToOptionsLikeCpp = 0x80;
const ATTACK_DISPLAY_DELAY_LIKE_CPP_MS: u32 = 200;
const DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP: f32 = 1.5;
const MIN_MELEE_REACH_LIKE_CPP: f32 = 2.0;
const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;
const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP: u32 = 0x0000_0010;
const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP: u32 = 0x0001_0000;

const QUEST_MENU_ICON_TURN_IN_LIKE_CPP: u8 = 0;
const QUEST_MENU_ICON_AVAILABLE_LIKE_CPP: u8 = 2;
const QUEST_MENU_ICON_COMPLETE_LIKE_CPP: u8 = 4;

const PACKET_SPOOF_BAN_REASON_LIKE_CPP: &str = "DOS (Packet Flooding/Spoofing";
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
const HOUR_SECS_LIKE_CPP: u64 = 60 * 60;
const MAP_BATTLEGROUND_LIKE_CPP: i8 = 3;
const MAP_ARENA_LIKE_CPP: i8 = 4;
const GROUP_XP_DISTANCE_LIKE_CPP: f32 = 74.0;
const BATTLEGROUND_WS_LIKE_CPP: u32 = 2;
// C++ `SpellCastSource::Normal` is encoded in the six-bit Cast GUID subtype.
// The capture contract validates this field rather than treating it as a
// runtime counter, so keep the canonical numeric value here.
const SPELL_CAST_SOURCE_NORMAL_LIKE_CPP: u8 = 3;
pub(crate) const CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP: u32 = 0x08000;

/// C++ `CAST_FLAG_PENDING` (`Spells/Spell.h:78`). `SendSpellStart` and
/// `SendSpellGo` set it for a triggered cast that is not `m_fromClient`.
pub(crate) const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;
const BATTLEGROUND_EY_LIKE_CPP: u32 = 7;

pub(crate) use wow_entities::PlayerAccountHeirloomDataLikeCpp as AccountHeirloomDataLikeCpp;
pub(crate) use wow_entities::PlayerFavoriteAppearanceStateLikeCpp as FavoriteAppearanceStateLikeCpp;

pub(crate) const BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP: u16 = 0x01;
pub(crate) const BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP: u8 = 1;
pub(crate) const BATTLE_PET_SLOT_COUNT_LIKE_CPP: usize = 3;
#[cfg(test)]
pub(crate) const BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP: u32 = 82_800;
#[allow(dead_code)]
pub(crate) const BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP: u8 = 3;
pub(crate) const BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP: u32 = 222;
pub(crate) const DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP: u8 = 3;
pub(crate) const MAX_BATTLE_PET_LEVEL_LIKE_CPP: u16 = 25;

pub(crate) const SKILL_FISHING_LIKE_CPP: u16 = 356;
pub(crate) const SKILL_RIDING_LIKE_CPP: u16 = 762;
pub(crate) const SKILL_ENCHANTING_LIKE_CPP: u16 = 333;
pub const LIQUID_MAP_IN_WATER_LIKE_CPP: u32 = 0x0000_0004;
pub const LIQUID_MAP_UNDER_WATER_LIKE_CPP: u32 = 0x0000_0008;
const TOY_FLAG_FAVORITE_LIKE_CPP: u32 = 0x01;
const TOY_FLAG_HAS_FANFARE_LIKE_CPP: u32 = 0x02;
const DAMAGE_FALL_LIKE_CPP: u8 = 2;
const DAMAGE_FIRE_LIKE_CPP: u8 = 5;
const DAMAGE_FALL_TO_VOID_LIKE_CPP: u8 = 6;
const SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP: i32 = 0x0000_0001;
const CREATURE_MODEL_DATA_FLAG_CAN_MOUNT_WHILE_TRANSFORMED_AS_THIS_LIKE_CPP: u32 = 0x0000_0080;
const CHR_RACES_FLAG_CAN_MOUNT_LIKE_CPP: i32 = 0x0000_0004;
pub(crate) const SPELL_DUEL_LIKE_CPP: u32 = 7266;
pub(crate) const SPELL_MOUNTED_DUEL_LIKE_CPP: u32 = 62875;
#[cfg(test)]
pub(crate) const SPELL_DUEL_BEG_LIKE_CPP: u32 = 7267;
pub(crate) const DUEL_COUNTDOWN_MS_LIKE_CPP: u32 = 3000;
pub type SharedCanonicalMapManager = Arc<Mutex<wow_map::MapManager>>;

// Compatibility paths while #578 moves cast consumers out of the Session adapter.
pub(crate) use wow_entities::PendingSpellCastRequestLikeCpp as RepresentedPendingSpellCastRequestLikeCpp;
pub use wow_entities::{SpellCastBattlePetItemModifiersLikeCpp, SpellCastMetadata, SpellCastState};

const SPELL_FAILED_DONT_REPORT_LIKE_CPP: i32 = 32;

/// Compatibility name while handler modules move to the Player-owned type.
pub use wow_entities::PlayerGossipOptionLikeCpp as GossipOptionInfo;

/// Compatibility name retained while handlers move onto Player-owned inventory
/// commands and queries. The concrete record is owned by `wow_entities::Player`.
pub use wow_entities::PlayerInventoryItem as InventoryItem;

pub(crate) use wow_entities::{
    PlayerEquipmentSetLikeCpp as RepresentedEquipmentSetLikeCpp,
    PlayerEquipmentSetTypeLikeCpp as RepresentedEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp as RepresentedEquipmentSetUpdateStateLikeCpp,
    PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp,
};

const FIRST_LOGIN_START_REPUTATION_STANDING_LIKE_CPP: i32 = 42_999;
const FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP: &[u32] = &[
    942, 935, 936, 1011, 970, 967, 989, 932, 934, 1038, 1077, 1106, 1104, 1090, 1098, 1156, 1073,
    1105, 1119, 1091,
];
const FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP: &[u32] = &[
    72, 47, 69, 930, 730, 978, 54, 946, 1037, 1068, 1126, 1094, 1050,
];
const FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP: &[u32] = &[
    76, 68, 81, 911, 729, 941, 530, 947, 1052, 1067, 1124, 1064, 1085,
];

const WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP: u8 = 80;

pub use wow_entities::AuraApplicationLikeCpp as AuraApplication;

pub use wow_entities::{RepresentedAuraEffectAmountLikeCpp, RepresentedAuraEffectLikeCpp};

const AFLAG_NOCASTER_LIKE_CPP: u32 = 0x0000_0001;
pub(crate) const AFLAG_SCALABLE_LIKE_CPP: u32 = 0x0000_0008;

pub(crate) const SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP: u32 = 0x0000_0800;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_ANIM_LIKE_CPP: u32 = 0x0000_0020;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP: u32 = 0x0000_0008;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP: u32 = 0x0000_0010;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP: u32 =
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP | SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP: u32 = 0x0200_0000;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP: u32 = 0x0000_0020;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP: u32 = 0x0000_4000;
pub(crate) const PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP: u32 = 0x0000_8000;

#[cfg(test)]
#[path = "../session_tests.rs"]
mod tests;
