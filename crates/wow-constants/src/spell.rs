// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell-related enums: cast results, schools, mechanics, aura interrupts, etc.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Spell schools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SpellSchools {
    Normal = 0,
    Holy = 1,
    Fire = 2,
    Nature = 3,
    Frost = 4,
    Shadow = 5,
    Arcane = 6,
    Max = 7,
}

/// Spell dispel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum DispelType {
    None = 0,
    Magic = 1,
    Curse = 2,
    Disease = 3,
    Poison = 4,
    Stealth = 5,
    Invisibility = 6,
    ALL = 7,
    SpeNPCOnly = 8,
    Enrage = 9,
    ZGTicket = 10,
    OldUnused = 11,
}

/// Spell mechanics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum Mechanics {
    None = 0,
    Charm = 1,
    Disoriented = 2,
    Disarm = 3,
    Distract = 4,
    Fear = 5,
    Grip = 6,
    Root = 7,
    SlowAttack = 8,
    Silence = 9,
    Sleep = 10,
    Snare = 11,
    Stun = 12,
    Freeze = 13,
    Knockout = 14,
    Bleed = 15,
    Bandage = 16,
    Polymorph = 17,
    Banish = 18,
    Shield = 19,
    Shackle = 20,
    Mount = 21,
    Infected = 22,
    Turn = 23,
    Horror = 24,
    Invulnerability = 25,
    Interrupt = 26,
    Daze = 27,
    Discovery = 28,
    ImmuneShield = 29,
    Sapped = 30,
    Enraged = 31,
    Wounded = 32,
    Infected2 = 33,
    Infected3 = 34,
    Infected4 = 35,
    Taunted = 36,
    Max = 37,
}

/// Spell cast result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SpellCastResult {
    Success = 0,
    AffectingCombat = 1,
    AlreadyAtFullHealth = 2,
    AlreadyAtFullMana = 3,
    AlreadyAtFullPower = 4,
    AlreadyBeingTamed = 5,
    AlreadyHaveCharm = 6,
    AlreadyHaveSummon = 7,
    AlreadyHavePet = 8,
    AlreadyOpen = 9,
    AuraBounced = 10,
    AutotrackInterrupted = 11,
    BadImplicitTargets = 12,
    BadTargets = 13,
    PvpTargetWhileUnflagged = 14,
    CantBeCharmed = 15,
    CantBeSalvaged = 16,
    CantBeSalvagedSkill = 17,
    CantBeEnchanted = 18,
    CantBeMilled = 19,
    CantBeProspected = 20,
    CantCastOnTapped = 21,
    CantDuelWhileInvisible = 22,
    CantDuelWhileStealthed = 23,
    CantStealth = 24,
    CantUntalent = 25,
    CasterAurastate = 26,
    CasterDead = 27,
    Charmed = 28,
    ChestInUse = 29,
    Confused = 30,
    DisabledByPowerScaling = 31,
    DontReport = 32,
    EquippedItem = 33,
    EquippedItemClass = 34,
    EquippedItemClassMainhand = 35,
    EquippedItemClassOffhand = 36,
    Error = 37,
    Falling = 38,
    Fizzle = 39,
    Fleeing = 40,
    FoodLowlevel = 41,
    GarrisonNotOwned = 42,
    GarrisonOwned = 43,
    GarrisonMaxLevel = 44,
    GarrisonNotUpgradeable = 45,
    GarrisonFollowerOnMission = 46,
    GarrisonFollowerInBuilding = 47,
    GarrisonFollowerMaxLevel = 48,
    GarrisonFollowerMinItemLevel = 49,
    GarrisonFollowerMaxItemLevel = 50,
    GarrisonFollowerMaxQuality = 51,
    GarrisonFollowerNotMaxLevel = 52,
    GarrisonFollowerHasAbility = 53,
    GarrisonFollowerHasSingleMissionAbility = 54,
    GarrisonFollowerRequiresEpic = 55,
    GarrisonMissionNotInProgress = 56,
    GarrisonMissionComplete = 57,
    GarrisonNoMissionsAvailable = 58,
    Highlevel = 59,
    HungerSatiated = 60,
    Immune = 61,
    IncorrectArea = 62,
    Interrupted = 63,
    InterruptedCombat = 64,
    ItemAlreadyEnchanted = 65,
    ItemGone = 66,
    ItemNotFound = 67,
    ItemNotReady = 68,
    LegacySpell = 69,
    LevelRequirement = 70,
    LineOfSight = 71,
    Lowlevel = 72,
    LowCastlevel = 73,
    MainhandEmpty = 74,
    Moving = 75,
    NeedAmmo = 76,
    NeedAmmoPouch = 77,
    NeedExoticAmmo = 78,
    NeedMoreItems = 79,
    NoPath = 80,
    NotBehind = 81,
    NotFishable = 82,
    NotFlying = 83,
    NotHere = 84,
    NotInfront = 85,
    NotInControl = 86,
    NotKnown = 87,
    NotMounted = 88,
    NotOnTaxi = 89,
    NotOnTransport = 90,
    NotReady = 91,
    NotShapeshift = 92,
    NotStanding = 93,
    NotTradeable = 94,
    NotTrading = 95,
    NotUnsheathed = 96,
    NotWhileGhost = 97,
    NotWhileLooting = 98,
    NoAmmo = 99,
    NoChargesRemain = 100,
    NoComboPoints = 101,
    NoDueling = 102,
    NoEndurance = 103,
    NoFish = 104,
    NoItemsWhileShapeshifted = 105,
    NoMountsAllowed = 106,
    NoPet = 107,
    NoPower = 108,
    NothingToDispel = 109,
    NothingToSteal = 110,
    OnlyAbovewater = 111,
    OnlyIndoors = 112,
    OnlyMounted = 113,
    OnlyOutdoors = 114,
    OnlyShapeshift = 115,
    OnlyStealthed = 116,
    OnlyUnderwater = 117,
    OutOfRange = 118,
    Pacified = 119,
    Possessed = 120,
    Reagents = 121,
    RequiresArea = 122,
    RequiresSpellFocus = 123,
    Rooted = 124,
    Silenced = 125,
    SpellInProgress = 126,
    SpellLearned = 127,
    SpellUnavailable = 128,
    Stunned = 129,
    TargetsDead = 130,
    TargetAffectingCombat = 131,
    TargetAurastate = 132,
    TargetDueling = 133,
    TargetEnemy = 134,
    TargetEnraged = 135,
    TargetFriendly = 136,
    TargetInCombat = 137,
    TargetInPetBattle = 138,
    TargetIsPlayer = 139,
    TargetIsPlayerControlled = 140,
    TargetNotDead = 141,
    TargetNotInParty = 142,
    TargetNotLooted = 143,
    TargetNotPlayer = 144,
    TargetNoPockets = 145,
    TargetNoWeapons = 146,
    TargetNoRangedWeapons = 147,
    TargetUnskinnable = 148,
    ThirstSatiated = 149,
    TooClose = 150,
    TooManyOfItem = 151,
    TotemCategory = 152,
    Totems = 153,
    TrainingPoints = 154,
    TryAgain = 155,
    UnitNotBehind = 156,
    UnitNotInfront = 157,
    VisionObscured = 158,
    WrongPetFood = 159,
    NotWhileFatigued = 160,
    TargetNotInInstance = 161,
    NotWhileTrading = 162,
    TargetNotInRaid = 163,
    TargetFreeforall = 164,
    NoEdibleCorpses = 165,
    OnlyBattlegrounds = 166,
    TargetNotGhost = 167,
    TooManySkills = 168,
    TransformUnusable = 169,
    WrongWeather = 170,
    DamageImmune = 171,
    PreventedByMechanic = 172,
    PlayTime = 173,
    Reputation = 174,
    MinSkill = 175,
    NotInRatedBattleground = 176,
    NotOnShapeshift = 177,
    NotOnStealthed = 178,
    NotOnDamageImmune = 179,
    NotOnMounted = 180,
    TooShallow = 181,
    TargetNotInSanctuary = 182,
    TargetIsTrivial = 183,
    BmOrInvisgod = 184,
    GroundMountNotAllowed = 185,
    FloatingMountNotAllowed = 186,
    UnderwaterMountNotAllowed = 187,
    FlyingMountNotAllowed = 188,
    ApprenticeRidingRequirement = 189,
    JourneymanRidingRequirement = 190,
    ExpertRidingRequirement = 191,
    ArtisanRidingRequirement = 192,
    MasterRidingRequirement = 193,
    ColdRidingRequirement = 194,
    FlightMasterRidingRequirement = 195,
    CsRidingRequirement = 196,
    PandaRidingRequirement = 197,
    DraenorRidingRequirement = 198,
    BrokenIslesRidingRequirement = 199,
    MountNoFloatHere = 200,
    MountNoUnderwaterHere = 201,
    MountAboveWaterHere = 202,
    MountCollectedOnOtherChar = 203,
    NotIdle = 204,
    NotInactive = 205,
    PartialPlaytime = 206,
    NoPlaytime = 207,
    NotInBattleground = 208,
    NotInRaidInstance = 209,
    OnlyInArena = 210,
    TargetLockedToRaidInstance = 211,
    OnUseEnchant = 212,
    NotOnGround = 213,
    CustomError = 214,
    CantDoThatRightNow = 215,
    TooManySockets = 216,
    InvalidGlyph = 217,
    UniqueGlyph = 218,
    GlyphSocketLocked = 219,
    GlyphExclusiveCategory = 220,
    GlyphInvalidSpec = 221,
    GlyphNoSpec = 222,
    NoActiveGlyphs = 223,
    NoValidTargets = 224,
    ItemAtMaxCharges = 225,
    NotInBarbershop = 226,
    FishingTooLow = 227,
    ItemEnchantTradeWindow = 228,
    SummonPending = 229,
    MaxSockets = 230,
    PetCanRename = 231,
    TargetCannotBeResurrected = 232,
    TargetHasResurrectPending = 233,
    NoActions = 234,
    CurrencyWeightMismatch = 235,
    WeightNotEnough = 236,
    WeightTooMuch = 237,
    NoVacantSeat = 238,
    NoLiquid = 239,
    OnlyNotSwimming = 240,
    ByNotMoving = 241,
    InCombatResLimitReached = 242,
    NotInArena = 243,
    TargetNotGrounded = 244,
    ExceededWeeklyUsage = 245,
    NotInLfgDungeon = 246,
    BadTargetFilter = 247,
    NotEnoughTargets = 248,
    NoSpec = 249,
    CantAddBattlePet = 250,
    CantUpgradeBattlePet = 251,
    WrongBattlePetType = 252,
    NoDungeonEncounter = 253,
    NoTeleportFromDungeon = 254,
    MaxLevelTooLow = 255,
    CantReplaceItemBonus = 256,
    GrantPetLevelFail = 257,
    SkillLineNotKnown = 258,
    BlueprintKnown = 259,
    FollowerKnown = 260,
    CantOverrideEnchantVisual = 261,
    ItemNotAWeapon = 262,
    SameEnchantVisual = 263,
    ToyUseLimitReached = 264,
    ToyAlreadyKnown = 265,
    ShipmentsFull = 266,
    NoShipmentsForContainer = 267,
    NoBuildingForShipment = 268,
    NotEnoughShipmentsForContainer = 269,
    HasMission = 270,
    BuildingActivateNotReady = 271,
    NotSoulbound = 272,
    RidingVehicle = 273,
    VeteranTrialAboveSkillRankMax = 274,
    NotWhileMercenary = 275,
    SpecDisabled = 276,
    CantBeObliterated = 277,
    CantBeScrapped = 278,
    FollowerClassSpecCap = 279,
    TransportNotReady = 280,
    TransmogSetAlreadyKnown = 281,
    DisabledByAuraLabel = 282,
    DisabledByMaxUsableLevel = 283,
    SpellAlreadyKnown = 284,
    MustKnowSupercedingSpell = 285,
    YouCannotUseThatInPvpInstance = 286,
    NoArtifactEquipped = 287,
    WrongArtifactEquipped = 288,
    TargetIsUntargetableByAnyone = 289,
    SpellEffectFailed = 290,
    NeedAllPartyMembers = 291,
    ArtifactAtFullPower = 292,
    ApItemFromPreviousTier = 293,
    AreaTriggerCreation = 294,
    AzeriteEmpoweredOnly = 295,
    AzeriteEmpoweredNoChoicesToUndo = 296,
    WrongFaction = 297,
    NotEnoughCurrency = 298,
    BattleForAzerothRidingRequirement = 299,
    MountEquipmentError = 300,
    NotWhileLevelLinked = 301,
    LevelLinkedLowLevel = 302,
    SummonMapCondition = 303,
    SetCovenantError = 304,
    RuneforgeLegendaryUpgrade = 305,
    SetChromieTimeError = 306,
    IneligibleWeaponAppearance = 307,
    PlayerCondition = 308,
    NotWhileChromieTimed = 309,
    CraftingReagents = 310,
    SpectatorOrCommentator = 311,
    SoulbindConduitLearnFailedInvalidCovenant = 312,
    ShadowlandsRidingRequirement = 313,
    NotInMageTower = 314,
    GarrisonFollowerAtMinLevel = 315,
    CantBeRecrafted = 316,
    PassiveReplaced = 317,
    CantFlyHere = 318,
    DragonridingRidingRequirement = 319,
    ItemModAppearanceGroupAlreadyKnown = 320,
    Unknown = 321,
}

bitflags! {
    /// Spell interrupt flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SpellInterruptFlags: u32 {
        const NONE = 0;
        const MOVEMENT = 0x01;
        const DAMAGE_PUSHBACK_PLAYER_ONLY = 0x02;
        const STUN = 0x04;
        const COMBAT = 0x08;
        const DAMAGE_CANCELS_PLAYER_ONLY = 0x10;
        const MELEE_COMBAT = 0x20;
        const IMMUNITY = 0x40;
        const DAMAGE_ABSORB = 0x80;
        const ZERO_DAMAGE_CANCELS = 0x100;
        const DAMAGE_PUSHBACK = 0x200;
        const DAMAGE_CANCELS = 0x400;
    }
}

bitflags! {
    /// Spell aura interrupt flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SpellAuraInterruptFlags: u32 {
        const NONE = 0;
        const HOSTILE_ACTION_RECEIVED = 0x01;
        const DAMAGE = 0x02;
        const ACTION = 0x04;
        const MOVING = 0x08;
        const TURNING = 0x10;
        const ANIM = 0x20;
        const DISMOUNT = 0x40;
        const UNDER_WATER = 0x80;
        const ABOVE_WATER = 0x100;
        const SHEATHING = 0x200;
        const INTERACTING = 0x400;
        const LOOTING = 0x800;
        const ATTACKING = 0x1000;
        const ITEM_USE = 0x2000;
        const DAMAGE_CHANNEL_DURATION = 0x4000;
        const SHAPESHIFTING = 0x8000;
        const ACTION_DELAYED = 0x10000;
        const MOUNT = 0x20000;
        const STANDING = 0x40000;
        const LEAVE_WORLD = 0x80000;
        const STEALTH_OR_INVIS = 0x100000;
        const INVULNERABILITY_BUFF = 0x200000;
        const ENTER_WORLD = 0x400000;
        const PVP_ACTIVE = 0x800000;
        const NON_PERIODIC_DAMAGE = 0x1000000;
        const LANDING_OR_FLIGHT = 0x2000000;
        const RELEASE = 0x4000000;
        const DAMAGE_CANCELS_SCRIPT = 0x8000000;
        const ENTERING_COMBAT = 0x10000000;
        const LOGIN = 0x20000000;
        const SUMMON = 0x40000000;
        const LEAVING_COMBAT = 0x80000000;

        const NOT_VICTIM = Self::HOSTILE_ACTION_RECEIVED.bits()
            | Self::DAMAGE.bits() | Self::NON_PERIODIC_DAMAGE.bits();
    }
}

/// Spell immunity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SpellImmunity {
    Effect = 0,
    State = 1,
    School = 2,
    Damage = 3,
    Dispel = 4,
    Mechanic = 5,
    Id = 6,
    Max = 7,
}

/// Spell modification operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SpellModOp {
    HealingAndDamage = 0,
    Duration = 1,
    Hate = 2,
    PointsIndex0 = 3,
    ProcCharges = 4,
    Range = 5,
    Radius = 6,
    CritChance = 7,
    Points = 8,
    ResistPushback = 9,
    ChangeCastTime = 10,
    Cooldown = 11,
    PointsIndex1 = 12,
    TargetResistance = 13,
    PowerCost0 = 14,
    CritDamageAndHealing = 15,
    HitChance = 16,
    ChainTargets = 17,
    ProcChance = 18,
    Period = 19,
    ChainAmplitude = 20,
    StartCooldown = 21,
    PeriodicHealingAndDamage = 22,
    PointsIndex2 = 23,
    BonusCoefficient = 24,
    TriggerDamage = 25,
    ProcFrequency = 26,
    Amplitude = 27,
    DispelResistance = 28,
    CrowdDamage = 29,
    PowerCostOnMiss = 30,
    Doses = 31,
    PointsIndex3 = 32,
    PointsIndex4 = 33,
    PowerCost1 = 34,
    ChainJumpDistance = 35,
    AreaTriggerMaxSummons = 36,
    MaxAuraStacks = 37,
    ProcCooldown = 38,
    PowerCost2 = 39,
    Max = 40,
}

/// Spell modification types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SpellModType {
    Flat = 0,
    Pct = 1,
    LabelFlat = 2,
    LabelPct = 3,
    End = 4,
}

/// Spell state during casting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SpellState {
    None = 0,
    Preparing = 1,
    Casting = 2,
    Finished = 3,
    Idle = 4,
    Delayed = 5,
}

bitflags! {
    /// Spell range flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SpellRangeFlag: u8 {
        const DEFAULT = 0;
        const MELEE = 1;
        const RANGED = 2;
    }
}
