// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-side aggregate used while constructing world sessions.

use std::sync::Arc;

use wow_social::group::{GroupRegistry, PendingInvites};
use wow_world::session::directory::PlayerRegistry;
use wow_world::session::mailbox::GameEventQuestCompleteCommandLikeCpp;
use wow_world::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, ReputationRatesLikeCpp,
};

/// Application-owned resources used to construct a `WorldSession`.
///
/// The outer world-server callback captures this aggregate; the listener
/// neither receives it nor exposes any field through the `wow-network` API.
pub(super) struct SessionResources {
    /// Required immutable ObjectMgr-style query capability. Construction is
    /// infallible only after every startup catalog loaded successfully.
    pub(super) object_mgr_catalogs: Arc<wow_world::session::ObjectMgrCatalogsLikeCpp>,
    /// Complete production persistence graph. Its four nested capabilities
    /// encode the C++ owner boundary and replace the optional resource slots.
    pub(super) persistence: wow_world::session::SessionPersistencePortsLikeCpp,
    /// Process-wide C++ trainer/default-trainer snapshot.
    pub(super) trainer_store: Option<Arc<wow_data::TrainerStoreLikeCpp>>,
    pub(super) guid_generator: Option<Arc<wow_core::ObjectGuidGenerator>>,
    /// Process-wide C++ `sObjectMgr->GetGenerator<HighGuid::Item>()` mirror.
    /// Shared so concurrent item creation cannot reuse `item_instance.guid`.
    pub(super) item_guid_generator: Option<Arc<wow_core::ObjectGuidGenerator>>,
    /// Shared C++ `_equipmentSetGuid` mirror for sets, outfits and sessions.
    pub(super) equipment_set_guid_generator:
        Option<Arc<wow_core::EquipmentSetGuidGeneratorLikeCpp>>,
    /// Shared C++ `_voidItemId`, independent from `item_instance.guid`.
    pub(super) void_storage_item_id_generator:
        Option<Arc<wow_core::VoidStorageItemIdGeneratorLikeCpp>>,
    pub(super) instance_lock_mgr: Option<Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>>,
    pub(super) bank_bag_slot_prices_store: Option<Arc<wow_data::BankBagSlotPricesStore>>,
    pub(super) currency_types_store: Option<Arc<wow_data::CurrencyTypesStore>>,
    pub(super) import_price_stores: Option<Arc<wow_data::ImportPriceStores>>,
    pub(super) emotes_store: Option<Arc<wow_data::EmotesStore>>,
    pub(super) emotes_text_store: Option<Arc<wow_data::EmotesTextStore>>,
    pub(super) item_class_store: Option<Arc<wow_data::ItemClassStore>>,
    pub(super) item_currency_cost_store: Option<Arc<wow_data::ItemCurrencyCostStore>>,
    pub(super) item_extended_cost_store: Option<Arc<wow_data::ItemExtendedCostStore>>,
    pub(super) item_appearance_store: Option<Arc<wow_data::ItemAppearanceStore>>,
    pub(super) item_store: Option<Arc<wow_data::ItemStore>>,
    pub(super) item_child_equipment_store: Option<Arc<wow_data::ItemChildEquipmentStore>>,
    pub(super) item_modified_appearance_store: Option<Arc<wow_data::ItemModifiedAppearanceStore>>,
    pub(super) item_search_name_store: Option<Arc<wow_data::ItemSearchNameStore>>,
    pub(super) trinity_string_store: Option<Arc<wow_data::TrinityStringStoreLikeCpp>>,
    pub(super) heirloom_store: Option<Arc<wow_data::HeirloomStore>>,
    pub(super) toy_store: Option<Arc<wow_data::ToyStore>>,
    pub(super) battle_pet_breed_quality_store: Option<Arc<wow_data::BattlePetBreedQualityStore>>,
    pub(super) battle_pet_breed_state_store: Option<Arc<wow_data::BattlePetBreedStateStore>>,
    pub(super) battle_pet_species_store: Option<Arc<wow_data::BattlePetSpeciesStore>>,
    /// World-DB battle-pet breed/quality tables for trainer purchase
    /// materialization (issue #161).
    pub(super) battle_pet_selection_store:
        Option<Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>>,
    pub(super) battle_pet_species_state_store: Option<Arc<wow_data::BattlePetSpeciesStateStore>>,
    pub(super) battle_pet_xp_game_table: Option<Arc<wow_data::BattlePetXpGameTableLikeCpp>>,
    pub(super) combat_ratings_game_table: Option<Arc<wow_data::CombatRatingsGameTableLikeCpp>>,
    pub(super) shield_block_regular_game_table:
        Option<Arc<wow_data::ShieldBlockRegularGameTableLikeCpp>>,
    pub(super) transmog_set_item_store: Option<Arc<wow_data::TransmogSetItemStore>>,
    pub(super) item_price_base_store: Option<Arc<wow_data::ItemPriceBaseStore>>,
    pub(super) item_limit_category_store: Option<Arc<wow_data::ItemLimitCategoryStore>>,
    pub(super) item_limit_category_condition_store:
        Option<Arc<wow_data::ItemLimitCategoryConditionStore>>,
    pub(super) player_create_info_store: Option<Arc<wow_data::PlayerCreateInfoStoreLikeCpp>>,
    pub(super) player_create_cast_spell_store:
        Option<Arc<wow_data::PlayerCreateInfoCastSpellStoreLikeCpp>>,
    pub(super) player_create_custom_spell_store:
        Option<Arc<wow_data::PlayerCreateInfoCustomSpellStoreLikeCpp>>,
    pub(super) player_stats: Option<Arc<wow_data::PlayerStatsStore>>,
    pub(super) item_bonus_db2_store: Option<Arc<wow_data::ItemBonusDb2Store>>,
    pub(super) pvp_item_store: Option<Arc<wow_data::PvpItemStore>>,
    pub(super) item_set_store: Option<Arc<wow_data::ItemSetStore>>,
    pub(super) item_set_spell_store: Option<Arc<wow_data::ItemSetSpellStore>>,
    pub(super) item_stats_store: Option<Arc<wow_data::ItemStatsStore>>,
    pub(super) durability_costs_store: Option<Arc<wow_data::DurabilityCostsStore>>,
    pub(super) durability_quality_store: Option<Arc<wow_data::DurabilityQualityStore>>,
    pub(super) item_effect_store: Option<Arc<wow_data::ItemEffectStore>>,
    pub(super) item_random_suffix_store: Option<Arc<wow_data::ItemRandomSuffixStore>>,
    pub(super) item_random_properties_store: Option<Arc<wow_data::ItemRandomPropertiesStore>>,
    pub(super) rand_prop_points_store: Option<Arc<wow_data::RandPropPointsStore>>,
    pub(super) item_random_enchantment_template_store:
        Option<Arc<wow_data::ItemRandomEnchantmentTemplateStore>>,
    pub(super) item_spec_override_store: Option<Arc<wow_data::ItemSpecOverrideStore>>,
    pub(super) item_disenchant_loot_store: Option<Arc<wow_data::ItemDisenchantLootStore>>,
    pub(super) loot_stores: Option<Arc<wow_loot::LootStores>>,
    pub(super) condition_store: Option<Arc<wow_data::ConditionEntriesByTypeStore>>,
    pub(super) player_condition_store: Option<Arc<wow_data::PlayerConditionStore>>,
    pub(super) adventure_map_poi_store: Option<Arc<wow_data::AdventureMapPoiStore>>,
    pub(super) content_tuning_store: Option<Arc<wow_data::progression_rewards::ContentTuningStore>>,
    pub(super) curve_store: Option<Arc<wow_data::progression_rewards::CurveStore>>,
    pub(super) curve_point_store: Option<Arc<wow_data::progression_rewards::CurvePointStore>>,
    pub(super) scaling_stat_distribution_store:
        Option<Arc<wow_data::progression_rewards::ScalingStatDistributionStore>>,
    pub(super) scaling_stat_values_store:
        Option<Arc<wow_data::progression_rewards::ScalingStatValuesStore>>,
    pub(super) progression_faction_store: Option<Arc<wow_data::progression_rewards::FactionStore>>,
    pub(super) faction_template_store:
        Option<Arc<wow_data::progression_rewards::FactionTemplateStore>>,
    pub(super) friendship_rep_reaction_store:
        Option<Arc<wow_data::progression_rewards::FriendshipRepReactionStore>>,
    pub(super) paragon_reputation_store:
        Option<Arc<wow_data::progression_rewards::ParagonReputationStore>>,
    pub(super) disable_mgr: Option<Arc<wow_data::DisableMgrLikeCpp>>,
    pub(super) difficulty_store: Option<Arc<wow_data::DifficultyStore>>,
    pub(super) lock_store: Option<Arc<wow_data::LockStore>>,
    pub(super) spell_item_enchantment_store: Option<Arc<wow_data::SpellItemEnchantmentStore>>,
    pub(super) spell_item_enchantment_condition_store:
        Option<Arc<wow_data::SpellItemEnchantmentConditionStore>>,
    pub(super) gem_properties_store: Option<Arc<wow_data::GemPropertiesStore>>,
    pub(super) spell_enchant_proc_store: Option<Arc<wow_data::SpellEnchantProcStoreLikeCpp>>,
    pub(super) hotfix_blob_cache: Option<Arc<wow_data::HotfixBlobCache>>,
    pub(super) tact_key_store: Option<Arc<wow_data::TactKeyStore>>,
    pub(super) skill_store: Option<Arc<wow_data::SkillStore>>,
    pub(super) trait_definition_store: Option<Arc<wow_data::trait_tree::TraitDefinitionStore>>,
    pub(super) trait_node_entry_store: Option<Arc<wow_data::trait_tree::TraitNodeEntryStore>>,
    pub(super) skill_line_store: Option<Arc<wow_data::SkillLineStore>>,
    pub(super) skill_tiers_store: Option<Arc<wow_data::SkillTiersStoreLikeCpp>>,
    pub(super) talent_store: Option<Arc<wow_data::TalentStore>>,
    pub(super) talent_tab_store: Option<Arc<wow_data::TalentTabStore>>,
    pub(super) num_talents_at_level_store:
        Option<Arc<wow_data::progression_rewards::NumTalentsAtLevelStore>>,
    pub(super) glyph_properties_store: Option<Arc<wow_data::GlyphPropertiesStore>>,
    pub(super) chr_races_store: Option<Arc<wow_data::character_progression::ChrRacesStore>>,
    pub(super) chr_classes_store: Option<Arc<wow_data::character_progression::ChrClassesStore>>,
    pub(super) power_type_store: Option<Arc<wow_data::character_progression::PowerTypeStore>>,
    pub(super) spell_chain_store: Option<Arc<wow_data::SpellChainStoreLikeCpp>>,
    pub(super) spell_store: Option<Arc<wow_data::SpellStore>>,
    /// Process-wide immutable acquisition projection composed from the
    /// effective spell metadata sources.
    pub(super) spell_acquisition_catalog: Option<Arc<wow_data::SpellAcquisitionCatalogLikeCpp>>,
    /// Startup-audited casts/crafts that the immutable acquisition planner may
    /// execute. Missing authority remains fail-closed in `wow-world`.
    pub(super) spell_acquisition_safe_cast_spell_ids: Option<Arc<std::collections::BTreeSet<u32>>>,
    pub(super) spell_acquisition_valid_craft_spell_ids:
        Option<Arc<std::collections::BTreeSet<u32>>>,
    /// Effective world-script bindings retained separately from trainer
    /// authority so spell/aura runtimes can prove a candidate has no C++
    /// script hook.
    pub(super) spell_script_exact_spell_ids: Option<Arc<std::collections::BTreeSet<u32>>>,
    pub(super) spell_script_all_rank_root_spell_ids: Option<Arc<std::collections::BTreeSet<u32>>>,
    pub(super) legacy_spell_script_spell_ids: Option<Arc<std::collections::BTreeSet<u32>>>,
    /// Absolute trigger IDs from rejected `spell_linked_spell` rows. The
    /// validated store cannot prove hook absence for these triggers.
    pub(super) spell_linked_rejected_trigger_spell_ids:
        Option<Arc<std::collections::BTreeSet<u32>>>,
    pub(super) spell_levels_store: Option<Arc<wow_data::SpellLevelsStore>>,
    pub(super) spell_category_store: Option<Arc<wow_data::SpellCategoryStore>>,
    pub(super) npc_spell_click_store: Option<Arc<wow_data::NpcSpellClickStoreLikeCpp>>,
    pub(super) spell_aura_options_store: Option<Arc<wow_data::SpellAuraOptionsStore>>,
    pub(super) spell_aura_restrictions_store: Option<Arc<wow_data::SpellAuraRestrictionsStore>>,
    pub(super) spell_target_restrictions_store: Option<Arc<wow_data::SpellTargetRestrictionsStore>>,
    pub(super) spell_equipped_items_store: Option<Arc<wow_data::SpellEquippedItemsStore>>,
    pub(super) spell_misc_store: Option<Arc<wow_data::SpellMiscStore>>,
    pub(super) spell_group_store: Option<Arc<wow_data::SpellGroupStoreLikeCpp>>,
    pub(super) spell_group_stack_rule_store: Option<Arc<wow_data::SpellGroupStackRuleStoreLikeCpp>>,
    pub(super) spell_linked_store: Option<Arc<wow_data::SpellLinkedStoreLikeCpp>>,
    pub(super) spell_pet_aura_store: Option<Arc<wow_data::SpellPetAuraStoreLikeCpp>>,
    pub(super) spell_area_store: Option<Arc<wow_data::SpellAreaStoreLikeCpp>>,
    pub(super) spell_custom_attribute_store:
        Option<Arc<wow_data::SpellCustomAttributeStoreLikeCpp>>,
    pub(super) serverside_spell_store: Option<Arc<wow_data::ServersideSpellStoreLikeCpp>>,
    pub(super) spell_learn_skill_store: Option<Arc<wow_data::SpellLearnSkillStoreLikeCpp>>,
    pub(super) spell_learn_spell_store: Option<Arc<wow_data::SpellLearnSpellStoreLikeCpp>>,
    pub(super) pet_levelup_spell_store: Option<Arc<wow_data::PetLevelupSpellStoreLikeCpp>>,
    pub(super) pet_default_spell_store: Option<Arc<wow_data::PetDefaultSpellStoreLikeCpp>>,
    pub(super) pet_family_spell_store: Option<Arc<wow_data::PetFamilySpellStoreLikeCpp>>,
    pub(super) spell_proc_store: Option<Arc<wow_data::SpellProcStoreLikeCpp>>,
    pub(super) spell_required_store: Option<Arc<wow_data::SpellRequiredStoreLikeCpp>>,
    pub(super) spell_threat_store: Option<Arc<wow_data::SpellThreatStoreLikeCpp>>,
    pub(super) spell_duration_store: Option<Arc<wow_data::SpellDurationStore>>,
    pub(super) spell_radius_store: Option<Arc<wow_data::SpellRadiusStore>>,
    pub(super) spell_range_store: Option<Arc<wow_data::SpellRangeStore>>,
    pub(super) spell_target_position_store: Option<Arc<wow_data::SpellTargetPositionStoreLikeCpp>>,
    pub(super) spell_totem_model_store: Option<Arc<wow_data::SpellTotemModelStoreLikeCpp>>,
    pub(super) movie_store: Option<Arc<wow_data::MovieStore>>,
    pub(super) script_name_interner: Option<Arc<wow_data::ScriptNameInternerLikeCpp>>,
    pub(super) area_table_store: Option<Arc<wow_data::AreaTableStore>>,
    pub(super) fishing_base_skill_store: Option<Arc<wow_data::FishingBaseSkillStoreLikeCpp>>,
    pub(super) area_trigger_db2_store: Option<Arc<wow_data::AreaTriggerDb2Store>>,
    pub(super) area_trigger_store: Option<Arc<wow_data::AreaTriggerStore>>,
    pub(super) area_trigger_script_store: Option<Arc<wow_data::AreaTriggerScriptStoreLikeCpp>>,
    pub(super) tavern_area_trigger_store: Option<Arc<wow_data::TavernAreaTriggerStoreLikeCpp>>,
    pub(super) graveyard_store: Option<Arc<wow_data::GraveyardStore>>,
    pub(super) area_trigger_template_store: Option<Arc<wow_data::AreaTriggerTemplateStore>>,
    pub(super) chr_specialization_store: Option<Arc<wow_data::ChrSpecializationStore>>,
    pub(super) dungeon_encounter_store: Option<Arc<wow_data::DungeonEncounterStore>>,
    pub(super) map_store: Option<Arc<wow_data::MapStore>>,
    pub(super) world_safe_loc_store: Option<Arc<wow_data::WorldSafeLocStore>>,
    pub(super) map_difficulty_store: Option<Arc<wow_data::MapDifficultyStore>>,
    pub(super) map_difficulty_x_condition_store:
        Option<Arc<wow_data::MapDifficultyXConditionStore>>,
    pub(super) access_requirement_store: Option<Arc<wow_data::AccessRequirementStoreLikeCpp>>,
    pub(super) lfg_dungeons_store: Option<Arc<wow_data::LfgDungeonsStore>>,
    pub(super) lfg_dungeon_store_like_cpp: Option<Arc<wow_data::LfgDungeonStoreLikeCpp>>,
    pub(super) battlemaster_list_store: Option<Arc<wow_data::BattlemasterListStore>>,
    pub(super) creature_template_lifecycle_store:
        Option<Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>>,
    pub(super) creature_template_mount_store:
        Option<Arc<wow_data::CreatureTemplateMountStoreLikeCpp>>,
    pub(super) creature_equipment_store: Option<Arc<wow_data::CreatureEquipmentStoreLikeCpp>>,
    pub(super) creature_display_info_store: Option<Arc<wow_data::CreatureDisplayInfoStore>>,
    pub(super) creature_display_info_extra_store:
        Option<Arc<wow_data::CreatureDisplayInfoExtraStore>>,
    pub(super) gameobject_display_info_store: Option<Arc<wow_data::GameObjectDisplayInfoStore>>,
    pub(super) creature_model_info_store: Option<Arc<wow_data::CreatureModelInfoStoreLikeCpp>>,
    pub(super) creature_addon_store: Option<Arc<wow_data::CreatureAddonStoreLikeCpp>>,
    pub(super) creature_difficulty_store: Option<Arc<wow_data::CreatureDifficultyStoreLikeCpp>>,
    pub(super) creature_base_stats_store: Option<Arc<wow_data::CreatureBaseStatsStoreLikeCpp>>,
    pub(super) creature_health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp,
    pub(super) creature_model_data_store: Option<Arc<wow_data::CreatureModelDataStore>>,
    pub(super) mount_store: Option<Arc<wow_data::MountStore>>,
    pub(super) mount_definition_store: Option<Arc<wow_data::MountDefinitionStoreLikeCpp>>,
    pub(super) mount_capability_store: Option<Arc<wow_data::MountCapabilityStore>>,
    pub(super) mount_type_x_capability_store: Option<Arc<wow_data::MountTypeXCapabilityStore>>,
    pub(super) mount_x_display_store: Option<Arc<wow_data::MountXDisplayStore>>,
    pub(super) spell_shapeshift_form_store: Option<Arc<wow_data::SpellShapeshiftFormStore>>,
    pub(super) vehicle_store: Option<Arc<wow_data::VehicleStore>>,
    pub(super) vehicle_seat_store: Option<Arc<wow_data::VehicleSeatStore>>,
    pub(super) vehicle_template_store: Option<Arc<wow_data::VehicleTemplateStoreLikeCpp>>,
    pub(super) vehicle_accessory_store: Option<Arc<wow_data::VehicleAccessoryStoreLikeCpp>>,
    pub(super) terrain_swap_store: Option<Arc<wow_data::TerrainSwapStore>>,
    pub(super) phase_store: Option<Arc<wow_data::PhaseStore>>,
    pub(super) phase_group_store: Option<Arc<wow_data::PhaseGroupStore>>,
    pub(super) quest_store: Option<Arc<wow_data::quest::QuestStore>>,
    pub(super) quest_xp_store: Option<Arc<wow_data::quest_xp::QuestXpStore>>,
    pub(super) quest_money_reward_store:
        Option<Arc<wow_data::progression_rewards::QuestMoneyRewardStore>>,
    pub(super) quest_v2_store: Option<Arc<wow_data::progression_rewards::QuestV2Store>>,
    pub(super) quest_info_store: Option<Arc<wow_data::progression_rewards::QuestInfoStore>>,
    pub(super) quest_package_item_store:
        Option<Arc<wow_data::progression_rewards::QuestPackageItemStore>>,
    pub(super) quest_faction_reward_store:
        Option<Arc<wow_data::progression_rewards::QuestFactionRewardStore>>,
    pub(super) reputation_reward_rate_store:
        Option<Arc<wow_data::reputation::ReputationRewardRateStoreLikeCpp>>,
    pub(super) creature_onkill_reputation_store:
        Option<Arc<wow_data::reputation::CreatureOnKillReputationStoreLikeCpp>>,
    pub(super) reputation_spillover_template_store:
        Option<Arc<wow_data::reputation::RepSpilloverTemplateStoreLikeCpp>>,
    /// XP required per level: index = level (1-based), value = xp_needed.
    pub(super) player_xp_table: Option<Arc<Vec<u32>>>,
    /// C++ `ObjectMgr::_baseXPTable` used by area exploration XP.
    pub(super) exploration_base_xp_store: Option<Arc<wow_data::ExplorationBaseXpStoreLikeCpp>>,
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
    /// Shared registry of all active player sessions (for broadcast).
    pub(super) player_registry: Option<Arc<PlayerRegistry>>,
    /// Trusted linked modules composed by the generated compositor (#229).
    ///
    /// Absent for the ordinary zero-module build, in which case no session
    /// ever consults a registry.
    pub(super) module_registry: Option<Arc<wow_module_api::ModuleRegistry>>,
    /// Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    pub(super) game_event_quest_complete_tx:
        Option<flume::Sender<GameEventQuestCompleteCommandLikeCpp>>,
    /// Shared registry of all active groups.
    pub(super) group_registry: Option<Arc<GroupRegistry>>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub(super) pending_invites: Option<Arc<PendingInvites>>,
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
