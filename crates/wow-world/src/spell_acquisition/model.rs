// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionRootLikeCpp {
    DirectLearn(u32),
    TrainerWrapperCast(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellPersistenceStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Removed,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerSpellAcquisitionRowLikeCpp {
    pub spell_id: u32,
    pub active: bool,
    pub disabled: bool,
    pub dependent: bool,
    pub favorite: bool,
    pub trait_definition_id: Option<i32>,
    pub state: PlayerSpellPersistenceStateLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ProfessionAssociationInputLikeCpp {
    Unassigned,
    Slot(u8),
    Invalid(i8),
}

impl ProfessionAssociationInputLikeCpp {
    pub(crate) const fn from_database_value_like_cpp(value: i8) -> Self {
        match value {
            -1 => Self::Unassigned,
            0 | 1 => Self::Slot(value as u8),
            other => Self::Invalid(other),
        }
    }

    pub(crate) const fn database_value_like_cpp(self) -> i8 {
        match self {
            Self::Unassigned => -1,
            Self::Slot(slot) => slot as i8,
            Self::Invalid(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSkillPersistenceStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerSkillAcquisitionRowLikeCpp {
    pub skill_id: u32,
    pub step: u16,
    pub value: u16,
    pub maximum: u16,
    pub profession_association: ProfessionAssociationInputLikeCpp,
    pub state: PlayerSkillPersistenceStateLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAcquisitionLifecycleLikeCpp {
    Loading,
    OutOfWorld,
    InWorld,
}

impl PlayerAcquisitionLifecycleLikeCpp {
    pub(super) const fn is_in_world(self) -> bool {
        matches!(self, Self::InWorld)
    }

    pub(super) const fn is_loading(self) -> bool {
        matches!(self, Self::Loading)
    }
}

/// Pre-evaluated, player-specific result of the C++ spell target pipeline.
///
/// `TRIGGERED_FULL_MASK` does not skip `SelectSpellTargets`,
/// `SpellHitResult`, or per-effect immunity. A wrapper may therefore execute
/// HANDLE_HIT `SKILL` effects and still suppress later HIT_TARGET
/// `LEARN_SPELL` / `SKILL_STEP` effects. Static spell metadata cannot prove
/// that outcome for an arbitrary live player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerCastAcquisitionResolutionLikeCpp {
    pub reached_immediate_phase: bool,
    pub executed_hit_target_effect_mask: u32,
}

/// One causal evaluation of C++
/// `SpellInfo::MeetsFutureSpellPlayerCondition`.
///
/// This is deliberately an ordered tape rather than a map. The same
/// condition can observe different logical player state after an earlier
/// skill reward has learned a spell or raised a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerFuturePlayerConditionResolutionLikeCpp {
    pub condition_id: u32,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerSpellAcquisitionSnapshotLikeCpp {
    pub spells: Vec<PlayerSpellAcquisitionRowLikeCpp>,
    pub skills: Vec<PlayerSkillAcquisitionRowLikeCpp>,
    /// Number of occupied C++ `ActivePlayerData::Skill` slots. Tombstoned
    /// map entries are not inferred from `skills.len()`.
    pub occupied_skill_slots: u16,
    /// Existing C++ `m_overrideSpells` edges.
    pub overrides: Vec<(u32, u32)>,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub lifecycle: PlayerAcquisitionLifecycleLikeCpp,
    /// Pure results for each causal invocation of
    /// `SpellInfo::MeetsFutureSpellPlayerCondition`, in invocation order.
    pub future_player_condition_resolutions: Vec<PlayerFuturePlayerConditionResolutionLikeCpp>,
    /// Exact outcomes for each acquisition-bearing self cast. Missing
    /// authority fails closed rather than assuming the player has no
    /// immunities or target-script interference.
    pub cast_resolutions: BTreeMap<u32, PlayerCastAcquisitionResolutionLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionProvenanceLikeCpp {
    Root {
        root: SpellAcquisitionRootLikeCpp,
    },
    PreviousRank {
        requested_spell_id: u32,
    },
    LearnDependency {
        source_spell_id: u32,
    },
    HigherDisabledRank {
        source_spell_id: u32,
    },
    RequiredDisabledSpell {
        required_spell_id: u32,
    },
    DirectLearnSkill {
        source_spell_id: u32,
    },
    SkillLineAbilityFallback {
        source_spell_id: u32,
        record_id: u32,
    },
    ParentSkill {
        child_skill_id: u32,
    },
    RootChildSkill {
        parent_skill_id: u32,
    },
    SkillReward {
        skill_id: u32,
        record_id: u32,
    },
    WrapperEffect {
        wrapper_spell_id: u32,
        effect_index: u8,
        record_id: u32,
    },
    AutocastEffect {
        source_spell_id: u32,
        effect_index: u8,
        record_id: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedSpellTransitionLikeCpp {
    pub spell_id: u32,
    pub before: Option<PlayerSpellAcquisitionRowLikeCpp>,
    pub after: Option<PlayerSpellAcquisitionRowLikeCpp>,
    pub provenance: SpellAcquisitionProvenanceLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedSkillTransitionLikeCpp {
    pub skill_id: u32,
    pub before: Option<PlayerSkillAcquisitionRowLikeCpp>,
    pub after: PlayerSkillAcquisitionRowLikeCpp,
    pub provenance: SpellAcquisitionProvenanceLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedOverrideTransitionLikeCpp {
    pub overridden_spell_id: u32,
    pub overriding_spell_id: u32,
    pub add: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedAcquisitionMutationLikeCpp {
    Spell(PlannedSpellTransitionLikeCpp),
    Skill(PlannedSkillTransitionLikeCpp),
    Override(PlannedOverrideTransitionLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedAcquisitionCastReasonLikeCpp {
    TalentLearnEffect,
    PassiveLearn,
    SkillStep,
    CastWhenLearned,
    TrainerWrapper,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionPostCommitActionLikeCpp {
    LearnedSpell {
        spell_id: u32,
        favorite: bool,
        suppress_messaging: bool,
    },
    SupersededSpell {
        old_spell_id: u32,
        new_spell_id: u32,
    },
    UnlearnedSpell {
        spell_id: u32,
    },
    GrantDualWield {
        source_spell_id: u32,
        effect_record_id: u32,
        effect_index: u8,
    },
    RefreshPassive {
        spell_id: u32,
    },
    UpdateLearnSpellQuestObjective {
        spell_id: u32,
    },
    UpdateLearnTradeskillSkillLineCriteria {
        source_spell_id: u32,
        skill_id: u32,
    },
    UpdateLearnSpellFromSkillLineCriteria {
        source_spell_id: u32,
        skill_id: u32,
    },
    UpdateLearnOrKnowSpellCriteria {
        spell_id: u32,
    },
    UpdateMountCapability {
        skill_id: u32,
    },
    UpdateSkillRaisedCriteria {
        skill_id: u32,
    },
    UpdateAchieveSkillStepCriteria {
        skill_id: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionDiagnosticLikeCpp {
    ExistingSpellAlreadyMatches {
        spell_id: u32,
    },
    ExistingInactiveSpellActivated {
        spell_id: u32,
    },
    SkillRaceClassNotApplicable {
        skill_id: u32,
    },
    RewardGateRejected {
        skill_id: u32,
        record_id: u32,
        gate: &'static str,
    },
    EffectHadNoRuntimeChange {
        spell_id: u32,
        effect_index: u8,
        reason: &'static str,
    },
    AcquisitionCastProjected {
        spell_id: u32,
        reason: PlannedAcquisitionCastReasonLikeCpp,
    },
    DualWieldEffectProjected {
        spell_id: u32,
        effect_record_id: u32,
        effect_index: u8,
    },
    CastStoppedBeforeImmediatePhase {
        spell_id: u32,
    },
    HitTargetEffectSuppressed {
        spell_id: u32,
        effect_index: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellAcquisitionPlanLikeCpp {
    pub root: SpellAcquisitionRootLikeCpp,
    /// Exact immutable player authority from which this plan was projected.
    /// The application boundary compares it byte-for-byte with the current
    /// snapshot before opening a transaction; transition-local `before`
    /// values alone cannot prove that untouched rows did not change.
    pub source_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    /// One cross-domain causal stream. The typed projections below are
    /// retained for focused consumers, but must never be used to reconstruct
    /// ordering between a skill write and a recursively learned spell.
    pub mutations: Vec<PlannedAcquisitionMutationLikeCpp>,
    pub spell_transitions: Vec<PlannedSpellTransitionLikeCpp>,
    pub skill_transitions: Vec<PlannedSkillTransitionLikeCpp>,
    pub override_transitions: Vec<PlannedOverrideTransitionLikeCpp>,
    pub root_primary_profession_skill_ids: Vec<u32>,
    pub profession_association_inputs: Vec<PlayerSkillAcquisitionRowLikeCpp>,
    pub post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
    pub diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
    pub resulting_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionIndeterminateLikeCpp {
    SnapshotAdapter(SpellAcquisitionSnapshotAdapterErrorLikeCpp),
    MissingTrainerProjectionMetadata,
    InvalidSnapshot {
        field: &'static str,
        value: i128,
    },
    DuplicateSnapshotSpell {
        spell_id: u32,
    },
    DuplicateSnapshotSkill {
        skill_id: u32,
    },
    MissingSpellCoverage {
        spell_id: u32,
        table: SpellAcquisitionTableLikeCpp,
    },
    IndeterminateSpellMetadata {
        spell_id: u32,
        table: SpellAcquisitionTableLikeCpp,
        reasons: Vec<SpellAcquisitionIndeterminateReasonLikeCpp>,
    },
    InvalidEffectiveValue {
        record_id: u32,
        field: &'static str,
        raw: i64,
    },
    RankChain {
        spell_id: u32,
        diagnostics: Vec<SpellChainLoadDiagnosticLikeCpp>,
    },
    SpellValidationCycle {
        spell_ids: Vec<u32>,
    },
    UnsupportedSpellValidityPath {
        spell_id: u32,
        effect_index: u8,
        effect_type: u32,
    },
    CraftSpellValidityAuthority {
        spell_id: u32,
        reasons: Vec<SpellAcquisitionCraftValidityIndeterminateReasonLikeCpp>,
    },
    LearnSkill {
        spell_id: u32,
        reason: SpellLearnSkillIndeterminateReasonLikeCpp,
    },
    MissingLearnSkillCoverage {
        spell_id: u32,
    },
    SkillLineAbility {
        spell_id: Option<u32>,
        skill_id: Option<u32>,
        diagnostics: Vec<SkillStoreLoadDiagnosticLikeCpp>,
    },
    MissingSkillLine {
        skill_id: u32,
    },
    MissingTraitDefinition {
        trait_definition_id: u32,
    },
    IncompleteSkillLine {
        skill_id: u32,
    },
    InvalidSkillIdentifier {
        value: i64,
        source: &'static str,
    },
    InvalidSkillStep {
        skill_id: u32,
        step: i64,
    },
    MissingSkillTier {
        skill_id: u32,
        skill_tier_id: i16,
    },
    InvalidSkillTierValue {
        skill_id: u32,
        value: u32,
    },
    SkillParentCycle {
        skill_ids: Vec<u32>,
    },
    MissingFuturePlayerConditionResolution {
        spell_id: u32,
        condition_id: u32,
        occurrence_index: usize,
    },
    FuturePlayerConditionResolutionMismatch {
        spell_id: u32,
        occurrence_index: usize,
        expected_condition_id: u32,
        actual_condition_id: u32,
    },
    MissingCastResolution {
        spell_id: u32,
    },
    InvalidCastResolution {
        spell_id: u32,
        effect_index: Option<u8>,
    },
    UnsupportedPassiveAcquisition {
        spell_id: u32,
    },
    UnsupportedMountAcquisition {
        spell_id: u32,
    },
    IncompleteMountAuthority {
        spell_id: u32,
    },
    UnsupportedRuntimeEffect {
        spell_id: u32,
        effect_index: u8,
        effect_type: u32,
    },
    CastAuthority {
        spell_id: u32,
        reasons: Vec<SpellAcquisitionCastIndeterminateReasonLikeCpp>,
    },
    RuntimeCalcValue {
        spell_id: u32,
        effect_index: u8,
        field: &'static str,
    },
    UnsupportedEffectTarget {
        spell_id: u32,
        effect_index: u8,
        targets: [i64; 2],
    },
    CastItemLearnPath {
        spell_id: u32,
        effect_index: u8,
    },
    PetLearnPath {
        spell_id: u32,
        effect_index: u8,
    },
    BattlePetOrSummonPath {
        spell_id: u32,
        effect_index: u8,
    },
    MissingDerivedDependency {
        source_spell_id: u32,
        learned_spell_id: u32,
    },
    UnsupportedSkillDecrease {
        skill_id: u32,
        old_value: u16,
        new_value: u16,
    },
    RewardSpellRemovalRequired {
        skill_id: u32,
        spell_id: u32,
        record_id: u32,
    },
    PlayerSkillCapacityExceeded,
    WorkLimitExceeded {
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionOutcomeLikeCpp {
    Deterministic(SpellAcquisitionPlanLikeCpp),
    Indeterminate(SpellAcquisitionIndeterminateLikeCpp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellAcquisitionSnapshotAdapterErrorLikeCpp {
    IncompleteSpellRows,
    IncompleteSkillRows,
    MissingSkillSlotOccupancy,
    IncompleteTraitDefinitions,
    IncompleteOverrides,
    InvalidSpellId(i32),
    InvalidTraitDefinitionId {
        spell_id: i32,
        trait_definition_id: i32,
    },
    OrphanTraitDefinition {
        spell_id: i32,
    },
    InvalidOverride {
        overridden_spell_id: i32,
        overriding_spell_id: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpellAcquisitionCastIndeterminateReasonLikeCpp {
    ScriptBinding,
    LegacySpellScriptCommand,
    SpellPetAura,
    LinkedCast,
    LinkedHit,
    LinkedAura,
    CastCondition,
    TargetCondition,
    SpellModifierClassOptions,
    SpellModifierLabel,
    AuraLearnSpell,
    RuntimeCalcValue,
    DisabledSpell,
    PassiveCastPrerequisites,
    HardcodedDummyHandler,
    DelayedOrChanneled,
    UnsupportedTargetSelection,
    UnmodelledCheckCast,
    RuntimeStateMutationBeforeClosure,
    IncompleteAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellAcquisitionCastAuditEvidenceLikeCpp {
    pub spell_id: u32,
    pub all_sources_complete: bool,
    pub has_script_binding: bool,
    pub has_legacy_spell_script_command: bool,
    pub has_spell_pet_aura: bool,
    pub has_linked_cast: bool,
    pub has_linked_hit: bool,
    pub has_linked_aura: bool,
    pub has_cast_condition: bool,
    pub has_target_condition: bool,
    pub has_spell_modifier_class_options: bool,
    pub has_spell_modifier_label: bool,
    pub has_aura_learn_spell: bool,
    pub has_runtime_calc_value: bool,
    pub is_disabled: bool,
    pub has_hardcoded_dummy_handler: bool,
    pub is_delayed_or_channeled: bool,
    pub has_unsupported_target_selection: bool,
    pub has_unmodelled_check_cast: bool,
    pub has_runtime_state_mutation_before_closure: bool,
    /// Required only for passive casts; proves stance/equipment/aura-state
    /// checks cannot suppress the acquisition-bearing cast.
    pub passive_cast_prerequisites_proven: bool,
    pub is_passive_cast: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpellAcquisitionCraftValidityIndeterminateReasonLikeCpp {
    MissingCreatedItemTemplate(u32),
    MissingReagentItemTemplate(u32),
    IncompleteAuthority,
}
