// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player lifecycle and authenticated-session persistence capabilities.
//!
//! This crate owns *what* the Player lifecycle and authenticated session need
//! to persist and *how the result is classified*. It owns no pool, row,
//! transaction, statement or SQL string, and has no dependencies at all — the
//! MariaDB/SQLx adapters live in `wow-database`, which remains the only concrete
//! owner of those.
//!
//! It exists because production uses it: `wow_world::session::lifecycle`
//! publishes offline state, account collections and the semantic character-save
//! snapshot through one port, while `WorldSession` loads and saves its own
//! account state through another. Neither reaches for a database handle. The
//! frozen Player-lifecycle order is documented in
//! `docs/migration/player-lifecycle-persistence-contract.md` (#187).

mod area_trigger_template_catalog;
mod area_trigger_world_catalog;
mod battle_pet_selection_catalog;
mod canonical_spawn_catalog;
mod character_administration;
mod chr_specialization_hotfix;
mod condition_disable_catalog;
mod creature_display_hotfix;
mod difficulty_hotfix;
mod exploration_base_xp_catalog;
mod game_event_world_catalog;
mod game_tele_catalog;
mod gameplay_rule_catalog;
mod gossip_startup_catalog;
mod hotfix_delivery_metadata;
mod instance_lock;
mod item_random_enchantment_catalog;
mod jump_charge_catalog;
mod lfg_dungeons_hotfix;
mod lfg_world_catalog;
mod loot_template_catalog;
mod mount_catalog;
mod phase_hotfix_catalog;
mod phase_world_catalog;
mod player_base_stats;
mod player_choice;
mod player_creation_catalog;
mod player_inventory;
mod player_quest;
mod quest_catalog;
mod quest_item_catalog;
mod reputation_catalog;
mod reserved_name_catalog;
mod skill_catalog_hotfix;
mod skill_world_rules;
mod spell_acquisition_startup;
mod spell_core_db2_hotfix;
mod spell_info_key_hotfix;
mod spell_world_catalog;
mod static_data_overlay;
mod stored_item;
mod trainer_catalog;
mod vehicle_catalog;
mod vendor_catalog;
mod vendor_trade;
mod visibility_spawn_catalog;
mod world_auxiliary_catalog;
mod world_object_catalog;
mod world_query_catalog;
mod world_reference_catalog;

pub use area_trigger_template_catalog::{
    AREA_TRIGGER_SHAPE_DATA_COUNT_LIKE_CPP, AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp,
    AreaTriggerCreatePropertiesPersistenceRowLikeCpp,
    AreaTriggerPolygonVertexPersistenceRowLikeCpp, AreaTriggerSplinePointPersistenceRowLikeCpp,
    AreaTriggerTemplateActionPersistenceRowLikeCpp, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp,
    AreaTriggerTemplateCatalogPersistencePortLikeCpp, AreaTriggerTemplateCatalogRowsLikeCpp,
    AreaTriggerTemplatePersistenceRowLikeCpp,
};
pub use area_trigger_world_catalog::{
    AreaTriggerDestinationPersistenceRowLikeCpp, AreaTriggerScriptPersistenceRowLikeCpp,
    AreaTriggerTeleportPersistenceRowLikeCpp, AreaTriggerWorldCatalogPersistencePortLikeCpp,
    AreaTriggerWorldLoadOutcomeLikeCpp, QuestAreaTriggerPersistenceRowLikeCpp,
    TavernAreaTriggerPersistenceRowLikeCpp,
};
pub use battle_pet_selection_catalog::{
    BattlePetBreedPersistenceRowLikeCpp, BattlePetQualityPersistenceRowLikeCpp,
    BattlePetSelectionCatalogLoadOutcomeLikeCpp, BattlePetSelectionCatalogPersistencePortLikeCpp,
};
pub use canonical_spawn_catalog::{
    AreaTriggerSpawnPersistenceRowLikeCpp, CanonicalSpawnCatalogLoadOutcomeLikeCpp,
    CanonicalSpawnCatalogPersistencePortLikeCpp, CreatureFormationPersistenceRowLikeCpp,
    CreatureSpawnPersistenceRowLikeCpp, GameObjectSpawnPersistenceRowLikeCpp,
    LinkedRespawnPersistenceRowLikeCpp, PoolAutospawnCandidatePersistenceRowLikeCpp,
    PoolMemberKindPersistenceLikeCpp, PoolMemberPersistenceRowLikeCpp,
    PoolTemplatePersistenceRowLikeCpp, SpawnGroupMemberPersistenceRowLikeCpp,
    WaypointPathCatalogLikeCpp, WaypointPathNodePersistenceRowLikeCpp,
    WaypointPathPersistenceRowLikeCpp, WorldStateSavedValuePersistenceRowLikeCpp,
    WorldStateStartupCatalogLikeCpp, WorldStateStartupLoadOutcomeLikeCpp,
    WorldStateStartupPersistencePortLikeCpp, WorldStateTemplatePersistenceRowLikeCpp,
};
pub use character_administration::{
    CharacterAdministrationLoadOutcomeLikeCpp, CharacterAdministrationMutationOutcomeLikeCpp,
    CharacterAdministrationPersistencePortLikeCpp, CharacterCreatePersistenceRequestLikeCpp,
    CharacterCustomizationPersistenceLikeCpp, CharacterCustomizeCandidateLikeCpp,
    CharacterRenameCandidateLikeCpp,
};
pub use chr_specialization_hotfix::{
    ChrSpecializationHotfixLoadOutcomeLikeCpp, ChrSpecializationHotfixPersistencePortLikeCpp,
    ChrSpecializationHotfixRowLikeCpp, ChrSpecializationHotfixRowsLikeCpp,
};
pub use condition_disable_catalog::{
    ConditionDisableCatalogPersistencePortLikeCpp, ConditionDisableRowsLoadOutcomeLikeCpp,
    ConditionPersistenceRowLikeCpp, DisablePersistenceRowLikeCpp,
};
pub use creature_display_hotfix::{
    CreatureDisplayHotfixLoadOutcomeLikeCpp, CreatureDisplayHotfixPersistencePortLikeCpp,
    CreatureDisplayInfoHotfixRowLikeCpp, CreatureModelDataHotfixRowLikeCpp,
};
pub use difficulty_hotfix::{
    DifficultyHotfixLoadOutcomeLikeCpp, DifficultyHotfixPersistencePortLikeCpp,
    DifficultyHotfixRowLikeCpp, DifficultyHotfixRowsLikeCpp,
};
pub use exploration_base_xp_catalog::{
    ExplorationBaseXpCatalogLoadOutcomeLikeCpp, ExplorationBaseXpCatalogPersistencePortLikeCpp,
    ExplorationBaseXpPersistenceRowLikeCpp,
};
pub use game_event_world_catalog::{
    CreatureEquipmentIdPersistenceRowLikeCpp, GameEventConditionPersistenceRowLikeCpp,
    GameEventDataPersistenceRowLikeCpp, GameEventModelEquipPersistenceRowLikeCpp,
    GameEventNpcFlagPersistenceRowLikeCpp, GameEventNpcVendorPersistenceRowLikeCpp,
    GameEventObjectGuidPersistenceRowLikeCpp, GameEventPoolPersistenceRowLikeCpp,
    GameEventPrerequisitePersistenceRowLikeCpp, GameEventQuestConditionPersistenceRowLikeCpp,
    GameEventQuestRelationPersistenceRowLikeCpp, GameEventWorldCatalogLoadOutcomeLikeCpp,
    GameEventWorldCatalogPersistencePortLikeCpp, GameEventWorldCatalogPrefixLikeCpp,
    GameEventWorldCatalogSuffixLikeCpp,
};
pub use game_tele_catalog::{
    GameTeleCatalogLoadOutcomeLikeCpp, GameTeleCatalogPersistencePortLikeCpp,
    GameTelePersistenceRowLikeCpp,
};
pub use gameplay_rule_catalog::{
    FactionChangePairPersistenceRowLikeCpp, FactionChangePersistenceRowsLikeCpp,
    GameplayRuleCatalogPersistencePortLikeCpp, GameplayRuleRowsLoadOutcomeLikeCpp,
    NpcSpellClickPersistenceRowLikeCpp, NpcVendorPersistenceRowLikeCpp,
};
pub use gossip_startup_catalog::{
    GossipMenuAddonPersistenceRowLikeCpp, GossipMenuOptionLocalePersistenceRowLikeCpp,
    GossipMenuPersistenceRowLikeCpp, GossipStartupCatalogLoadOutcomeLikeCpp,
    GossipStartupCatalogPersistencePortLikeCpp,
};
pub use hotfix_delivery_metadata::{
    HotfixBlobPersistenceRowLikeCpp, HotfixDataPersistenceRowLikeCpp,
    HotfixDeliveryMetadataLoadOutcomeLikeCpp, HotfixDeliveryMetadataPersistencePortLikeCpp,
    HotfixOptionalDataPersistenceRowLikeCpp,
};
pub use instance_lock::{
    CharacterInstanceLockPersistenceRowLikeCpp, InstanceLockPersistenceLoadOutcomeLikeCpp,
    InstanceLockPersistenceMutationLikeCpp, InstanceLockPersistenceOutcomeLikeCpp,
    InstanceLockPersistencePlanLikeCpp, InstanceLockPersistencePortLikeCpp,
    SharedInstanceLockPersistenceRowLikeCpp,
};
pub use item_random_enchantment_catalog::{
    ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp,
    ItemRandomEnchantmentCatalogPersistencePortLikeCpp, ItemRandomEnchantmentPersistenceRowLikeCpp,
};
pub use jump_charge_catalog::{
    JumpChargeCatalogLoadOutcomeLikeCpp, JumpChargeCatalogPersistencePortLikeCpp,
    JumpChargeParamsPersistenceRowLikeCpp,
};
pub use lfg_dungeons_hotfix::{
    LfgDungeonsHotfixLoadOutcomeLikeCpp, LfgDungeonsHotfixPersistencePortLikeCpp,
    LfgDungeonsHotfixRowLikeCpp,
};
pub use lfg_world_catalog::{
    LfgDungeonRewardPersistenceRowLikeCpp, LfgDungeonTemplatePersistenceRowLikeCpp,
    LfgWorldCatalogLoadOutcomeLikeCpp, LfgWorldCatalogPersistencePortLikeCpp,
};
pub use loot_template_catalog::{
    LootConditionPersistenceRowLikeCpp, LootTemplateCatalogOutcomeLikeCpp,
    LootTemplateCatalogPersistencePortLikeCpp, LootTemplatePersistenceRowLikeCpp,
    LootTemplateTablePersistenceLikeCpp,
};
pub use mount_catalog::{
    MountCapabilityHotfixRowLikeCpp, MountCatalogLoadOutcomeLikeCpp,
    MountCatalogPersistencePortLikeCpp, MountDefinitionRowLikeCpp, MountHotfixRowLikeCpp,
    MountTypeXCapabilityHotfixRowLikeCpp, MountXDisplayHotfixRowLikeCpp,
};
pub use phase_hotfix_catalog::{
    PhaseGroupHotfixRowLikeCpp, PhaseHotfixLoadOutcomeLikeCpp, PhaseHotfixPersistencePortLikeCpp,
    PhaseHotfixRowLikeCpp,
};
pub use phase_world_catalog::{
    PhaseAreaPersistenceRowLikeCpp, PhaseNamePersistenceRowLikeCpp,
    PhaseWorldCatalogLoadOutcomeLikeCpp, PhaseWorldCatalogPersistencePortLikeCpp,
    TerrainSwapDefaultPersistenceRowLikeCpp, TerrainWorldMapPersistenceRowLikeCpp,
};
pub use player_base_stats::{
    PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP, PlayerBaseStatsLoadOutcomeLikeCpp,
    PlayerBaseStatsPersistencePortLikeCpp, PlayerClassLevelStatsPersistenceRowLikeCpp,
    PlayerRaceStatsPersistenceRowLikeCpp,
};
pub use player_choice::{
    PlayerChoiceCatalogCoreRowsLikeCpp, PlayerChoiceCatalogLoadOutcomeLikeCpp,
    PlayerChoiceCatalogLocaleRowsLikeCpp, PlayerChoiceCatalogPersistencePortLikeCpp,
    PlayerChoiceLocaleRowLikeCpp, PlayerChoiceResponseLocaleRowLikeCpp,
    PlayerChoiceResponseMawPowerRowLikeCpp, PlayerChoiceResponseRewardCurrencyRowLikeCpp,
    PlayerChoiceResponseRewardFactionRowLikeCpp, PlayerChoiceResponseRewardItemRowLikeCpp,
    PlayerChoiceResponseRewardRowLikeCpp, PlayerChoiceResponseRowLikeCpp, PlayerChoiceRowLikeCpp,
};
pub use player_creation_catalog::{
    PlayerCreateCastSpellPersistenceRowLikeCpp, PlayerCreateCustomSpellPersistenceRowLikeCpp,
    PlayerCreateInfoPersistenceRowLikeCpp, PlayerCreationCatalogLoadOutcomeLikeCpp,
    PlayerCreationCatalogPersistencePortLikeCpp,
};
pub use player_inventory::{
    InventoryDestroyNodePersistenceLikeCpp, InventoryEquipPersistenceLikeCpp,
    InventoryGraphDestroyPersistenceLikeCpp, InventoryItemMutablePersistenceLikeCpp,
    InventoryLinkPersistenceLikeCpp, InventoryPartialDestroyPersistenceLikeCpp,
    InventoryStackMergePersistenceLikeCpp, InventoryStackMergeSourcePersistenceLikeCpp,
    InventoryStorageMovePersistenceLikeCpp, InventorySwapPersistenceLikeCpp,
    LootDirectItemGrantPersistenceLikeCpp, LootDisenchantBatchPersistenceLikeCpp,
    LootExistingStackPersistenceLikeCpp, LootNewStackPersistenceLikeCpp,
    LootQuestBoundProgressPersistenceLikeCpp, PlayerInventoryPersistencePortLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp, QuestItemExistingStackPersistenceLikeCpp,
    QuestItemGrantPersistenceLikeCpp, QuestItemNewStackPersistenceLikeCpp,
    QuestTurnInItemPersistenceLikeCpp, QuestTurnInPersistenceLikeCpp,
    StoredItemLootSourcePersistenceLikeCpp,
};
pub use player_quest::{
    PlayerQuestActivePersistenceRowLikeCpp, PlayerQuestDailyPersistenceRowLikeCpp,
    PlayerQuestIdPersistenceRowLikeCpp, PlayerQuestLoadOutcomeLikeCpp,
    PlayerQuestLockoutPersistenceRequestLikeCpp, PlayerQuestObjectivePersistenceRowLikeCpp,
    PlayerQuestPersistencePortLikeCpp, PlayerQuestSeasonalCompletionPersistenceLikeCpp,
    PlayerQuestSeasonalPersistenceRowLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
    QuestObjectiveCountPersistenceLikeCpp, QuestStatusPersistenceLikeCpp,
};
pub use quest_catalog::*;
pub use quest_item_catalog::{
    CreatureQuestItemPersistenceRowLikeCpp, GameObjectQuestItemPersistenceRowLikeCpp,
    QuestItemCatalogLoadOutcomeLikeCpp, QuestItemCatalogPersistencePortLikeCpp,
};
pub use reputation_catalog::{
    CreatureOnKillReputationPersistenceRowLikeCpp, REPUTATION_SPILLOVER_SLOT_COUNT_LIKE_CPP,
    ReputationCatalogLoadOutcomeLikeCpp, ReputationCatalogPersistencePortLikeCpp,
    ReputationRewardRatePersistenceRowLikeCpp, ReputationSpilloverTemplatePersistenceRowLikeCpp,
};
pub use reserved_name_catalog::{
    ReservedNameCatalogLoadOutcomeLikeCpp, ReservedNameCatalogPersistencePortLikeCpp,
    ReservedNamePersistenceRowLikeCpp,
};
pub use skill_catalog_hotfix::{
    SkillCatalogHotfixLoadOutcomeLikeCpp, SkillCatalogHotfixPersistencePortLikeCpp,
    SkillLineAbilityHotfixRowLikeCpp, SkillLineHotfixRowLikeCpp, SkillLineHotfixRowsLikeCpp,
    SkillRaceClassInfoHotfixRowLikeCpp, SkillRelationHotfixRowsLikeCpp,
};
pub use skill_world_rules::{
    FishingBaseSkillPersistenceRowLikeCpp, SKILL_TIER_VALUE_COUNT_LIKE_CPP,
    SkillTierPersistenceRowLikeCpp, SkillWorldRulesLoadOutcomeLikeCpp,
    SkillWorldRulesPersistencePortLikeCpp,
};
pub use spell_acquisition_startup::{
    BattlePetSpeciesHotfixPersistenceRowLikeCpp, ServersideSpellEffectPersistenceRowLikeCpp,
    ServersideSpellPersistenceRowLikeCpp, SpellAcquisitionHotfixPersistenceRowLikeCpp,
    SpellAcquisitionHotfixTablePersistenceLikeCpp, SpellAcquisitionStartupLoadOutcomeLikeCpp,
    SpellAcquisitionStartupPersistencePortLikeCpp, SpellCustomAttributePersistenceRowLikeCpp,
    SpellEffectHotfixPersistenceRowLikeCpp, SpellLearnSpellHotfixPersistenceRowLikeCpp,
    SpellLearnSpellWorldPersistenceRowLikeCpp, SpellLevelsHotfixPersistenceRowLikeCpp,
    SpellMiscHotfixPersistenceRowLikeCpp, SpellReagentsPersistenceRowLikeCpp,
    SummonPropertiesHotfixPersistenceRowLikeCpp, TalentHotfixPersistenceRowLikeCpp,
    TrainerSpellAuditPersistenceCatalogLikeCpp,
};
pub use spell_core_db2_hotfix::{
    SpellAuraRestrictionsHotfixRowLikeCpp, SpellCastTimesHotfixRowLikeCpp,
    SpellCastingRequirementsHotfixRowLikeCpp, SpellCategoriesHotfixRowLikeCpp,
    SpellCategoryHotfixRowLikeCpp, SpellCooldownsHotfixRowLikeCpp,
    SpellCoreDb2HotfixLoadOutcomeLikeCpp, SpellCoreDb2HotfixPersistencePortLikeCpp,
    SpellDurationHotfixRowLikeCpp, SpellEffectHotfixRowLikeCpp, SpellEquippedItemsHotfixRowLikeCpp,
    SpellInterruptsHotfixRowLikeCpp, SpellMiscHotfixRowLikeCpp, SpellNameHotfixRowLikeCpp,
    SpellPowerDifficultyHotfixRowLikeCpp, SpellPowerHotfixRowLikeCpp, SpellRadiusHotfixRowLikeCpp,
    SpellRangeHotfixRowLikeCpp, SpellShapeshiftHotfixRowLikeCpp,
    SpellTargetRestrictionsHotfixRowLikeCpp, SpellXSpellVisualHotfixRowLikeCpp,
};
pub use spell_info_key_hotfix::{
    SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP, SpellInfoKeyContributorHotfixBatchLikeCpp,
    SpellInfoKeyContributorHotfixRowLikeCpp, SpellInfoKeyContributorLikeCpp,
    SpellInfoKeyHotfixLoadOutcomeLikeCpp, SpellInfoKeyHotfixPersistencePortLikeCpp,
    SpellInfoKeyHotfixRowsLikeCpp, SpellInfoPowerDifficultyHotfixRowLikeCpp,
};
pub use spell_world_catalog::{
    SpellAreaPersistenceRowLikeCpp, SpellGroupPersistenceRowLikeCpp,
    SpellGroupStackRulePersistenceRowLikeCpp, SpellLinkedPersistenceRowLikeCpp,
    SpellPetAuraPersistenceRowLikeCpp, SpellProcPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellTargetPositionPersistenceRowLikeCpp,
    SpellThreatPersistenceRowLikeCpp, SpellTotemModelPersistenceRowLikeCpp,
    SpellWorldCatalogLoadOutcomeLikeCpp, SpellWorldCatalogPersistencePortLikeCpp,
};
pub use static_data_overlay::{
    AreaTableHotfixRowLikeCpp, PowerTypeHotfixRowLikeCpp, SpellEnchantProcPersistenceRowLikeCpp,
    StaticDataOverlayPersistencePortLikeCpp, StaticDataRowsLoadOutcomeLikeCpp,
    UiMapXMapArtHotfixRowLikeCpp,
};
pub use stored_item::{
    InventoryItemCountPersistenceRequestLikeCpp, InventoryItemDestroyPersistenceRequestLikeCpp,
    StoredItemLoadOutcomeLikeCpp, StoredItemLootPersistenceRowLikeCpp,
    StoredItemLootSaveRequestLikeCpp, StoredItemPersistencePortLikeCpp,
    WrappedGiftOpenPersistenceRequestLikeCpp, WrappedGiftPersistenceRowLikeCpp,
};
pub use trainer_catalog::{
    CreatureTrainerPersistenceRowLikeCpp, TrainerCatalogLoadOutcomeLikeCpp,
    TrainerCatalogPersistencePortLikeCpp, TrainerCatalogPersistenceRowsLikeCpp,
    TrainerLocalePersistenceRowLikeCpp, TrainerPersistenceRowLikeCpp,
    TrainerSpellPersistenceRowLikeCpp,
};
pub use vehicle_catalog::{
    VEHICLE_SEAT_COUNT_LIKE_CPP, VehicleHotfixLoadOutcomeLikeCpp,
    VehicleHotfixPersistencePortLikeCpp, VehicleHotfixPersistenceRowLikeCpp,
    VehicleSeatHotfixPersistenceRowLikeCpp, VehicleSpawnAccessoryPersistenceRowLikeCpp,
    VehicleTemplateAccessoryPersistenceRowLikeCpp, VehicleTemplatePersistenceRowLikeCpp,
    VehicleWorldCatalogLoadOutcomeLikeCpp, VehicleWorldCatalogPersistencePortLikeCpp,
};
pub use vendor_catalog::{
    VendorCatalogOutcomeLikeCpp, VendorCatalogPersistencePortLikeCpp, VendorCatalogRowLikeCpp,
};
pub use vendor_trade::*;
pub use visibility_spawn_catalog::{
    CreatureVisibilityPersistenceRowLikeCpp, GameObjectVisibilityPersistenceRowLikeCpp,
    VisibilitySpawnCatalogOutcomeLikeCpp, VisibilitySpawnCatalogPersistencePortLikeCpp,
    VisibilitySpawnCatalogRequestLikeCpp,
};
pub use world_auxiliary_catalog::{
    AccessRequirementPersistenceRowLikeCpp, GraveyardZonePersistenceRowLikeCpp,
    SceneTemplatePersistenceRowLikeCpp, SpawnGroupTemplatePersistenceRowLikeCpp,
    TrinityStringPersistenceRowLikeCpp, WorldAuxiliaryCatalogPersistencePortLikeCpp,
    WorldAuxiliaryRowsLoadOutcomeLikeCpp,
};
pub use world_object_catalog::*;
pub use world_query_catalog::*;
pub use world_reference_catalog::{
    WorldObjectIdCatalogKindLikeCpp, WorldReferenceCatalogPersistencePortLikeCpp,
    WorldReferenceRowsLoadOutcomeLikeCpp, WorldSafeLocPersistenceRowLikeCpp,
    WorldSpawnCatalogKindLikeCpp,
};

mod outcome;
pub use outcome::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

mod world_runtime;
pub use world_runtime::{
    GameEventConditionSaveLoadOutcomeLikeCpp, GameEventConditionSavePersistenceRowLikeCpp,
    GameEventPersistenceMutationLikeCpp, GameEventPersistenceMutationOutcomeLikeCpp,
    GameEventPersistencePortLikeCpp, MapCorpseAuxiliaryLoadOutcomeLikeCpp,
    MapCorpseCustomizationLoadRowLikeCpp, MapCorpseLoadOutcomeLikeCpp, MapCorpseLoadRequestLikeCpp,
    MapCorpseLoadRowLikeCpp, MapCorpsePersistencePortLikeCpp, MapCorpsePhaseLoadRowLikeCpp,
    RespawnPersistenceKeyLikeCpp, RespawnPersistenceLoadOutcomeLikeCpp,
    RespawnPersistenceMutationLikeCpp, RespawnPersistenceMutationOutcomeLikeCpp,
    RespawnPersistencePortLikeCpp, RespawnPersistenceRowLikeCpp,
};

mod battle_pet;
pub use battle_pet::{
    BATTLE_PET_GUID_COUNTER_LIMIT_LIKE_CPP, BattlePetAccountPersistencePortLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetDeclinedNamesLikeCpp, BattlePetPersistenceErrorLikeCpp,
    BattlePetProcessLeaseLikeCpp, BattlePetPurchaseChargeOutcomeLikeCpp,
    BattlePetPurchaseCommandLikeCpp, BattlePetPurchaseCommitFenceLikeCpp,
    BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseMarkOutcomeLikeCpp,
    BattlePetPurchasePersistencePortLikeCpp, BattlePetPurchaseStatusLikeCpp,
    BattlePetPurchaseStoreErrorLikeCpp, DurableBattlePetAddLikeCpp,
    DurableBattlePetAddReceiptLikeCpp, DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp,
    LoadedBattlePetAccountLikeCpp, PersistBattlePetAddOutcomeLikeCpp,
    reconcile_battle_pet_purchase_charge_like_cpp, reconcile_battle_pet_purchase_mark_like_cpp,
};

mod player_lifecycle;
pub use player_lifecycle::{
    PlayerHomebindPersistenceRequestLikeCpp, PlayerLifecyclePortLikeCpp, PlayerOfflineMarkLikeCpp,
};

mod session_administration;
pub use session_administration::{
    PacketSpoofAffectedAccountsLoadOutcomeLikeCpp, PacketSpoofBanPersistencePortLikeCpp,
    PacketSpoofBanTargetLikeCpp, PacketSpoofBanWriteRequestLikeCpp,
    SupportBugReportPersistencePortLikeCpp, SupportBugReportWriteRequestLikeCpp,
};

mod item_template_addon_catalog;
pub use item_template_addon_catalog::{
    ItemTemplateAddonCatalogPersistencePortLikeCpp, ItemTemplateAddonCatalogRequestLikeCpp,
    ItemTemplateAddonLootMetadataOutcomeLikeCpp, ItemTemplateAddonLootMetadataRowLikeCpp,
    ItemTemplateAddonMoneyOutcomeLikeCpp, ItemTemplateAddonMoneyRowLikeCpp,
};

mod gossip_query;
pub use gossip_query::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogPersistencePortLikeCpp,
    GossipCatalogReadOutcomeLikeCpp, GossipCreatureMenuRequestLikeCpp,
    GossipMenuCatalogRequestLikeCpp, GossipMenuOptionCatalogRowLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp,
};

mod player_name_query;
pub use player_name_query::{
    PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp,
};

mod player_economy;
pub use player_economy::{
    PlayerBankSlotPurchaseRequestLikeCpp, PlayerCurrencySaveKindLikeCpp,
    PlayerCurrencySaveRequestLikeCpp, PlayerCurrencySaveRowLikeCpp,
    PlayerDurabilityRepairSaveLikeCpp, PlayerMoneyTransactionOutcomeLikeCpp,
    PlayerMoneyTransactionRequestLikeCpp, PlayerMoneyWriteRequestLikeCpp,
    PlayerUncageItemStateLikeCpp, PlayerUncageItemStateLoadOutcomeLikeCpp,
    PlayerUncageItemStateRequestLikeCpp,
};

mod void_storage;
pub use void_storage::{
    VoidStorageDepositWriteLikeCpp, VoidStorageDestroyedItemWriteLikeCpp,
    VoidStorageItemWriteLikeCpp, VoidStorageMergedInventoryItemWriteLikeCpp,
    VoidStorageNewInventoryItemWriteLikeCpp, VoidStoragePersistencePortLikeCpp,
    VoidStorageQuestObjectiveWriteLikeCpp, VoidStorageQuestStatusWriteLikeCpp,
    VoidStorageSwapWriteRequestLikeCpp, VoidStorageTransferWriteRequestLikeCpp,
    VoidStorageUnlockWriteRequestLikeCpp, VoidStorageWithdrawalInventoryWriteLikeCpp,
    VoidStorageWithdrawalWriteLikeCpp,
};

mod social;
pub use social::{
    SocialAddCandidateLikeCpp, SocialAddCandidateLoadOutcomeLikeCpp,
    SocialContactListLoadOutcomeLikeCpp, SocialContactLoadRowLikeCpp,
    SocialPartyInviteLookupOutcomeLikeCpp, SocialPersistencePortLikeCpp,
    SocialRelationshipKindLikeCpp, SocialRelationshipStateLikeCpp,
};

mod session_account;
pub use session_account::{
    SessionAccountDataLoadOutcomeLikeCpp, SessionAccountDataRowLikeCpp,
    SessionAccountDataSaveLikeCpp, SessionAccountDataScopeLikeCpp, SessionAccountStatePortLikeCpp,
    SessionTutorialsLoadOutcomeLikeCpp,
};

mod player_login;
pub use player_login::{
    PlayerActionButtonLoadRowLikeCpp, PlayerBagInventoryLoadRowLikeCpp,
    PlayerBattlegroundLocationLoadRowLikeCpp, PlayerBuybackClearRequestLikeCpp,
    PlayerCharacterAuraEffectLoadRowLikeCpp, PlayerCharacterAuraLoadRowLikeCpp,
    PlayerCharacterBaseLoadOutcomeLikeCpp, PlayerCharacterBaseLoadRequestLikeCpp,
    PlayerCharacterBaseLoadRowLikeCpp, PlayerCufProfileLoadRowLikeCpp,
    PlayerCurrencyLoadRowLikeCpp, PlayerCustomizationLoadRowLikeCpp,
    PlayerEquipmentInventoryLoadRowLikeCpp, PlayerEquipmentSetLoadRowLikeCpp,
    PlayerGlyphLoadRowLikeCpp, PlayerGuildMembershipLoadRowLikeCpp,
    PlayerHomebindLocationLoadRowLikeCpp, PlayerInitialWorldStateRowsLikeCpp,
    PlayerInitialWorldStateTemplateRowLikeCpp, PlayerInitialWorldStateValueRowLikeCpp,
    PlayerInitialWorldStatesLoadOutcomeLikeCpp, PlayerInstanceTimeRestrictionLoadRowLikeCpp,
    PlayerInventoryItemLoadRowLikeCpp, PlayerLoginAdmissionLoadOutcomeLikeCpp,
    PlayerLoginAdmissionLoadRequestLikeCpp, PlayerLoginAdmissionLoadedLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, PlayerLoginItemRepairActionLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerLoginPetTalentResetOutcomeLikeCpp,
    PlayerLoginTransportLoadOutcomeLikeCpp, PlayerLoginTransportLoadRequestLikeCpp,
    PlayerLoginTransportLoadRowLikeCpp, PlayerMailLoadRowLikeCpp, PlayerOnlineMarkRequestLikeCpp,
    PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetDeclinedNamesLoadRowLikeCpp, PlayerPetSpellChargeLoadRowLikeCpp,
    PlayerPetSpellCooldownLoadRowLikeCpp, PlayerPetSpellLoadRowLikeCpp,
    PlayerPetStableLoadRowLikeCpp, PlayerRealmCharacterCountRefreshRequestLikeCpp,
    PlayerReputationLoadRowLikeCpp, PlayerSkillLoadRowLikeCpp, PlayerSpellChargeLoadRowLikeCpp,
    PlayerSpellCooldownLoadRowLikeCpp, PlayerSpellLoadRowLikeCpp, PlayerTalentLoadRowLikeCpp,
    PlayerTraitConfigLoadRowLikeCpp, PlayerTraitEntryLoadRowLikeCpp,
    PlayerTransmogOutfitLoadRowLikeCpp, PlayerVoidStorageLoadRowLikeCpp,
};

mod account_collections;
pub use account_collections::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomLoadRowLikeCpp, AccountHeirloomRowLikeCpp, AccountMaskBlockLikeCpp,
    AccountMountLoadRowLikeCpp, AccountMountRowLikeCpp, AccountToyLoadRowLikeCpp,
    AccountToyRowLikeCpp,
};

mod player_save;
pub use player_save::{
    PlayerActionButtonSaveLikeCpp, PlayerActionButtonsSaveLikeCpp,
    PlayerCharacterCommittedGroupsLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerCharacterSnapshotSaveLikeCpp,
    PlayerCufProfileSaveLikeCpp, PlayerCufProfileSlotSaveLikeCpp, PlayerEquipmentSetSaveLikeCpp,
    PlayerEquipmentSetStateLikeCpp, PlayerEquipmentSetTypeLikeCpp, PlayerFallbackSpellSaveLikeCpp,
    PlayerGlyphSaveLikeCpp, PlayerInstanceLockTimeSaveLikeCpp, PlayerPlayedTimeSaveLikeCpp,
    PlayerPositionSaveLikeCpp, PlayerReputationSaveLikeCpp, PlayerSkillSaveLikeCpp,
    PlayerSpellChargeSaveLikeCpp, PlayerSpellCooldownSaveLikeCpp, PlayerSpellSaveGroupLikeCpp,
    PlayerSpellSaveLikeCpp, PlayerSpellStateLikeCpp, PlayerTalentResetPersistenceRequestLikeCpp,
    PlayerTalentResetSaveRowLikeCpp, PlayerTalentSaveLikeCpp, PlayerTutorialsSaveLikeCpp,
    PlayerVoidStorageSaveLikeCpp, PlayerVoidStorageSlotSaveLikeCpp,
    PlayerXpPersistenceRequestLikeCpp, PlayerXpRestStateSaveLikeCpp,
};

mod character_enumeration;
pub use character_enumeration::{
    CharacterEnumerationLoadOutcomeLikeCpp, CharacterEnumerationPersistencePortLikeCpp,
    CharacterEnumerationRequestLikeCpp, CharacterEnumerationRowLikeCpp,
};

mod quest_poi;
pub use quest_poi::{
    QuestPoiBlobLoadRowLikeCpp, QuestPoiLoadOutcomeLikeCpp, QuestPoiLoadStageLikeCpp,
    QuestPoiPersistencePortLikeCpp, QuestPoiPointLoadRowLikeCpp,
};

mod stored_item_money;
pub use stored_item_money::{
    STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP, StoredItemMoneyPersistenceAttemptLikeCpp,
    StoredItemMoneyPersistenceOutcomeLikeCpp, StoredItemMoneyPersistencePortLikeCpp,
    StoredItemMoneyPersistenceRequestLikeCpp, StoredItemMoneyReconciliationLikeCpp,
    StoredItemMoneyRollbackKindLikeCpp, classify_stored_item_money_reconciliation_like_cpp,
    stored_item_money_zero_without_source_outcome_like_cpp,
};

mod spell_acquisition;
pub use spell_acquisition::{
    PlayerSpellAcquisitionAuthorityLikeCpp, PlayerSpellAcquisitionDurableOperationLikeCpp,
    PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    PlayerSpellAcquisitionPersistenceAttemptLikeCpp, PlayerSpellAcquisitionPersistencePortLikeCpp,
    PlayerSpellAcquisitionPersistenceRequestLikeCpp, PlayerSpellAcquisitionSkillRowLikeCpp,
    PlayerSpellAcquisitionSpellRowLikeCpp,
    classify_player_spell_acquisition_money_reconciliation_like_cpp,
};

mod group;
pub use group::{
    GroupLootMoneyPayoutLikeCpp, GroupLootMoneyPersistenceAttemptLikeCpp,
    GroupLootMoneyPersistenceOutcomeLikeCpp, GroupLootMoneyPersistencePortLikeCpp,
    GroupLootMoneyPersistenceRequestLikeCpp, GroupLootMoneyReconciliationLikeCpp,
    GroupLootMoneyRollbackKindLikeCpp, RepresentedGroupDifficultyKindLikeCpp,
    RepresentedGroupPersistenceCommandLikeCpp, RepresentedGroupPersistenceModeLikeCpp,
    RepresentedGroupPersistenceOutcomeLikeCpp, RepresentedGroupPersistencePortLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp, RepresentedGroupStartupCharacterLikeCpp,
    RepresentedGroupStartupGroupRowLikeCpp, RepresentedGroupStartupLoadOutcomeLikeCpp,
    RepresentedGroupStartupLoadPortLikeCpp, RepresentedGroupStartupLoadStageLikeCpp,
    RepresentedGroupStartupMemberRowLikeCpp, classify_group_loot_money_reconciliation_like_cpp,
};

#[cfg(test)]
mod tests;
