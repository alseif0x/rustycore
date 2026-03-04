// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Object type enums: TypeId, TypeMask, HighGuid, and related types.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Object type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum TypeId {
    Object = 0,
    Item = 1,
    Container = 2,
    AzeriteEmpoweredItem = 3,
    AzeriteItem = 4,
    Unit = 5,
    Player = 6,
    ActivePlayer = 7,
    GameObject = 8,
    DynamicObject = 9,
    Corpse = 10,
    AreaTrigger = 11,
    SceneObject = 12,
    Conversation = 13,
    Max = 14,
}

bitflags! {
    /// Bitmask of object types.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TypeMask: u32 {
        const OBJECT = 0x01;
        const ITEM = 0x02;
        const CONTAINER = 0x04;
        const AZERITE_EMPOWERED_ITEM = 0x08;
        const AZERITE_ITEM = 0x10;
        const UNIT = 0x20;
        const PLAYER = 0x40;
        const ACTIVE_PLAYER = 0x80;
        const GAME_OBJECT = 0x100;
        const DYNAMIC_OBJECT = 0x200;
        const CORPSE = 0x400;
        const AREA_TRIGGER = 0x800;
        const SCENE_OBJECT = 0x1000;
        const CONVERSATION = 0x2000;

        const SEER = Self::PLAYER.bits() | Self::UNIT.bits() | Self::DYNAMIC_OBJECT.bits();
        const WORLD_OBJECT = Self::UNIT.bits() | Self::GAME_OBJECT.bits() | Self::DYNAMIC_OBJECT.bits()
            | Self::CORPSE.bits() | Self::AREA_TRIGGER.bits() | Self::SCENE_OBJECT.bits()
            | Self::CONVERSATION.bits();
    }
}

/// High GUID types for object identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum HighGuid {
    Null = 0,
    Uniq = 1,
    Player = 2,
    Item = 3,
    WorldTransaction = 4,
    StaticDoor = 5,
    Transport = 6,
    Conversation = 7,
    Creature = 8,
    Vehicle = 9,
    Pet = 10,
    GameObject = 11,
    DynamicObject = 12,
    AreaTrigger = 13,
    Corpse = 14,
    LootObject = 15,
    SceneObject = 16,
    Scenario = 17,
    AIGroup = 18,
    DynamicDoor = 19,
    ClientActor = 20,
    Vignette = 21,
    CallForHelp = 22,
    AIResource = 23,
    AILock = 24,
    AILockTicket = 25,
    ChatChannel = 26,
    Party = 27,
    Guild = 28,
    WowAccount = 29,
    BNetAccount = 30,
    GMTask = 31,
    MobileSession = 32,
    RaidGroup = 33,
    Spell = 34,
    Mail = 35,
    WebObj = 36,
    LFGObject = 37,
    LFGList = 38,
    UserRouter = 39,
    PVPQueueGroup = 40,
    UserClient = 41,
    PetBattle = 42,
    UniqUserClient = 43,
    BattlePet = 44,
    CommerceObj = 45,
    ClientSession = 46,
    Cast = 47,
    ClientConnection = 48,
    ClubFinder = 49,
    ToolsClient = 50,
    WorldLayer = 51,
    ArenaTeam = 52,
    LMMParty = 53,
    LMMLobby = 54,

    Count = 55,
}

bitflags! {
    /// Notification flags for world objects.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct NotifyFlags: u8 {
        const NONE = 0x00;
        const AI_RELOCATION = 0x01;
        const VISIBILITY_CHANGED = 0x02;
        const ALL = 0xFF;
    }
}

/// Temporary summon types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum TempSummonType {
    TimedOrDeadDespawn = 1,
    TimedOrCorpseDespawn = 2,
    TimedDespawn = 3,
    TimedDespawnOutOfCombat = 4,
    CorpseDespawn = 5,
    CorpseTimedDespawn = 6,
    DeadDespawn = 7,
    ManualDespawn = 8,
}

/// Summon categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SummonCategory {
    Wild = 0,
    Ally = 1,
    Pet = 2,
    Puppet = 3,
    Vehicle = 4,
    Unk = 5,
}

/// Stealth types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum StealthType {
    General = 0,
    Trap = 1,
    Max = 2,
}

/// Server-side visibility types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ServerSideVisibilityType {
    GM = 0,
    Ghost = 1,
}
