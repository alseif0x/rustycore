//! Stats packets.
//!
//! Separated from item.rs under #701.

use super::*;

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
    /// C++ `CurrencyTypesFlags`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CurrencyTypesFlags: u32 {
        const TRADABLE = 0x00000001;
        const APPEARS_IN_LOOT_WINDOW = 0x00000002;
        const COMPUTED_WEEKLY_MAXIMUM = 0x00000004;
        const SCALER_100 = 0x00000008;
        const NO_LOW_LEVEL_DROP = 0x00000010;
        const IGNORE_MAX_QTY_ON_LOAD = 0x00000020;
        const LOG_ON_WORLD_CHANGE = 0x00000040;
        const TRACK_QUANTITY = 0x00000080;
        const RESET_TRACKED_QUANTITY = 0x00000100;
        const UPDATE_VERSION_IGNORE_MAX = 0x00000200;
        const SUPPRESS_CHAT_MESSAGE_ON_VERSION_CHANGE = 0x00000400;
        const SINGLE_DROP_IN_LOOT = 0x00000800;
        const HAS_WEEKLY_CATCHUP = 0x00001000;
        const DO_NOT_COMPRESS_CHAT = 0x00002000;
        const DO_NOT_LOG_ACQUISITION_TO_BI = 0x00004000;
        const NO_RAID_DROP = 0x00008000;
        const NOT_PERSISTENT = 0x00010000;
        const DEPRECATED = 0x00020000;
        const DYNAMIC_MAXIMUM = 0x00040000;
        const SUPPRESS_CHAT_MESSAGES = 0x00080000;
        const DO_NOT_TOAST = 0x00100000;
        const DESTROY_EXTRA_ON_LOOT = 0x00200000;
        const DONT_SHOW_TOTAL_IN_TOOLTIP = 0x00400000;
        const DONT_COALESCE_IN_LOOT_WINDOW = 0x00800000;
        const ACCOUNT_WIDE = 0x01000000;
        const ALLOW_OVERFLOW_MAILER = 0x02000000;
        const HIDE_AS_REWARD = 0x04000000;
        const HAS_WARMODE_BONUS = 0x08000000;
        const IS_ALLIANCE_ONLY = 0x10000000;
        const IS_HORDE_ONLY = 0x20000000;
        const LIMIT_WARMODE_BONUS_ONCE_PER_TOOLTIP = 0x40000000;
        const DEPRECATED_CURRENCY_FLAG = 0x80000000;
    }
}

bitflags! {
    /// C++ `CurrencyTypesFlagsB`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CurrencyTypesFlagsB: u32 {
        const USE_TOTAL_EARNED_FOR_EARNED = 0x01;
        const SHOW_QUEST_XP_GAIN_IN_TOOLTIP = 0x02;
        const NO_NOTIFICATION_MAIL_ON_OFFLINE_PROGRESS = 0x04;
        const BATTLENET_VIRTUAL_CURRENCY = 0x08;
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
