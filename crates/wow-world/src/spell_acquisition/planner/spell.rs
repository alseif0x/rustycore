// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl SpellAcquisitionPlannerLikeCpp<'_> {
    pub(super) fn learn_spell_like_cpp(
        &mut self,
        spell_id: u32,
        dependent: bool,
        from_skill: u32,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let existing = self.spells.get(&spell_id).copied();
        let was_disabled = existing.is_some_and(|spell| spell.disabled);
        let active = existing
            .filter(|spell| spell.disabled)
            .map(|spell| spell.active)
            .unwrap_or(true);
        let favorite = existing.is_some_and(|spell| spell.favorite);
        let learning = self.add_spell_like_cpp(
            spell_id,
            active,
            true,
            dependent,
            false,
            false,
            from_skill,
            favorite,
            provenance.clone(),
        )?;

        if learning && self.lifecycle.is_in_world() {
            self.publication_requirements.push(
                SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                    spell_id,
                    favorite,
                    suppress_messaging: false,
                },
            );
            self.post_commit_actions
                .push(SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                    spell_id,
                    favorite,
                    suppress_messaging: false,
                });
        }

        if was_disabled {
            if let Some(next_spell_id) = self.next_spell_id_like_cpp(spell_id)?
                && self
                    .spells
                    .get(&next_spell_id)
                    .is_some_and(|spell| spell.disabled)
            {
                self.learn_spell_like_cpp(
                    next_spell_id,
                    false,
                    from_skill,
                    SpellAcquisitionProvenanceLikeCpp::HigherDisabledRank {
                        source_spell_id: spell_id,
                    },
                )?;
            }

            let requiring = self
                .metadata
                .spell_required
                .spells_requiring_spell_like_cpp(spell_id)
                .to_vec();
            for requiring_spell_id in requiring {
                if self
                    .spells
                    .get(&requiring_spell_id)
                    .is_some_and(|spell| spell.disabled)
                {
                    self.learn_spell_like_cpp(
                        requiring_spell_id,
                        false,
                        from_skill,
                        SpellAcquisitionProvenanceLikeCpp::RequiredDisabledSpell {
                            required_spell_id: spell_id,
                        },
                    )?;
                }
            }
        } else {
            self.publication_requirements.push(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id,
                },
            );
            self.post_commit_actions.push(
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id,
                },
            );
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_spell_like_cpp(
        &mut self,
        spell_id: u32,
        mut active: bool,
        learning: bool,
        dependent: bool,
        disabled: bool,
        _loading: bool,
        from_skill: u32,
        favorite: bool,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<bool, SpellAcquisitionIndeterminateLikeCpp> {
        self.validate_spell_definition_like_cpp(spell_id)?;
        self.consume_work_like_cpp()?;
        let projection = self.effective_spell_projection_like_cpp(spell_id)?;
        let mut state = if learning {
            PlayerSpellPersistenceStateLikeCpp::New
        } else {
            PlayerSpellPersistenceStateLikeCpp::Unchanged
        };
        let mut dependent_set = false;
        let mut disabled_case = false;
        let mut superceded_old = false;

        if let Some(existing) = self.spells.get(&spell_id).copied() {
            if existing.state == PlayerSpellPersistenceStateLikeCpp::Temporary {
                self.remove_spell_row_like_cpp(spell_id, provenance.clone());
            } else {
                let mut row = existing;
                let mut next_active_spell_id = None;
                if let Some(chain) = projection.chain
                    && let Some(next) = chain.next_spell_id
                    && self.has_spell_like_cpp(next)
                {
                    active = false;
                    next_active_spell_id = Some(next);
                }

                if row.state != PlayerSpellPersistenceStateLikeCpp::Removed
                    && row.active == active
                    && row.dependent == dependent
                    && row.disabled == disabled
                {
                    if !self.lifecycle.is_in_world() && !learning {
                        row.state = PlayerSpellPersistenceStateLikeCpp::Unchanged;
                        self.replace_spell_row_like_cpp(row, provenance);
                    }
                    self.diagnostics.push(
                        SpellAcquisitionDiagnosticLikeCpp::ExistingSpellAlreadyMatches { spell_id },
                    );
                    return Ok(false);
                }

                if row.state != PlayerSpellPersistenceStateLikeCpp::Removed
                    && !row.dependent
                    && dependent
                {
                    row.dependent = true;
                    if row.state != PlayerSpellPersistenceStateLikeCpp::New {
                        row.state = PlayerSpellPersistenceStateLikeCpp::Changed;
                    }
                    dependent_set = true;
                }
                if let Some(trait_definition_id) = row.trait_definition_id {
                    let trait_definition_id = u32::try_from(trait_definition_id).map_err(|_| {
                        SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                            field: "trait_definition_id",
                            value: i128::from(trait_definition_id),
                        }
                    })?;
                    let definition = self
                        .metadata
                        .trait_definitions
                        .get(trait_definition_id)
                        .ok_or(
                            SpellAcquisitionIndeterminateLikeCpp::MissingTraitDefinition {
                                trait_definition_id,
                            },
                        )?;
                    // C++ implicitly converts this signed DB2 field to the
                    // uint32 override-map key. Preserve valid values, but do
                    // not turn malformed negative metadata into a wrapped
                    // spell ID in the pure plan.
                    let overridden_spell_id = u32::try_from(definition.overrides_spell_id)
                        .map_err(|_| {
                            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                                record_id: definition.id,
                                field: "TraitDefinition.OverridesSpellID",
                                raw: i64::from(definition.overrides_spell_id),
                            }
                        })?;
                    if overridden_spell_id != 0 {
                        self.remove_override_like_cpp(overridden_spell_id, spell_id);
                    }
                    row.trait_definition_id = None;
                }
                row.favorite = favorite;

                if row.active != active
                    && row.state != PlayerSpellPersistenceStateLikeCpp::Removed
                    && !row.disabled
                {
                    row.active = active;
                    if !self.lifecycle.is_in_world() && !learning && !dependent_set {
                        row.state = PlayerSpellPersistenceStateLikeCpp::Unchanged;
                    } else if row.state != PlayerSpellPersistenceStateLikeCpp::New {
                        row.state = PlayerSpellPersistenceStateLikeCpp::Changed;
                    }
                    self.replace_spell_row_like_cpp(row, provenance);
                    if active {
                        self.diagnostics.push(
                            SpellAcquisitionDiagnosticLikeCpp::ExistingInactiveSpellActivated {
                                spell_id,
                            },
                        );
                        let passive = projection
                            .misc
                            .as_ref()
                            .map(|misc| {
                                misc.is_passive_checked().map_err(|error| {
                                    SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                                        record_id: misc.record_id,
                                        field: error.field,
                                        raw: error.raw,
                                    }
                                })
                            })
                            .transpose()?
                            .unwrap_or(false);
                        if passive {
                            if projection
                                .effects
                                .iter()
                                .any(effect_can_change_acquisition_like_cpp)
                            {
                                self.simulate_or_defer_cast_like_cpp(
                                    spell_id,
                                    &projection.effects,
                                    PlannedAcquisitionCastReasonLikeCpp::PassiveLearn,
                                    false,
                                )?;
                            } else if self.require_cast_authority_or_defer_like_cpp(
                                spell_id,
                                PlannedAcquisitionCastReasonLikeCpp::PassiveLearn,
                            )? {
                                // No acquisition handler was projected, so
                                // the ordinary passive cast remains the only
                                // owner of its runtime work.
                                self.post_commit_actions.push(
                                    SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive {
                                        spell_id,
                                    },
                                );
                            }
                        }
                    } else if let Some(next_spell_id) = next_active_spell_id {
                        if self.lifecycle.is_in_world() {
                            self.post_commit_actions.push(
                                SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                                    old_spell_id: spell_id,
                                    new_spell_id: next_spell_id,
                                },
                            );
                        }
                    } else if self.lifecycle.is_in_world() {
                        self.post_commit_actions.push(
                            SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id },
                        );
                    }
                    return Ok(active);
                }

                if row.disabled != disabled
                    && row.state != PlayerSpellPersistenceStateLikeCpp::Removed
                {
                    if row.state != PlayerSpellPersistenceStateLikeCpp::New {
                        row.state = PlayerSpellPersistenceStateLikeCpp::Changed;
                    }
                    row.disabled = disabled;
                    self.replace_spell_row_like_cpp(row, provenance.clone());
                    if disabled {
                        return Ok(false);
                    }
                    disabled_case = true;
                } else {
                    match row.state {
                        PlayerSpellPersistenceStateLikeCpp::Unchanged => {
                            self.replace_spell_row_like_cpp(row, provenance);
                            return Ok(false);
                        }
                        PlayerSpellPersistenceStateLikeCpp::Removed => {
                            self.remove_spell_row_like_cpp(spell_id, provenance.clone());
                            state = PlayerSpellPersistenceStateLikeCpp::Changed;
                        }
                        PlayerSpellPersistenceStateLikeCpp::Changed
                        | PlayerSpellPersistenceStateLikeCpp::New
                        | PlayerSpellPersistenceStateLikeCpp::Temporary => {
                            if !self.lifecycle.is_in_world() && !learning && !dependent_set {
                                row.state = PlayerSpellPersistenceStateLikeCpp::Unchanged;
                            }
                            self.replace_spell_row_like_cpp(row, provenance);
                            return Ok(false);
                        }
                    }
                }
            }
        }

        if !disabled_case {
            if let Some(previous_spell_id) = projection.chain.and_then(|chain| chain.prev_spell_id)
            {
                let previous_provenance = SpellAcquisitionProvenanceLikeCpp::PreviousRank {
                    requested_spell_id: spell_id,
                };
                if !self.lifecycle.is_in_world() || disabled {
                    self.add_spell_like_cpp(
                        previous_spell_id,
                        active,
                        true,
                        true,
                        disabled,
                        false,
                        from_skill,
                        false,
                        previous_provenance,
                    )?;
                } else {
                    self.learn_spell_like_cpp(
                        previous_spell_id,
                        true,
                        from_skill,
                        previous_provenance,
                    )?;
                }
            }

            let inserted = !self.spells.contains_key(&spell_id);
            let new_row = PlayerSpellAcquisitionRowLikeCpp {
                spell_id,
                active,
                dependent,
                disabled,
                favorite,
                trait_definition_id: None,
                state: if inserted {
                    state
                } else {
                    PlayerSpellPersistenceStateLikeCpp::Changed
                },
            };
            self.replace_spell_row_like_cpp(new_row, provenance.clone());

            if new_row.active && !new_row.disabled && projection.chain.is_some() {
                let current_chain = projection.chain.expect("checked above");
                // C++ visits every active row in `m_spells`, including
                // disabled rows, and keeps iterating after the new row is
                // made inactive. The planner's BTreeMap supplies the stable
                // order required by this pure projection.
                let candidate_ids = self.spells.keys().copied().collect::<Vec<_>>();
                for candidate_spell_id in candidate_ids {
                    if candidate_spell_id == spell_id {
                        continue;
                    }
                    let Some(candidate) = self.spells.get(&candidate_spell_id).copied() else {
                        continue;
                    };
                    if candidate.state == PlayerSpellPersistenceStateLikeCpp::Removed
                        || !candidate.active
                    {
                        continue;
                    }
                    let candidate_chain = match self
                        .metadata
                        .spell_chains
                        .spell_chain_lookup_like_cpp(candidate_spell_id)
                    {
                        SpellChainLookupLikeCpp::Node(node) => *node,
                        SpellChainLookupLikeCpp::Unranked => continue,
                        SpellChainLookupLikeCpp::Indeterminate(diagnostics) => {
                            return Err(SpellAcquisitionIndeterminateLikeCpp::RankChain {
                                spell_id: candidate_spell_id,
                                diagnostics: diagnostics.to_vec(),
                            });
                        }
                    };
                    if candidate_chain.first_spell_id != current_chain.first_spell_id {
                        continue;
                    }
                    if current_chain.rank > candidate_chain.rank {
                        let mut changed = candidate;
                        changed.active = false;
                        if changed.state != PlayerSpellPersistenceStateLikeCpp::New {
                            changed.state = PlayerSpellPersistenceStateLikeCpp::Changed;
                        }
                        self.replace_spell_row_like_cpp(changed, provenance.clone());
                        if self.lifecycle.is_in_world() {
                            self.post_commit_actions.push(
                                SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                                    old_spell_id: candidate_spell_id,
                                    new_spell_id: spell_id,
                                },
                            );
                        }
                        superceded_old = true;
                    } else {
                        let mut current = self
                            .spells
                            .get(&spell_id)
                            .copied()
                            .expect("new spell inserted");
                        current.active = false;
                        if current.state != PlayerSpellPersistenceStateLikeCpp::New {
                            current.state = PlayerSpellPersistenceStateLikeCpp::Changed;
                        }
                        self.replace_spell_row_like_cpp(current, provenance.clone());
                        if self.lifecycle.is_in_world() {
                            self.post_commit_actions.push(
                                SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                                    old_spell_id: spell_id,
                                    new_spell_id: candidate_spell_id,
                                },
                            );
                        }
                    }
                }
            }

            if disabled {
                return Ok(false);
            }
        }

        let cast_result = self.project_add_spell_autocast_like_cpp(spell_id, &projection)?;
        if cast_result == AddSpellAutocastResultLikeCpp::SkillStepReturned {
            return Ok(false);
        }

        if let Some(learn_skill) = projection.learn_skill {
            if u32::from(learn_skill.skill) != from_skill {
                self.apply_direct_learn_skill_like_cpp(
                    spell_id,
                    learn_skill,
                    SpellAcquisitionProvenanceLikeCpp::DirectLearnSkill {
                        source_spell_id: spell_id,
                    },
                )?;
            }
        } else {
            self.apply_skill_line_fallback_like_cpp(spell_id, from_skill)?;
        }

        for dependency in projection.dependencies {
            if !dependency.auto_learned {
                let dependency_provenance = SpellAcquisitionProvenanceLikeCpp::LearnDependency {
                    source_spell_id: spell_id,
                };
                if !self.lifecycle.is_in_world() || !dependency.active {
                    self.add_spell_like_cpp(
                        dependency.spell,
                        dependency.active,
                        true,
                        true,
                        false,
                        false,
                        0,
                        false,
                        dependency_provenance,
                    )?;
                } else {
                    self.learn_spell_like_cpp(dependency.spell, true, 0, dependency_provenance)?;
                }
            }
            if dependency.overrides_spell != 0 && dependency.active {
                self.add_override_like_cpp(dependency.overrides_spell, dependency.spell);
            }
        }

        self.record_spell_criteria_like_cpp(spell_id)?;
        let mounts = self
            .metadata
            .mounts
            .ok_or(SpellAcquisitionIndeterminateLikeCpp::IncompleteMountAuthority { spell_id })?;
        if mounts.get_by_source_spell_id_like_cpp(spell_id).is_some() {
            return Err(
                SpellAcquisitionIndeterminateLikeCpp::UnsupportedMountAcquisition { spell_id },
            );
        }

        let final_active = self.spells.get(&spell_id).is_some_and(|spell| spell.active);
        Ok(final_active && !disabled && !superceded_old)
    }

    fn record_spell_criteria_like_cpp(
        &mut self,
        spell_id: u32,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        // C++ gates the complete criteria block on PlayerLoading, including
        // the final LearnOrKnowSpell update.
        if self.lifecycle.is_loading() {
            return Ok(());
        }

        let spell_id_i32 = i32::try_from(spell_id).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "spell_id_i32",
                value: i128::from(spell_id),
            }
        })?;
        match self
            .metadata
            .skills
            .skill_line_ability_coverage_by_spell_like_cpp(spell_id_i32)
        {
            SkillLineAbilityCoverageLikeCpp::CoveredZero => {}
            SkillLineAbilityCoverageLikeCpp::Rows(rows) => {
                // C++ intentionally emits both updates for every multimap
                // row. Preserve repeated SkillLine IDs and effective row
                // order; neither filtering nor deduplication is valid here.
                for row in rows {
                    let skill_id = u32::from(row.skill_line);
                    self.publication_requirements.push(
                        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                            source_spell_id: spell_id,
                            skill_id,
                        },
                    );
                    self.post_commit_actions.push(
                        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                            source_spell_id: spell_id,
                            skill_id,
                        },
                    );
                    self.publication_requirements.push(
                        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                            source_spell_id: spell_id,
                            skill_id,
                        },
                    );
                    self.post_commit_actions.push(
                        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                            source_spell_id: spell_id,
                            skill_id,
                        },
                    );
                }
            }
            SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                    spell_id: Some(spell_id),
                    skill_id: None,
                    diagnostics: diagnostics.to_vec(),
                });
            }
        }
        self.publication_requirements.push(
            SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id,
            },
        );
        self.post_commit_actions.push(
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        );
        Ok(())
    }

    fn next_spell_id_like_cpp(
        &self,
        spell_id: u32,
    ) -> Result<Option<u32>, SpellAcquisitionIndeterminateLikeCpp> {
        match self
            .metadata
            .spell_chains
            .spell_chain_lookup_like_cpp(spell_id)
        {
            SpellChainLookupLikeCpp::Unranked => Ok(None),
            SpellChainLookupLikeCpp::Node(node) => Ok(node.next_spell_id),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics) => {
                Err(SpellAcquisitionIndeterminateLikeCpp::RankChain {
                    spell_id,
                    diagnostics: diagnostics.to_vec(),
                })
            }
        }
    }

    fn add_override_like_cpp(&mut self, overridden_spell_id: u32, overriding_spell_id: u32) {
        if self
            .overrides
            .insert((overridden_spell_id, overriding_spell_id))
        {
            let transition = PlannedOverrideTransitionLikeCpp {
                overridden_spell_id,
                overriding_spell_id,
                add: true,
            };
            self.override_transitions.push(transition);
            self.mutations
                .push(PlannedAcquisitionMutationLikeCpp::Override(transition));
        }
    }

    fn remove_override_like_cpp(&mut self, overridden_spell_id: u32, overriding_spell_id: u32) {
        if self
            .overrides
            .remove(&(overridden_spell_id, overriding_spell_id))
        {
            let transition = PlannedOverrideTransitionLikeCpp {
                overridden_spell_id,
                overriding_spell_id,
                add: false,
            };
            self.override_transitions.push(transition);
            self.mutations
                .push(PlannedAcquisitionMutationLikeCpp::Override(transition));
        }
    }
}
