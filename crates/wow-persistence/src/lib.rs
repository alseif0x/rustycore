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

mod catalogs;
mod character_administration;
mod hotfix;
mod hotfix_delivery_metadata;
mod instance_lock;
mod player;
mod quest;
mod skill_world_rules;
mod spell;
mod static_data_overlay;
mod stored_item;
mod vendor_trade;
mod world;

pub use catalogs::{
    AREA_TRIGGER_SHAPE_DATA_COUNT_LIKE_CPP, AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp,
    AreaTriggerCreatePropertiesPersistenceRowLikeCpp,
    AreaTriggerPolygonVertexPersistenceRowLikeCpp, AreaTriggerSplinePointPersistenceRowLikeCpp,
    AreaTriggerTemplateActionPersistenceRowLikeCpp, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp,
    AreaTriggerTemplateCatalogPersistencePortLikeCpp, AreaTriggerTemplateCatalogRowsLikeCpp,
    AreaTriggerTemplatePersistenceRowLikeCpp,
};
pub use catalogs::{
    AreaTriggerDestinationPersistenceRowLikeCpp, AreaTriggerScriptPersistenceRowLikeCpp,
    AreaTriggerTeleportPersistenceRowLikeCpp, AreaTriggerWorldCatalogPersistencePortLikeCpp,
    AreaTriggerWorldLoadOutcomeLikeCpp, QuestAreaTriggerPersistenceRowLikeCpp,
    TavernAreaTriggerPersistenceRowLikeCpp,
};
pub use catalogs::{
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
pub use catalogs::{
    BattlePetBreedPersistenceRowLikeCpp, BattlePetQualityPersistenceRowLikeCpp,
    BattlePetSelectionCatalogLoadOutcomeLikeCpp, BattlePetSelectionCatalogPersistencePortLikeCpp,
};
pub use catalogs::{
    ConditionDisableCatalogPersistencePortLikeCpp, ConditionDisableRowsLoadOutcomeLikeCpp,
    ConditionPersistenceRowLikeCpp, DisablePersistenceRowLikeCpp,
};
pub use catalogs::{
    CreatureEquipmentIdPersistenceRowLikeCpp, GameEventConditionPersistenceRowLikeCpp,
    GameEventDataPersistenceRowLikeCpp, GameEventModelEquipPersistenceRowLikeCpp,
    GameEventNpcFlagPersistenceRowLikeCpp, GameEventNpcVendorPersistenceRowLikeCpp,
    GameEventObjectGuidPersistenceRowLikeCpp, GameEventPoolPersistenceRowLikeCpp,
    GameEventPrerequisitePersistenceRowLikeCpp, GameEventQuestConditionPersistenceRowLikeCpp,
    GameEventQuestRelationPersistenceRowLikeCpp, GameEventWorldCatalogLoadOutcomeLikeCpp,
    GameEventWorldCatalogPersistencePortLikeCpp, GameEventWorldCatalogPrefixLikeCpp,
    GameEventWorldCatalogSuffixLikeCpp,
};
pub use catalogs::{
    CreatureOnKillReputationPersistenceRowLikeCpp, REPUTATION_SPILLOVER_SLOT_COUNT_LIKE_CPP,
    ReputationCatalogLoadOutcomeLikeCpp, ReputationCatalogPersistencePortLikeCpp,
    ReputationRewardRatePersistenceRowLikeCpp, ReputationSpilloverTemplatePersistenceRowLikeCpp,
};
pub use catalogs::{
    CreatureTrainerPersistenceRowLikeCpp, TrainerCatalogLoadOutcomeLikeCpp,
    TrainerCatalogPersistencePortLikeCpp, TrainerCatalogPersistenceRowsLikeCpp,
    TrainerLocalePersistenceRowLikeCpp, TrainerPersistenceRowLikeCpp,
    TrainerSpellPersistenceRowLikeCpp,
};
pub use catalogs::{
    CreatureVisibilityPersistenceRowLikeCpp, GameObjectVisibilityPersistenceRowLikeCpp,
    VisibilitySpawnCatalogOutcomeLikeCpp, VisibilitySpawnCatalogPersistencePortLikeCpp,
    VisibilitySpawnCatalogRequestLikeCpp,
};
pub use catalogs::{
    ExplorationBaseXpCatalogLoadOutcomeLikeCpp, ExplorationBaseXpCatalogPersistencePortLikeCpp,
    ExplorationBaseXpPersistenceRowLikeCpp,
};
pub use catalogs::{
    FactionChangePairPersistenceRowLikeCpp, FactionChangePersistenceRowsLikeCpp,
    GameplayRuleCatalogPersistencePortLikeCpp, GameplayRuleRowsLoadOutcomeLikeCpp,
    NpcSpellClickPersistenceRowLikeCpp, NpcVendorPersistenceRowLikeCpp,
};
pub use catalogs::{
    GameTeleCatalogLoadOutcomeLikeCpp, GameTeleCatalogPersistencePortLikeCpp,
    GameTelePersistenceRowLikeCpp,
};
pub use catalogs::{
    GossipMenuAddonPersistenceRowLikeCpp, GossipMenuOptionLocalePersistenceRowLikeCpp,
    GossipMenuPersistenceRowLikeCpp, GossipStartupCatalogLoadOutcomeLikeCpp,
    GossipStartupCatalogPersistencePortLikeCpp,
};
pub use catalogs::{
    ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp,
    ItemRandomEnchantmentCatalogPersistencePortLikeCpp, ItemRandomEnchantmentPersistenceRowLikeCpp,
};
pub use catalogs::{
    JumpChargeCatalogLoadOutcomeLikeCpp, JumpChargeCatalogPersistencePortLikeCpp,
    JumpChargeParamsPersistenceRowLikeCpp,
};
pub use catalogs::{
    LfgDungeonRewardPersistenceRowLikeCpp, LfgDungeonTemplatePersistenceRowLikeCpp,
    LfgWorldCatalogLoadOutcomeLikeCpp, LfgWorldCatalogPersistencePortLikeCpp,
};
pub use catalogs::{
    LootConditionPersistenceRowLikeCpp, LootTemplateCatalogOutcomeLikeCpp,
    LootTemplateCatalogPersistencePortLikeCpp, LootTemplatePersistenceRowLikeCpp,
    LootTemplateTablePersistenceLikeCpp,
};
pub use catalogs::{
    MountCapabilityHotfixRowLikeCpp, MountCatalogLoadOutcomeLikeCpp,
    MountCatalogPersistencePortLikeCpp, MountDefinitionRowLikeCpp, MountHotfixRowLikeCpp,
    MountTypeXCapabilityHotfixRowLikeCpp, MountXDisplayHotfixRowLikeCpp,
};
pub use catalogs::{
    PhaseAreaPersistenceRowLikeCpp, PhaseNamePersistenceRowLikeCpp,
    PhaseWorldCatalogLoadOutcomeLikeCpp, PhaseWorldCatalogPersistencePortLikeCpp,
    TerrainSwapDefaultPersistenceRowLikeCpp, TerrainWorldMapPersistenceRowLikeCpp,
};
pub use catalogs::{
    PhaseGroupHotfixRowLikeCpp, PhaseHotfixLoadOutcomeLikeCpp, PhaseHotfixPersistencePortLikeCpp,
    PhaseHotfixRowLikeCpp,
};
pub use catalogs::{
    ReservedNameCatalogLoadOutcomeLikeCpp, ReservedNameCatalogPersistencePortLikeCpp,
    ReservedNamePersistenceRowLikeCpp,
};
pub use catalogs::{
    VEHICLE_SEAT_COUNT_LIKE_CPP, VehicleHotfixLoadOutcomeLikeCpp,
    VehicleHotfixPersistencePortLikeCpp, VehicleHotfixPersistenceRowLikeCpp,
    VehicleSeatHotfixPersistenceRowLikeCpp, VehicleSpawnAccessoryPersistenceRowLikeCpp,
    VehicleTemplateAccessoryPersistenceRowLikeCpp, VehicleTemplatePersistenceRowLikeCpp,
    VehicleWorldCatalogLoadOutcomeLikeCpp, VehicleWorldCatalogPersistencePortLikeCpp,
};
pub use catalogs::{
    VendorCatalogOutcomeLikeCpp, VendorCatalogPersistencePortLikeCpp, VendorCatalogRowLikeCpp,
};
pub use character_administration::{
    CharacterAdministrationLoadOutcomeLikeCpp, CharacterAdministrationMutationOutcomeLikeCpp,
    CharacterAdministrationPersistencePortLikeCpp, CharacterCreatePersistenceRequestLikeCpp,
    CharacterCustomizationPersistenceLikeCpp, CharacterCustomizeCandidateLikeCpp,
    CharacterRenameCandidateLikeCpp,
};
pub use hotfix::{
    ChrSpecializationHotfixLoadOutcomeLikeCpp, ChrSpecializationHotfixPersistencePortLikeCpp,
    ChrSpecializationHotfixRowLikeCpp, ChrSpecializationHotfixRowsLikeCpp,
};
pub use hotfix::{
    CreatureDisplayHotfixLoadOutcomeLikeCpp, CreatureDisplayHotfixPersistencePortLikeCpp,
    CreatureDisplayInfoHotfixRowLikeCpp, CreatureModelDataHotfixRowLikeCpp,
};
pub use hotfix::{
    DifficultyHotfixLoadOutcomeLikeCpp, DifficultyHotfixPersistencePortLikeCpp,
    DifficultyHotfixRowLikeCpp, DifficultyHotfixRowsLikeCpp,
};
pub use hotfix::{
    LfgDungeonsHotfixLoadOutcomeLikeCpp, LfgDungeonsHotfixPersistencePortLikeCpp,
    LfgDungeonsHotfixRowLikeCpp,
};
pub use hotfix::{
    SkillCatalogHotfixLoadOutcomeLikeCpp, SkillCatalogHotfixPersistencePortLikeCpp,
    SkillLineAbilityHotfixRowLikeCpp, SkillLineHotfixRowLikeCpp, SkillLineHotfixRowsLikeCpp,
    SkillRaceClassInfoHotfixRowLikeCpp, SkillRelationHotfixRowsLikeCpp,
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
pub use player::{
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
pub use player::{
    PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP, PlayerBaseStatsLoadOutcomeLikeCpp,
    PlayerBaseStatsPersistencePortLikeCpp, PlayerClassLevelStatsPersistenceRowLikeCpp,
    PlayerRaceStatsPersistenceRowLikeCpp,
};
pub use player::{
    PlayerChoiceCatalogCoreRowsLikeCpp, PlayerChoiceCatalogLoadOutcomeLikeCpp,
    PlayerChoiceCatalogLocaleRowsLikeCpp, PlayerChoiceCatalogPersistencePortLikeCpp,
    PlayerChoiceLocaleRowLikeCpp, PlayerChoiceResponseLocaleRowLikeCpp,
    PlayerChoiceResponseMawPowerRowLikeCpp, PlayerChoiceResponseRewardCurrencyRowLikeCpp,
    PlayerChoiceResponseRewardFactionRowLikeCpp, PlayerChoiceResponseRewardItemRowLikeCpp,
    PlayerChoiceResponseRewardRowLikeCpp, PlayerChoiceResponseRowLikeCpp, PlayerChoiceRowLikeCpp,
};
pub use player::{
    PlayerCreateCastSpellPersistenceRowLikeCpp, PlayerCreateCustomSpellPersistenceRowLikeCpp,
    PlayerCreateInfoPersistenceRowLikeCpp, PlayerCreationCatalogLoadOutcomeLikeCpp,
    PlayerCreationCatalogPersistencePortLikeCpp,
};
pub use player::{
    PlayerQuestActivePersistenceRowLikeCpp, PlayerQuestDailyPersistenceRowLikeCpp,
    PlayerQuestIdPersistenceRowLikeCpp, PlayerQuestLoadOutcomeLikeCpp,
    PlayerQuestLockoutPersistenceRequestLikeCpp, PlayerQuestObjectivePersistenceRowLikeCpp,
    PlayerQuestPersistencePortLikeCpp, PlayerQuestSeasonalCompletionPersistenceLikeCpp,
    PlayerQuestSeasonalPersistenceRowLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
    QuestObjectiveCountPersistenceLikeCpp, QuestStatusPersistenceLikeCpp,
};
pub use player::{
    PlayerQuestRewardCommitOutcomeLikeCpp, PlayerQuestRewardCommitWitnessLikeCpp,
    PlayerQuestRewardDurableRequestLikeCpp, PlayerQuestRewardMoneyLikeCpp,
    PlayerQuestRewardPersistencePortLikeCpp,
};
pub use quest::*;
pub use quest::{
    CreatureQuestItemPersistenceRowLikeCpp, GameObjectQuestItemPersistenceRowLikeCpp,
    QuestItemCatalogLoadOutcomeLikeCpp, QuestItemCatalogPersistencePortLikeCpp,
};
pub use skill_world_rules::{
    FishingBaseSkillPersistenceRowLikeCpp, SKILL_TIER_VALUE_COUNT_LIKE_CPP,
    SkillTierPersistenceRowLikeCpp, SkillWorldRulesLoadOutcomeLikeCpp,
    SkillWorldRulesPersistencePortLikeCpp,
};
pub use spell::{
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
pub use spell::{
    SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP, SpellInfoKeyContributorHotfixBatchLikeCpp,
    SpellInfoKeyContributorHotfixRowLikeCpp, SpellInfoKeyContributorLikeCpp,
    SpellInfoKeyHotfixLoadOutcomeLikeCpp, SpellInfoKeyHotfixPersistencePortLikeCpp,
    SpellInfoKeyHotfixRowsLikeCpp, SpellInfoPowerDifficultyHotfixRowLikeCpp,
};
pub use spell::{
    SpellAreaPersistenceRowLikeCpp, SpellGroupPersistenceRowLikeCpp,
    SpellGroupStackRulePersistenceRowLikeCpp, SpellLinkedPersistenceRowLikeCpp,
    SpellPetAuraPersistenceRowLikeCpp, SpellProcPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellTargetPositionPersistenceRowLikeCpp,
    SpellThreatPersistenceRowLikeCpp, SpellTotemModelPersistenceRowLikeCpp,
    SpellWorldCatalogLoadOutcomeLikeCpp, SpellWorldCatalogPersistencePortLikeCpp,
};
pub use spell::{
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
pub use vendor_trade::*;
pub use world::*;
pub use world::{
    AccessRequirementPersistenceRowLikeCpp, GraveyardZonePersistenceRowLikeCpp,
    SceneTemplatePersistenceRowLikeCpp, SpawnGroupTemplatePersistenceRowLikeCpp,
    TrinityStringPersistenceRowLikeCpp, WorldAuxiliaryCatalogPersistencePortLikeCpp,
    WorldAuxiliaryRowsLoadOutcomeLikeCpp,
};
pub use world::{
    WorldObjectIdCatalogKindLikeCpp, WorldReferenceCatalogPersistencePortLikeCpp,
    WorldReferenceRowsLoadOutcomeLikeCpp, WorldSafeLocPersistenceRowLikeCpp,
    WorldSpawnCatalogKindLikeCpp,
};

mod outcome;
pub use outcome::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

pub use world::{
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

pub use player::{
    PlayerHomebindPersistenceRequestLikeCpp, PlayerLifecyclePortLikeCpp, PlayerOfflineMarkLikeCpp,
};

mod session_administration;
pub use session_administration::{
    PacketSpoofAffectedAccountsLoadOutcomeLikeCpp, PacketSpoofBanPersistencePortLikeCpp,
    PacketSpoofBanTargetLikeCpp, PacketSpoofBanWriteRequestLikeCpp,
    SupportBugReportPersistencePortLikeCpp, SupportBugReportWriteRequestLikeCpp,
};

pub use catalogs::{
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

pub use player::{
    PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp,
};

pub use player::{
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

pub use player::{
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

pub use player::{
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

pub use quest::{
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

pub use spell::{
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
