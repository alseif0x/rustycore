// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-side aggregate used while constructing world sessions.

use std::sync::Arc;

use wow_network::SocketTimeoutsLikeCpp;
use wow_social::group::{GroupRegistry, PendingInvites};
use wow_world::session::directory::PlayerRegistry;
use wow_world::session::mailbox::GameEventQuestCompleteCommandLikeCpp;
use wow_world::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, ReputationRatesLikeCpp, WorldSession,
};

/// Application-owned resources used to construct a `WorldSession`.
///
/// The outer world-server callback captures this aggregate; the listener
/// neither receives it nor exposes any field through the `wow-network` API.
pub(super) struct SessionResources {
    pub(super) core: SessionCoreCapabilitiesLikeCpp,
    pub(super) inventory: SessionInventoryCapabilitiesLikeCpp,
    pub(super) player: SessionPlayerCatalogCapabilitiesLikeCpp,
    pub(super) spells: SessionSpellCatalogCapabilitiesLikeCpp,
    pub(super) world: SessionWorldCatalogCapabilitiesLikeCpp,
    pub(super) progression: SessionProgressionCapabilitiesLikeCpp,
    pub(super) runtime: SessionRuntimePolicyCapabilitiesLikeCpp,
    pub(super) realm: SessionRealmCapabilitiesLikeCpp,
}

/// Required process capabilities shared by every admitted session.
pub(super) struct SessionCoreCapabilitiesLikeCpp {
    /// Required immutable capabilities borrowed by the outer driver for one
    /// session pass. Production sessions never retain this aggregate.
    pub(super) handler_catalogs: Arc<wow_world::session::SessionHandlerCatalogsLikeCpp>,
    pub(super) gameobject_template_lifecycle_store:
        Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    /// Complete production persistence graph. Its four nested capabilities
    /// encode the C++ owner boundary and replace the optional resource slots.
    pub(super) persistence: wow_world::session::SessionPersistencePortsLikeCpp,
    /// Process-wide C++ trainer/default-trainer snapshot.
    pub(super) trainer_store: Arc<wow_data::TrainerStoreLikeCpp>,
    /// Process-wide C++ `sObjectMgr->GetGenerator<HighGuid::Item>()` mirror.
    /// Shared so concurrent item creation cannot reuse `item_instance.guid`.
    pub(super) item_guid_generator: Arc<wow_core::ObjectGuidGenerator>,
    pub(super) instance_lock_mgr: Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>,
}

/// Immutable item, equipment, collection, battle-pet and loot catalogs.
pub(super) struct SessionInventoryCapabilitiesLikeCpp {
    pub(super) currency_types_store: Arc<wow_data::CurrencyTypesStore>,
    pub(super) import_price_stores: Arc<wow_data::ImportPriceStores>,
    pub(super) item_class_store: Arc<wow_data::ItemClassStore>,
    pub(super) item_currency_cost_store: Arc<wow_data::ItemCurrencyCostStore>,
    pub(super) item_extended_cost_store: Arc<wow_data::ItemExtendedCostStore>,
    pub(super) item_appearance_store: Arc<wow_data::ItemAppearanceStore>,
    pub(super) item_store: Arc<wow_data::ItemStore>,
    pub(super) item_child_equipment_store: Arc<wow_data::ItemChildEquipmentStore>,
    pub(super) item_modified_appearance_store: Arc<wow_data::ItemModifiedAppearanceStore>,
    pub(super) item_search_name_store: Arc<wow_data::ItemSearchNameStore>,
    pub(super) trinity_string_store: Arc<wow_data::TrinityStringStoreLikeCpp>,
    pub(super) heirloom_store: Arc<wow_data::HeirloomStore>,
    pub(super) toy_store: Arc<wow_data::ToyStore>,
    pub(super) battle_pet_breed_quality_store: Arc<wow_data::BattlePetBreedQualityStore>,
    pub(super) battle_pet_breed_state_store: Arc<wow_data::BattlePetBreedStateStore>,
    pub(super) battle_pet_species_store: Arc<wow_data::BattlePetSpeciesStore>,
    /// World-DB battle-pet breed/quality tables for trainer purchase
    /// materialization (issue #161).
    pub(super) battle_pet_selection_store:
        Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    pub(super) battle_pet_species_state_store: Arc<wow_data::BattlePetSpeciesStateStore>,
    pub(super) battle_pet_xp_game_table: Arc<wow_data::BattlePetXpGameTableLikeCpp>,
    pub(super) combat_ratings_game_table: Arc<wow_data::CombatRatingsGameTableLikeCpp>,
    pub(super) shield_block_regular_game_table: Arc<wow_data::ShieldBlockRegularGameTableLikeCpp>,
    pub(super) transmog_set_item_store: Arc<wow_data::TransmogSetItemStore>,
    pub(super) item_price_base_store: Arc<wow_data::ItemPriceBaseStore>,
    pub(super) item_limit_category_store: Arc<wow_data::ItemLimitCategoryStore>,
    pub(super) item_limit_category_condition_store: Arc<wow_data::ItemLimitCategoryConditionStore>,
    pub(super) player_create_info_store: Arc<wow_data::PlayerCreateInfoStoreLikeCpp>,
    pub(super) player_create_cast_spell_store: Arc<wow_data::PlayerCreateInfoCastSpellStoreLikeCpp>,
    pub(super) player_create_custom_spell_store:
        Arc<wow_data::PlayerCreateInfoCustomSpellStoreLikeCpp>,
    pub(super) player_stats: Arc<wow_data::PlayerStatsStore>,
    pub(super) item_bonus_db2_store: Arc<wow_data::ItemBonusDb2Store>,
    pub(super) pvp_item_store: Arc<wow_data::PvpItemStore>,
    pub(super) item_set_store: Arc<wow_data::ItemSetStore>,
    pub(super) item_set_spell_store: Arc<wow_data::ItemSetSpellStore>,
    pub(super) item_stats_store: Arc<wow_data::ItemStatsStore>,
    pub(super) durability_costs_store: Arc<wow_data::DurabilityCostsStore>,
    pub(super) durability_quality_store: Arc<wow_data::DurabilityQualityStore>,
    pub(super) item_effect_store: Arc<wow_data::ItemEffectStore>,
    pub(super) item_random_suffix_store: Arc<wow_data::ItemRandomSuffixStore>,
    pub(super) item_random_properties_store: Arc<wow_data::ItemRandomPropertiesStore>,
    pub(super) rand_prop_points_store: Arc<wow_data::RandPropPointsStore>,
    pub(super) item_random_enchantment_template_store:
        Arc<wow_data::ItemRandomEnchantmentTemplateStore>,
    pub(super) item_spec_override_store: Arc<wow_data::ItemSpecOverrideStore>,
    pub(super) item_disenchant_loot_store: Arc<wow_data::ItemDisenchantLootStore>,
    pub(super) loot_stores: Arc<wow_loot::LootStores>,
}

impl SessionInventoryCapabilitiesLikeCpp {
    /// Installs the complete immutable inventory/catalog capability as one
    /// composition-root operation. Production startup has already validated
    /// that every member is present before the listener is published.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_currency_types_store(Arc::clone(&self.currency_types_store));
        session.set_import_price_stores(Arc::clone(&self.import_price_stores));
        session.set_item_class_store(Arc::clone(&self.item_class_store));
        session.set_item_currency_cost_store(Arc::clone(&self.item_currency_cost_store));
        session.set_item_extended_cost_store(Arc::clone(&self.item_extended_cost_store));
        session.set_item_store(Arc::clone(&self.item_store));
        session.set_item_child_equipment_store(Arc::clone(&self.item_child_equipment_store));
        session.set_item_appearance_store(Arc::clone(&self.item_appearance_store));
        session
            .set_item_modified_appearance_store(Arc::clone(&self.item_modified_appearance_store));
        session.set_item_search_name_store(Arc::clone(&self.item_search_name_store));
        session.set_trinity_string_store(Arc::clone(&self.trinity_string_store));
        session.set_heirloom_store(Arc::clone(&self.heirloom_store));
        session.set_toy_store(Arc::clone(&self.toy_store));
        session
            .set_battle_pet_breed_quality_store(Arc::clone(&self.battle_pet_breed_quality_store));
        session.set_battle_pet_breed_state_store(Arc::clone(&self.battle_pet_breed_state_store));
        session.set_battle_pet_species_store(Arc::clone(&self.battle_pet_species_store));
        session
            .set_battle_pet_selection_store_like_cpp(Arc::clone(&self.battle_pet_selection_store));
        session
            .set_battle_pet_species_state_store(Arc::clone(&self.battle_pet_species_state_store));
        session.set_battle_pet_xp_game_table(Arc::clone(&self.battle_pet_xp_game_table));
        session.set_combat_ratings_game_table(Arc::clone(&self.combat_ratings_game_table));
        session
            .set_shield_block_regular_game_table(Arc::clone(&self.shield_block_regular_game_table));
        session.set_transmog_set_item_store(Arc::clone(&self.transmog_set_item_store));
        session.set_item_price_base_store(Arc::clone(&self.item_price_base_store));
        session.set_item_limit_category_store(Arc::clone(&self.item_limit_category_store));
        session.set_item_limit_category_condition_store(Arc::clone(
            &self.item_limit_category_condition_store,
        ));
        session.set_player_create_info_store_like_cpp(Arc::clone(&self.player_create_info_store));
        session.set_player_create_cast_spell_store_like_cpp(Arc::clone(
            &self.player_create_cast_spell_store,
        ));
        session.set_player_create_custom_spell_store_like_cpp(Arc::clone(
            &self.player_create_custom_spell_store,
        ));
        session.set_player_stats(Arc::clone(&self.player_stats));
        session.set_item_bonus_db2_store(Arc::clone(&self.item_bonus_db2_store));
        session.set_pvp_item_store(Arc::clone(&self.pvp_item_store));
        session.set_item_set_store(Arc::clone(&self.item_set_store));
        session.set_item_set_spell_store(Arc::clone(&self.item_set_spell_store));
        session.set_item_stats_store(Arc::clone(&self.item_stats_store));
        session.set_durability_costs_store(Arc::clone(&self.durability_costs_store));
        session.set_durability_quality_store(Arc::clone(&self.durability_quality_store));
        session.set_item_effect_store(Arc::clone(&self.item_effect_store));
        session.set_item_random_suffix_store(Arc::clone(&self.item_random_suffix_store));
        session.set_item_random_properties_store(Arc::clone(&self.item_random_properties_store));
        session.set_item_spec_override_store(Arc::clone(&self.item_spec_override_store));
        session.set_rand_prop_points_store(Arc::clone(&self.rand_prop_points_store));
        session.set_item_random_enchantment_template_store(Arc::clone(
            &self.item_random_enchantment_template_store,
        ));
        session.set_item_disenchant_loot_store(Arc::clone(&self.item_disenchant_loot_store));
        session.set_loot_stores(Arc::clone(&self.loot_stores));
    }
}

/// Immutable Player creation, condition, stat and skill catalogs.
pub(super) struct SessionPlayerCatalogCapabilitiesLikeCpp {
    pub(super) condition_store: Arc<wow_data::ConditionEntriesByTypeStore>,
    pub(super) player_condition_store: Arc<wow_data::PlayerConditionStore>,
    pub(super) content_tuning_store: Arc<wow_data::progression_rewards::ContentTuningStore>,
    pub(super) curve_store: Arc<wow_data::progression_rewards::CurveStore>,
    pub(super) curve_point_store: Arc<wow_data::progression_rewards::CurvePointStore>,
    pub(super) scaling_stat_distribution_store:
        Arc<wow_data::progression_rewards::ScalingStatDistributionStore>,
    pub(super) scaling_stat_values_store:
        Arc<wow_data::progression_rewards::ScalingStatValuesStore>,
    pub(super) disable_mgr: Arc<wow_data::DisableMgrLikeCpp>,
    pub(super) difficulty_store: Arc<wow_data::DifficultyStore>,
    pub(super) lock_store: Arc<wow_data::LockStore>,
    pub(super) spell_item_enchantment_store: Arc<wow_data::SpellItemEnchantmentStore>,
    pub(super) spell_item_enchantment_condition_store:
        Arc<wow_data::SpellItemEnchantmentConditionStore>,
    pub(super) gem_properties_store: Arc<wow_data::GemPropertiesStore>,
    pub(super) hotfix_blob_cache: Arc<wow_data::HotfixBlobCache>,
    pub(super) skill_store: Arc<wow_data::SkillStore>,
    pub(super) trait_definition_store: Arc<wow_data::trait_tree::TraitDefinitionStore>,
    pub(super) trait_node_entry_store: Arc<wow_data::trait_tree::TraitNodeEntryStore>,
    pub(super) skill_line_store: Arc<wow_data::SkillLineStore>,
    pub(super) skill_tiers_store: Arc<wow_data::SkillTiersStoreLikeCpp>,
    pub(super) talent_store: Arc<wow_data::TalentStore>,
    pub(super) talent_tab_store: Arc<wow_data::TalentTabStore>,
    pub(super) num_talents_at_level_store:
        Arc<wow_data::progression_rewards::NumTalentsAtLevelStore>,
    pub(super) glyph_properties_store: Arc<wow_data::GlyphPropertiesStore>,
    pub(super) chr_races_store: Arc<wow_data::character_progression::ChrRacesStore>,
    pub(super) chr_classes_store: Arc<wow_data::character_progression::ChrClassesStore>,
    pub(super) power_type_store: Arc<wow_data::character_progression::PowerTypeStore>,
}

/// Immutable SpellMgr-style catalogs and audited spell authority.
pub(super) struct SessionSpellCatalogCapabilitiesLikeCpp {
    pub(super) spell_chain_store: Arc<wow_data::SpellChainStoreLikeCpp>,
    pub(super) spell_store: Arc<wow_data::SpellStore>,
    /// Process-wide immutable acquisition projection composed from the
    /// effective spell metadata sources.
    pub(super) spell_acquisition_catalog: Arc<wow_data::SpellAcquisitionCatalogLikeCpp>,
    /// Startup-audited casts/crafts that the immutable acquisition planner may
    /// execute. Missing authority remains fail-closed in `wow-world`.
    pub(super) spell_acquisition_safe_cast_spell_ids: Arc<std::collections::BTreeSet<u32>>,
    pub(super) spell_acquisition_valid_craft_spell_ids: Arc<std::collections::BTreeSet<u32>>,
    /// Effective world-script bindings retained separately from trainer
    /// authority so spell/aura runtimes can prove a candidate has no C++
    /// script hook.
    pub(super) spell_script_exact_spell_ids: Arc<std::collections::BTreeSet<u32>>,
    pub(super) spell_script_all_rank_root_spell_ids: Arc<std::collections::BTreeSet<u32>>,
    pub(super) legacy_spell_script_spell_ids: Arc<std::collections::BTreeSet<u32>>,
    /// Absolute trigger IDs from rejected `spell_linked_spell` rows. The
    /// validated store cannot prove hook absence for these triggers.
    pub(super) spell_linked_rejected_trigger_spell_ids: Arc<std::collections::BTreeSet<u32>>,
    pub(super) spell_levels_store: Arc<wow_data::SpellLevelsStore>,
    pub(super) spell_category_store: Arc<wow_data::SpellCategoryStore>,
    pub(super) npc_spell_click_store: Arc<wow_data::NpcSpellClickStoreLikeCpp>,
    pub(super) spell_aura_options_store: Arc<wow_data::SpellAuraOptionsStore>,
    pub(super) spell_aura_restrictions_store: Arc<wow_data::SpellAuraRestrictionsStore>,
    pub(super) spell_target_restrictions_store: Arc<wow_data::SpellTargetRestrictionsStore>,
    pub(super) spell_equipped_items_store: Arc<wow_data::SpellEquippedItemsStore>,
    pub(super) spell_misc_store: Arc<wow_data::SpellMiscStore>,
    pub(super) spell_group_store: Arc<wow_data::SpellGroupStoreLikeCpp>,
    pub(super) spell_group_stack_rule_store: Arc<wow_data::SpellGroupStackRuleStoreLikeCpp>,
    pub(super) spell_linked_store: Arc<wow_data::SpellLinkedStoreLikeCpp>,
    pub(super) spell_pet_aura_store: Arc<wow_data::SpellPetAuraStoreLikeCpp>,
    pub(super) spell_area_store: Arc<wow_data::SpellAreaStoreLikeCpp>,
    pub(super) spell_custom_attribute_store: Arc<wow_data::SpellCustomAttributeStoreLikeCpp>,
    pub(super) spell_learn_skill_store: Arc<wow_data::SpellLearnSkillStoreLikeCpp>,
    pub(super) spell_learn_spell_store: Arc<wow_data::SpellLearnSpellStoreLikeCpp>,
    pub(super) spell_proc_store: Arc<wow_data::SpellProcStoreLikeCpp>,
    pub(super) spell_required_store: Arc<wow_data::SpellRequiredStoreLikeCpp>,
    pub(super) spell_threat_store: Arc<wow_data::SpellThreatStoreLikeCpp>,
    pub(super) spell_duration_store: Arc<wow_data::SpellDurationStore>,
    pub(super) spell_radius_store: Arc<wow_data::SpellRadiusStore>,
    pub(super) spell_range_store: Arc<wow_data::SpellRangeStore>,
    pub(super) spell_target_position_store: Arc<wow_data::SpellTargetPositionStoreLikeCpp>,
    pub(super) movie_store: Arc<wow_data::MovieStore>,
    pub(super) script_name_interner: Arc<wow_data::ScriptNameInternerLikeCpp>,
}

/// Immutable map, area, creature, mount, vehicle and phase catalogs.
pub(super) struct SessionWorldCatalogCapabilitiesLikeCpp {
    pub(super) area_table_store: Arc<wow_data::AreaTableStore>,
    pub(super) fishing_base_skill_store: Arc<wow_data::FishingBaseSkillStoreLikeCpp>,
    pub(super) area_trigger_template_store: Arc<wow_data::AreaTriggerTemplateStore>,
    pub(super) chr_specialization_store: Arc<wow_data::ChrSpecializationStore>,
    pub(super) dungeon_encounter_store: Arc<wow_data::DungeonEncounterStore>,
    pub(super) map_store: Arc<wow_data::MapStore>,
    pub(super) world_safe_loc_store: Arc<wow_data::WorldSafeLocStore>,
    pub(super) map_difficulty_store: Arc<wow_data::MapDifficultyStore>,
    pub(super) map_difficulty_x_condition_store: Arc<wow_data::MapDifficultyXConditionStore>,
    pub(super) access_requirement_store: Arc<wow_data::AccessRequirementStoreLikeCpp>,
    pub(super) lfg_dungeons_store: Arc<wow_data::LfgDungeonsStore>,
    pub(super) creature_template_lifecycle_store:
        Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    pub(super) creature_template_mount_store: Arc<wow_data::CreatureTemplateMountStoreLikeCpp>,
    pub(super) creature_equipment_store: Arc<wow_data::CreatureEquipmentStoreLikeCpp>,
    pub(super) creature_display_info_store: Arc<wow_data::CreatureDisplayInfoStore>,
    pub(super) creature_display_info_extra_store: Arc<wow_data::CreatureDisplayInfoExtraStore>,
    pub(super) gameobject_display_info_store: Arc<wow_data::GameObjectDisplayInfoStore>,
    pub(super) creature_model_info_store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    pub(super) creature_addon_store: Arc<wow_data::CreatureAddonStoreLikeCpp>,
    pub(super) creature_difficulty_store: Arc<wow_data::CreatureDifficultyStoreLikeCpp>,
    pub(super) creature_base_stats_store: Arc<wow_data::CreatureBaseStatsStoreLikeCpp>,
    pub(super) creature_health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp,
    pub(super) creature_model_data_store: Arc<wow_data::CreatureModelDataStore>,
    pub(super) mount_store: Arc<wow_data::MountStore>,
    pub(super) mount_definition_store: Arc<wow_data::MountDefinitionStoreLikeCpp>,
    pub(super) mount_capability_store: Arc<wow_data::MountCapabilityStore>,
    pub(super) mount_type_x_capability_store: Arc<wow_data::MountTypeXCapabilityStore>,
    pub(super) mount_x_display_store: Arc<wow_data::MountXDisplayStore>,
    pub(super) spell_shapeshift_form_store: Arc<wow_data::SpellShapeshiftFormStore>,
    pub(super) vehicle_store: Arc<wow_data::VehicleStore>,
    pub(super) vehicle_seat_store: Arc<wow_data::VehicleSeatStore>,
    pub(super) vehicle_accessory_store: Arc<wow_data::VehicleAccessoryStoreLikeCpp>,
    pub(super) terrain_swap_store: Arc<wow_data::TerrainSwapStore>,
    pub(super) phase_store: Arc<wow_data::PhaseStore>,
    pub(super) phase_group_store: Arc<wow_data::PhaseGroupStore>,
}

/// Immutable quest/reputation/XP catalogs and progression policy.
pub(super) struct SessionProgressionCapabilitiesLikeCpp {
    pub(super) quest_store: Arc<wow_data::quest::QuestStore>,
    pub(super) quest_xp_store: Arc<wow_data::quest_xp::QuestXpStore>,
    pub(super) quest_money_reward_store: Arc<wow_data::progression_rewards::QuestMoneyRewardStore>,
    pub(super) quest_v2_store: Arc<wow_data::progression_rewards::QuestV2Store>,
    pub(super) quest_info_store: Arc<wow_data::progression_rewards::QuestInfoStore>,
    pub(super) quest_package_item_store: Arc<wow_data::progression_rewards::QuestPackageItemStore>,
    pub(super) quest_faction_reward_store:
        Arc<wow_data::progression_rewards::QuestFactionRewardStore>,
    pub(super) progression_faction_store: Arc<wow_data::progression_rewards::FactionStore>,
    pub(super) faction_template_store: Arc<wow_data::progression_rewards::FactionTemplateStore>,
    pub(super) friendship_rep_reaction_store:
        Arc<wow_data::progression_rewards::FriendshipRepReactionStore>,
    pub(super) paragon_reputation_store: Arc<wow_data::progression_rewards::ParagonReputationStore>,
    pub(super) reputation_reward_rate_store:
        Arc<wow_data::reputation::ReputationRewardRateStoreLikeCpp>,
    pub(super) creature_onkill_reputation_store:
        Arc<wow_data::reputation::CreatureOnKillReputationStoreLikeCpp>,
    pub(super) reputation_spillover_template_store:
        Arc<wow_data::reputation::RepSpilloverTemplateStoreLikeCpp>,
    /// XP required per level: index = level (1-based), value = xp_needed.
    pub(super) player_xp_table: Arc<Vec<u32>>,
    /// C++ `ObjectMgr::_baseXPTable` used by area exploration XP.
    pub(super) exploration_base_xp_store: Arc<wow_data::ExplorationBaseXpStoreLikeCpp>,
    /// C++ `sWorld->getRate(RATE_XP_EXPLORE)`.
    pub(super) exploration_xp_rate: f32,
    /// C++ `CONFIG_MAX_PLAYER_LEVEL`.
    pub(super) max_player_level_config: u32,
    /// C++ `CONFIG_MAX_PRIMARY_TRADE_SKILL`.
    pub(super) max_primary_trade_skills: u8,
    /// C++ PvP/RP-PvP/FFA-PvP `CONFIG_GAME_TYPE` classification.
    pub(super) is_pvp_realm: bool,
    /// C++ `World::IsFFAPvPRealm()` classification.
    pub(super) is_ffa_pvp_realm: bool,
    /// C++ `CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL`.
    pub(super) max_recruit_a_friend_bonus_player_level: u32,
    /// C++ `CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL_DIFFERENCE`.
    pub(super) max_recruit_a_friend_bonus_player_level_difference: u32,
    /// C++ `sWorld->getRate(RATE_REST_OFFLINE_IN_WILDERNESS)`.
    pub(super) rest_offline_wilderness_rate: f32,
    /// C++ `sWorld->getRate(RATE_REST_OFFLINE_IN_TAVERN_OR_CITY)`.
    pub(super) rest_offline_tavern_or_city_rate: f32,
    /// C++ `sWorld->getRate(RATE_REST_INGAME)`.
    pub(super) rest_ingame_rate: f32,
    /// C++ `CONFIG_MIN_QUEST_SCALED_XP_RATIO`.
    pub(super) min_quest_scaled_xp_ratio: u32,
    /// C++ `CONFIG_MIN_DISCOVERED_SCALED_XP_RATIO`.
    pub(super) min_discovered_scaled_xp_ratio: u32,
}

/// Runtime registries, module seams and immutable world/session policy.
pub(super) struct SessionRuntimePolicyCapabilitiesLikeCpp {
    /// Shared registry of all active player sessions (for broadcast).
    pub(super) player_registry: Arc<PlayerRegistry>,
    /// Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    pub(super) game_event_quest_complete_tx: flume::Sender<GameEventQuestCompleteCommandLikeCpp>,
    /// Shared registry of all active groups.
    pub(super) group_registry: Arc<GroupRegistry>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub(super) pending_invites: Arc<PendingInvites>,
    pub(super) loot_drop_rates: LootDropRatesLikeCpp,
    pub(super) reputation_rates: ReputationRatesLikeCpp,
    pub(super) repair_cost_rate: f32,
    /// C++ `CONFIG_RESET_SCHEDULE_{HOUR,WEEK_DAY}` for instance lock expiry.
    pub(super) reset_schedule: wow_instances::ResetSchedule,
    /// C++ `CONFIG_NO_RESET_TALENT_COST` / `NoResetTalentsCost`.
    pub(super) no_reset_talent_cost: bool,
    /// C++ `CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN` / `OffhandCheckAtSpellUnlearn`.
    pub(super) offhand_check_at_spell_unlearn: bool,
    /// C++ `CONFIG_VMAP_INDOOR_CHECK` / `vmap.enableIndoorCheck`.
    pub(super) vmap_indoor_check: bool,
    /// C++ `CONFIG_START_ALL_EXPLORED` / `PlayerStart.MapsExplored`.
    pub(super) start_all_explored: bool,
    /// C++ `CONFIG_START_ALL_REP` / `PlayerStart.AllReputation`.
    pub(super) start_all_reputation: bool,
    /// C++ `CONFIG_START_ALL_SPELLS` / `PlayerStart.AllSpells`.
    pub(super) start_all_spells: bool,
    /// C++ `CONFIG_SUPPORT_ENABLED` / `Support.Enabled`.
    pub(super) support_enabled: bool,
    /// C++ `CONFIG_SUPPORT_TICKETS_ENABLED` / `Support.TicketsEnabled`.
    pub(super) support_tickets_enabled: bool,
    /// C++ `CONFIG_SUPPORT_BUGS_ENABLED` / `Support.BugsEnabled`.
    pub(super) support_bugs_enabled: bool,
    /// C++ `CONFIG_SUPPORT_COMPLAINTS_ENABLED` / `Support.ComplaintsEnabled`.
    pub(super) support_complaints_enabled: bool,
    /// C++ `CONFIG_SUPPORT_SUGGESTIONS_ENABLED` / `Support.SuggestionsEnabled`.
    pub(super) support_suggestions_enabled: bool,
    pub(super) quest_low_level_hide_diff: u32,
    pub(super) quest_high_level_hide_diff: u32,
    pub(super) enable_ae_loot: bool,
    pub(super) addon_channel: bool,
    /// C++ `CONFIG_EXPANSION`; used by map-entry expansion gates.
    pub(super) server_expansion: u8,
    /// C++ `CONFIG_CHARACTERS_PER_REALM` / `CharactersPerRealm`.
    pub(super) characters_per_realm: u32,
    /// C++ `CONFIG_DECLINED_NAMES_USED` / `DeclinedNames`.
    pub(super) declined_names_used: bool,
    /// C++ `CONFIG_FEATURE_SYSTEM_BPAY_STORE_ENABLED`.
    pub(super) feature_system_bpay_store_enabled: bool,
    /// C++ `CONFIG_FEATURE_SYSTEM_CHARACTER_UNDELETE_ENABLED`.
    pub(super) feature_system_character_undelete_enabled: bool,
    /// C++ `CONFIG_INSTANCE_IGNORE_RAID` / `Instance.IgnoreRaid`.
    pub(super) instance_ignore_raid: bool,
    /// C++ `CONFIG_INSTANCE_IGNORE_LEVEL` / `Instance.IgnoreLevel`.
    pub(super) instance_ignore_level: bool,
    /// C++ `CONFIG_MAX_INSTANCES_PER_HOUR` / `AccountInstancesPerHour`.
    pub(super) max_instances_per_hour: u32,
    pub(super) chat_fake_message_preventing: bool,
    pub(super) party_raid_warnings: bool,
    /// C++ `CONFIG_ALLOW_GM_GROUP` / `GM.AllowInvite`.
    pub(super) allow_gm_group: bool,
    /// C++ `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP` / `AllowTwoSide.Interaction.Group`.
    pub(super) allow_two_side_interaction_group: bool,
    /// C++ `CONFIG_PARTY_LEVEL_REQ` / `PartyLevelReq`.
    pub(super) party_level_req: u32,
    pub(super) chat_strict_link_checking_kick: bool,
    pub(super) chat_level_requirements: ChatLevelRequirementsLikeCpp,
    pub(super) chat_listen_ranges: ChatListenRangesLikeCpp,
    pub(super) chat_flood_config: ChatFloodConfigLikeCpp,
    pub(super) packet_spoof_config: PacketSpoofConfigLikeCpp,
    /// C++ `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` in milliseconds.
    pub(super) player_save_interval_ms: u32,
}

/// Immutable identity and address snapshot for the selected realm.
pub(super) struct SessionRealmCapabilitiesLikeCpp {
    pub(super) realm_id: u16,
    /// Region from `realmlist.Region`, used in C++ `RealmHandle::GetAddress()`.
    pub(super) realm_region: u8,
    /// Battlegroup/site from `realmlist.Battlegroup`, used in C++ `RealmHandle::GetAddress()`.
    pub(super) realm_battlegroup: u8,
    /// `(RealmHandle::GetAddress(), Name, NormalizedName)` records from the current realm-list snapshot.
    pub(super) realm_names: Arc<Vec<(u32, String, String)>>,
    /// External (public) IP from `realmlist.address`.
    pub(super) realm_external_address: [u8; 4],
    /// Local (LAN) IP from `realmlist.localAddress`.
    pub(super) realm_local_address: [u8; 4],
}

impl SessionCoreCapabilitiesLikeCpp {
    /// Installs the required process capabilities as one composition-root operation.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_gameobject_template_lifecycle_store(Arc::clone(
            &self.gameobject_template_lifecycle_store,
        ));
        session.set_required_persistence_capabilities_like_cpp(self.persistence.clone());
        session.set_item_guid_generator_like_cpp(Arc::clone(&self.item_guid_generator));
        session.set_instance_lock_mgr(Arc::clone(&self.instance_lock_mgr));
        session.set_trainer_store_like_cpp(Arc::clone(&self.trainer_store));
    }
}

impl SessionPlayerCatalogCapabilitiesLikeCpp {
    /// Installs this complete capability group after bootstrap construction.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_condition_store(Arc::clone(&self.condition_store));
        session.set_player_condition_store(Arc::clone(&self.player_condition_store));
        session.set_content_tuning_store(Arc::clone(&self.content_tuning_store));
        session.set_curve_store(Arc::clone(&self.curve_store));
        session.set_curve_point_store(Arc::clone(&self.curve_point_store));
        session
            .set_scaling_stat_distribution_store(Arc::clone(&self.scaling_stat_distribution_store));
        session.set_scaling_stat_values_store(Arc::clone(&self.scaling_stat_values_store));
        session.set_disable_mgr(Arc::clone(&self.disable_mgr));
        session.set_difficulty_store(Arc::clone(&self.difficulty_store));
        session.set_lock_store(Arc::clone(&self.lock_store));
        session.set_spell_item_enchantment_store(Arc::clone(&self.spell_item_enchantment_store));
        session.set_spell_item_enchantment_condition_store(Arc::clone(
            &self.spell_item_enchantment_condition_store,
        ));
        session.set_gem_properties_store(Arc::clone(&self.gem_properties_store));
        session.set_hotfix_blob_cache(Arc::clone(&self.hotfix_blob_cache));
        session.set_skill_store(Arc::clone(&self.skill_store));
        session.set_trait_definition_store(Arc::clone(&self.trait_definition_store));
        session.set_trait_node_entry_store(Arc::clone(&self.trait_node_entry_store));
        session.set_skill_line_store(Arc::clone(&self.skill_line_store));
        session.set_skill_tiers_store(Arc::clone(&self.skill_tiers_store));
        session.set_talent_store(Arc::clone(&self.talent_store));
        session.set_talent_tab_store(Arc::clone(&self.talent_tab_store));
        session.set_num_talents_at_level_store(Arc::clone(&self.num_talents_at_level_store));
        session.set_glyph_properties_store(Arc::clone(&self.glyph_properties_store));
        session.set_chr_races_store(Arc::clone(&self.chr_races_store));
        session.set_chr_classes_store(Arc::clone(&self.chr_classes_store));
        session.set_power_type_store(Arc::clone(&self.power_type_store));
    }
}

impl SessionSpellCatalogCapabilitiesLikeCpp {
    /// Installs this complete capability group after bootstrap construction.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_spell_store(Arc::clone(&self.spell_store));
        session.set_spell_acquisition_catalog(Arc::clone(&self.spell_acquisition_catalog));
        session.set_spell_acquisition_static_authority_like_cpp(
            self.spell_acquisition_safe_cast_spell_ids.iter().copied(),
            self.spell_acquisition_valid_craft_spell_ids.iter().copied(),
        );
        session.set_spell_runtime_script_authority_like_cpp(
            Arc::clone(&self.spell_script_exact_spell_ids),
            Arc::clone(&self.spell_script_all_rank_root_spell_ids),
            Arc::clone(&self.legacy_spell_script_spell_ids),
            Arc::clone(&self.spell_linked_rejected_trigger_spell_ids),
        );
        session.set_spell_levels_store(Arc::clone(&self.spell_levels_store));
        session.set_spell_chain_store(Arc::clone(&self.spell_chain_store));
        session.set_spell_category_store(Arc::clone(&self.spell_category_store));
        session.set_npc_spell_click_store(Arc::clone(&self.npc_spell_click_store));
        session.set_spell_aura_options_store(Arc::clone(&self.spell_aura_options_store));
        session.set_spell_aura_restrictions_store(Arc::clone(&self.spell_aura_restrictions_store));
        session
            .set_spell_target_restrictions_store(Arc::clone(&self.spell_target_restrictions_store));
        session.set_spell_equipped_items_store(Arc::clone(&self.spell_equipped_items_store));
        session.set_spell_misc_store(Arc::clone(&self.spell_misc_store));
        session.set_spell_group_store(Arc::clone(&self.spell_group_store));
        session.set_spell_group_stack_rule_store(Arc::clone(&self.spell_group_stack_rule_store));
        session.set_spell_linked_store(Arc::clone(&self.spell_linked_store));
        session.set_spell_pet_aura_store(Arc::clone(&self.spell_pet_aura_store));
        session.set_spell_area_store(Arc::clone(&self.spell_area_store));
        session.set_spell_custom_attribute_store(Arc::clone(&self.spell_custom_attribute_store));
        session.set_spell_learn_skill_store(Arc::clone(&self.spell_learn_skill_store));
        session.set_spell_learn_spell_store(Arc::clone(&self.spell_learn_spell_store));
        session.set_spell_proc_store(Arc::clone(&self.spell_proc_store));
        session.set_spell_required_store(Arc::clone(&self.spell_required_store));
        session.set_spell_threat_store(Arc::clone(&self.spell_threat_store));
        session.set_spell_duration_store(Arc::clone(&self.spell_duration_store));
        session.set_spell_radius_store(Arc::clone(&self.spell_radius_store));
        session.set_spell_range_store(Arc::clone(&self.spell_range_store));
        session.set_spell_target_position_store(Arc::clone(&self.spell_target_position_store));
        session.set_movie_store(Arc::clone(&self.movie_store));
        session.set_script_name_interner(Arc::clone(&self.script_name_interner));
    }
}

impl SessionWorldCatalogCapabilitiesLikeCpp {
    /// Installs this complete capability group after bootstrap construction.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_area_table_store(Arc::clone(&self.area_table_store));
        session.set_fishing_base_skill_store(Arc::clone(&self.fishing_base_skill_store));
        session.set_chr_specialization_store(Arc::clone(&self.chr_specialization_store));
        session.set_dungeon_encounter_store(Arc::clone(&self.dungeon_encounter_store));
        session.set_map_store(Arc::clone(&self.map_store));
        session.set_world_safe_loc_store_like_cpp(Arc::clone(&self.world_safe_loc_store));
        session.set_map_difficulty_store(Arc::clone(&self.map_difficulty_store));
        session.set_map_difficulty_x_condition_store(Arc::clone(
            &self.map_difficulty_x_condition_store,
        ));
        session.set_access_requirement_store(Arc::clone(&self.access_requirement_store));
        session.set_lfg_dungeons_store(Arc::clone(&self.lfg_dungeons_store));
        session.set_creature_template_lifecycle_store_like_cpp(Arc::clone(
            &self.creature_template_lifecycle_store,
        ));
        session.set_creature_template_mount_store(Arc::clone(&self.creature_template_mount_store));
        session.set_creature_equipment_store_like_cpp(Arc::clone(&self.creature_equipment_store));
        session.set_creature_display_info_store(Arc::clone(&self.creature_display_info_store));
        session.set_creature_display_info_extra_store(Arc::clone(
            &self.creature_display_info_extra_store,
        ));
        session.set_gameobject_display_info_store(Arc::clone(&self.gameobject_display_info_store));
        session.set_creature_model_info_store(Arc::clone(&self.creature_model_info_store));
        session.set_creature_addon_store_like_cpp(Arc::clone(&self.creature_addon_store));
        session.set_creature_difficulty_store_like_cpp(Arc::clone(&self.creature_difficulty_store));
        session.set_creature_base_stats_store_like_cpp(Arc::clone(&self.creature_base_stats_store));
        session.set_creature_health_rates_like_cpp(self.creature_health_rates);
        session.set_creature_model_data_store(Arc::clone(&self.creature_model_data_store));
        session.set_mount_store(Arc::clone(&self.mount_store));
        session.set_mount_definition_store_like_cpp(Arc::clone(&self.mount_definition_store));
        session.set_mount_capability_store(Arc::clone(&self.mount_capability_store));
        session.set_mount_type_x_capability_store(Arc::clone(&self.mount_type_x_capability_store));
        session.set_mount_x_display_store(Arc::clone(&self.mount_x_display_store));
        session.set_spell_shapeshift_form_store(Arc::clone(&self.spell_shapeshift_form_store));
        session.set_vehicle_store(Arc::clone(&self.vehicle_store));
        session.set_vehicle_seat_store(Arc::clone(&self.vehicle_seat_store));
        session.set_vehicle_accessory_store(Arc::clone(&self.vehicle_accessory_store));
        session.set_terrain_swap_store(Arc::clone(&self.terrain_swap_store));
        session.set_phase_store(Arc::clone(&self.phase_store));
        session.set_phase_group_store(Arc::clone(&self.phase_group_store));
    }
}

impl SessionProgressionCapabilitiesLikeCpp {
    /// Installs this complete capability group after bootstrap construction.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_quest_store(Arc::clone(&self.quest_store));
        session.set_quest_xp_store(Arc::clone(&self.quest_xp_store));
        session.set_quest_money_reward_store(Arc::clone(&self.quest_money_reward_store));
        session.set_quest_v2_store(Arc::clone(&self.quest_v2_store));
        session.set_quest_info_store(Arc::clone(&self.quest_info_store));
        session.set_quest_package_item_store(Arc::clone(&self.quest_package_item_store));
        session.set_quest_faction_reward_store(Arc::clone(&self.quest_faction_reward_store));
        session.set_faction_store(Arc::clone(&self.progression_faction_store));
        session.set_faction_template_store(Arc::clone(&self.faction_template_store));
        session.set_friendship_rep_reaction_store(Arc::clone(&self.friendship_rep_reaction_store));
        session.set_paragon_reputation_store(Arc::clone(&self.paragon_reputation_store));
        session.set_reputation_reward_rate_store(Arc::clone(&self.reputation_reward_rate_store));
        session.set_creature_onkill_reputation_store(Arc::clone(
            &self.creature_onkill_reputation_store,
        ));
        session.set_reputation_spillover_template_store(Arc::clone(
            &self.reputation_spillover_template_store,
        ));
        session.set_player_xp_table(Arc::clone(&self.player_xp_table));
        session.set_exploration_base_xp_store_like_cpp(Arc::clone(&self.exploration_base_xp_store));
        session.set_exploration_xp_rate_like_cpp(self.exploration_xp_rate);
        session.set_rested_xp_config_like_cpp(
            self.max_player_level_config,
            self.rest_offline_wilderness_rate,
            self.rest_offline_tavern_or_city_rate,
            self.rest_ingame_rate,
        );
        session.set_max_primary_trade_skills_like_cpp(self.max_primary_trade_skills);
        session.set_pvp_realm_like_cpp(self.is_pvp_realm);
        session.set_ffa_pvp_realm_like_cpp(self.is_ffa_pvp_realm);
        session.set_recruit_a_friend_xp_config_like_cpp(
            self.max_recruit_a_friend_bonus_player_level,
            self.max_recruit_a_friend_bonus_player_level_difference,
        );
        session.set_min_quest_scaled_xp_ratio_like_cpp(self.min_quest_scaled_xp_ratio);
        session.set_min_discovered_scaled_xp_ratio_like_cpp(self.min_discovered_scaled_xp_ratio);
    }
}

impl SessionRuntimePolicyCapabilitiesLikeCpp {
    /// Installs this complete capability group after bootstrap construction.
    pub(super) fn install_into_session_like_cpp(
        &self,
        session: &mut WorldSession,
        socket_timeouts: SocketTimeoutsLikeCpp,
    ) {
        session.set_quest_low_level_hide_diff_like_cpp(self.quest_low_level_hide_diff);
        session.set_quest_high_level_hide_diff_like_cpp(self.quest_high_level_hide_diff);
        session.set_player_registry(Arc::clone(&self.player_registry));
        session.set_game_event_quest_complete_sender_like_cpp(
            self.game_event_quest_complete_tx.clone(),
        );
        session.set_loot_drop_rates_like_cpp(self.loot_drop_rates);
        session.set_reputation_rates_like_cpp(self.reputation_rates);
        session.set_repair_cost_rate_like_cpp(self.repair_cost_rate);
        session.set_reset_schedule_like_cpp(self.reset_schedule);
        session.set_no_reset_talent_cost_like_cpp(self.no_reset_talent_cost);
        session.set_offhand_check_at_spell_unlearn_like_cpp(self.offhand_check_at_spell_unlearn);
        session.set_vmap_indoor_check_like_cpp(self.vmap_indoor_check);
        session.set_start_all_explored_like_cpp(self.start_all_explored);
        session.set_start_all_reputation_like_cpp(self.start_all_reputation);
        session.set_start_all_spells_like_cpp(self.start_all_spells);
        session.set_represented_support_enabled_like_cpp(self.support_enabled);
        session.set_represented_support_tickets_enabled_like_cpp(self.support_tickets_enabled);
        session.set_represented_support_bugs_enabled_like_cpp(self.support_bugs_enabled);
        session
            .set_represented_support_complaints_enabled_like_cpp(self.support_complaints_enabled);
        session
            .set_represented_support_suggestions_enabled_like_cpp(self.support_suggestions_enabled);
        session.set_enable_ae_loot_like_cpp(self.enable_ae_loot);
        session.set_addon_channel_like_cpp(self.addon_channel);
        session.set_server_expansion_like_cpp(self.server_expansion);
        session.set_characters_per_realm_like_cpp(self.characters_per_realm);
        session.set_declined_names_used_like_cpp(self.declined_names_used);
        session
            .set_feature_system_bpay_store_enabled_like_cpp(self.feature_system_bpay_store_enabled);
        session.set_feature_system_character_undelete_enabled_like_cpp(
            self.feature_system_character_undelete_enabled,
        );
        session.set_instance_ignore_raid_like_cpp(self.instance_ignore_raid);
        session.set_instance_ignore_level_like_cpp(self.instance_ignore_level);
        session.set_max_instances_per_hour_like_cpp(self.max_instances_per_hour);
        session.set_chat_fake_message_preventing_like_cpp(self.chat_fake_message_preventing);
        session.set_party_raid_warnings_like_cpp(self.party_raid_warnings);
        session.set_allow_gm_group_like_cpp(self.allow_gm_group);
        session
            .set_allow_two_side_interaction_group_like_cpp(self.allow_two_side_interaction_group);
        session.set_party_level_req_like_cpp(self.party_level_req);
        session.set_chat_strict_link_checking_kick_like_cpp(self.chat_strict_link_checking_kick);
        session.set_chat_level_requirements_like_cpp(self.chat_level_requirements);
        session.set_chat_listen_ranges_like_cpp(self.chat_listen_ranges);
        session.set_chat_flood_config_like_cpp(self.chat_flood_config);
        session.set_socket_timeouts_like_cpp(socket_timeouts);
        session.set_packet_spoof_config_like_cpp(self.packet_spoof_config);
        session.set_player_save_interval_ms_like_cpp(self.player_save_interval_ms);
        session.set_group_registry(
            Arc::clone(&self.group_registry),
            Arc::clone(&self.pending_invites),
        );
    }
}

impl SessionRealmCapabilitiesLikeCpp {
    /// Installs this complete capability group after bootstrap construction.
    pub(super) fn install_into_session_like_cpp(&self, session: &mut WorldSession) {
        session.set_realm_handle_like_cpp(self.realm_region, self.realm_battlegroup, self.realm_id);
        session.set_realm_names_like_cpp(self.realm_names.iter().cloned());
    }
}
