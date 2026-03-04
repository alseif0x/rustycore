// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-related enums and flags: quality, classes, subclasses, inventory types, etc.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Socket gem types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum SocketType {
    None = 0,
    Meta = 1,
    Red = 2,
    Yellow = 3,
    Blue = 4,
    Prismatic = 5,
}

bitflags! {
    /// Socket color flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SocketColor: i32 {
        const NONE      = 0;
        const META      = 1 << 0; // 1 << (Meta - 1)
        const RED       = 1 << 1; // 1 << (Red - 1)
        const YELLOW    = 1 << 2; // 1 << (Yellow - 1)
        const BLUE      = 1 << 3; // 1 << (Blue - 1)

        const PRISMATIC = Self::RED.bits() | Self::YELLOW.bits() | Self::BLUE.bits();
        const ORANGE    = Self::RED.bits() | Self::YELLOW.bits();
        const GREEN     = Self::YELLOW.bits() | Self::BLUE.bits();
        const VIOLET    = Self::RED.bits() | Self::BLUE.bits();
    }
}

bitflags! {
    /// Item extended cost flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemExtendedCostFlags: u32 {
        const REQUIRE_GUILD           = 0x01;
        const REQUIRE_SEASON_EARNED_1 = 0x02;
        const REQUIRE_SEASON_EARNED_2 = 0x04;
        const REQUIRE_SEASON_EARNED_3 = 0x08;
        const REQUIRE_SEASON_EARNED_4 = 0x10;
        const REQUIRE_SEASON_EARNED_5 = 0x20;
    }
}

/// Item modification type (stat type on items).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ItemModType {
    None = -1,
    Mana = 0,
    Health = 1,
    Agility = 3,
    Strength = 4,
    Intellect = 5,
    Spirit = 6,
    Stamina = 7,
    DefenseSkillRating = 12,
    DodgeRating = 13,
    ParryRating = 14,
    BlockRating = 15,
    HitMeleeRating = 16,
    HitRangedRating = 17,
    HitSpellRating = 18,
    CritMeleeRating = 19,
    CritRangedRating = 20,
    CritSpellRating = 21,
    Corruption = 22,
    CorruptionResistance = 23,
    ModifiedCraftingStat1 = 24,
    ModifiedCraftingStat2 = 25,
    CritTakenRangedRating = 26,
    CritTakenSpellRating = 27,
    HasteMeleeRating = 28,
    HasteRangedRating = 29,
    HasteSpellRating = 30,
    HitRating = 31,
    CritRating = 32,
    HitTakenRating = 33,
    CritTakenRating = 34,
    ResilienceRating = 35,
    HasteRating = 36,
    ExpertiseRating = 37,
    AttackPower = 38,
    RangedAttackPower = 39,
    Versatility = 40,
    SpellHealingDone = 41,
    SpellDamageDone = 42,
    ManaRegeneration = 43,
    ArmorPenetrationRating = 44,
    SpellPower = 45,
    HealthRegen = 46,
    SpellPenetration = 47,
    BlockValue = 48,
    MasteryRating = 49,
    ExtraArmor = 50,
    FireResistance = 51,
    FrostResistance = 52,
    HolyResistance = 53,
    ShadowResistance = 54,
    NatureResistance = 55,
    ArcaneResistance = 56,
    PvpPower = 57,
    Unused0 = 58,
    Unused1 = 59,
    Unused3 = 60,
    CrSpeed = 61,
    CrLifesteal = 62,
    CrAvoidance = 63,
    CrSturdiness = 64,
    CrUnused7 = 65,
    Unused27 = 66,
    CrUnused9 = 67,
    CrUnused10 = 68,
    CrUnused11 = 69,
    CrUnused12 = 70,
    AgiStrInt = 71,
    AgiStr = 72,
    AgiInt = 73,
    StrInt = 74,
}

/// Item spell trigger types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ItemSpelltriggerType {
    OnUse = 0,
    OnEquip = 1,
    OnProc = 2,
    SummonedBySpell = 3,
    OnDeath = 4,
    OnPickup = 5,
    OnLearn = 6,
    OnLooted = 7,
}

/// Buy bank slot result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum BuyBankSlotResult {
    FailedTooMany = 0,
    InsufficientFunds = 1,
    NotBanker = 2,
    OK = 3,
}

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

/// Enchantment slot identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u16)]
pub enum EnchantmentSlot {
    EnhancementPermanent = 0,
    EnhancementTemporary = 1,
    EnhancementSocket = 2,
    EnhancementSocket2 = 3,
    EnhancementSocket3 = 4,
    EnhancementSocketBonus = 5,
    EnhancementSocketPrismatic = 6,
    EnhancementUse = 7,
    Property0 = 8,
    Property1 = 9,
    Property2 = 10,
    Property3 = 11,
    Property4 = 12,
}

/// Item enchantment type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ItemEnchantmentType {
    None = 0,
    CombatSpell = 1,
    Damage = 2,
    EquipSpell = 3,
    Resistance = 4,
    Stat = 5,
    Totem = 6,
    UseSpell = 7,
    PrismaticSocket = 8,
    ArtifactPowerBonusRankByType = 9,
    ArtifactPowerBonusRankByID = 10,
    BonusListID = 11,
    BonusListCurve = 12,
    ArtifactPowerBonusRankPicker = 13,
}

bitflags! {
    /// Bag family mask flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BagFamilyMask: u32 {
        const NONE                  = 0x00;
        const ARROWS                = 0x01;
        const BULLETS               = 0x02;
        const SOUL_SHARDS           = 0x04;
        const LEATHERWORKING_SUPP   = 0x08;
        const INSCRIPTION_SUPP      = 0x10;
        const HERBS                 = 0x20;
        const ENCHANTING_SUPP       = 0x40;
        const ENGINEERING_SUPP      = 0x80;
        const KEYS                  = 0x100;
        const GEMS                  = 0x200;
        const MINING_SUPP           = 0x400;
        const SOULBOUND_EQUIPMENT   = 0x800;
        const VANITY_PETS           = 0x1000;
        const CURRENCY_TOKENS       = 0x2000;
        const QUEST_ITEMS           = 0x4000;
        const FISHING_SUPP          = 0x8000;
        const COOKING_SUPP          = 0x10000;
    }
}

/// Inventory type (where an item can be equipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum InventoryType {
    NonEquip = 0,
    Head = 1,
    Neck = 2,
    Shoulders = 3,
    Body = 4,
    Chest = 5,
    Waist = 6,
    Legs = 7,
    Feet = 8,
    Wrists = 9,
    Hands = 10,
    Finger = 11,
    Trinket = 12,
    Weapon = 13,
    Shield = 14,
    Ranged = 15,
    Cloak = 16,
    Weapon2Hand = 17,
    Bag = 18,
    Tabard = 19,
    Robe = 20,
    WeaponMainhand = 21,
    WeaponOffhand = 22,
    Holdable = 23,
    Ammo = 24,
    Thrown = 25,
    RangedRight = 26,
    Quiver = 27,
    Relic = 28,
    ProfessionTool = 29,
    ProfessionGear = 30,
    EquipableSpellOffensive = 31,
    EquipableSpellUtility = 32,
    EquipableSpellDefensive = 33,
    EquipableSpellMobility = 34,
}

/// Visible equipment slot indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum VisibleEquipmentSlot {
    Head = 0,
    Shoulder = 2,
    Shirt = 3,
    Chest = 4,
    Belt = 5,
    Pants = 6,
    Boots = 7,
    Wrist = 8,
    Gloves = 9,
    Back = 14,
    Tabard = 18,
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

/// Item class (major item category).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ItemClass {
    None = -1,
    Consumable = 0,
    Container = 1,
    Weapon = 2,
    Gem = 3,
    Armor = 4,
    Reagent = 5,
    Projectile = 6,
    TradeGoods = 7,
    ItemEnhancement = 8,
    Recipe = 9,
    Money = 10,
    Quiver = 11,
    Quest = 12,
    Key = 13,
    Permanent = 14,
    Miscellaneous = 15,
    Glyph = 16,
    BattlePets = 17,
    WowToken = 18,
    Profession = 19,
}

/// Consumable subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassConsumable {
    Consumable = 0,
    Potion = 1,
    Elixir = 2,
    Flask = 3,
    Scroll = 4,
    FoodDrink = 5,
    ItemEnhancement = 6,
    Bandage = 7,
    ConsumableOther = 8,
    VantusRune = 9,
}

/// Container subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassContainer {
    Container = 0,
    SoulContainer = 1,
    HerbContainer = 2,
    EnchantingContainer = 3,
    EngineeringContainer = 4,
    GemContainer = 5,
    MiningContainer = 6,
    LeatherworkingContainer = 7,
    InscriptionContainer = 8,
    TackleContainer = 9,
    CookingContainer = 10,
    ReagentContainer = 11,
}

/// Weapon subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassWeapon {
    Axe = 0,
    Axe2 = 1,
    Bow = 2,
    Gun = 3,
    Mace = 4,
    Mace2 = 5,
    Polearm = 6,
    Sword = 7,
    Sword2 = 8,
    Warglaives = 9,
    Staff = 10,
    Exotic = 11,
    Exotic2 = 12,
    Fist = 13,
    Miscellaneous = 14,
    Dagger = 15,
    Thrown = 16,
    Spear = 17,
    Crossbow = 18,
    Wand = 19,
    FishingPole = 20,
}

/// Gem subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassGem {
    Intellect = 0,
    Agility = 1,
    Strength = 2,
    Stamina = 3,
    Spirit = 4,
    CriticalStrike = 5,
    Mastery = 6,
    Haste = 7,
    Versatility = 8,
    Other = 9,
    MultipleStats = 10,
    ArtifactRelic = 11,
}

/// Armor subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassArmor {
    Miscellaneous = 0,
    Cloth = 1,
    Leather = 2,
    Mail = 3,
    Plate = 4,
    Cosmetic = 5,
    Shield = 6,
    Libram = 7,
    Idol = 8,
    Totem = 9,
    Sigil = 10,
    Relic = 11,
}

/// Reagent subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassReagent {
    Reagent = 0,
    Keystone = 1,
    ContextToken = 2,
}

/// Projectile subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassProjectile {
    Wand = 0,
    Bolt = 1,
    Arrow = 2,
    Bullet = 3,
    Thrown = 4,
}

/// Trade goods subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassTradeGoods {
    TradeGoods = 0,
    Parts = 1,
    Explosives = 2,
    Devices = 3,
    Jewelcrafting = 4,
    Cloth = 5,
    Leather = 6,
    MetalStone = 7,
    Meat = 8,
    Herb = 9,
    Elemental = 10,
    TradeGoodsOther = 11,
    Enchanting = 12,
    Material = 13,
    Enchantment = 14,
    WeaponEnchantment = 15,
    Inscription = 16,
    ExplosivesDevices = 17,
    OptionalReagent = 18,
    FinishingReagent = 19,
}

/// Item enhancement subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassItemEnhancement {
    Head = 0,
    Neck = 1,
    Shoulder = 2,
    Cloak = 3,
    Chest = 4,
    Wrist = 5,
    Hands = 6,
    Waist = 7,
    Legs = 8,
    Feet = 9,
    Finger = 10,
    Weapon = 11,
    TwoHandedWeapon = 12,
    ShieldOffHand = 13,
    Misc = 14,
}

/// Recipe subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassRecipe {
    Book = 0,
    LeatherworkingPattern = 1,
    TailoringPattern = 2,
    EngineeringSchematic = 3,
    Blacksmithing = 4,
    CookingRecipe = 5,
    AlchemyRecipe = 6,
    FirstAidManual = 7,
    EnchantingFormula = 8,
    FishingManual = 9,
    JewelcraftingRecipe = 10,
    InscriptionTechnique = 11,
}

/// Money subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassMoney {
    Money = 0,
}

/// Quiver subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassQuiver {
    Quiver0 = 0,
    Quiver1 = 1,
    Quiver = 2,
    AmmoPouch = 3,
}

/// Quest item subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassQuest {
    Quest = 0,
    Unk3 = 3,
    Unk8 = 8,
}

/// Key subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassKey {
    Key = 0,
    Lockpick = 1,
}

/// Permanent subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassPermanent {
    Permanent = 0,
}

/// Miscellaneous item subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassMisc {
    Junk = 0,
    Reagent = 1,
    CompanionPet = 2,
    Holiday = 3,
    Other = 4,
    Mount = 5,
    MountEquipment = 6,
}

/// Glyph subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassGlyph {
    Warrior = 1,
    Paladin = 2,
    Hunter = 3,
    Rogue = 4,
    Priest = 5,
    DeathKnight = 6,
    Shaman = 7,
    Mage = 8,
    Warlock = 9,
    Monk = 10,
    Druid = 11,
    DemonHunter = 12,
}

/// Battle pet subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassBattlePet {
    BattlePet = 0,
}

/// WoW token subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassWowToken {
    WowToken = 0,
}

/// Profession subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassProfession {
    Blacksmithing = 0,
    Leatherworking = 1,
    Alchemy = 2,
    Herbalism = 3,
    Cooking = 4,
    Mining = 5,
    Tailoring = 6,
    Engineering = 7,
    Enchanting = 8,
    Fishing = 9,
    Skinning = 10,
    Jewelcrafting = 11,
    Inscription = 12,
    Archaeology = 13,
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

bitflags! {
    /// Custom item flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemFlagsCustom: u32 {
        const UNUSED                = 0x0001;
        const IGNORE_QUEST_STATUS   = 0x0002;
        const FOLLOW_LOOT_RULES     = 0x0004;
    }
}

/// Inventory operation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum InventoryResult {
    Ok = 0,
    CantEquipLevelI = 1,
    CantEquipSkill = 2,
    WrongSlot = 3,
    BagFull = 4,
    BagInBag = 5,
    TradeEquippedBag = 6,
    AmmoOnly = 7,
    ProficiencyNeeded = 8,
    NoSlotAvailable = 9,
    CantEquipEver = 10,
    CantEquipEver2 = 11,
    NoSlotAvailable2 = 12,
    Equipped2handed = 13,
    TwoHandSkillNotFound = 14,
    WrongBagType = 15,
    WrongBagType2 = 16,
    ItemMaxCount = 17,
    NoSlotAvailable3 = 18,
    CantStack = 19,
    NotEquippable = 20,
    CantSwap = 21,
    SlotEmpty = 22,
    ItemNotFound = 23,
    DropBoundItem = 24,
    OutOfRange = 25,
    TooFewToSplit = 26,
    SplitFailed = 27,
    SpellFailedReagentsGeneric = 28,
    CantTradeGold = 29,
    NotEnoughMoney = 30,
    NotABag = 31,
    DestroyNonemptyBag = 32,
    NotOwner = 33,
    OnlyOneQuiver = 34,
    NoBankSlot = 35,
    NoBankHere = 36,
    ItemLocked = 37,
    GenericStunned = 38,
    PlayerDead = 39,
    ClientLockedOut = 40,
    InternalBagError = 41,
    OnlyOneBolt = 42,
    OnlyOneAmmo = 43,
    CantWrapStackable = 44,
    CantWrapEquipped = 45,
    CantWrapWrapped = 46,
    CantWrapBound = 47,
    CantWrapUnique = 48,
    CantWrapBags = 49,
    LootGone = 50,
    InvFull = 51,
    BankFull = 52,
    VendorSoldOut = 53,
    BagFull2 = 54,
    ItemNotFound2 = 55,
    CantStack2 = 56,
    BagFull3 = 57,
    VendorSoldOut2 = 58,
    ObjectIsBusy = 59,
    CantBeDisenchanted = 60,
    NotInCombat = 61,
    NotWhileDisarmed = 62,
    BagFull4 = 63,
    CantEquipRank = 64,
    CantEquipReputation = 65,
    TooManySpecialBags = 66,
    LootCantLootThatNow = 67,
    ItemUniqueEquippable = 68,
    VendorMissingTurnins = 69,
    NotEnoughHonorPoints = 70,
    NotEnoughArenaPoints = 71,
    ItemMaxCountSocketed = 72,
    MailBoundItem = 73,
    InternalBagError2 = 74,
    BagFull5 = 75,
    ItemMaxCountEquippedSocketed = 76,
    ItemUniqueEquippableSocketed = 77,
    TooMuchGold = 78,
    NotDuringArenaMatch = 79,
    TradeBoundItem = 80,
    CantEquipRating = 81,
    EventAutoequipBindConfirm = 82,
    NotSameAccount = 83,
    EquipNone3 = 84,
    ItemMaxLimitCategoryCountExceededIs = 85,
    ItemMaxLimitCategorySocketedExceededIs = 86,
    ScalingStatItemLevelExceeded = 87,
    PurchaseLevelTooLow = 88,
    CantEquipNeedTalent = 89,
    ItemMaxLimitCategoryEquippedExceededIs = 90,
    ShapeshiftFormCannotEquip = 91,
    ItemInventoryFullSatchel = 92,
    ScalingStatItemLevelTooLow = 93,
    CantBuyQuantity = 94,
    ItemIsBattlePayLocked = 95,
    ReagentBankFull = 96,
    ReagentBankLocked = 97,
    WrongBagType3 = 98,
    CantUseItem = 99,
    CantBeObliterated = 100,
    GuildBankConjuredItem = 101,
    BagFull6 = 102,
    BagFull7 = 103,
    CantBeScrapped = 104,
    BagFull8 = 105,
    NotInPetBattle = 106,
    BagFull9 = 107,
    CantDoThatRightNow = 108,
    CantDoThatRightNow2 = 109,
    NotInNPE = 110,
    ItemCooldown = 111,
    NotInRatedBattleground = 112,
    EquipableSpellsSlotsFull = 113,
    CantBeRecrafted = 114,
    ReagentBagWrongSlot = 115,
    SlotOnlyReagentBag = 116,
    ReagentBagItemType = 117,
    CantBulkSellItemWithRefund = 118,
}

/// Buy result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum BuyResult {
    CantFindItem = 0,
    ItemAlreadySold = 1,
    NotEnoughtMoney = 2,
    SellerDontLikeYou = 4,
    DistanceTooFar = 5,
    ItemSoldOut = 7,
    CantCarryMore = 8,
    RankRequire = 11,
    ReputationRequire = 12,
}

/// Sell result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SellResult {
    CantFindItem = 1,
    CantSellItem = 2,
    CantFindVendor = 3,
    YouDontOwnThatItem = 4,
    Unk = 5,
    OnlyEmptyBag = 6,
    CantSellToThisMerchant = 7,
    MustRepairDurability = 8,
    VendorRefuseScappableAzerite = 9,
    InternalBagError = 10,
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

/// Item vendor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ItemVendorType {
    None = 0,
    Item = 1,
    Currency = 2,
    Spell = 3,
    MawPower = 4,
}

/// Currency types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CurrencyTypes {
    JusticePoints = 395,
    ValorPoints = 396,
    ApexisCrystals = 823,
    Azerite = 1553,
    AncientMana = 1155,
}

/// Item transmogrification weapon category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemTransmogrificationWeaponCategory {
    Melee2H = 0,
    Ranged = 1,
    AxeMaceSword1H = 2,
    Dagger = 3,
    Fist = 4,
    Invalid = 5,
}
