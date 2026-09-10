//! Flags packets.
//!
//! Separated from item.rs under #701.

use super::*;

bitflags! {
    /// Spell item enchantment flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SpellItemEnchantmentFlags: u16 {
        const SOULBOUND                     = 0x01;
        const DO_NOT_LOG                    = 0x02;
        const MAINHAND_ONLY                 = 0x04;
        const ALLOW_ENTERING_ARENA          = 0x08;
        const DO_NOT_SAVE_TO_DB             = 0x10;
        const SCALE_AS_A_GEM                = 0x20;
        const DISABLE_IN_CHALLENGE_MODES    = 0x40;
        const DISABLE_IN_PROVING_GROUNDS    = 0x80;
        const ALLOW_TRANSMOG                = 0x100;
        const HIDE_UNTIL_COLLECTED          = 0x200;
    }
}

/// Item modifier identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ItemModifier {
    TransmogAppearanceAllSpecs = 0,
    TransmogAppearanceSpec1 = 1,
    UpgradeId = 2,
    BattlePetSpeciesId = 3,
    BattlePetBreedData = 4,
    BattlePetLevel = 5,
    BattlePetDisplayId = 6,
    EnchantIllusionAllSpecs = 7,
    ArtifactAppearanceId = 8,
    TimewalkerLevel = 9,
    EnchantIllusionSpec1 = 10,
    TransmogAppearanceSpec2 = 11,
    EnchantIllusionSpec2 = 12,
    TransmogAppearanceSpec3 = 13,
    EnchantIllusionSpec3 = 14,
    TransmogAppearanceSpec4 = 15,
    EnchantIllusionSpec4 = 16,
    ChallengeMapChallengeModeId = 17,
    ChallengeKeystoneLevel = 18,
    ChallengeKeystoneAffixId1 = 19,
    ChallengeKeystoneAffixId2 = 20,
    ChallengeKeystoneAffixId3 = 21,
    ChallengeKeystoneAffixId4 = 22,
    ArtifactKnowledgeLevel = 23,
    ArtifactTier = 24,
    TransmogAppearanceSpec5 = 25,
    PvpRating = 26,
    EnchantIllusionSpec5 = 27,
    ContentTuningId = 28,
    ChangeModifiedCraftingStat1 = 29,
    ChangeModifiedCraftingStat2 = 30,
    TransmogSecondaryAppearanceAllSpecs = 31,
    TransmogSecondaryAppearanceSpec1 = 32,
    TransmogSecondaryAppearanceSpec2 = 33,
    TransmogSecondaryAppearanceSpec3 = 34,
    TransmogSecondaryAppearanceSpec4 = 35,
    TransmogSecondaryAppearanceSpec5 = 36,
    SoulbindConduitRank = 37,
    CraftingQualityId = 38,
    CraftingSkillLineAbilityId = 39,
    CraftingDataId = 40,
    CraftingSkillReagents = 41,
    CraftingSkillWatermark = 42,
    CraftingReagentSlot0 = 43,
    CraftingReagentSlot1 = 44,
    CraftingReagentSlot2 = 45,
    CraftingReagentSlot3 = 46,
    CraftingReagentSlot4 = 47,
    CraftingReagentSlot5 = 48,
    CraftingReagentSlot6 = 49,
    CraftingReagentSlot7 = 50,
    CraftingReagentSlot8 = 51,
    CraftingReagentSlot9 = 52,
    CraftingReagentSlot10 = 53,
    CraftingReagentSlot11 = 54,
    CraftingReagentSlot12 = 55,
    CraftingReagentSlot13 = 56,
    CraftingReagentSlot14 = 57,
}

/// Item bonus types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ItemBonusType {
    ItemLevel = 1,
    Stat = 2,
    Quality = 3,
    NameSubtitle = 4,
    Suffix = 5,
    Socket = 6,
    Appearance = 7,
    RequiredLevel = 8,
    DisplayToastMethod = 9,
    RepairCostMuliplier = 10,
    ScalingStatDistribution = 11,
    DisenchantLootId = 12,
    ScalingStatDistributionFixed = 13,
    ItemLevelCanIncrease = 14,
    RandomEnchantment = 15,
    Bounding = 16,
    RelicType = 17,
    OverrideRequiredLevel = 18,
    AzeriteTierUnlockSet = 19,
    ScrappingLootId = 20,
    OverrideCanDisenchant = 21,
    OverrideCanScrap = 22,
    ItemEffectId = 23,
    ModifiedCraftingStat = 25,
    RequiredLevelCurve = 27,
    DescriptionText = 30,
    OverrideName = 31,
    ItemBonusListGroup = 34,
    ItemLimitCategory = 35,
    ItemConversion = 37,
    ItemHistorySlot = 38,
}

/// Item context (source of the item).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ItemContext {
    None = 0,
    DungeonNormal = 1,
    DungeonHeroic = 2,
    RaidNormal = 3,
    RaidRaidFinder = 4,
    RaidHeroic = 5,
    RaidMythic = 6,
    PvpUnranked1 = 7,
    PvpRanked1Unrated = 8,
    ScenarioNormal = 9,
    ScenarioHeroic = 10,
    QuestReward = 11,
    InGameStore = 12,
    TradeSkill = 13,
    Vendor = 14,
    BlackMarket = 15,
    MythicplusEndOfRun = 16,
    DungeonLvlUp1 = 17,
    DungeonLvlUp2 = 18,
    DungeonLvlUp3 = 19,
    DungeonLvlUp4 = 20,
    ForceToNone = 21,
    Timewalking = 22,
    DungeonMythic = 23,
    PvpHonorReward = 24,
    WorldQuest1 = 25,
    WorldQuest2 = 26,
    WorldQuest3 = 27,
    WorldQuest4 = 28,
    WorldQuest5 = 29,
    WorldQuest6 = 30,
    MissionReward1 = 31,
    MissionReward2 = 32,
    MythicplusEndOfRunTimeChest = 33,
    ZzchallengeMode3 = 34,
    MythicplusJackpot = 35,
    WorldQuest7 = 36,
    WorldQuest8 = 37,
    PvpRanked2Combatant = 38,
    PvpRanked3Challenger = 39,
    PvpRanked4Rival = 40,
    PvpUnranked2 = 41,
    WorldQuest9 = 42,
    WorldQuest10 = 43,
    PvpRanked5Duelist = 44,
    PvpRanked6Elite = 45,
    PvpRanked7 = 46,
    PvpUnranked3 = 47,
    PvpUnranked4 = 48,
    PvpUnranked5 = 49,
    PvpUnranked6 = 50,
    PvpUnranked7 = 51,
    PvpRanked8 = 52,
    WorldQuest11 = 53,
    WorldQuest12 = 54,
    WorldQuest13 = 55,
    PvpRankedJackpot = 56,
    TournamentRealm = 57,
    Relinquished = 58,
    LegendaryForge = 59,
    QuestBonusLoot = 60,
    CharacterBoostBfa = 61,
    CharacterBoostShadowlands = 62,
    LegendaryCrafting1 = 63,
    LegendaryCrafting2 = 64,
    LegendaryCrafting3 = 65,
    LegendaryCrafting4 = 66,
    LegendaryCrafting5 = 67,
    LegendaryCrafting6 = 68,
    LegendaryCrafting7 = 69,
    LegendaryCrafting8 = 70,
    LegendaryCrafting9 = 71,
    WeeklyRewardsAdditional = 72,
    WeeklyRewardsConcession = 73,
    WorldQuestJackpot = 74,
    NewCharacter = 75,
    WarMode = 76,
    PvpBrawl1 = 77,
    PvpBrawl2 = 78,
    Torghast = 79,
    CorpseRecovery = 80,
    WorldBoss = 81,
    RaidNormalExtended = 82,
    RaidRaidFinderExtended = 83,
    RaidHeroicExtended = 84,
    RaidMythicExtended = 85,
    CharacterTemplate91 = 86,
    ChallengeMode4 = 87,
    PvpRanked9 = 88,
    RaidNormalExtended2 = 89,
    RaidFinderExtended2 = 90,
    RaidHeroicExtended2 = 91,
    RaidMythicExtended2 = 92,
    RaidNormalExtended3 = 93,
    RaidFinderExtended3 = 94,
    RaidHeroicExtended3 = 95,
    RaidMythicExtended3 = 96,
    TemplateCharacter1 = 97,
    TemplateCharacter2 = 98,
    TemplateCharacter3 = 99,
    TemplateCharacter4 = 100,
}

/// Item bonding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemBondingType {
    None = 0,
    OnAcquire = 1,
    OnEquip = 2,
    OnUse = 3,
    Quest = 4,
}

/// Item quality (rarity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ItemQuality {
    None = -1,
    Poor = 0,
    Normal = 1,
    Uncommon = 2,
    Rare = 3,
    Epic = 4,
    Legendary = 5,
    Artifact = 6,
    Heirloom = 7,
}

bitflags! {
    /// Item field flags (dynamic item state).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemFieldFlags: u32 {
        const SOULBOUND                         = 0x01;
        const TRANSLATED                        = 0x02;
        const UNLOCKED                          = 0x04;
        const WRAPPED                           = 0x08;
        const UNK2                              = 0x10;
        const UNK3                              = 0x20;
        const UNK4                              = 0x40;
        const UNK5                              = 0x80;
        const BOP_TRADEABLE                     = 0x100;
        const READABLE                          = 0x200;
        const UNK6                              = 0x400;
        const UNK7                              = 0x800;
        const REFUNDABLE                        = 0x1000;
        const UNK8                              = 0x2000;
        const UNK9                              = 0x4000;
        const UNK10                             = 0x8000;
        const UNK11                             = 0x00010000;
        const UNK12                             = 0x00020000;
        const UNK13                             = 0x00040000;
        const CHILD                             = 0x00080000;
        const UNK15                             = 0x00100000;
        const NEW_ITEM                          = 0x00200000;
        const AZERITE_EMPOWERED_ITEM_VIEWED     = 0x00400000;
        const UNK18                             = 0x00800000;
        const UNK19                             = 0x01000000;
        const UNK20                             = 0x02000000;
        const UNK21                             = 0x04000000;
        const UNK22                             = 0x08000000;
        const UNK23                             = 0x10000000;
        const UNK24                             = 0x20000000;
        const UNK25                             = 0x40000000;
        const UNK26                             = 0x80000000;
    }
}

bitflags! {
    /// Secondary item field flags (dynamic item state).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemFieldFlags2: u32 {
        const EQUIPPED = 0x01;
    }
}

bitflags! {
    /// Item flags (from item template).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemFlags: u64 {
        const NO_PICKUP                             = 0x01;
        const CONJURED                              = 0x02;
        const HAS_LOOT                              = 0x04;
        const HEROIC_TOOLTIP                        = 0x08;
        const DEPRECATED                            = 0x10;
        const NO_USER_DESTROY                       = 0x20;
        const PLAYERCAST                            = 0x40;
        const NO_EQUIP_COOLDOWN                     = 0x80;
        const LEGACY                                = 0x100;
        const IS_WRAPPER                            = 0x200;
        const USES_RESOURCES                        = 0x400;
        const MULTI_DROP                            = 0x800;
        const ITEM_PURCHASE_RECORD                  = 0x1000;
        const PETITION                              = 0x2000;
        const HAS_TEXT                              = 0x4000;
        const NO_DISENCHANT                         = 0x8000;
        const REAL_DURATION                         = 0x10000;
        const NO_CREATOR                            = 0x20000;
        const IS_PROSPECTABLE                       = 0x40000;
        const UNIQUE_EQUIPPABLE                     = 0x80000;
        const DISABLE_AUTO_QUOTES                   = 0x100000;
        const IGNORE_DEFAULT_ARENA_RESTRICTIONS     = 0x200000;
        const NO_DURABILITY_LOSS                    = 0x400000;
        const USE_WHEN_SHAPESHIFTED                 = 0x800000;
        const HAS_QUEST_GLOW                        = 0x1000000;
        const HIDE_UNUSABLE_RECIPE                  = 0x2000000;
        const NOT_USEABLE_IN_ARENA                  = 0x4000000;
        const IS_BOUND_TO_ACCOUNT                   = 0x8000000;
        const NO_REAGENT_COST                       = 0x10000000;
        const IS_MILLABLE                           = 0x20000000;
        const REPORT_TO_GUILD_CHAT                  = 0x40000000;
        const NO_PROGRESSIVE_LOOT                   = 0x80000000;
    }
}

/// Item flags 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemFlags2 {
    FactionHorde = 0x01,
    FactionAlliance = 0x02,
    DontIgnoreBuyPrice = 0x04,
    ClassifyAsCaster = 0x08,
    ClassifyAsPhysical = 0x10,
    EveryoneCanRollNeed = 0x20,
    NoTradeBindOnAcquire = 0x40,
    CanTradeBindOnAcquire = 0x80,
    CanOnlyRollGreed = 0x100,
    CasterWeapon = 0x200,
    DeleteOnLogin = 0x400,
    InternalItem = 0x800,
    NoVendorValue = 0x1000,
    ShowBeforeDiscovered = 0x2000,
    OverrideGoldCost = 0x4000,
    IgnoreDefaultRatedBgRestrictions = 0x8000,
    NotUsableInRatedBg = 0x10000,
    BnetAccountTradeOk = 0x20000,
    ConfirmBeforeUse = 0x40000,
    ReevaluateBondingOnTransform = 0x80000,
    NoTransformOnChargeDepletion = 0x100000,
    NoAlterItemVisual = 0x200000,
    NoSourceForItemVisual = 0x400000,
    IgnoreQualityForItemVisualSource = 0x800000,
    NoDurability = 0x1000000,
    RoleTank = 0x2000000,
    RoleHealer = 0x4000000,
    RoleDamage = 0x8000000,
    CanDropInChallengeMode = 0x10000000,
    NeverStackInLootUi = 0x20000000,
    DisenchantToLootTable = 0x40000000,
    UsedInATradeskill = 0x80000000,
}

/// Item flags 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemFlags3 {
    DontDestroyOnQuestAccept = 0x01,
    ItemCanBeUpgraded = 0x02,
    UpgradeFromItemOverridesDropUpgrade = 0x04,
    AlwaysFfaInLoot = 0x08,
    HideUpgradeLevelsIfNotUpgraded = 0x10,
    UpdateInteractions = 0x20,
    UpdateDoesntLeaveProgressiveWinHistory = 0x40,
    IgnoreItemHistoryTracker = 0x80,
    IgnoreItemLevelCapInPvp = 0x100,
    DisplayAsHeirloom = 0x200,
    SkipUseCheckOnPickup = 0x400,
    Obsolete = 0x800,
    DontDisplayInGuildNews = 0x1000,
    PvpTournamentGear = 0x2000,
    RequiresStackChangeLog = 0x4000,
    UnusedFlag = 0x8000,
    HideNameSuffix = 0x10000,
    PushLoot = 0x20000,
    DontReportLootLogToParty = 0x40000,
    AlwaysAllowDualWield = 0x80000,
    Obliteratable = 0x100000,
    ActsAsTransmogHiddenVisualOption = 0x200000,
    ExpireOnWeeklyReset = 0x400000,
    DoesntShowUpInTransmogUntilCollected = 0x800000,
    CanStoreEnchants = 0x1000000,
    HideQuestItemFromObjectTooltip = 0x2000000,
    DoNotToast = 0x4000000,
    IgnoreCreationContextForProgressiveWinHistory = 0x8000000,
    ForceAllSpecsForItemHistory = 0x10000000,
    SaveOnConsume = 0x20000000,
    ContainerSavesPlayerData = 0x40000000,
    NoVoidStorage = 0x80000000,
}

/// Item flags 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemFlags4 {
    HandleOnUseEffectImmediately = 0x01,
    AlwaysShowItemLevelInTooltip = 0x02,
    ShowsGenerationWithRandomStats = 0x04,
    ActivateOnEquipEffectsWhenTransmogrified = 0x08,
    EnforceTransmogWithChildItem = 0x10,
    Scrapable = 0x20,
    BypassRepRequirementsForTransmog = 0x40,
    DisplayOnlyOnDefinedRaces = 0x80,
    RegulatedCommodity = 0x100,
    CreateLootImmediately = 0x200,
    GenerateLootSpecItem = 0x400,
    HiddenInRewardsSummaries = 0x800,
    DisallowWhileLevelLinked = 0x1000,
    DisallowEnchant = 0x2000,
    SquishUsingItemLevelAsPlayerLevel = 0x4000,
    AlwaysShowPriceInTooltip = 0x8000,
    CosmeticItem = 0x10000,
    NoSpellEffectTooltipPrefixes = 0x20000,
    IgnoreCosmeticCollectionBehavior = 0x40000,
    NpcOnly = 0x80000,
    NotRestorable = 0x100000,
    DontDisplayAsCraftingReagent = 0x200000,
    DisplayReagentQualityAsCraftedQuality = 0x400000,
    NoSalvage = 0x800000,
    Recraftable = 0x1000000,
    CcTrinket = 0x2000000,
    KeepThroughFactionChange = 0x4000000,
    NotMulticraftable = 0x8000000,
    DontReportLootLogToSelf = 0x10000000,
}

/// Item update state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemUpdateState {
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Removed = 3,
}
