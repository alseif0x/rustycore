// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared constants and enums used across the WoW server.
//! Includes Locale, Faction, Reputation, Difficulty, Map flags, etc.
//!
//! Note: Race, Class, Gender, PowerType, Stats, Expansion, ChatMsg, and Team
//! are defined in the `unit` module to avoid duplication.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Locale identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum Locale {
    EnUS = 0,
    KoKR = 1,
    FrFR = 2,
    DeDE = 3,
    ZhCN = 4,
    ZhTW = 5,
    EsES = 6,
    EsMX = 7,
    RuRU = 8,
    None = 9,
    PtBR = 10,
    ItIT = 11,
    Total = 12,
    AllLanguages = -1,
}

bitflags! {
    /// Locale bitmask.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LocaleMask: u16 {
        const EN_US = 1 << 0;
        const KO_KR = 1 << 1;
        const FR_FR = 1 << 2;
        const DE_DE = 1 << 3;
        const ZH_CN = 1 << 4;
        const ZH_TW = 1 << 5;
        const ES_ES = 1 << 6;
        const ES_MX = 1 << 7;
        const RU_RU = 1 << 8;
        const NONE  = 1 << 9;
        const PT_BR = 1 << 10;
        const IT_IT = 1 << 11;
        const TOTAL = (1 << 12) - 1;
    }
}

/// CASC locale bit indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CascLocaleBit {
    None = 0,
    EnUS = 1,
    KoKR = 2,
    Reserved = 3,
    FrFR = 4,
    DeDE = 5,
    ZhCN = 6,
    EsES = 7,
    ZhTW = 8,
    EnGB = 9,
    EnCN = 10,
    EnTW = 11,
    EsMX = 12,
    RuRU = 13,
    PtBR = 14,
    ItIT = 15,
    PtPT = 16,
}

/// Comparison type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ComparisionType {
    EQ = 0,
    High = 1,
    Low = 2,
    HighEQ = 3,
    LowEQ = 4,
}

/// XP color character classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum XPColorChar {
    Red = 0,
    Orange = 1,
    Yellow = 2,
    Green = 3,
    Gray = 4,
}

/// Content level ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ContentLevels {
    Content1_60 = 0,
    Content61_70 = 1,
    Content71_80 = 2,
}

bitflags! {
    /// Faction masks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FactionMasks: u8 {
        const PLAYER    = 1;
        const ALLIANCE  = 2;
        const HORDE     = 4;
        const MONSTER   = 8;
    }
}

bitflags! {
    /// Faction template flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FactionTemplateFlags: u16 {
        const PVP                   = 0x800;
        const CONTESTED_GUARD       = 0x1000;
        const HOSTILE_BY_DEFAULT    = 0x2000;
    }
}

/// Reputation rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum ReputationRank {
    None = -1,
    Hated = 0,
    Hostile = 1,
    Unfriendly = 2,
    Neutral = 3,
    Friendly = 4,
    Honored = 5,
    Revered = 6,
    Exalted = 7,
}

/// Faction template IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum FactionTemplates {
    None = 0,
    Creature = 7,
    EscorteeANeutralPassive = 10,
    Monster = 14,
    Monster2 = 16,
    TrollBloodscalp = 28,
    Prey = 31,
    EscorteeHNeutralPassive = 33,
    Friendly = 35,
    TrollFrostmane = 37,
    Ogre = 45,
    OrcDragonmaw = 62,
    HordeGeneric = 83,
    AllianceGeneric = 84,
    Demon = 90,
    Elemental = 91,
    DragonflightBlack = 103,
    EscorteeNNeutralPassive = 113,
    Enemy = 168,
    EscorteeANeutralActive = 231,
    EscorteeHNeutralActive = 232,
    EscorteeNNeutralActive = 250,
    EscorteeNFriendPassive = 290,
    Titan = 415,
    EscorteeNFriendActive = 495,
    Ratchet = 637,
    GoblinDarkIronBarPatron = 736,
    DarkIronDwarves = 754,
    EscorteeAPassive = 774,
    EscorteeHPassive = 775,
    UndeadScourge = 974,
    EarthenRing = 1726,
    AllianceGenericWg = 1732,
    HordeGenericWg = 1735,
    Arakkoa = 1738,
    AshtongueDeathsworn = 1820,
    FlayerHunter = 1840,
    MonsterSparBuddy = 1868,
    EscorteeNActive = 1986,
    EscorteeHActive = 2046,
    UndeadScourge2 = 2068,
    UndeadScourge3 = 2084,
    ScarletCrusade = 2089,
    ScarletCrusade2 = 2096,
}

/// Reputation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ReputationSource {
    Kill = 0,
    Quest = 1,
    DailyQuest = 2,
    WeeklyQuest = 3,
    MonthlyQuest = 4,
    RepeatableQuest = 5,
    Spell = 6,
}

bitflags! {
    /// Reputation flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ReputationFlags: u16 {
        const NONE                      = 0x00;
        const VISIBLE                   = 0x01;
        const AT_WAR                    = 0x02;
        const HIDDEN                    = 0x04;
        const HEADER                    = 0x08;
        const PEACEFUL                  = 0x10;
        const INACTIVE                  = 0x20;
        const SHOW_PROPAGATED           = 0x40;
        const HEADER_SHOWS_BAR          = 0x80;
        const CAPITAL_CITY_FOR_RACE_CHANGE = 0x100;
        const GUILD                     = 0x200;
        const GARRISON_INVASION         = 0x400;
    }
}

/// Game table class indices (different ordering from Class enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum GtClass {
    None = 0,
    Rogue = 1,
    Druid = 2,
    Hunter = 3,
    Mage = 4,
    Paladin = 5,
    Priest = 6,
    Shaman = 7,
    Warlock = 8,
    Warrior = 9,
    DeathKnight = 10,
    Monk = 11,
    DemonHunter = 12,
}

/// Trainer type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum TrainerType {
    None = 0,
    Talent = 1,
    Tradeskills = 2,
    Pets = 3,
}

/// Trainer spell state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum TrainerSpellState {
    Known = 0,
    Available = 1,
    Unavailable = 2,
}

/// Trainer failure reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum TrainerFailReason {
    Unavailable = 0,
    NotEnoughMoney = 1,
}

/// Chat restriction type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ChatRestrictionType {
    Restricted = 0,
    Throttled = 1,
    Squelched = 2,
    YellRestricted = 3,
    RaidRestricted = 4,
}

/// Curve interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CurveInterpolationMode {
    Linear = 0,
    Cosine = 1,
    CatmullRom = 2,
    Bezier3 = 3,
    Bezier4 = 4,
    Bezier = 5,
    Constant = 6,
}

/// Dungeon/Raid difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Difficulty {
    None = 0,
    Normal = 1,
    Heroic = 2,
    Raid10N = 3,
    Raid25N = 4,
    Raid10HC = 5,
    Raid25HC = 6,
    LFR = 7,
    MythicKeystone = 8,
    Raid40 = 9,
    Scenario3ManHC = 11,
    Scenario3ManN = 12,
    NormalRaid = 14,
    HeroicRaid = 15,
    MythicRaid = 16,
    LFRNew = 17,
    EventRaid = 18,
    EventDungeon = 19,
    EventScenario = 20,
    Mythic = 23,
    Timewalking = 24,
    WorldPvPScenario = 25,
    Scenario5ManN = 26,
    Scenario20ManN = 27,
    PvEvPScenario = 29,
    EventScenario6 = 30,
    WorldPvPScenario2 = 32,
    TimewalkingRaid = 33,
    Pvp = 34,
    NormalIsland = 38,
    HeroicIsland = 39,
    MythicIsland = 40,
    PvpIsland = 45,
    NormalWarfront = 147,
    HeroicWarfront = 149,
    LFR15thAnniversary = 151,
    VisionsOfNzoth = 152,
    TeemingIsland = 153,
}

bitflags! {
    /// Difficulty flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DifficultyFlags: u8 {
        const HEROIC_STYLE_LOCKOUTS = 0x01;
        const DEFAULT               = 0x02;
        const CAN_SELECT            = 0x04;
        const LFG_ONLY              = 0x10;
        const LEGACY                = 0x20;
        const DISPLAY_HEROIC        = 0x40;
        const DISPLAY_MYTHIC        = 0x80;
    }
}

bitflags! {
    /// Map flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MapFlags: u32 {
        const OPTIMIZE                          = 0x01;
        const DEVELOPMENT_MAP                   = 0x02;
        const WEIGHTED_BLEND                    = 0x04;
        const VERTEX_COLORING                   = 0x08;
        const SORT_OBJECTS                      = 0x10;
        const LIMIT_TO_PLAYERS_FROM_ONE_REALM   = 0x20;
        const ENABLE_LIGHTING                   = 0x40;
        const INVERTED_TERRAIN                  = 0x80;
        const DYNAMIC_DIFFICULTY                = 0x100;
        const OBJECT_FILE                       = 0x200;
        const TEXTURE_FILE                      = 0x400;
        const GENERATE_NORMALS                  = 0x800;
        const FIX_BORDER_SHADOW_SEAMS           = 0x1000;
        const INFINITE_OCEAN                    = 0x2000;
        const UNDERWATER_MAP                    = 0x4000;
        const FLEXIBLE_RAID_LOCKING             = 0x8000;
        const LIMIT_FARCLIP                     = 0x10000;
        const USE_PARENT_MAP_FLIGHT_BOUNDS      = 0x20000;
        const NO_RACE_CHANGE_ON_THIS_MAP        = 0x40000;
        const DISABLED_FOR_NON_GMS              = 0x80000;
        const WEIGHTED_NORMALS_1                = 0x100000;
        const DISABLE_LOW_DETAIL_TERRAIN        = 0x200000;
        const ENABLE_ORG_ARENA_BLINK_RULE       = 0x400000;
        const WEIGHTED_HEIGHT_BLEND             = 0x800000;
        const COALESCING_AREA_SHARING           = 0x1000000;
        const PROVING_GROUNDS                   = 0x2000000;
        const GARRISON                          = 0x4000000;
        const ENABLE_AI_NEED_SYSTEM             = 0x8000000;
        const SINGLE_V_SERVER                   = 0x10000000;
        const USE_INSTANCE_POOL                 = 0x20000000;
        const MAP_USES_RAID_GRAPHICS            = 0x40000000;
        const FORCE_CUSTOM_UI_MAP               = 0x80000000;
    }
}

bitflags! {
    /// Map flags 2.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MapFlags2: u32 {
        const DONT_ACTIVATE_SHOW_MAP                        = 0x01;
        const NO_VOTE_KICKS                                 = 0x02;
        const NO_INCOMING_TRANSFERS                         = 0x04;
        const DONT_VOXELIZE_PATH_DATA                       = 0x08;
        const TERRAIN_LOD                                   = 0x10;
        const UNCLAMPED_POINT_LIGHTS                        = 0x20;
        const PVP                                           = 0x40;
        const IGNORE_INSTANCE_FARM_LIMIT                    = 0x80;
        const DONT_INHERIT_AREA_LIGHTS_FROM_PARENT          = 0x100;
        const FORCE_LIGHT_BUFFER_ON                         = 0x200;
        const WMO_LIQUID_SCALE                              = 0x400;
        const SPELL_CLUTTER_ON                              = 0x800;
        const SPELL_CLUTTER_OFF                             = 0x1000;
        const REDUCED_PATH_MAP_HEIGHT_VALIDATION            = 0x2000;
        const NEW_MINIMAP_GENERATION                        = 0x4000;
        const AI_BOTS_DETECTED_LIKE_PLAYERS                 = 0x8000;
        const LINEARLY_LIT_TERRAIN                          = 0x10000;
        const FOG_OF_WAR                                    = 0x20000;
        const DISABLE_SHARED_WEATHER_SYSTEMS                = 0x40000;
        const HONOR_SPELL_ATTRIBUTE_11_LOS_HITS_NOCAMCOLLIDE = 0x80000;
        const BELONGS_TO_LAYER                              = 0x100000;
    }
}

/// Loot type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum LootType {
    None = 0,
    Corpse = 1,
    Pickpocketing = 2,
    Fishing = 3,
    Disenchanting = 4,
    Skinning = 6,
    Prospecting = 7,
    Milling = 8,
    Insignia = 21,
    Fishinghole = 22,
    FishingJunk = 25,
}

/// Scaling class for item stat scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum ScalingClass {
    Unknown2 = -9,
    Unknown = -8,
    Item2 = -7,
    Health = -6,
    Gem3 = -5,
    Gem2 = -4,
    Gem1 = -3,
    Consumable = -2,
    Item1 = -1,
    None = 0,
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
}
