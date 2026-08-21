// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered authority facts and persistence/publication intents.
//!
//! These types carry no transport or SQL surface: encoding, session addressing
//! and the database adapter stay with the caller.

use wow_core::ObjectGuid;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupAuthorityErrorLikeCpp {
    MissingGroup,
    MissingMember,
    NotLeader,
    NotLeaderOrAssistant,
    InvalidSubgroup,
    SubgroupFull,
    NotRaid,
    GroupTooLarge,
    NoChange,
    LfgGroup,
    LfgBootLimit,
    LfgBootTooFewPlayers,
    LfgBootDungeonComplete,
    LfgBootLootRolls,
    LfgBootInCombat,
    LfgKickOwnedByVote,
    InviteRestricted,
    TargetIsLeader,
}

#[derive(Debug, Clone)]
pub struct GroupTransitionOutcomeLikeCpp<T> {
    pub group: GroupInfo,
    pub facts: T,
    /// Ordered durability work emitted by the aggregate. Application adapters
    /// materialize these intents after the registry mutation and after its
    /// backing-map guard has been released.
    pub persistence: Vec<GroupPersistenceIntentLikeCpp>,
}

/// Database-neutral durability commands emitted by represented C++ `Group`
/// transitions. Keeping statement construction in the application adapter
/// prevents SQL/database types from leaking into the aggregate boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupPersistenceIntentLikeCpp {
    InsertGroup {
        db_store_id: u32,
        leader_guid: ObjectGuid,
        loot_method: u8,
        looter_guid: ObjectGuid,
        loot_threshold: u8,
        group_flags: u16,
        dungeon_difficulty_id: u32,
        raid_difficulty_id: u32,
        legacy_raid_difficulty_id: u32,
        master_looter_guid: ObjectGuid,
    },
    InsertMember {
        db_store_id: u32,
        member_guid: ObjectGuid,
        member_flags: u8,
        subgroup: u8,
        roles: u8,
    },
    DeleteGroup {
        db_store_id: u32,
    },
    DeleteAllMembers {
        db_store_id: u32,
    },
    DeleteLfgData {
        db_store_id: u32,
    },
    DeleteMember {
        member_guid: ObjectGuid,
    },
    UpdateLeader {
        db_store_id: u32,
        leader_guid: ObjectGuid,
    },
    UpdateGroupType {
        db_store_id: u32,
        group_flags: u16,
    },
    UpdateMemberSubgroup {
        member_guid: ObjectGuid,
        subgroup: u8,
    },
    UpdateMemberFlags {
        member_guid: ObjectGuid,
        flags: u8,
    },
    UpdateDifficulty {
        db_store_id: u32,
        kind: GroupDifficultyKindLikeCpp,
        difficulty_id: u32,
    },
}

#[derive(Debug, Clone)]
pub struct GroupMemberRemovalFactsLikeCpp {
    pub removed_guid: ObjectGuid,
    pub db_store_id: u32,
    pub disbanded: bool,
    pub new_leader_guid: Option<ObjectGuid>,
    pub remaining_members: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMemberRemovalKindLikeCpp {
    Leave,
    Kick {
        actor_guid: ObjectGuid,
        actor_in_battleground: bool,
        target_has_loot_rolls: bool,
        any_member_in_actor_map_combat: bool,
    },
}
