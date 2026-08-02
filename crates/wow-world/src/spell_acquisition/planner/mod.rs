// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[derive(Clone, Copy)]
pub(crate) struct SpellAcquisitionMetadataLikeCpp<'a> {
    pub catalog: &'a SpellAcquisitionCatalogLikeCpp,
    pub spell_chains: &'a SpellChainStoreLikeCpp,
    pub spell_learn_skills: &'a SpellLearnSkillStoreLikeCpp,
    pub spell_learn_spells: &'a SpellLearnSpellStoreLikeCpp,
    pub spell_required: &'a SpellRequiredStoreLikeCpp,
    pub spell_custom_attributes: &'a SpellCustomAttributeStoreLikeCpp,
    pub trait_definitions: &'a TraitDefinitionStore,
    pub cast_authority: &'a SpellAcquisitionCastAuthorityLikeCpp,
    pub craft_validity_authority: &'a SpellAcquisitionCraftValidityAuthorityLikeCpp,
    /// Complete effective `Mount.db2` source-spell index. Mount collection
    /// mutation is outside this pure prerequisite; a mount spell fails closed
    /// because C++ `CollectionMgr::AddMount` may recursively learn its
    /// faction-specific counterpart.
    pub mounts: Option<&'a MountStore>,
    pub skills: &'a SkillStore,
    pub skill_lines: &'a SkillLineStore,
    pub skill_tiers: &'a SkillTiersStoreLikeCpp,
}

#[derive(Debug, Clone)]
struct EffectiveSpellProjectionLikeCpp {
    effects: Vec<SpellAcquisitionEffectLikeCpp>,
    misc: Option<SpellAcquisitionMiscLikeCpp>,
    talent: bool,
    chain: Option<SpellChainNodeLikeCpp>,
    learn_skill: Option<SpellLearnSkillNodeLikeCpp>,
    dependencies: Vec<SpellLearnSpellNodeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CastSideEffectProjectionPolicyLikeCpp {
    RequireComplete,
    DeferUnavailable,
}

#[derive(Clone)]
struct SpellAcquisitionPlannerLikeCpp<'a> {
    root: SpellAcquisitionRootLikeCpp,
    source_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    metadata: SpellAcquisitionMetadataLikeCpp<'a>,
    cast_side_effect_policy: CastSideEffectProjectionPolicyLikeCpp,
    race: u8,
    class: u8,
    level: u8,
    lifecycle: PlayerAcquisitionLifecycleLikeCpp,
    future_player_condition_resolutions: Vec<PlayerFuturePlayerConditionResolutionLikeCpp>,
    future_player_condition_resolution_cursor: usize,
    cast_resolutions: BTreeMap<u32, PlayerCastAcquisitionResolutionLikeCpp>,
    spells: BTreeMap<u32, PlayerSpellAcquisitionRowLikeCpp>,
    skills: BTreeMap<u32, PlayerSkillAcquisitionRowLikeCpp>,
    occupied_skill_slots: u16,
    overrides: BTreeSet<(u32, u32)>,
    /// Structural `SkillLine.ParentSkillLineID` path only.
    ///
    /// C++ can legitimately re-enter `SetSkill` through rewarded spells after
    /// it has provisionally written a skill row.  Treating every re-entry as a
    /// parent cycle rejects those convergent graphs.  Parent recursion is the
    /// malformed structural cycle that must be diagnosed eagerly; any other
    /// non-converging graph is stopped by `work_limit`.
    parent_skill_path: Vec<u32>,
    /// Absent skill rows that are being expanded before their provisional
    /// insertion. This is deliberately separate from the structural parent
    /// path: C++ root-child expansion may revisit the pending row without
    /// forming a malformed `ParentSkillLineID` cycle.
    pending_skill_insertions: Vec<u32>,
    spell_validation_visiting: Vec<u32>,
    validated_spells: BTreeSet<u32>,
    spell_transitions: Vec<PlannedSpellTransitionLikeCpp>,
    skill_transitions: Vec<PlannedSkillTransitionLikeCpp>,
    override_transitions: Vec<PlannedOverrideTransitionLikeCpp>,
    mutations: Vec<PlannedAcquisitionMutationLikeCpp>,
    root_primary_profession_skill_ids: Vec<u32>,
    publication_requirements: Vec<SpellAcquisitionPublicationRequirementLikeCpp>,
    post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
    diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
    work_count: usize,
    work_limit: usize,
}

pub(crate) fn project_spell_acquisition_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    metadata: SpellAcquisitionMetadataLikeCpp<'_>,
    root: SpellAcquisitionRootLikeCpp,
) -> SpellAcquisitionOutcomeLikeCpp {
    project_spell_acquisition_with_cast_policy_like_cpp(
        snapshot,
        metadata,
        root,
        CastSideEffectProjectionPolicyLikeCpp::RequireComplete,
    )
}

pub(crate) fn project_effect_learn_spell_acquisition_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    metadata: SpellAcquisitionMetadataLikeCpp<'_>,
    spell_id: u32,
) -> SpellAcquisitionOutcomeLikeCpp {
    project_spell_acquisition_with_cast_policy_like_cpp(
        snapshot,
        metadata,
        SpellAcquisitionRootLikeCpp::DirectLearn(spell_id),
        CastSideEffectProjectionPolicyLikeCpp::DeferUnavailable,
    )
}

fn project_spell_acquisition_with_cast_policy_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    metadata: SpellAcquisitionMetadataLikeCpp<'_>,
    root: SpellAcquisitionRootLikeCpp,
    cast_side_effect_policy: CastSideEffectProjectionPolicyLikeCpp,
) -> SpellAcquisitionOutcomeLikeCpp {
    let result =
        SpellAcquisitionPlannerLikeCpp::new(snapshot, metadata, root, cast_side_effect_policy)
            .and_then(SpellAcquisitionPlannerLikeCpp::project);
    match result {
        Ok(plan) => SpellAcquisitionOutcomeLikeCpp::Deterministic(plan),
        Err(reason) => SpellAcquisitionOutcomeLikeCpp::Indeterminate(reason),
    }
}

impl<'a> SpellAcquisitionPlannerLikeCpp<'a> {
    fn new(
        snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        metadata: SpellAcquisitionMetadataLikeCpp<'a>,
        root: SpellAcquisitionRootLikeCpp,
        cast_side_effect_policy: CastSideEffectProjectionPolicyLikeCpp,
    ) -> Result<Self, SpellAcquisitionIndeterminateLikeCpp> {
        if race_mask_for_race_like_cpp(snapshot.race) == 0 {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "race",
                value: i128::from(snapshot.race),
            });
        }
        if !(CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP).contains(&snapshot.class) {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "class",
                value: i128::from(snapshot.class),
            });
        }
        if snapshot.level == 0 {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "level",
                value: 0,
            });
        }

        let mut spells = BTreeMap::new();
        for row in &snapshot.spells {
            if row.spell_id == 0 {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "spell_id",
                    value: 0,
                });
            }
            if row.trait_definition_id.is_some_and(|id| id <= 0) {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "trait_definition_id",
                    value: i128::from(row.trait_definition_id.unwrap_or_default()),
                });
            }
            if spells.insert(row.spell_id, *row).is_some() {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::DuplicateSnapshotSpell {
                        spell_id: row.spell_id,
                    },
                );
            }
        }

        let mut skills = BTreeMap::new();
        for row in &snapshot.skills {
            if row.skill_id == 0 {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "skill_id",
                    value: 0,
                });
            }
            if u16::try_from(row.skill_id).is_err() {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::InvalidSkillIdentifier {
                        value: i64::from(row.skill_id),
                        source: "player snapshot",
                    },
                );
            }
            if usize::from(row.step) > wow_data::MAX_SKILL_STEP_LIKE_CPP {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                    skill_id: row.skill_id,
                    step: i64::from(row.step),
                });
            }
            match row.profession_association {
                ProfessionAssociationInputLikeCpp::Invalid(value) => {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                        field: "profession_association",
                        value: i128::from(value),
                    });
                }
                ProfessionAssociationInputLikeCpp::Slot(slot) if slot > 1 => {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                        field: "profession_association",
                        value: i128::from(slot),
                    });
                }
                ProfessionAssociationInputLikeCpp::Unassigned
                | ProfessionAssociationInputLikeCpp::Slot(_) => {}
            }
            if row.state == PlayerSkillPersistenceStateLikeCpp::Deleted
                && (row.step != 0
                    || row.value != 0
                    || row.maximum != 0
                    || row.profession_association != ProfessionAssociationInputLikeCpp::Unassigned)
            {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "deleted_skill_payload",
                    value: i128::from(row.skill_id),
                });
            }
            if row.value > row.maximum && row.maximum != 0 {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "skill_value_above_maximum",
                    value: i128::from(row.skill_id),
                });
            }
            if skills.insert(row.skill_id, *row).is_some() {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::DuplicateSnapshotSkill {
                        skill_id: row.skill_id,
                    },
                );
            }
        }
        let mut primary_profession_skill_ids = snapshot.primary_profession_skill_ids.clone();
        primary_profession_skill_ids.sort_unstable();
        if primary_profession_skill_ids
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "primary_profession_skill_ids",
                value: 0,
            });
        }
        let mut expected_primary_profession_skill_ids = Vec::new();
        for skill in skills.values().filter(|skill| {
            skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted && skill.value != 0
        }) {
            match metadata
                .skill_lines
                .is_primary_profession_skill_like_cpp(skill.skill_id)
            {
                Some(true) => expected_primary_profession_skill_ids.push(skill.skill_id),
                Some(false) => {}
                None => {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSkillLine {
                        skill_id: skill.skill_id,
                    });
                }
            }
        }
        expected_primary_profession_skill_ids.sort_unstable();
        if primary_profession_skill_ids != expected_primary_profession_skill_ids {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "primary_profession_skill_ids",
                value: 0,
            });
        }
        if usize::from(snapshot.occupied_skill_slots) > MAX_PLAYER_SKILLS_LIKE_CPP {
            return Err(SpellAcquisitionIndeterminateLikeCpp::PlayerSkillCapacityExceeded);
        }
        // C++ `mSkillStatus` retains one identity for every occupied
        // SkillLineID update-field slot, including SKILL_DELETED rows. A
        // larger count would therefore conceal a skill identity whose
        // HasSkill/reward/parent semantics the projection cannot prove.
        if usize::from(snapshot.occupied_skill_slots) != skills.len() {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "occupied_skill_slots",
                value: i128::from(snapshot.occupied_skill_slots),
            });
        }
        let mut overrides = BTreeSet::new();
        for &(overridden_spell_id, overriding_spell_id) in &snapshot.overrides {
            if overridden_spell_id == 0 || overriding_spell_id == 0 {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "spell_override",
                    value: i128::from(overridden_spell_id.min(overriding_spell_id)),
                });
            }
            overrides.insert((overridden_spell_id, overriding_spell_id));
        }
        for (&spell_id, resolution) in &snapshot.cast_resolutions {
            if spell_id == 0
                || (!resolution.reached_immediate_phase
                    && (resolution.executed_hit_target_effect_mask != 0
                        || resolution
                            .executed_dual_wield_effects
                            .iter()
                            .next()
                            .is_some()))
                || resolution.executed_dual_wield_effects.iter().any(|effect| {
                    effect.effect_record_id == 0
                        || effect.effect_index >= 32
                        || resolution.executed_hit_target_effect_mask
                            & (1_u32 << u32::from(effect.effect_index))
                            == 0
                })
                || resolution
                    .executed_dual_wield_effects
                    .iter()
                    .map(|effect| (effect.effect_index, effect.effect_record_id))
                    .collect::<BTreeSet<_>>()
                    .len()
                    != resolution.executed_dual_wield_effects.len()
                || resolution.effective_effects.iter().any(|effect| {
                    effect.spell_id_checked().ok() != Some(spell_id)
                        || effect.effect_index_checked().is_err()
                })
                || resolution
                    .effective_effects
                    .iter()
                    .filter_map(|effect| effect.effect_index_checked().ok())
                    .collect::<BTreeSet<_>>()
                    .len()
                    != resolution.effective_effects.len()
            {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::InvalidCastResolution {
                        spell_id,
                        effect_index: None,
                    },
                );
            }
        }

        Ok(Self {
            root,
            source_snapshot: snapshot.clone(),
            metadata,
            cast_side_effect_policy,
            race: snapshot.race,
            class: snapshot.class,
            level: snapshot.level,
            lifecycle: snapshot.lifecycle,
            future_player_condition_resolutions: snapshot
                .future_player_condition_resolutions
                .clone(),
            future_player_condition_resolution_cursor: 0,
            cast_resolutions: snapshot.cast_resolutions.clone(),
            spells,
            skills,
            occupied_skill_slots: snapshot.occupied_skill_slots,
            overrides,
            parent_skill_path: Vec::new(),
            pending_skill_insertions: Vec::new(),
            spell_validation_visiting: Vec::new(),
            validated_spells: BTreeSet::new(),
            spell_transitions: Vec::new(),
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            mutations: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: Vec::new(),
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            work_count: 0,
            work_limit: DEFAULT_ACQUISITION_WORK_LIMIT,
        })
    }

    fn project(
        mut self,
    ) -> Result<SpellAcquisitionPlanLikeCpp, SpellAcquisitionIndeterminateLikeCpp> {
        let root_provenance = SpellAcquisitionProvenanceLikeCpp::Root { root: self.root };
        match self.root {
            SpellAcquisitionRootLikeCpp::DirectLearn(spell_id) => {
                self.learn_spell_like_cpp(spell_id, false, 0, root_provenance)?;
            }
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(spell_id) => {
                if !self.lifecycle.is_in_world() {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                        field: "trainer_wrapper_lifecycle",
                        value: self.lifecycle as i128,
                    });
                }
                let projection = self.effective_spell_projection_like_cpp(spell_id)?;
                self.simulate_cast_like_cpp(
                    spell_id,
                    &projection.effects,
                    PlannedAcquisitionCastReasonLikeCpp::TrainerWrapper,
                    true,
                )?;
            }
        }

        let profession_association_inputs = self.skills.values().copied().collect::<Vec<_>>();
        let resulting_snapshot = PlayerSpellAcquisitionSnapshotLikeCpp {
            character_guid: self.source_snapshot.character_guid,
            spells: self.spells.values().copied().collect(),
            skills: self.skills.values().copied().collect(),
            occupied_skill_slots: self.occupied_skill_slots,
            overrides: self.overrides.iter().copied().collect(),
            primary_profession_skill_ids: self
                .skills
                .values()
                .filter(|skill| {
                    skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                        && skill.value != 0
                        && self
                            .metadata
                            .skill_lines
                            .is_primary_profession_skill_like_cpp(skill.skill_id)
                            == Some(true)
                })
                .map(|skill| skill.skill_id)
                .collect(),
            non_durable_skill_tombstone_ids: self
                .source_snapshot
                .non_durable_skill_tombstone_ids
                .iter()
                .copied()
                .filter(|skill_id| {
                    self.skills.get(skill_id).is_some_and(|skill| {
                        skill.step == 0
                            && skill.value == 0
                            && skill.maximum == 0
                            && skill.profession_association
                                == ProfessionAssociationInputLikeCpp::Unassigned
                            && matches!(
                                skill.state,
                                PlayerSkillPersistenceStateLikeCpp::Unchanged
                                    | PlayerSkillPersistenceStateLikeCpp::Deleted
                            )
                    })
                })
                .collect(),
            race: self.race,
            class: self.class,
            level: self.level,
            lifecycle: self.lifecycle,
            // Rechecks must receive the same immutable authority tape. The
            // planner cursor is deliberately not persisted into player state.
            future_player_condition_resolutions: self.future_player_condition_resolutions,
            cast_resolutions: self.cast_resolutions,
        };

        Ok(SpellAcquisitionPlanLikeCpp {
            root: self.root,
            source_snapshot: self.source_snapshot,
            mutations: self.mutations,
            spell_transitions: self.spell_transitions,
            skill_transitions: self.skill_transitions,
            override_transitions: self.override_transitions,
            root_primary_profession_skill_ids: self.root_primary_profession_skill_ids,
            publication_requirements: self.publication_requirements,
            profession_association_inputs,
            post_commit_actions: self.post_commit_actions,
            diagnostics: self.diagnostics,
            resulting_snapshot,
        })
    }

    fn consume_work_like_cpp(&mut self) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        self.work_count = self.work_count.saturating_add(1);
        if self.work_count > self.work_limit {
            return Err(SpellAcquisitionIndeterminateLikeCpp::WorkLimitExceeded {
                limit: self.work_limit,
            });
        }
        Ok(())
    }

    fn has_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.spells.get(&spell_id).is_some_and(|spell| {
            spell.state != PlayerSpellPersistenceStateLikeCpp::Removed && !spell.disabled
        })
    }

    fn has_skill_like_cpp(&self, skill_id: u32) -> bool {
        self.skills.get(&skill_id).is_some_and(|skill| {
            skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted && skill.value > 0
        })
    }

    fn effective_spell_projection_like_cpp(
        &mut self,
        spell_id: u32,
    ) -> Result<EffectiveSpellProjectionLikeCpp, SpellAcquisitionIndeterminateLikeCpp> {
        self.consume_work_like_cpp()?;
        let effects = match self
            .metadata
            .catalog
            .difficulty_none_effects_like_cpp(spell_id)
        {
            SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSpellCoverage {
                    spell_id,
                    table: SpellAcquisitionTableLikeCpp::SpellEffect,
                });
            }
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(reasons) => {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                        spell_id,
                        table: SpellAcquisitionTableLikeCpp::SpellEffect,
                        reasons: reasons.to_vec(),
                    },
                );
            }
            SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) => effects.to_vec(),
        };

        let misc = match self
            .metadata
            .catalog
            .misc_for_spell_like_cpp(spell_id, DIFFICULTY_NONE_LIKE_CPP)
        {
            SpellAcquisitionMetadataLookupLikeCpp::MissingCoverage => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSpellCoverage {
                    spell_id,
                    table: SpellAcquisitionTableLikeCpp::SpellMisc,
                });
            }
            SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(reasons) => {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                        spell_id,
                        table: SpellAcquisitionTableLikeCpp::SpellMisc,
                        reasons: reasons.to_vec(),
                    },
                );
            }
            SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow => None,
            SpellAcquisitionMetadataLookupLikeCpp::Present(misc) => Some(misc.clone()),
        };

        match self
            .metadata
            .catalog
            .dependency_rows_lookup_like_cpp(spell_id)
        {
            SpellAcquisitionDependenciesLookupLikeCpp::MissingCoverage => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSpellCoverage {
                    spell_id,
                    table: SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                });
            }
            SpellAcquisitionDependenciesLookupLikeCpp::Indeterminate(reasons) => {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                        spell_id,
                        table: SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                        reasons: reasons.to_vec(),
                    },
                );
            }
            SpellAcquisitionDependenciesLookupLikeCpp::Covered(rows) => {
                let derived = self
                    .metadata
                    .spell_learn_spells
                    .get_spell_learn_spell_map_bounds_like_cpp(spell_id);
                for row in rows {
                    let learned_spell_id = row.learned_spell_id_checked().map_err(|error| {
                        SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                            record_id: row.record_id,
                            field: error.field,
                            raw: error.raw,
                        }
                    })?;
                    if !derived.iter().any(|node| node.spell == learned_spell_id) {
                        return Err(
                            SpellAcquisitionIndeterminateLikeCpp::MissingDerivedDependency {
                                source_spell_id: spell_id,
                                learned_spell_id,
                            },
                        );
                    }
                }
            }
        }

        let talent = if self
            .metadata
            .spell_custom_attributes
            .attributes_for_spell_difficulty_like_cpp(spell_id, DIFFICULTY_NONE_LIKE_CPP)
            & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
            != 0
        {
            true
        } else {
            match self.metadata.catalog.talent_membership_like_cpp(spell_id) {
                SpellAcquisitionTalentLookupLikeCpp::Talent => true,
                SpellAcquisitionTalentLookupLikeCpp::NotTalent => false,
                SpellAcquisitionTalentLookupLikeCpp::Indeterminate(reasons) => {
                    return Err(
                        SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                            spell_id,
                            table: SpellAcquisitionTableLikeCpp::Talent,
                            reasons: reasons.to_vec(),
                        },
                    );
                }
            }
        };

        let chain = match self
            .metadata
            .spell_chains
            .spell_chain_lookup_like_cpp(spell_id)
        {
            SpellChainLookupLikeCpp::Unranked => None,
            SpellChainLookupLikeCpp::Node(node) => Some(*node),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::RankChain {
                    spell_id,
                    diagnostics: diagnostics.to_vec(),
                });
            }
        };

        let learn_skill = match self
            .metadata
            .spell_learn_skills
            .spell_learn_skill_lookup_like_cpp(spell_id)
        {
            SpellLearnSkillLookupLikeCpp::Present(node) => Some(*node),
            SpellLearnSkillLookupLikeCpp::CoveredWithoutNode => None,
            SpellLearnSkillLookupLikeCpp::Indeterminate(reason) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::LearnSkill {
                    spell_id,
                    reason: reason.clone(),
                });
            }
            SpellLearnSkillLookupLikeCpp::MissingCoverage => {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::MissingLearnSkillCoverage { spell_id },
                );
            }
        };

        Ok(EffectiveSpellProjectionLikeCpp {
            effects,
            misc,
            talent,
            chain,
            learn_skill,
            dependencies: self
                .metadata
                .spell_learn_spells
                .get_spell_learn_spell_map_bounds_like_cpp(spell_id)
                .to_vec(),
        })
    }

    fn validate_spell_definition_like_cpp(
        &mut self,
        spell_id: u32,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        if self.validated_spells.contains(&spell_id) {
            return Ok(());
        }
        if let Some(cycle_start) = self
            .spell_validation_visiting
            .iter()
            .position(|visiting| *visiting == spell_id)
        {
            let mut spell_ids = self.spell_validation_visiting[cycle_start..].to_vec();
            spell_ids.push(spell_id);
            return Err(SpellAcquisitionIndeterminateLikeCpp::SpellValidationCycle { spell_ids });
        }
        self.consume_work_like_cpp()?;
        let effects = match self
            .metadata
            .catalog
            .difficulty_none_effects_like_cpp(spell_id)
        {
            SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSpellCoverage {
                    spell_id,
                    table: SpellAcquisitionTableLikeCpp::SpellEffect,
                });
            }
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(reasons) => {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                        spell_id,
                        table: SpellAcquisitionTableLikeCpp::SpellEffect,
                        reasons: reasons.to_vec(),
                    },
                );
            }
            SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) => effects.to_vec(),
        };

        self.spell_validation_visiting.push(spell_id);
        let result = (|| {
            for effect in effects {
                let effect_type = effect.effect_type_checked().map_err(|error| {
                    SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    }
                })?;
                let _effect_index = effect.effect_index_checked().map_err(|error| {
                    SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    }
                })?;
                if matches!(
                    effect_type,
                    SPELL_EFFECT_CREATE_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP
                ) {
                    // Exact C++ validity also needs ItemTemplate and reagent
                    // authority.  The bootstrap proof is separate from cast
                    // safety because this check runs before PlayerSpellMap
                    // state is inspected.
                    self.metadata
                        .craft_validity_authority
                        .require_valid_like_cpp(spell_id)?;
                }
                if effect_type == SPELL_EFFECT_LEARN_SPELL {
                    let learned_spell_id = effect.trigger_spell_id_checked().map_err(|error| {
                        SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                            record_id: effect.record_id,
                            field: error.field,
                            raw: error.raw,
                        }
                    })?;
                    self.validate_spell_definition_like_cpp(learned_spell_id)?;
                }
            }
            Ok(())
        })();
        let popped = self.spell_validation_visiting.pop();
        debug_assert_eq!(popped, Some(spell_id));
        if result.is_ok() {
            self.validated_spells.insert(spell_id);
        }
        result
    }

    fn record_spell_transition_like_cpp(
        &mut self,
        spell_id: u32,
        before: Option<PlayerSpellAcquisitionRowLikeCpp>,
        after: Option<PlayerSpellAcquisitionRowLikeCpp>,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) {
        if before != after {
            let transition = PlannedSpellTransitionLikeCpp {
                spell_id,
                before,
                after,
                provenance,
            };
            self.spell_transitions.push(transition.clone());
            self.mutations
                .push(PlannedAcquisitionMutationLikeCpp::Spell(transition));
        }
    }

    fn record_skill_transition_like_cpp(
        &mut self,
        skill_id: u32,
        before: Option<PlayerSkillAcquisitionRowLikeCpp>,
        after: PlayerSkillAcquisitionRowLikeCpp,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) {
        if before == Some(after) {
            return;
        }
        let transition = PlannedSkillTransitionLikeCpp {
            skill_id,
            before,
            after,
            provenance,
        };
        self.skill_transitions.push(transition.clone());
        self.mutations
            .push(PlannedAcquisitionMutationLikeCpp::Skill(transition));
    }

    fn replace_spell_row_like_cpp(
        &mut self,
        row: PlayerSpellAcquisitionRowLikeCpp,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) {
        let before = self.spells.insert(row.spell_id, row);
        self.record_spell_transition_like_cpp(row.spell_id, before, Some(row), provenance);
    }

    fn remove_spell_row_like_cpp(
        &mut self,
        spell_id: u32,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) {
        let before = self.spells.remove(&spell_id);
        self.record_spell_transition_like_cpp(spell_id, before, None, provenance);
    }
}

mod cast;
mod skill;
mod spell;

use cast::{AddSpellAutocastResultLikeCpp, effect_can_change_acquisition_like_cpp};
