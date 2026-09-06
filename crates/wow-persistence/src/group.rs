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

/// One recipient in an atomic group corpse-loot payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupLootMoneyPayoutLikeCpp {
    pub recipient_guid: u64,
    pub requested_delta: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupLootMoneyPersistenceRequestLikeCpp {
    pub payouts: Vec<GroupLootMoneyPayoutLikeCpp>,
    pub max_money: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupLootMoneyPersistenceOutcomeLikeCpp {
    pub recipient_guid: u64,
    pub before: u64,
    pub after: u64,
    pub applied_delta: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupLootMoneyRollbackKindLikeCpp {
    MissingPlayer { recipient_guid: u64 },
    Database,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupLootMoneyPersistenceAttemptLikeCpp {
    Applied(Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>),
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
    observed: &[(u64, Option<u64>)],
) -> GroupLootMoneyReconciliationLikeCpp {
    let changed = outcomes
        .iter()
        .filter(|outcome| outcome.before != outcome.after)
        .collect::<Vec<_>>();
    if changed.is_empty() {
        return GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop;
    }
    if changed.len() != observed.len() {
        return GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None };
    }

    let mut all_before = true;
    let mut all_after = true;
    for outcome in changed {
        let Some((_, Some(current))) = observed
            .iter()
            .find(|(recipient_guid, _)| *recipient_guid == outcome.recipient_guid)
        else {
            return GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None };
        };
        all_before &= *current == outcome.before;
        all_after &= *current == outcome.after;
    }
    match (all_before, all_after) {
        (true, false) => GroupLootMoneyReconciliationLikeCpp::RolledBack,
        (false, true) => GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop,
        _ => GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None },
    }
}

/// SQLx-free Characters-database capability for one group loot-money payout.
pub trait GroupLootMoneyPersistencePortLikeCpp: Send + Sync {
    fn attempt_group_loot_money_like_cpp(
        &self,
        request: GroupLootMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyPersistenceAttemptLikeCpp>;

    fn reconcile_group_loot_money_like_cpp(
        &self,
        outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyReconciliationLikeCpp>;
}
