// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit-related enums: powers, flags, race, class, gender, etc.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Player/unit power types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum PowerType {
    Health = -2,
    None = -1,
    Mana = 0,
    Rage = 1,
    Focus = 2,
    Energy = 3,
    Happiness = 4,
    Runes = 5,
    RunicPower = 6,
    SoulShards = 7,
    LunarPower = 8,
    HolyPower = 9,
    AlternatePower = 10,
    Maelstrom = 11,
    Chi = 12,
    Insanity = 13,
    ComboPoints = 14,
    DemonicFury = 15,
    ArcaneCharges = 16,
    Fury = 17,
    Pain = 18,
    Essence = 19,
    RuneBlood = 20,
    RuneFrost = 21,
    RuneUnholy = 22,
    AlternateQuest = 23,
    AlternateEncounter = 24,
    AlternateMount = 25,
    Max = 26,
}

/// Player gender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum Gender {
    Unknown = -1,
    Male = 0,
    Female = 1,
    None = 2,
}

/// Player race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Race {
    None = 0,
    Human = 1,
    Orc = 2,
    Dwarf = 3,
    NightElf = 4,
    Undead = 5,
    Tauren = 6,
    Gnome = 7,
    Troll = 8,
    Goblin = 9,
    BloodElf = 10,
    Draenei = 11,
    Worgen = 22,
    PandarenNeutral = 24,
    PandarenAlliance = 25,
    PandarenHorde = 26,
    Nightborne = 27,
    HighmountainTauren = 28,
    VoidElf = 29,
    LightforgedDraenei = 30,
    ZandalariTroll = 31,
    KulTiran = 32,
    DarkIronDwarf = 34,
    Vulpera = 35,
    MagharOrc = 36,
    MechaGnome = 37,
    DracthyrAlliance = 52,
    DracthyrHorde = 70,
    Max = 78,
}

/// Player class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Class {
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
    DemonHunter = 12,
    Evoker = 13,
    Adventurer = 14,
}

/// Expansion identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum Expansion {
    Unk = -2,
    LevelCurrent = -1,
    Classic = 0,
    BurningCrusade = 1,
    WrathOfTheLichKing = 2,
    Cataclysm = 3,
    MistsOfPandaria = 4,
    WarlordsOfDraenor = 5,
    Legion = 6,
    BattleForAzeroth = 7,
    ShadowLands = 8,
    Dragonflight = 9,
}

/// Player stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum Stats {
    Strength = 0,
    Agility = 1,
    Stamina = 2,
    Intellect = 3,
    Spirit = 4,
    Max = 5,
}

bitflags! {
    /// Primary unit flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UnitFlags: u32 {
        const SERVER_CONTROLLED             = 0x01;
        const NON_ATTACKABLE                = 0x02;
        const REMOVE_CLIENT_CONTROL         = 0x04;
        const PLAYER_CONTROLLED             = 0x08;
        const RENAME                        = 0x10;
        const PREPARATION                   = 0x20;
        const UNK6                          = 0x40;
        const NOT_ATTACKABLE_1              = 0x80;
        const IMMUNE_TO_PC                  = 0x100;
        const IMMUNE_TO_NPC                 = 0x200;
        const LOOTING                       = 0x400;
        const PET_IN_COMBAT                 = 0x800;
        const PVP_ENABLING                  = 0x1000;
        const SILENCED                      = 0x2000;
        const CANT_SWIM                     = 0x4000;
        const CAN_SWIM                      = 0x8000;
        const NON_ATTACKABLE_2              = 0x10000;
        const PACIFIED                      = 0x20000;
        const STUNNED                       = 0x40000;
        const IN_COMBAT                     = 0x80000;
        const ON_TAXI                       = 0x100000;
        const DISARMED                      = 0x200000;
        const CONFUSED                      = 0x400000;
        const FLEEING                       = 0x800000;
        const POSSESSED                     = 0x1000000;
        const UNINTERACTIBLE                = 0x2000000;
        const SKINNABLE                     = 0x4000000;
        const MOUNT                         = 0x8000000;
        const UNK28                         = 0x10000000;
        const PREVENT_EMOTES_FROM_CHAT_TEXT = 0x20000000;
        const SHEATHE                       = 0x40000000;
        const IMMUNE                        = 0x80000000;
    }
}

bitflags! {
    /// Secondary unit flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UnitFlags2: u32 {
        const FEIGN_DEATH                                      = 0x01;
        const HIDE_BODY                                        = 0x02;
        const IGNORE_REPUTATION                                = 0x04;
        const COMPREHEND_LANG                                  = 0x08;
        const MIRROR_IMAGE                                     = 0x10;
        const DONT_FADE_IN                                     = 0x20;
        const FORCE_MOVEMENT                                   = 0x40;
        const DISARM_OFFHAND                                   = 0x80;
        const DISABLE_PRED_STATS                               = 0x100;
        const ALLOW_CHANGING_TALENTS                           = 0x200;
        const DISARM_RANGED                                    = 0x400;
        const REGENERATE_POWER                                 = 0x800;
        const RESTRICT_PARTY_INTERACTION                       = 0x1000;
        const PREVENT_SPELL_CLICK                              = 0x2000;
        const INTERACT_WHILE_HOSTILE                           = 0x4000;
        const CANNOT_TURN                                      = 0x8000;
        const UNK2                                             = 0x10000;
        const PLAY_DEATH_ANIM                                  = 0x20000;
        const ALLOW_CHEAT_SPELLS                               = 0x40000;
        const SUPPRESS_HIGHLIGHT_WHEN_TARGETED_OR_MOUSED_OVER  = 0x00080000;
        const TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS            = 0x100000;
        const LARGE_AOI                                        = 0x00200000;
        const GIGANTIC_AOI                                     = 0x400000;
        const NO_ACTIONS                                       = 0x800000;
        const AI_WILL_ONLY_SWIM_IF_TARGET_SWIMS                = 0x1000000;
        const DONT_GENERATE_COMBAT_LOG_WHEN_ENGAGED_WITH_NPCS  = 0x2000000;
        const UNTARGETABLE_BY_CLIENT                           = 0x4000000;
        const ATTACKER_IGNORES_MINIMUM_RANGES                  = 0x8000000;
        const UNINTERACTIBLE_IF_HOSTILE                        = 0x10000000;
        const UNUSED11                                         = 0x20000000;
        const INFINITE_AOI                                     = 0x40000000;
        const UNUSED13                                         = 0x80000000;
    }
}

bitflags! {
    /// Tertiary unit flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UnitFlags3: u32 {
        const UNK0                                          = 0x01;
        const UNCONSCIOUS_ON_DEATH                          = 0x02;
        const ALLOW_MOUNTED_COMBAT                          = 0x04;
        const GARRISON_PET                                  = 0x08;
        const UI_CAN_GET_POSITION                           = 0x10;
        const AI_OBSTACLE                                   = 0x20;
        const ALTERNATIVE_DEFAULT_LANGUAGE                   = 0x40;
        const SUPPRESS_ALL_NPC_FEEDBACK                     = 0x80;
        const IGNORE_COMBAT                                 = 0x100;
        const SUPPRESS_NPC_FEEDBACK                         = 0x200;
        const UNK10                                         = 0x400;
        const UNK11                                         = 0x800;
        const UNK12                                         = 0x1000;
        const FAKE_DEAD                                     = 0x2000;
        const NO_FACING_ON_INTERACT_AND_FAST_FACING_CHASE   = 0x4000;
        const UNTARGETABLE_FROM_UI                          = 0x8000;
        const NO_FACING_ON_INTERACT_WHILE_FAKE_DEAD         = 0x10000;
        const ALREADY_SKINNED                               = 0x20000;
        const SUPPRESS_ALL_NPC_SOUNDS                       = 0x40000;
        const SUPPRESS_NPC_SOUNDS                           = 0x80000;
        const ALLOW_INTERACTION_WHILE_IN_COMBAT             = 0x100000;
        const UNK21                                         = 0x200000;
        const DONT_FADE_OUT                                 = 0x400000;
        const UNK23                                         = 0x800000;
        const FORCE_HIDE_NAMEPLATE                          = 0x1000000;
        const UNK25                                         = 0x2000000;
        const UNK26                                         = 0x4000000;
        const UNK27                                         = 0x8000000;
        const UNK28                                         = 0x10000000;
        const UNK29                                         = 0x20000000;
        const UNK30                                         = 0x40000000;
        const UNK31                                         = 0x80000000;
    }
}

bitflags! {
    /// Unit state flags (internal server tracking).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UnitState: u32 {
        const DIED                  = 0x01;
        const MELEE_ATTACKING       = 0x02;
        const CHARMED               = 0x04;
        const STUNNED               = 0x08;
        const ROAMING               = 0x10;
        const CHASE                 = 0x20;
        const FOCUSING              = 0x40;
        const FLEEING               = 0x80;
        const IN_FLIGHT             = 0x100;
        const FOLLOW                = 0x200;
        const ROOT                  = 0x400;
        const CONFUSED              = 0x800;
        const DISTRACTED            = 0x1000;
        const ISOLATED              = 0x2000;
        const ATTACK_PLAYER         = 0x4000;
        const CASTING               = 0x8000;
        const POSSESSED             = 0x10000;
        const CHARGING              = 0x20000;
        const JUMPING               = 0x40000;
        const FOLLOW_FORMATION      = 0x80000;
        const MOVE                  = 0x100000;
        const ROTATING              = 0x200000;
        const EVADE                 = 0x400000;
        const ROAMING_MOVE          = 0x800000;
        const CONFUSED_MOVE         = 0x1000000;
        const FLEEING_MOVE          = 0x2000000;
        const CHASE_MOVE            = 0x4000000;
        const FOLLOW_MOVE           = 0x8000000;
        const IGNORE_PATHFINDING    = 0x10000000;
        const FOLLOW_FORMATION_MOVE = 0x20000000;

        const MOVING = Self::ROAMING_MOVE.bits() | Self::CONFUSED_MOVE.bits()
            | Self::FLEEING_MOVE.bits() | Self::CHASE_MOVE.bits()
            | Self::FOLLOW_MOVE.bits() | Self::FOLLOW_FORMATION_MOVE.bits();

        const CONTROLLED = Self::CONFUSED.bits() | Self::STUNNED.bits() | Self::FLEEING.bits();

        const LOST_CONTROL = Self::CONTROLLED.bits() | Self::POSSESSED.bits()
            | Self::JUMPING.bits() | Self::CHARGING.bits();

        const NOT_MOVE = Self::ROOT.bits() | Self::STUNNED.bits()
            | Self::DIED.bits() | Self::DISTRACTED.bits();

        const ALL_STATE = 0xffffffff;
    }
}

bitflags! {
    /// NPC flags (first set).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct NPCFlags1: u32 {
        const NONE                = 0x00;
        const GOSSIP              = 0x01;
        const QUEST_GIVER         = 0x02;
        const UNK1                = 0x04;
        const UNK2                = 0x08;
        const TRAINER             = 0x10;
        const TRAINER_CLASS       = 0x20;
        const TRAINER_PROFESSION  = 0x40;
        const VENDOR              = 0x80;
        const VENDOR_AMMO         = 0x100;
        const VENDOR_FOOD         = 0x200;
        const VENDOR_POISON       = 0x400;
        const VENDOR_REAGENT      = 0x800;
        const REPAIR              = 0x1000;
        const FLIGHT_MASTER       = 0x2000;
        const SPIRIT_HEALER       = 0x4000;
        const AREA_SPIRIT_HEALER  = 0x8000;
        const INNKEEPER           = 0x10000;
        const BANKER              = 0x20000;
        const PETITIONER          = 0x40000;
        const TABARD_DESIGNER     = 0x80000;
        const BATTLE_MASTER       = 0x100000;
        const AUCTIONEER          = 0x200000;
        const STABLE_MASTER       = 0x400000;
        const GUILD_BANKER        = 0x800000;
        const SPELL_CLICK         = 0x1000000;
        const PLAYER_VEHICLE      = 0x2000000;
        const MAILBOX             = 0x4000000;
        const ARTIFACT_POWER_RESPEC = 0x8000000;
        const TRANSMOGRIFIER      = 0x10000000;
        const VAULT_KEEPER        = 0x20000000;
        const WILD_BATTLE_PET     = 0x40000000;
        const BLACK_MARKET        = 0x80000000;
    }
}

bitflags! {
    /// NPC flags (second set).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct NPCFlags2: u32 {
        const NONE                                          = 0x00;
        const ITEM_UPGRADE_MASTER                           = 0x01;
        const GARRISON_ARCHITECT                            = 0x02;
        const STEERING                                      = 0x04;
        const AREA_SPIRIT_HEALER_INDIVIDUAL                 = 0x08;
        const SHIPMENT_CRAFTER                              = 0x10;
        const GARRISON_MISSION_NPC                          = 0x20;
        const TRADESKILL_NPC                                = 0x40;
        const BLACK_MARKET_VIEW                             = 0x80;
        const GARRISON_TALENT_NPC                           = 0x200;
        const CONTRIBUTION_COLLECTOR                        = 0x400;
        const AZERITE_RESPEC                                = 0x4000;
        const ISLANDS_QUEUE                                 = 0x8000;
        const SUPPRESS_NPC_SOUNDS_EXCEPT_END_OF_INTERACTION = 0x10000;
        const PERSONAL_TABARD_DESIGNER                      = 0x200000;
    }
}

bitflags! {
    /// Hit information flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct HitInfo: u32 {
        const NORMAL_SWING      = 0x0;
        const UNK1              = 0x01;
        const AFFECTS_VICTIM    = 0x02;
        const OFF_HAND          = 0x04;
        const UNK2              = 0x08;
        const MISS              = 0x10;
        const FULL_ABSORB       = 0x20;
        const PARTIAL_ABSORB    = 0x40;
        const FULL_RESIST       = 0x80;
        const PARTIAL_RESIST    = 0x100;
        const CRITICAL_HIT      = 0x200;
        const UNK10             = 0x400;
        const UNK11             = 0x800;
        const UNK12             = 0x1000;
        const BLOCK             = 0x2000;
        const UNK14             = 0x4000;
        const UNK15             = 0x8000;
        const GLANCING          = 0x10000;
        const CRUSHING          = 0x20000;
        const NO_ANIMATION      = 0x40000;
        const UNK19             = 0x80000;
        const UNK20             = 0x100000;
        const SWING_NO_HIT_SOUND = 0x200000;
        const UNK22             = 0x00400000;
        const RAGE_GAIN         = 0x800000;
        const FAKE_DAMAGE       = 0x1000000;
    }
}

/// Unit movement types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum UnitMoveType {
    Walk = 0,
    Run = 1,
    RunBack = 2,
    Swim = 3,
    SwimBack = 4,
    TurnRate = 5,
    Flight = 6,
    FlightBack = 7,
    PitchRate = 8,
    Max = 9,
}

/// Weapon attack type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum WeaponAttackType {
    BaseAttack = 0,
    OffAttack = 1,
    RangedAttack = 2,
    Max = 3,
}

/// Death state of a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum DeathState {
    Alive = 0,
    JustDied = 1,
    Corpse = 2,
    Dead = 3,
    JustRespawned = 4,
}

/// Unit stand state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum UnitStandStateType {
    Stand = 0,
    Sit = 1,
    SitChair = 2,
    Sleep = 3,
    SitLowChair = 4,
    SitMediumChair = 5,
    SitHighChair = 6,
    Dead = 7,
    Kneel = 8,
    Submerged = 9,
    Max = 10,
}

/// Shapeshift forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ShapeShiftForm {
    None = 0,
    CatForm = 1,
    TreeForm = 2,
    TravelForm = 3,
    AquaticForm = 4,
    BearForm = 5,
    Ambient = 6,
    Ghoul = 7,
    DireBearForm = 8,
    CraneStance = 9,
    TharonjaSkeleton = 10,
    DarkmoonTestOfStrength = 11,
    BlbPlayer = 12,
    ShadowDance = 13,
    CreatureBear = 14,
    CreatureCat = 15,
    GhostWolf = 16,
    BattleStance = 17,
    DefensiveStance = 18,
    BerserkerStance = 19,
    SerpentStance = 20,
    Zombie = 21,
    Metamorphosis = 22,
    OxStance = 23,
    TigerStance = 24,
    Undead = 25,
    Frenzy = 26,
    FlightEpicForm = 27,
    Shadowform = 28,
    FlightForm = 29,
    Stealth = 30,
    MoonkinForm = 31,
    SpiritOfRedemption = 32,
    GladiatorStance = 33,
    Metamorphosis2 = 34,
    MoonkinRestoration = 35,
    TreantForm = 36,
    SpiritOwlForm = 37,
    SpiritOwl2 = 38,
    WispForm = 39,
    Wisp2 = 40,
    Soulshape = 41,
    ForgeborneReveries = 42,
}

/// Sheath state for weapons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SheathState {
    Unarmed = 0,
    Melee = 1,
    Ranged = 2,
    Max = 3,
}

/// Damage effect types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum DamageEffectType {
    Direct = 0,
    SpellDirect = 1,
    DOT = 2,
    Heal = 3,
    NoDamage = 4,
    SelfDamage = 5,
}

/// Unit dynamic flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum UnitDynFlags {
    None = 0x00,
    HideModel = 0x02,
    Lootable = 0x04,
    TrackUnit = 0x08,
    Tapped = 0x10,
    SpecialInfo = 0x20,
    CanSkin = 0x40,
    ReferAFriend = 0x80,
}

/// Chat message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ChatMsg {
    Addon = -1,
    System = 0x00,
    Say = 0x01,
    Party = 0x02,
    Raid = 0x03,
    Guild = 0x04,
    Officer = 0x05,
    Yell = 0x06,
    Whisper = 0x07,
    WhisperForeign = 0x08,
    WhisperInform = 0x09,
    Emote = 0x0a,
    TextEmote = 0x0b,
    MonsterSay = 0x0c,
    MonsterParty = 0x0d,
    MonsterYell = 0x0e,
    MonsterWhisper = 0x0f,
    MonsterEmote = 0x10,
    Channel = 0x11,
    ChannelJoin = 0x12,
    ChannelLeave = 0x13,
    ChannelList = 0x14,
    ChannelNotice = 0x15,
    ChannelNoticeUser = 0x16,
    Afk = 0x17,
    Dnd = 0x18,
    Ignored = 0x19,
    Skill = 0x1a,
    Loot = 0x1b,
    Money = 0x1c,
    Opening = 0x1d,
    Tradeskills = 0x1e,
    PetInfo = 0x1f,
    CombatMiscInfo = 0x20,
    CombatXpGain = 0x21,
    CombatHonorGain = 0x22,
    CombatFactionChange = 0x23,
    BgSystemNeutral = 0x24,
    BgSystemAlliance = 0x25,
    BgSystemHorde = 0x26,
    RaidLeader = 0x27,
    RaidWarning = 0x28,
}

/// Team affiliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum Team {
    Horde = 67,
    Alliance = 469,
    Other = 0,
}
