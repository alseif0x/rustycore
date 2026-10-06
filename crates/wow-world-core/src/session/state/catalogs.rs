// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::catalogs` sub-state (#1241 F2): moved fields, no logic.

use std::collections::HashMap;
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::ObjectMgrCatalogsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::PlayerBootstrapCatalogTestFixtureLikeCpp;
use crate::session::WaypointPathResolverLikeCpp;
use wow_data::character_progression::PowerTypeStore;
use wow_data::progression_rewards::{
    ContentTuningStore, CurvePointStore, CurveStore, FriendshipRepReactionStore,
    NumTalentsAtLevelStore, ParagonReputationStore, ScalingStatDistributionStore,
    ScalingStatValuesStore,
};
use wow_data::reputation::{RepSpilloverTemplateStoreLikeCpp, ReputationRewardRateStoreLikeCpp};
use wow_data::trait_tree::TraitDefinitionStore;
use wow_data::{
    AccessRequirementStoreLikeCpp, AreaTableStore, CinematicSequencesStore,
    CombatRatingsGameTableLikeCpp, ConditionEntriesByTypeStore, CurrencyTypesStore,
    DifficultyStore, DisableMgrLikeCpp, DungeonEncounterStore, DurabilityCostsStore,
    DurabilityQualityStore, FishingBaseSkillStoreLikeCpp, GameObjectTemplateLifecycleStoreLikeCpp,
    GemPropertiesStore, HeirloomStore, LfgDungeonsStore, LockStore, MountCapabilityStore,
    MountDefinitionStoreLikeCpp, MountStore, MountTypeXCapabilityStore, MountXDisplayStore,
    MovieStore, PhaseGroupStore, PhaseStore, PlayerConditionStore, PlayerStatsStore, PvpItemStore,
    RandPropPointsStore, RegenGameTablesLikeCpp, ScriptNameInternerLikeCpp,
    ShieldBlockRegularGameTableLikeCpp, SkillLineStore, SkillStore, SkillTiersStoreLikeCpp,
    TalentStore, ToyStore, TrainerStoreLikeCpp, TransmogSetItemStore, TrinityStringStoreLikeCpp,
    VehicleAccessoryStoreLikeCpp, VehicleSeatStore, VehicleStore, WorldSafeLocStore,
};
use wow_loot::LootStores;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{
    AdventureMapPoiStore, AreaTriggerDb2Store, AreaTriggerScriptStoreLikeCpp, AreaTriggerStore,
    BankBagSlotPricesStore, BattlemasterListStore, CreatureAddonStoreLikeCpp,
    CreatureBaseStatsStoreLikeCpp, CreatureDifficultyStoreLikeCpp, CreatureEquipmentStoreLikeCpp,
    EmotesStore, EmotesTextStore, ExplorationBaseXpStoreLikeCpp, GraveyardStore, ImportPriceStores,
    ItemClassStore, ItemCurrencyCostStore, ItemDisenchantLootStore, ItemPriceBaseStore,
    LfgDungeonStoreLikeCpp, TactKeyStore, TavernAreaTriggerStoreLikeCpp,
    VehicleTemplateStoreLikeCpp,
};

/// Immutable catalog bundles and DB2/world-DB store handles injected at composition; read-only
/// after construction.
pub struct SessionCatalogs {
    /// Detached Player bootstrap-catalog inputs used only by tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_bootstrap_catalog_test_fixture_like_cpp: PlayerBootstrapCatalogTestFixtureLikeCpp,

    // Dispatch table (built once, shared ref)

    // FIFO sender for C++ CharacterDatabase.Execute-style detached homebind
    // writes. Its single worker drains queued jobs after session teardown and
    // preserves call order.

    // C++ ObjectMgr trainer definitions and creature bindings.
    pub trainer_store_like_cpp: Option<Arc<TrainerStoreLikeCpp>>,

    // BankBagSlotPrices.db2 store used by C++ HandleBuyBankSlotOpcode.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub bank_bag_slot_prices_store: Option<Arc<BankBagSlotPricesStore>>,

    // Currency types store (CurrencyTypes.db2 data)
    pub currency_types_store: Option<Arc<CurrencyTypesStore>>,

    // Import price stores (ImportPrice*.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    pub import_price_stores: Option<Arc<ImportPriceStores>>,

    // Emotes.db2 / EmotesText.db2 stores used by C++ chat text-emote handling.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub emotes_store: Option<Arc<EmotesStore>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub emotes_text_store: Option<Arc<EmotesTextStore>>,

    // Item class store (ItemClass.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    pub item_class_store: Option<Arc<ItemClassStore>>,

    // Item currency cost store (ItemCurrencyCost.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    pub item_currency_cost_store: Option<Arc<ItemCurrencyCostStore>>,

    /// Item template and item-data catalogs a session reads. Owned by one type (#670).
    pub items: crate::catalogs::item::ItemCatalogsLikeCpp,

    // Trinity strings loaded from world DB `trinity_string`.
    pub trinity_string_store: Option<Arc<TrinityStringStoreLikeCpp>>,

    // Heirloom store (Heirloom.db2 data)
    pub heirloom_store: Option<Arc<HeirloomStore>>,

    // Toy store (Toy.db2 data)
    pub toy_store: Option<Arc<ToyStore>>,

    // C++ `sCombatRatingsGameTable` used by `Player::GetRatingMultiplier`.
    pub combat_ratings_game_table: Option<Arc<CombatRatingsGameTableLikeCpp>>,

    // C++ `sRegenMPPerSptGameTable` / `sRegenHPPerSptGameTable` /
    // `sOCTRegenHPGameTable` consumed by `Player::Regenerate*`.
    pub regen_game_tables: Option<Arc<RegenGameTablesLikeCpp>>,

    // C++ `sShieldBlockRegularGameTable` used by `ItemTemplate::GetShieldBlockValue`.
    pub shield_block_regular_game_table: Option<Arc<ShieldBlockRegularGameTableLikeCpp>>,

    // C++ `Spell::_executeLogEffects` (`Spell.h:519`, `Spell.cpp:5048-5095`):
    // the current cast's execute-log effects, published once by
    // `Spell::FinishTargetProcessing`.

    // Transmog set item store (TransmogSetItem.db2 data)
    pub transmog_set_item_store: Option<Arc<TransmogSetItemStore>>,

    // Item price base store (ItemPriceBase.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    pub item_price_base_store: Option<Arc<ItemPriceBaseStore>>,

    // Player level stats store (race/class/level → base stats)
    pub player_stats: Option<Arc<PlayerStatsStore>>,

    pub pvp_item_store: Option<Arc<PvpItemStore>>,
    /// Every spell and aura catalog slot, owned by one type (#668).
    pub spell_catalogs: crate::catalogs::spell::SpellCatalogsLikeCpp,
    pub durability_costs_store: Option<Arc<DurabilityCostsStore>>,
    pub durability_quality_store: Option<Arc<DurabilityQualityStore>>,
    pub item_template_addon_quest_log_item_ids_like_cpp: HashMap<u32, u32>,

    // RandPropPoints store (RandPropPoints.db2 data)
    pub rand_prop_points_store: Option<Arc<RandPropPointsStore>>,

    // ItemDisenchantLoot store (ItemDisenchantLoot.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    pub item_disenchant_loot_store: Option<Arc<ItemDisenchantLootStore>>,

    // C++ LootTemplates_* store foundation.
    pub loot_stores: Option<Arc<LootStores>>,

    // C++ ConditionMgr condition store loaded from world.conditions.
    pub condition_store: Option<Arc<ConditionEntriesByTypeStore>>,

    // C++ PlayerCondition.db2 store used by ConditionMgr player-condition checks.
    pub player_condition_store: Option<Arc<PlayerConditionStore>>,

    // C++ AdventureMapPOI.db2 store used by Adventure Map quest starts.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub adventure_map_poi_store: Option<Arc<AdventureMapPoiStore>>,

    // C++ ContentTuning.db2 store used by level gates such as Meeting Stone.
    pub content_tuning_store: Option<Arc<ContentTuningStore>>,
    pub curve_store: Option<Arc<CurveStore>>,
    pub curve_point_store: Option<Arc<CurvePointStore>>,
    pub scaling_stat_distribution_store: Option<Arc<ScalingStatDistributionStore>>,
    pub scaling_stat_values_store: Option<Arc<ScalingStatValuesStore>>,

    // C++ DisableMgr store loaded from world.disables.
    pub disable_mgr: Option<Arc<DisableMgrLikeCpp>>,

    // C++ Difficulty.db2 store used by sDifficultyStore difficulty changes.
    pub difficulty_store: Option<Arc<DifficultyStore>>,

    // Lock store (Lock.db2 data)
    pub lock_store: Option<Arc<LockStore>>,

    pub gem_properties_store: Option<Arc<GemPropertiesStore>>,

    #[cfg(any(test, feature = "test-fixtures"))]
    pub tact_key_store: Option<Arc<TactKeyStore>>,

    // Skill store (auto-learned spells from SkillLineAbility.db2 + SkillRaceClassInfo.db2)
    pub skill_store: Option<Arc<SkillStore>>,

    // TraitDefinition.db2 store used by represented PlayerSpell::TraitDefinitionId cleanup.
    pub trait_definition_store: Option<Arc<TraitDefinitionStore>>,

    // C++ TraitMgr profession tree projection used by trait-config login validation.
    pub trait_tree_skill_line_index:
        Option<Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>>,

    // SkillLine.db2 store for C++ parent/expansion skill resolution.
    pub skill_line_store: Option<Arc<SkillLineStore>>,

    // C++ ObjectMgr::_skillTiers loaded from world.skill_tiers.
    pub skill_tiers_store: Option<Arc<SkillTiersStoreLikeCpp>>,

    // Area table store (area hierarchy + mount flags)
    pub area_table_store: Option<Arc<AreaTableStore>>,

    // C++ ObjectMgr fishing base skill levels loaded from skill_fishing_base_level.
    pub fishing_base_skill_store: Option<Arc<FishingBaseSkillStoreLikeCpp>>,

    // Area-trigger catalogs are process-owned and borrowed for each
    // production session pass. These retained fields are test fixtures only.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub area_trigger_db2_store: Option<Arc<AreaTriggerDb2Store>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub area_trigger_store: Option<Arc<AreaTriggerStore>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub area_trigger_script_store: Option<Arc<AreaTriggerScriptStoreLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub tavern_area_trigger_store: Option<Arc<TavernAreaTriggerStoreLikeCpp>>,

    // C++ ObjectMgr::GraveyardStore loaded from graveyard_zone plus attached conditions.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub graveyard_store: Option<Arc<GraveyardStore>>,

    /// Character race/class catalogs a session reads. Owned by one type (#670).
    pub chr: crate::catalogs::chr::ChrCatalogsLikeCpp,

    /// Map and map-difficulty catalogs a session reads. Owned by one type (#670).
    pub maps: crate::catalogs::map::MapCatalogsLikeCpp,
    /// C++ `sDungeonEncounterStore`, shared immutable catalog used by
    /// `Player::IsLockedToDungeonEncounter` during encounter loot filtering.
    pub dungeon_encounter_store: Option<Arc<DungeonEncounterStore>>,
    pub world_safe_loc_store_like_cpp: Option<Arc<WorldSafeLocStore>>,
    pub access_requirement_store: Option<Arc<AccessRequirementStoreLikeCpp>>,
    pub lfg_dungeons_store: Option<Arc<LfgDungeonsStore>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub lfg_dungeon_store_like_cpp: Option<Arc<LfgDungeonStoreLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub battlemaster_list_store: Option<Arc<BattlemasterListStore>>,
    /// Faction and reputation catalogs a session reads. Owned by one type (#670).
    pub factions: crate::catalogs::faction::FactionCatalogsLikeCpp,
    pub friendship_rep_reaction_store: Option<Arc<FriendshipRepReactionStore>>,
    pub paragon_reputation_store: Option<Arc<ParagonReputationStore>>,
    pub reputation_reward_rate_store: Option<Arc<ReputationRewardRateStoreLikeCpp>>,
    /// Creature template and creature-data catalogs a session reads. Owned by one type (#670).
    pub creatures: crate::catalogs::creature::CreatureCatalogsLikeCpp,
    pub reputation_spillover_template_store: Option<Arc<RepSpilloverTemplateStoreLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub creature_equipment_store_like_cpp: Option<Arc<CreatureEquipmentStoreLikeCpp>>,
    /// GameObject template catalogs a session reads. Owned by one type (#670).
    pub gameobjects: crate::catalogs::gameobject::GameObjectCatalogsLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub creature_addon_store_like_cpp: Option<Arc<CreatureAddonStoreLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub creature_difficulty_store_like_cpp: Option<Arc<CreatureDifficultyStoreLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub creature_base_stats_store_like_cpp: Option<Arc<CreatureBaseStatsStoreLikeCpp>>,
    pub mount_store: Option<Arc<MountStore>>,
    pub mount_definition_store_like_cpp: Option<Arc<MountDefinitionStoreLikeCpp>>,
    pub mount_capability_store: Option<Arc<MountCapabilityStore>>,
    pub mount_type_x_capability_store: Option<Arc<MountTypeXCapabilityStore>>,
    pub mount_x_display_store: Option<Arc<MountXDisplayStore>>,
    pub vehicle_store: Option<Arc<VehicleStore>>,
    pub vehicle_seat_store: Option<Arc<VehicleSeatStore>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub vehicle_template_store: Option<Arc<VehicleTemplateStoreLikeCpp>>,
    pub vehicle_accessory_store: Option<Arc<VehicleAccessoryStoreLikeCpp>>,
    pub terrain_swap_store: Option<Arc<wow_data::TerrainSwapStore>>,
    pub phase_store: Option<Arc<PhaseStore>>,
    pub phase_group_store: Option<Arc<PhaseGroupStore>>,

    pub talent_store: Option<Arc<TalentStore>>,
    pub num_talents_at_level_store: Option<Arc<NumTalentsAtLevelStore>>,
    pub power_type_store: Option<Arc<PowerTypeStore>>,
    pub cinematic_sequences_store: Option<Arc<CinematicSequencesStore>>,
    pub movie_store: Option<Arc<MovieStore>>,
    pub script_name_interner: Option<Arc<ScriptNameInternerLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub object_mgr_catalogs_like_cpp: Option<Arc<ObjectMgrCatalogsLikeCpp>>,
    pub gameobject_template_lifecycle_store_like_cpp:
        Option<Arc<GameObjectTemplateLifecycleStoreLikeCpp>>,

    /// Quest template and quest-rule catalogs, owned by one type (#674).
    pub quests: crate::catalogs::quest::QuestCatalogsLikeCpp,
    /// C++ `ObjectMgr::_questPOIStore`, loaded from `quest_poi` / `quest_poi_points`.
    pub quest_poi_store_like_cpp:
        Option<Arc<HashMap<i32, wow_packet::packets::query::QuestPoiData>>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_xp_table: Option<Arc<Vec<u32>>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub exploration_base_xp_store: Option<Arc<ExplorationBaseXpStoreLikeCpp>>,
    /// C++ `sWaypointMgr->GetPath(pathId)` resolver for session-created legacy `WorldCreature`
    /// compatibility objects. The canonical path store is owned by `world-server`.
    pub waypoint_path_resolver_like_cpp: Option<WaypointPathResolverLikeCpp>,
}

impl Default for SessionCatalogs {
    fn default() -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            player_bootstrap_catalog_test_fixture_like_cpp:
                PlayerBootstrapCatalogTestFixtureLikeCpp::default(),

            trainer_store_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            bank_bag_slot_prices_store: None,
            currency_types_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            import_price_stores: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            emotes_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            emotes_text_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            item_class_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            item_currency_cost_store: None,
            items: crate::catalogs::item::ItemCatalogsLikeCpp::default(),
            trinity_string_store: None,
            heirloom_store: None,
            toy_store: None,
            combat_ratings_game_table: None,
            regen_game_tables: None,
            shield_block_regular_game_table: None,

            transmog_set_item_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            item_price_base_store: None,
            player_stats: None,
            pvp_item_store: None,
            spell_catalogs: crate::catalogs::spell::SpellCatalogsLikeCpp::default(),
            durability_costs_store: None,
            durability_quality_store: None,
            item_template_addon_quest_log_item_ids_like_cpp: HashMap::new(),
            rand_prop_points_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            item_disenchant_loot_store: None,
            loot_stores: None,
            condition_store: None,
            player_condition_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            adventure_map_poi_store: None,
            content_tuning_store: None,
            curve_store: None,
            curve_point_store: None,
            scaling_stat_distribution_store: None,
            scaling_stat_values_store: None,
            disable_mgr: None,
            difficulty_store: None,
            lock_store: None,
            gem_properties_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            tact_key_store: None,
            skill_store: None,
            trait_definition_store: None,
            trait_tree_skill_line_index: None,
            skill_line_store: None,
            skill_tiers_store: None,
            area_table_store: None,
            fishing_base_skill_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            area_trigger_db2_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            area_trigger_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            area_trigger_script_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            tavern_area_trigger_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            graveyard_store: None,
            chr: crate::catalogs::chr::ChrCatalogsLikeCpp::default(),
            maps: crate::catalogs::map::MapCatalogsLikeCpp::default(),
            dungeon_encounter_store: None,
            world_safe_loc_store_like_cpp: None,
            access_requirement_store: None,
            lfg_dungeons_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            lfg_dungeon_store_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            battlemaster_list_store: None,
            factions: crate::catalogs::faction::FactionCatalogsLikeCpp::default(),
            friendship_rep_reaction_store: None,
            paragon_reputation_store: None,
            reputation_reward_rate_store: None,
            creatures: crate::catalogs::creature::CreatureCatalogsLikeCpp::default(),
            reputation_spillover_template_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            creature_equipment_store_like_cpp: None,
            gameobjects: crate::catalogs::gameobject::GameObjectCatalogsLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            creature_addon_store_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            creature_difficulty_store_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            creature_base_stats_store_like_cpp: None,
            mount_store: None,
            mount_definition_store_like_cpp: None,
            mount_capability_store: None,
            mount_type_x_capability_store: None,
            mount_x_display_store: None,
            vehicle_store: None,
            vehicle_seat_store: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            vehicle_template_store: None,
            vehicle_accessory_store: None,
            terrain_swap_store: None,
            phase_store: None,
            phase_group_store: None,

            talent_store: None,
            num_talents_at_level_store: None,
            power_type_store: None,
            cinematic_sequences_store: None,
            movie_store: None,
            script_name_interner: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            object_mgr_catalogs_like_cpp: None,
            gameobject_template_lifecycle_store_like_cpp: None,
            quests: crate::catalogs::quest::QuestCatalogsLikeCpp::default(),
            quest_poi_store_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_xp_table: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            exploration_base_xp_store: None,
            waypoint_path_resolver_like_cpp: None,
        }
    }
}
