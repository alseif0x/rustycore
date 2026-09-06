//! Trainer spell/skill acquisition durability and exact-attempt reconciliation contract.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp};

/// One durable `character_spell` row produced by the represented
/// `Player::_SaveSpells` projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellAcquisitionSpellRowLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
}

/// One durable `character_skills` row produced by the represented
/// `Player::_SaveSkills` projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellAcquisitionSkillRowLikeCpp {
    pub skill_id: u16,
    pub value: u16,
    pub maximum: u16,
    pub profession_slot: i8,
}

/// Complete durable spell/skill authority used for optimistic comparison and
/// lost-COMMIT reconciliation. Vectors are sorted by their stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellAcquisitionAuthorityLikeCpp {
    pub spells: Vec<PlayerSpellAcquisitionSpellRowLikeCpp>,
    pub favorite_spell_ids: Vec<i32>,
    pub skills: Vec<PlayerSpellAcquisitionSkillRowLikeCpp>,
}

/// The exact deterministic replacement operations emitted by the application
/// planner. The MariaDB adapter executes them in vector order after it has
/// locked and compared the source authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpellAcquisitionDurableOperationLikeCpp {
    LockCharacter,
    DeleteSpells,
    DeleteFavoriteSpells,
    DeleteSkills,
    InsertSpell(PlayerSpellAcquisitionSpellRowLikeCpp),
    InsertFavoriteSpell(i32),
    InsertSkill(PlayerSpellAcquisitionSkillRowLikeCpp),
}

/// One atomic trainer fee plus spell/favorite/skill replacement attempt.
///
/// The opaque token attributes an ambiguous COMMIT to this exact attempt; it
/// is not a business identifier and the adapter must compare all 16 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellAcquisitionPersistenceRequestLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub operation_token: [u8; 16],
    pub source_authority: PlayerSpellAcquisitionAuthorityLikeCpp,
    pub resulting_authority: PlayerSpellAcquisitionAuthorityLikeCpp,
    pub operations: Vec<PlayerSpellAcquisitionDurableOperationLikeCpp>,
}

impl PlayerSpellAcquisitionPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerSpellAcquisitionPersistenceAttemptLikeCpp {
    Applied,
    DefinitelyRolledBack {
        reason: String,
        retryable_deadlock: bool,
    },
    CommitOutcomeUnknown {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpellAcquisitionMoneyReconciliationLikeCpp {
    Committed,
    Indeterminate,
}

pub fn classify_player_spell_acquisition_money_reconciliation_like_cpp(
    money_after: u64,
    observed_money: u64,
    durable_matches: bool,
    operation_matches: bool,
) -> PlayerSpellAcquisitionMoneyReconciliationLikeCpp {
    if observed_money == money_after && durable_matches && operation_matches {
        PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Committed
    } else {
        PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Indeterminate
    }
}

/// SQLx-free Characters-database capability for the represented trainer
/// spell-acquisition transaction and its strict lost-COMMIT proof.
pub trait PlayerSpellAcquisitionPersistencePortLikeCpp: Send + Sync {
    fn attempt_player_spell_acquisition_like_cpp(
        &self,
        request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerSpellAcquisitionPersistenceAttemptLikeCpp>;

    fn reconcile_player_spell_acquisition_like_cpp(
        &self,
        request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerSpellAcquisitionMoneyReconciliationLikeCpp>;
}
