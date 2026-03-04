// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature-related enums and flags: types, families, static flags, etc.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Creature linked respawn type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureLinkedRespawnType {
    CreatureToCreature = 0,
    CreatureToGO = 1,
    GOToGO = 2,
    GOToCreature = 3,
}

/// AI reaction types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum AiReaction {
    Alert = 0,
    Friendly = 1,
    Hostile = 2,
    Afraid = 3,
    Destory = 4,
}

/// Creature classifications (elite status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureClassifications {
    Normal = 0,
    Elite = 1,
    RareElite = 2,
    Obsolete = 3,
    Rare = 4,
    Trivial = 5,
    MinusMob = 6,
}

bitflags! {
    /// Creature type flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureTypeFlags: u32 {
        const TAMEABLE                          = 0x00000001;
        const VISIBLE_TO_GHOSTS                 = 0x00000002;
        const BOSS_MOB                          = 0x00000004;
        const DO_NOT_PLAY_WOUND_ANIM            = 0x00000008;
        const NO_FACTION_TOOLTIP                = 0x00000010;
        const MORE_AUDIBLE                      = 0x00000020;
        const SPELL_ATTACKABLE                  = 0x00000040;
        const INTERACT_WHILE_DEAD               = 0x00000080;
        const SKIN_WITH_HERBALISM               = 0x00000100;
        const SKIN_WITH_MINING                  = 0x00000200;
        const NO_DEATH_MESSAGE                  = 0x00000400;
        const ALLOW_MOUNTED_COMBAT              = 0x00000800;
        const CAN_ASSIST                        = 0x00001000;
        const NO_PET_BAR                        = 0x00002000;
        const MASK_UID                          = 0x00004000;
        const SKIN_WITH_ENGINEERING             = 0x00008000;
        const TAMEABLE_EXOTIC                   = 0x00010000;
        const USE_MODEL_COLLISION_SIZE          = 0x00020000;
        const ALLOW_INTERACTION_WHILE_IN_COMBAT = 0x00040000;
        const COLLIDE_WITH_MISSILES             = 0x00080000;
        const NO_NAME_PLATE                     = 0x00100000;
        const DO_NOT_PLAY_MOUNTED_ANIMATIONS    = 0x00200000;
        const LINK_ALL                          = 0x00400000;
        const INTERACT_ONLY_WITH_CREATOR        = 0x00800000;
        const DO_NOT_PLAY_UNIT_EVENT_SOUNDS     = 0x01000000;
        const HAS_NO_SHADOW_BLOB                = 0x02000000;
        const TREAT_AS_RAID_UNIT                = 0x04000000;
        const FORCE_GOSSIP                      = 0x08000000;
        const DO_NOT_SHEATHE                    = 0x10000000;
        const DO_NOT_TARGET_ON_INTERACTION      = 0x20000000;
        const DO_NOT_RENDER_OBJECT_NAME         = 0x40000000;
        const QUEST_BOSS                        = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags (primary).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags: u32 {
        const MOUNTABLE                 = 0x00000001;
        const NO_XP                     = 0x00000002;
        const NO_LOOT                   = 0x00000004;
        const UNKILLABLE                = 0x00000008;
        const TAMEABLE                  = 0x00000010;
        const IMMUNE_TO_PC              = 0x00000020;
        const IMMUNE_TO_NPC             = 0x00000040;
        const CAN_WIELD_LOOT            = 0x00000080;
        const SESSILE                   = 0x00000100;
        const UNINTERACTIBLE            = 0x00000200;
        const NO_AUTOMATIC_REGEN        = 0x00000400;
        const DESPAWN_INSTANTLY         = 0x00000800;
        const CORPSE_RAID               = 0x00001000;
        const CREATOR_LOOT              = 0x00002000;
        const NO_DEFENSE                = 0x00004000;
        const NO_SPELL_DEFENSE          = 0x00008000;
        const BOSS_MOB                  = 0x00010000;
        const COMBAT_PING               = 0x00020000;
        const AQUATIC                   = 0x00040000;
        const AMPHIBIOUS                = 0x00080000;
        const NO_MELEE_FLEE             = 0x00100000;
        const VISIBLE_TO_GHOSTS         = 0x00200000;
        const PVP_ENABLING              = 0x00400000;
        const DO_NOT_PLAY_WOUND_ANIM    = 0x00800000;
        const NO_FACTION_TOOLTIP        = 0x01000000;
        const IGNORE_COMBAT             = 0x02000000;
        const ONLY_ATTACK_PVP_ENABLING  = 0x04000000;
        const CALLS_GUARDS              = 0x08000000;
        const CAN_SWIM                  = 0x10000000;
        const FLOATING                  = 0x20000000;
        const MORE_AUDIBLE              = 0x40000000;
        const LARGE_AOI                 = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags 2.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags2: u32 {
        const NO_PET_SCALING                            = 0x00000001;
        const FORCE_PARTY_MEMBERS_INTO_COMBAT           = 0x00000002;
        const LOCK_TAPPERS_TO_RAID_ON_DEATH             = 0x00000004;
        const SPELL_ATTACKABLE                          = 0x00000008;
        const NO_CRUSHING_BLOWS                         = 0x00000010;
        const NO_OWNER_THREAT                           = 0x00000020;
        const NO_WOUNDED_SLOWDOWN                       = 0x00000040;
        const USE_CREATOR_BONUSES                       = 0x00000080;
        const IGNORE_FEIGN_DEATH                        = 0x00000100;
        const IGNORE_SANCTUARY                          = 0x00000200;
        const ACTION_TRIGGERS_WHILE_CHARMED             = 0x00000400;
        const INTERACT_WHILE_DEAD                       = 0x00000800;
        const NO_INTERRUPT_SCHOOL_COOLDOWN              = 0x00001000;
        const RETURN_SOUL_SHARD_TO_MASTER_OF_PET        = 0x00002000;
        const SKIN_WITH_HERBALISM                       = 0x00004000;
        const SKIN_WITH_MINING                          = 0x00008000;
        const ALERT_CONTENT_TEAM_ON_DEATH               = 0x00010000;
        const ALERT_CONTENT_TEAM_AT_90PCT_HP            = 0x00020000;
        const ALLOW_MOUNTED_COMBAT                      = 0x00040000;
        const PVP_ENABLING_OOC                          = 0x00080000;
        const NO_DEATH_MESSAGE                          = 0x00100000;
        const IGNORE_PATHING_FAILURE                    = 0x00200000;
        const FULL_SPELL_LIST                           = 0x00400000;
        const DOES_NOT_REDUCE_REPUTATION_FOR_RAIDS      = 0x00800000;
        const IGNORE_MISDIRECTION                       = 0x01000000;
        const HIDE_BODY                                 = 0x02000000;
        const SPAWN_DEFENSIVE                           = 0x04000000;
        const SERVER_ONLY                               = 0x08000000;
        const CAN_SAFE_FALL                             = 0x10000000;
        const CAN_ASSIST                                = 0x20000000;
        const NO_SKILL_GAINS                            = 0x40000000;
        const NO_PET_BAR                                = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags 3.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags3: u32 {
        const NO_DAMAGE_HISTORY                     = 0x00000001;
        const DONT_PVP_ENABLE_OWNER                 = 0x00000002;
        const DO_NOT_FADE_IN                        = 0x00000004;
        const MASK_UID                              = 0x00000008;
        const SKIN_WITH_ENGINEERING                 = 0x00000010;
        const NO_AGGRO_ON_LEASH                     = 0x00000020;
        const NO_FRIENDLY_AREA_AURAS                = 0x00000040;
        const EXTENDED_CORPSE_DURATION              = 0x00000080;
        const CANNOT_SWIM                           = 0x00000100;
        const TAMEABLE_EXOTIC                       = 0x00000200;
        const GIGANTIC_AOI                          = 0x00000400;
        const INFINITE_AOI                          = 0x00000800;
        const CANNOT_PENETRATE_WATER                = 0x00001000;
        const NO_NAME_PLATE                         = 0x00002000;
        const CHECKS_LIQUIDS                        = 0x00004000;
        const NO_THREAT_FEEDBACK                    = 0x00008000;
        const USE_MODEL_COLLISION_SIZE              = 0x00010000;
        const ATTACKER_IGNORES_FACING               = 0x00020000;
        const ALLOW_INTERACTION_WHILE_IN_COMBAT     = 0x00040000;
        const SPELL_CLICK_FOR_PARTY_ONLY            = 0x00080000;
        const FACTION_LEADER                        = 0x00100000;
        const IMMUNE_TO_PLAYER_BUFFS                = 0x00200000;
        const COLLIDE_WITH_MISSILES                 = 0x00400000;
        const CAN_BE_MULTITAPPED                    = 0x00800000;
        const DO_NOT_PLAY_MOUNTED_ANIMATIONS        = 0x01000000;
        const CANNOT_TURN                           = 0x02000000;
        const ENEMY_CHECK_IGNORES_LOS               = 0x04000000;
        const FOREVER_CORPSE_DURATION               = 0x08000000;
        const PETS_ATTACK_WITH_3D_PATHING           = 0x10000000;
        const LINK_ALL                              = 0x20000000;
        const AI_CAN_AUTO_TAKEOFF_IN_COMBAT         = 0x40000000;
        const AI_CAN_AUTO_LAND_IN_COMBAT            = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags 4.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags4: u32 {
        const NO_BIRTH_ANIM                                         = 0x00000001;
        const TREAT_AS_PLAYER_FOR_DIMINISHING_RETURNS               = 0x00000002;
        const TREAT_AS_PLAYER_FOR_PVP_DEBUFF_DURATION               = 0x00000004;
        const INTERACT_ONLY_WITH_CREATOR                            = 0x00000008;
        const DO_NOT_PLAY_UNIT_EVENT_SOUNDS                         = 0x00000010;
        const HAS_NO_SHADOW_BLOB                                    = 0x00000020;
        const DEALS_TRIPLE_DAMAGE_TO_PC_CONTROLLED_PETS             = 0x00000040;
        const NO_NPC_DAMAGE_BELOW_85PTC                             = 0x00000080;
        const OBEYS_TAUNT_DIMINISHING_RETURNS                       = 0x00000100;
        const NO_MELEE_APPROACH                                     = 0x00000200;
        const UPDATE_CREATURE_RECORD_WHEN_INSTANCE_CHANGES_DIFFICULTY = 0x00000400;
        const CANNOT_DAZE                                           = 0x00000800;
        const FLAT_HONOR_AWARD                                      = 0x00001000;
        const IGNORE_LOS_WHEN_CASTING_ON_ME                         = 0x00002000;
        const GIVE_QUEST_KILL_CREDIT_WHILE_OFFLINE                  = 0x00004000;
        const TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS                 = 0x00008000;
        const DONT_REPOSITION_IF_MELEE_TARGET_IS_TOO_CLOSE          = 0x00010000;
        const PET_OR_GUARDIAN_AI_DONT_GO_BEHIND_TARGET              = 0x00020000;
        const MINUTE_LOOT_ROLL_TIMER                                = 0x00040000;
        const FORCE_GOSSIP                                          = 0x00080000;
        const DONT_REPOSITION_WITH_FRIENDS_IN_COMBAT                = 0x00100000;
        const DO_NOT_SHEATHE                                        = 0x00200000;
        const IGNORE_SPELL_MIN_RANGE_RESTRICTIONS                   = 0x00400000;
        const SUPPRESS_INSTANCE_WIDE_RELEASE_IN_COMBAT              = 0x00800000;
        const PREVENT_SWIM                                          = 0x01000000;
        const HIDE_IN_COMBAT_LOG                                    = 0x02000000;
        const ALLOW_NPC_COMBAT_WHILE_UNINTERACTIBLE                 = 0x04000000;
        const PREFER_NPCS_WHEN_SEARCHING_FOR_ENEMIES                = 0x08000000;
        const ONLY_GENERATE_INITIAL_THREAT                          = 0x10000000;
        const DO_NOT_TARGET_ON_INTERACTION                          = 0x20000000;
        const DO_NOT_RENDER_OBJECT_NAME                             = 0x40000000;
        const QUEST_BOSS                                            = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags 5.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags5: u32 {
        const UNTARGETABLE_BY_CLIENT                                = 0x00000001;
        const FORCE_SELF_MOUNTING                                   = 0x00000002;
        const UNINTERACTIBLE_IF_HOSTILE                             = 0x00000004;
        const DISABLES_XP_AWARD                                     = 0x00000008;
        const DISABLE_AI_PREDICTION                                 = 0x00000010;
        const NO_LEAVECOMBAT_STATE_RESTORE                          = 0x00000020;
        const BYPASS_INTERACT_INTERRUPTS                            = 0x00000040;
        const BACK_ARC_240_DEGREE                                   = 0x00000080;
        const INTERACT_WHILE_HOSTILE                                = 0x00000100;
        const DONT_DISMISS_ON_FLYING_MOUNT                          = 0x00000200;
        const PREDICTIVE_POWER_REGEN                                = 0x00000400;
        const HIDE_LEVEL_INFO_IN_TOOLTIP                            = 0x00000800;
        const HIDE_HEALTH_BAR_UNDER_TOOLTIP                         = 0x00001000;
        const SUPPRESS_HIGHLIGHT_WHEN_TARGETED_OR_MOUSED_OVER       = 0x00002000;
        const AI_PREFER_PATHABLE_TARGETS                            = 0x00004000;
        const FREQUENT_AREA_TRIGGER_CHECKS                          = 0x00008000;
        const ASSIGN_KILL_CREDIT_TO_ENCOUNTER_LIST                  = 0x00010000;
        const NEVER_EVADE                                           = 0x00020000;
        const AI_CANT_PATH_ON_STEEP_SLOPES                          = 0x00040000;
        const AI_IGNORE_LOS_TO_MELEE_TARGET                         = 0x00080000;
        const NO_TEXT_IN_CHAT_BUBBLE                                = 0x00100000;
        const CLOSE_IN_ON_UNPATHABLE_TARGET                         = 0x00200000;
        const DONT_GO_BEHIND_ME                                     = 0x00400000;
        const NO_DEATH_THUD                                         = 0x00800000;
        const CLIENT_LOCAL_CREATURE                                 = 0x01000000;
        const CAN_DROP_LOOT_WHILE_IN_A_CHALLENGE_MODE_INSTANCE      = 0x02000000;
        const HAS_SAFE_LOCATION                                     = 0x04000000;
        const NO_HEALTH_REGEN                                       = 0x08000000;
        const NO_POWER_REGEN                                        = 0x10000000;
        const NO_PET_UNIT_FRAME                                     = 0x20000000;
        const NO_INTERACT_ON_LEFT_CLICK                             = 0x40000000;
        const GIVE_CRITERIA_KILL_CREDIT_WHEN_CHARMED                = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags 6.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags6: u32 {
        const DO_NOT_AUTO_RESUMMON                                  = 0x00000001;
        const REPLACE_VISIBLE_UNIT_IF_AVAILABLE                     = 0x00000002;
        const IGNORE_REALM_COALESCING_HIDING_CODE                   = 0x00000004;
        const TAPS_TO_FACTION                                       = 0x00000008;
        const ONLY_QUESTGIVER_FOR_SUMMONER                          = 0x00000010;
        const AI_COMBAT_RETURN_PRECISE                              = 0x00000020;
        const HOME_REALM_ONLY_LOOT                                  = 0x00000040;
        const NO_INTERACT_RESPONSE                                  = 0x00000080;
        const NO_INITIAL_POWER                                      = 0x00000100;
        const DONT_CANCEL_CHANNEL_ON_MASTER_MOUNTING                = 0x00000200;
        const CAN_TOGGLE_BETWEEN_DEATH_AND_PERSONAL_LOOT            = 0x00000400;
        const ALWAYS_STAND_ON_TOP_OF_TARGET                         = 0x00000800;
        const UNCONSCIOUS_ON_DEATH                                  = 0x00001000;
        const DONT_REPORT_TO_LOCAL_DEFENSE_CHANNEL_ON_DEATH         = 0x00002000;
        const PREFER_UNENGAGED_MONSTERS                             = 0x00004000;
        const USE_PVP_POWER_AND_RESILIENCE                          = 0x00008000;
        const DONT_CLEAR_DEBUFFS_ON_LEAVE_COMBAT                    = 0x00010000;
        const PERSONAL_LOOT_HAS_FULL_SECURITY                       = 0x00020000;
        const TRIPLE_SPELL_VISUALS                                  = 0x00040000;
        const USE_GARRISON_OWNER_LEVEL                              = 0x00080000;
        const IMMEDIATE_AOI_UPDATE_ON_SPAWN                         = 0x00100000;
        const UI_CAN_GET_POSITION                                   = 0x00200000;
        const SEAMLESS_TRANSFER_PROHIBITED                          = 0x00400000;
        const ALWAYS_USE_GROUP_LOOT_METHOD                          = 0x00800000;
        const NO_BOSS_KILL_BANNER                                   = 0x01000000;
        const FORCE_TRIGGERING_PLAYER_LOOT_ONLY                     = 0x02000000;
        const SHOW_BOSS_FRAME_WHILE_UNINTERACTABLE                  = 0x04000000;
        const SCALES_TO_PLAYER_LEVEL                                = 0x08000000;
        const AI_DONT_LEAVE_MELEE_FOR_RANGED_WHEN_TARGET_GETS_ROOTED = 0x10000000;
        const DONT_USE_COMBAT_REACH_FOR_CHAINING                    = 0x20000000;
        const DO_NOT_PLAY_PROCEDURAL_WOUND_ANIM                     = 0x40000000;
        const APPLY_PROCEDURAL_WOUND_ANIM_TO_BASE                   = 0x80000000;
    }
}

bitflags! {
    /// Creature static flags 7.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags7: u32 {
        const IMPORTANT_NPC                                     = 0x00000001;
        const IMPORTANT_QUEST_NPC                               = 0x00000002;
        const LARGE_NAMEPLATE                                   = 0x00000004;
        const TRIVIAL_PET                                       = 0x00000008;
        const AI_ENEMIES_DONT_BACKUP_WHEN_I_GET_ROOTED          = 0x00000010;
        const NO_AUTOMATIC_COMBAT_ANCHOR                        = 0x00000020;
        const ONLY_TARGETABLE_BY_CREATOR                        = 0x00000040;
        const TREAT_AS_PLAYER_FOR_IS_PLAYER_CONTROLLED          = 0x00000080;
        const GENERATE_NO_THREAT_OR_DAMAGE                      = 0x00000100;
        const INTERACT_ONLY_ON_QUEST                            = 0x00000200;
        const DISABLE_KILL_CREDIT_FOR_OFFLINE_PLAYERS           = 0x00000400;
        const AI_ADDITIONAL_PATHING                             = 0x00080000;
    }
}

bitflags! {
    /// Creature static flags 8.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureStaticFlags8: u32 {
        const FORCE_CLOSE_IN_ON_PATH_FAIL_BEHAVIOR     = 0x00000002;
        const USE_2D_CHASING_CALCULATION                = 0x00000020;
        const USE_FAST_CLASSIC_HEARTBEAT                = 0x00000040;
    }
}

bitflags! {
    /// Extra creature flags (database flags).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureFlagsExtra: u32 {
        const INSTANCE_BIND             = 0x01;
        const CIVILIAN                  = 0x02;
        const NO_PARRY                  = 0x04;
        const NO_PARRY_HASTEN           = 0x08;
        const NO_BLOCK                  = 0x10;
        const NO_CRUSHING_BLOWS         = 0x20;
        const NO_XP                     = 0x40;
        const TRIGGER                   = 0x80;
        const NO_TAUNT                  = 0x100;
        const NO_MOVE_FLAGS_UPDATE      = 0x200;
        const GHOST_VISIBILITY          = 0x400;
        const USE_OFFHAND_ATTACK        = 0x800;
        const NO_SELL_VENDOR            = 0x1000;
        const CANNOT_ENTER_COMBAT       = 0x2000;
        const WORLDEVENT                = 0x4000;
        const GUARD                     = 0x8000;
        const IGNORE_FEIGH_DEATH        = 0x10000;
        const NO_CRIT                   = 0x20000;
        const NO_SKILL_GAINS            = 0x40000;
        const OBEYS_TAUNT_DIMINISHING_RETURNS = 0x80000;
        const ALL_DIMINISH              = 0x100000;
        const NO_PLAYER_DAMAGE_REQ      = 0x200000;
        const UNUSED22                  = 0x400000;
        const UNUSED23                  = 0x800000;
        const UNUSED24                  = 0x1000000;
        const UNUSED25                  = 0x2000000;
        const UNUSED26                  = 0x4000000;
        const UNUSED27                  = 0x8000000;
        const DUNGEON_BOSS              = 0x10000000;
        const IGNORE_PATHFINDING        = 0x20000000;
        const IMMUNITY_KNOCKBACK        = 0x40000000;
        const UNUSED31                  = 0x80000000;

        const ALL_UNUSED = Self::UNUSED22.bits() | Self::UNUSED23.bits()
            | Self::UNUSED24.bits() | Self::UNUSED25.bits()
            | Self::UNUSED26.bits() | Self::UNUSED27.bits()
            | Self::UNUSED31.bits();

        const DB_ALLOWED = 0xFFFFFFFF & !(Self::ALL_UNUSED.bits() | Self::DUNGEON_BOSS.bits());
    }
}

/// Creature type (unit type classification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureType {
    None = 0,
    Beast = 1,
    Dragonkin = 2,
    Demon = 3,
    Elemental = 4,
    Giant = 5,
    Undead = 6,
    Humanoid = 7,
    Critter = 8,
    Mechanical = 9,
    NotSpecified = 10,
    Totem = 11,
    NonCombatPet = 12,
    GasCloud = 13,
    WildPet = 14,
    Aberration = 15,
}

bitflags! {
    /// Creature type bitmask.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CreatureTypeMask: u32 {
        const BEAST             = 1 << 0;  // 1 << (Beast - 1)
        const DRAGONKIN         = 1 << 1;
        const DEMON             = 1 << 2;
        const ELEMENTAL         = 1 << 3;
        const GIANT             = 1 << 4;
        const UNDEAD            = 1 << 5;
        const HUMANOID          = 1 << 6;
        const CRITTER           = 1 << 7;
        const MECHANICAL        = 1 << 8;
        const NOT_SPECIFIED     = 1 << 9;
        const TOTEM             = 1 << 10;
        const NON_COMBAT_PET    = 1 << 11;
        const GAS_CLOUD         = 1 << 12;
        const WILD_PET          = 1 << 13;
        const ABERRATION        = 1 << 14;

        const MASK_DEMON_OR_UNDEAD = Self::DEMON.bits() | Self::UNDEAD.bits();
        const MASK_HUMANOID_OR_UNDEAD = Self::HUMANOID.bits() | Self::UNDEAD.bits();
        const MASK_MECHANICAL_OR_ELEMENTAL = Self::MECHANICAL.bits() | Self::ELEMENTAL.bits();

        const ALL = Self::BEAST.bits() | Self::DRAGONKIN.bits() | Self::DEMON.bits()
            | Self::ELEMENTAL.bits() | Self::GIANT.bits() | Self::UNDEAD.bits()
            | Self::HUMANOID.bits() | Self::CRITTER.bits() | Self::MECHANICAL.bits()
            | Self::NOT_SPECIFIED.bits() | Self::TOTEM.bits() | Self::NON_COMBAT_PET.bits()
            | Self::GAS_CLOUD.bits() | Self::WILD_PET.bits() | Self::ABERRATION.bits();
    }
}

/// Creature family (pet family).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureFamily {
    None = 0,
    Wolf = 1,
    Cat = 2,
    Spider = 3,
    Bear = 4,
    Boar = 5,
    Crocolisk = 6,
    CarrionBird = 7,
    Crab = 8,
    Gorilla = 9,
    Raptor = 11,
    Tallstrider = 12,
    Felhunter = 15,
    Voidwalker = 16,
    Succubus = 17,
    Doomguard = 19,
    Scorpid = 20,
    Turtle = 21,
    Imp = 23,
    Bat = 24,
    Hyena = 25,
    BirdOfPrey = 26,
    WindSerpent = 27,
    RemoteControl = 28,
    Felguard = 29,
    Dragonhawk = 30,
    Ravager = 31,
    WarpStalker = 32,
    Sporebat = 33,
    Ray = 34,
    Serpent = 35,
    Moth = 37,
    Chimaera = 38,
    Devilsaur = 39,
    Ghoul = 40,
    Aqiri = 41,
    Worm = 42,
    Clefthoof = 43,
    Wasp = 44,
    CoreHound = 45,
    SpiritBeast = 46,
    WaterElemental = 49,
    Fox = 50,
    Monkey = 51,
    Hound = 52,
    Beetle = 53,
    ShaleBeast = 55,
    Zombie = 56,
    QaTest = 57,
    Hydra = 68,
    Felimp = 100,
    Voidlord = 101,
    Shivara = 102,
    Observer = 103,
    Wrathguard = 104,
    Infernal = 108,
    Fireelemental = 116,
    Earthelemental = 117,
    Crane = 125,
    Waterstrider = 126,
    Rodent = 127,
    StoneHound = 128,
    Gruffhorn = 129,
    Basilisk = 130,
    Direhorn = 138,
    Stormelemental = 145,
    Torrorguard = 147,
    Abyssal = 148,
    Riverbeast = 150,
    Stag = 151,
    Mechanical = 154,
    Abomination = 155,
    Scalehide = 156,
    Oxen = 157,
    Feathermane = 160,
    Lizard = 288,
    Pterrordax = 290,
    Toad = 291,
    Carapid = 292,
    BloodBeast = 296,
    Camel = 298,
    Courser = 299,
    Mammoth = 300,
    Incubus = 302,
}

/// Inhabit type flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum InhabitType {
    Ground = 1,
    Water = 2,
    Air = 4,
    Root = 8,
}

/// Evade reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum EvadeReason {
    NoHostiles = 0,
    Boundary = 1,
    NoPath = 2,
    SequenceBreak = 3,
    Other = 4,
}

/// Target selection method for AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SelectTargetMethod {
    Random = 0,
    MaxThreat = 1,
    MinThreat = 2,
    MaxDistance = 3,
    MinDistance = 4,
}

bitflags! {
    /// Group AI flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GroupAIFlags: u32 {
        const NONE                      = 0;
        const MEMBERS_ASSIST_LEADER     = 0x01;
        const LEADER_ASSISTS_MEMBER     = 0x02;
        const MEMBERS_ASSIST_MEMBER     = Self::MEMBERS_ASSIST_LEADER.bits() | Self::LEADER_ASSISTS_MEMBER.bits();
        const IDLE_IN_FORMATION         = 0x200;
    }
}

/// Creature ground movement type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureGroundMovementType {
    None = 0,
    Run = 1,
    Hover = 2,
}

/// Creature flight movement type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureFlightMovementType {
    None = 0,
    DisableGravity = 1,
    CanFly = 2,
}

/// Creature chase movement type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureChaseMovementType {
    Run = 0,
    CanWalk = 1,
    AlwaysWalk = 2,
}

/// Creature random movement type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CreatureRandomMovementType {
    Walk = 0,
    CanRun = 1,
    AlwaysRun = 2,
}

/// Vendor inventory reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum VendorInventoryReason {
    None = 0,
    Empty = 1,
}
