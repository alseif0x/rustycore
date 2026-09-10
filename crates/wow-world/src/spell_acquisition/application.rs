// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Validated application boundary for a deterministic acquisition plan.
//!
//! The planner is the sole semantic owner.  This module only proves that its
//! causal stream is internally coherent, pins it to the exact current player
//! authority, and translates the final snapshot into durable/runtime rows.

use super::*;
use crate::profession::{
    MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP, PrimaryProfessionCapacityPlanLikeCpp,
    PrimaryProfessionEquipmentSlotLikeCpp,
};
use wow_persistence::{
    PlayerSpellAcquisitionAuthorityLikeCpp as DurablePlayerSpellAcquisitionAuthorityLikeCpp,
    PlayerSpellAcquisitionDurableOperationLikeCpp, PlayerSpellAcquisitionPersistencePortLikeCpp,
    PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    PlayerSpellAcquisitionSkillRowLikeCpp as DurablePlayerSkillRowLikeCpp,
    PlayerSpellAcquisitionSpellRowLikeCpp as DurablePlayerSpellRowLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    classify_player_spell_acquisition_money_reconciliation_like_cpp,
};

mod runtime;
pub(crate) use runtime::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedPlayerSpellAcquisitionLikeCpp {
    pub character_guid: Option<wow_core::ObjectGuid>,
    pub root: SpellAcquisitionRootLikeCpp,
    pub source_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    /// C++ post-`Player::LearnSpell` state before the ordinary
    /// `Player::SaveToDB` lifecycle consumes `_SaveSpells`/`_SaveSkills` dirty
    /// states. Generic `EffectLearnSpell` applies this snapshot immediately;
    /// database-gated consumers use `runtime_snapshot` after their commit.
    pub pending_save_runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    /// C++ post-`_SaveSpells`/`_SaveSkills` in-memory state. Removed spells
    /// disappear; other non-temporary persistence states become unchanged.
    /// Deleted skill tombstones remain in the live slot map but are omitted
    /// from Character DB, matching C++ until the next login rebuild.
    pub runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    pub durable_spells: Vec<DurablePlayerSpellRowLikeCpp>,
    pub durable_favorite_spell_ids: Vec<i32>,
    pub durable_skills: Vec<DurablePlayerSkillRowLikeCpp>,
    /// Skill update-field slots that C++ retains in memory after `_SaveSkills`
    /// deletes their durable rows. The represented full-save path must omit
    /// these normalized tombstones until the slot is reused.
    pub non_durable_skill_tombstone_ids: BTreeSet<u16>,
    pub durable_operations: Vec<PlayerSpellAcquisitionDurableOperationLikeCpp>,
    pub post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionPublicationFaultPointLikeCpp {
    BeforeAction(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedPlayerSpellAcquisitionOutcomeLikeCpp {
    Ready(PreparedPlayerSpellAcquisitionLikeCpp),
    ActionsOnly(PreparedPlayerSpellAcquisitionActionsLikeCpp),
    AlreadyApplied,
    NoChange,
}

pub(crate) enum PlayerSpellAcquisitionPersistenceOutcomeLikeCpp {
    Applied,
    ReconciledCommit(String),
    DefinitelyRolledBack(String),
    Indeterminate(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedPlayerSpellAcquisitionActionsLikeCpp {
    pub character_guid: Option<wow_core::ObjectGuid>,
    pub runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    pub post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
    runtime_actions_already_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionPrepareErrorLikeCpp {
    StaleSnapshot,
    InvalidSpellId(u32),
    InvalidSkillId(u32),
    InvalidTraitDefinitionId(i32),
    InvalidProfessionAssociation(i8),
    ConflictingProfessionAssociation(u8),
    DuplicateSpell(u32),
    DuplicateSkill(u32),
    DuplicateOverride(u32, u32),
    TransitionBeforeMismatch { domain: &'static str, id: u32 },
    TransitionIdMismatch { domain: &'static str, id: u32 },
    DuplicateOverrideMutation { overridden: u32, overriding: u32 },
    MissingOverrideMutation { overridden: u32, overriding: u32 },
    TypedProjectionMismatch(&'static str),
    ResultingSnapshotMismatch,
    SnapshotIdentityChanged(&'static str),
    SkillOccupancyMismatch,
    InvalidDeletedSkill(u32),
    InvalidNonDurableSkillTombstone(u32),
    ProfessionInputsMismatch,
    ProfessionPlanMismatch(&'static str),
    InvalidPostCommitAction { domain: &'static str, id: u32 },
    PostCommitActionCausalityMismatch { action: &'static str, id: u32 },
    LearnedActionRowMismatch(u32),
    ProvenanceMismatch(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp {
    InvalidPreparedRuntime,
    PublicationInterrupted,
}

#[cfg(test)]
mod tests;

mod prepare;
mod translate;
mod validate_plan;
mod validate_post_commit;

use validate_plan::{validate_plan_replay_like_cpp, validate_profession_plan_like_cpp};

use validate_post_commit::validate_post_commit_actions_like_cpp;

use translate::{
    override_set_like_cpp, skill_map_like_cpp, spell_map_like_cpp, translate_plan_like_cpp,
};

pub(crate) use prepare::{
    persist_player_spell_acquisition_through_port_like_cpp,
    player_spell_acquisition_persistence_request_like_cpp,
    prepare_player_spell_acquisition_like_cpp, snapshot_has_pending_durable_save_like_cpp,
};

// The plan-validation tests reach this through `use super::*`; the plain build
// has no other reader, so the binding is test-only.
#[cfg(test)]
use prepare::stable_source_durable_authority_like_cpp;
