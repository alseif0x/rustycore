//! Group startup/command durability and group loot-money reconciliation contracts.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp};

/// One database-neutral durability command emitted by the represented C++
/// `Group` aggregate. GUIDs are low counters because the Characters schema
/// stores those columns independently from the runtime `ObjectGuid` encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedGroupPersistenceCommandLikeCpp {
    InsertGroup {
        db_store_id: u32,
        leader_guid: u64,
        loot_method: u8,
        looter_guid: u64,
        loot_threshold: u8,
        group_flags: u16,
        dungeon_difficulty_id: u32,
        raid_difficulty_id: u32,
        legacy_raid_difficulty_id: u32,
        master_looter_guid: u64,
    },
    InsertMember {
        db_store_id: u32,
        member_guid: u64,
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
        member_guid: u64,
    },
    UpdateLeader {
        db_store_id: u32,
        leader_guid: u64,
    },
    UpdateGroupType {
        db_store_id: u32,
        group_flags: u16,
    },
    UpdateMemberSubgroup {
        member_guid: u64,
        subgroup: u8,
    },
    UpdateMemberFlags {
        member_guid: u64,
        flags: u8,
    },
    UpdateDifficulty {
        db_store_id: u32,
        kind: RepresentedGroupDifficultyKindLikeCpp,
        difficulty_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedGroupDifficultyKindLikeCpp {
    Dungeon,
    Raid,
    LegacyRaid,
}

/// The general Group path mirrors C++ `CharacterDatabase.Execute` one command
/// at a time. The represented difficulty path already uses one explicit Rust
/// transaction; retaining that distinction keeps this extraction behavior-
/// preserving even though its current atomic batch contains one command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedGroupPersistenceModeLikeCpp {
    Sequential,
    Atomic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedGroupPersistenceRequestLikeCpp {
    pub commands: Vec<RepresentedGroupPersistenceCommandLikeCpp>,
    pub mode: RepresentedGroupPersistenceModeLikeCpp,
}

impl RepresentedGroupPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepresentedGroupPersistenceOutcomeLikeCpp {
    Applied {
        command_count: usize,
    },
    FailedAfterPrefix {
        applied: usize,
        reason: String,
    },
    DefinitelyRolledBack {
        reason: String,
    },
    CommitOutcomeUnknown {
        command_count: usize,
        reason: String,
    },
}

pub trait RepresentedGroupPersistencePortLikeCpp: Send + Sync {
    fn persist_group_commands_like_cpp(
        &self,
        request: RepresentedGroupPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupPersistenceOutcomeLikeCpp>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedGroupStartupCharacterLikeCpp {
    pub guid: u64,
    pub name: String,
    pub race: u8,
    pub class: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedGroupStartupGroupRowLikeCpp {
    pub leader_guid_low: u64,
    pub loot_method: u8,
    pub looter_guid_low: u64,
    pub loot_threshold: u8,
    pub target_icons: [[u8; 16]; 8],
    pub group_flags: u16,
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
    pub master_looter_guid_low: u64,
    pub db_store_id: u32,
    pub lfg_dungeon_id: Option<u32>,
    pub lfg_state: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedGroupStartupMemberRowLikeCpp {
    pub db_store_id: u32,
    pub member_guid_low: u64,
    pub member_flags: u8,
    pub subgroup: u8,
    pub roles: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedGroupStartupLoadStageLikeCpp {
    DeleteMembersWithoutCharacter,
    DeleteGroupsWithoutLeader,
    DeleteGroupsWithFewerThanTwoMembers,
    DeleteMembersWithoutGroup,
    CharacterCache,
    Groups,
    Members,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepresentedGroupStartupLoadOutcomeLikeCpp {
    Loaded {
        characters: Vec<RepresentedGroupStartupCharacterLikeCpp>,
        groups: Vec<RepresentedGroupStartupGroupRowLikeCpp>,
        members: Vec<RepresentedGroupStartupMemberRowLikeCpp>,
    },
    Failed {
        stage: RepresentedGroupStartupLoadStageLikeCpp,
        reason: String,
    },
}

pub trait RepresentedGroupStartupLoadPortLikeCpp: Send + Sync {
    fn load_represented_groups_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupStartupLoadOutcomeLikeCpp>;
}

/// LootHandler.cpp applies CalculatePct to each base share separately. The
/// target computes in single precision and truncates the copper fraction.
pub fn loot_money_bonus(base: u64, percent: i32) -> u64 {
    (base as f32 * percent as f32 / 100.0) as u64
}

/// One recipient in an atomic group corpse-loot payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupLootMoneyPayoutLikeCpp {
    pub recipient_guid: u64,
    pub requested_delta: u64,
    /// Base corpse-loot share before the recipient's aura bonus. Zero means
    /// the recipient is not eligible for criterion 4093 on this claim.
    pub base_looted_amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupLootMoneyPersistenceRequestLikeCpp {
    /// Absent only for a zero-coin observation of an already empty pool.
    pub claim_id: Option<[u8; 16]>,
    pub payouts: Vec<GroupLootMoneyPayoutLikeCpp>,
    pub max_money: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupLootMoneyPersistenceOutcomeLikeCpp {
    pub recipient_guid: u64,
    pub before: u64,
    pub after: u64,
    pub applied_delta: u64,
    pub criteria: Option<crate::LootMoneyCriteriaProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupLootMoneyRollbackKindLikeCpp {
    MissingPlayer { recipient_guid: u64 },
    Database,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupLootMoneyPersistenceAttemptLikeCpp {
    Applied(Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>),
    AlreadySettled,
    DefinitelyRolledBack {
        kind: GroupLootMoneyRollbackKindLikeCpp,
        reason: String,
        retryable_deadlock: bool,
    },
    CommitOutcomeUnknown {
        reason: String,
        outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupLootMoneyReconciliationLikeCpp {
    CommittedOrCapOnlyNoop,
    RolledBack,
    Indeterminate { reason: Option<String> },
}

pub fn classify_group_loot_money_reconciliation_like_cpp(
    outcomes: &[GroupLootMoneyPersistenceOutcomeLikeCpp],
    receipt: Option<&[GroupLootMoneyPersistenceOutcomeLikeCpp]>,
) -> GroupLootMoneyReconciliationLikeCpp {
    match receipt {
        None => GroupLootMoneyReconciliationLikeCpp::RolledBack,
        Some(recorded) if !recorded.is_empty() && recorded == outcomes => {
            GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop
        }
        Some(_) => GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None },
    }
}

/// Persistent GameObject spawn identity across world-server processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldLootMoneySource {
    pub realm_id: u16,
    pub map_id: u32,
    pub instance_id: u32,
    pub spawn_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldLootMoneyBinding {
    pub generation: u64,
    pub claim_id: [u8; 16],
    pub spent: bool,
}

/// Characters-database capability for group money payouts and their receipts.
pub trait GroupLootMoneyPersistencePortLikeCpp: Send + Sync {
    /// None means this source has not reached its persisted restock deadline.
    fn bind_world_chest_money(
        &self,
        _source: WorldLootMoneySource,
        _personal_owner: u64,
    ) -> PersistenceFutureLikeCpp<'_, Result<Option<WorldLootMoneyBinding>, String>> {
        Box::pin(std::future::ready(Err(
            "world chest money persistence unavailable".into(),
        )))
    }
    fn attempt_group_loot_money_like_cpp(
        &self,
        request: GroupLootMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyPersistenceAttemptLikeCpp>;

    fn reconcile_group_loot_money_like_cpp(
        &self,
        claim_id: Option<[u8; 16]>,
        outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyReconciliationLikeCpp>;
}
