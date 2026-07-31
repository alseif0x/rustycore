// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl SpellAcquisitionPlannerLikeCpp<'_> {
    pub(super) fn apply_direct_learn_skill_like_cpp(
        &mut self,
        spell_id: u32,
        learned_skill: SpellLearnSkillNodeLikeCpp,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let skill_id = u32::from(learned_skill.skill);
        if usize::from(learned_skill.step) > wow_data::MAX_SKILL_STEP_LIKE_CPP {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id,
                step: i64::from(learned_skill.step),
            });
        }
        self.skill_line_fields_like_cpp(skill_id)?;
        let mut skill_value = self
            .skills
            .get(&skill_id)
            .filter(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted)
            .map(|skill| skill.value)
            .unwrap_or(0)
            .max(learned_skill.value);
        let skill_maximum = self
            .skills
            .get(&skill_id)
            .filter(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted)
            .map(|skill| skill.maximum)
            .unwrap_or(0);
        let mut new_maximum = learned_skill.maxvalue;

        if new_maximum == 0 {
            let race_class = match self
                .metadata
                .skills
                .skill_race_class_info_coverage_for_player_like_cpp(
                    learned_skill.skill,
                    self.race,
                    self.class,
                ) {
                SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero => None,
                SkillRaceClassInfoMatchCoverageLikeCpp::Row(row) => Some(row.clone()),
                SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics) => {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                        spell_id: Some(spell_id),
                        skill_id: Some(skill_id),
                        diagnostics: diagnostics.to_vec(),
                    });
                }
            };
            if let Some(race_class) = race_class {
                match self.metadata.skills.skill_range_type_like_cpp(
                    &race_class,
                    self.metadata.skill_lines,
                    self.metadata.skill_tiers,
                ) {
                    SkillRangeTypeLikeCpp::Language => {
                        skill_value = 300;
                        new_maximum = 300;
                    }
                    SkillRangeTypeLikeCpp::Level => {
                        new_maximum = u16::from(self.level).checked_mul(5).ok_or(
                            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                                skill_id,
                                value: u32::from(self.level) * 5,
                            },
                        )?;
                    }
                    SkillRangeTypeLikeCpp::Mono => new_maximum = 1,
                    SkillRangeTypeLikeCpp::Rank => {
                        if learned_skill.step == 0 {
                            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                                skill_id,
                                step: 0,
                            });
                        }
                        let tier = u32::try_from(race_class.skill_tier_id)
                            .ok()
                            .and_then(|tier_id| {
                                self.metadata.skill_tiers.get_skill_tier_like_cpp(tier_id)
                            })
                            .ok_or(SpellAcquisitionIndeterminateLikeCpp::MissingSkillTier {
                                skill_id,
                                skill_tier_id: race_class.skill_tier_id,
                            })?;
                        let value = tier
                            .get_value_for_tier_index_like_cpp(u32::from(learned_skill.step - 1));
                        new_maximum = u16::try_from(value).map_err(|_| {
                            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                                skill_id,
                                value,
                            }
                        })?;
                    }
                    SkillRangeTypeLikeCpp::None => {
                        return Err(SpellAcquisitionIndeterminateLikeCpp::IncompleteSkillLine {
                            skill_id,
                        });
                    }
                }
                if race_class.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    skill_value = new_maximum;
                }
            }
        }

        self.set_skill_like_cpp(
            skill_id,
            learned_skill.step,
            skill_value,
            skill_maximum.max(new_maximum),
            provenance,
        )
    }

    pub(super) fn apply_skill_line_fallback_like_cpp(
        &mut self,
        spell_id: u32,
        from_skill: u32,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let rows = match self
            .metadata
            .skills
            .skill_line_ability_coverage_by_spell_like_cpp(i32::try_from(spell_id).map_err(
                |_| SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "spell_id_i32",
                    value: i128::from(spell_id),
                },
            )?) {
            SkillLineAbilityCoverageLikeCpp::CoveredZero => return Ok(()),
            SkillLineAbilityCoverageLikeCpp::Rows(rows) => rows.to_vec(),
            SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                    spell_id: Some(spell_id),
                    skill_id: None,
                    diagnostics: diagnostics.to_vec(),
                });
            }
        };

        for row in rows {
            let skill_id = u32::from(row.skill_line);
            self.skill_line_fields_like_cpp(skill_id)?;
            if skill_id == from_skill {
                continue;
            }
            let should_learn = (row.acquire_method
                == SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP
                && !self.has_skill_like_cpp(skill_id))
                || (row.skill_line == SKILL_RUNEFORGING_LIKE_CPP && row.trivial_rank_high == 0);
            if !should_learn {
                continue;
            }
            let race_class = match self
                .metadata
                .skills
                .skill_race_class_info_coverage_for_player_like_cpp(
                    row.skill_line,
                    self.race,
                    self.class,
                ) {
                SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero => {
                    self.diagnostics.push(
                        SpellAcquisitionDiagnosticLikeCpp::SkillRaceClassNotApplicable { skill_id },
                    );
                    continue;
                }
                SkillRaceClassInfoMatchCoverageLikeCpp::Row(info) => info.clone(),
                SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics) => {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                        spell_id: Some(spell_id),
                        skill_id: Some(skill_id),
                        diagnostics: diagnostics.to_vec(),
                    });
                }
            };
            self.learn_default_skill_like_cpp(
                &race_class,
                SpellAcquisitionProvenanceLikeCpp::SkillLineAbilityFallback {
                    source_spell_id: spell_id,
                    record_id: row.id,
                },
            )?;
        }
        Ok(())
    }

    fn learn_default_skill_like_cpp(
        &mut self,
        race_class: &wow_data::SkillRaceClassInfoRecord,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let skill_id = u32::from(race_class.skill_id);
        self.skill_line_fields_like_cpp(skill_id)?;
        let max_for_level = u16::from(self.level).checked_mul(5).ok_or(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                skill_id,
                value: u32::from(self.level) * 5,
            },
        )?;
        let (step, mut value, maximum) = match self.metadata.skills.skill_range_type_like_cpp(
            race_class,
            self.metadata.skill_lines,
            self.metadata.skill_tiers,
        ) {
            SkillRangeTypeLikeCpp::Language => (0, 300, 300),
            SkillRangeTypeLikeCpp::Level => {
                let value = if race_class.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    max_for_level
                } else if self.class == CLASS_DEATH_KNIGHT_LIKE_CPP {
                    u16::from(self.level.saturating_sub(1))
                        .saturating_mul(5)
                        .max(1)
                        .min(max_for_level)
                } else {
                    1
                };
                (0, value, max_for_level)
            }
            SkillRangeTypeLikeCpp::Mono => (0, 1, 1),
            SkillRangeTypeLikeCpp::Rank => {
                let tier = u32::try_from(race_class.skill_tier_id)
                    .ok()
                    .and_then(|tier_id| self.metadata.skill_tiers.get_skill_tier_like_cpp(tier_id))
                    .ok_or(SpellAcquisitionIndeterminateLikeCpp::MissingSkillTier {
                        skill_id,
                        skill_tier_id: race_class.skill_tier_id,
                    })?;
                let maximum = tier.get_value_for_tier_index_like_cpp(0);
                let maximum = u16::try_from(maximum).map_err(|_| {
                    SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                        skill_id,
                        value: maximum,
                    }
                })?;
                let value = if race_class.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    maximum
                } else if self.class == CLASS_DEATH_KNIGHT_LIKE_CPP {
                    u16::from(self.level.saturating_sub(1))
                        .saturating_mul(5)
                        .max(1)
                        .min(maximum)
                } else {
                    1
                };
                (1, value, maximum)
            }
            SkillRangeTypeLikeCpp::None => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::IncompleteSkillLine { skill_id });
            }
        };
        if race_class.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
            value = maximum;
        }
        self.set_skill_like_cpp(skill_id, step, value, maximum, provenance)
    }

    pub(super) fn skill_line_fields_like_cpp(
        &self,
        skill_id: u32,
    ) -> Result<SkillLineAcquisitionFieldsLikeCpp, SpellAcquisitionIndeterminateLikeCpp> {
        match self
            .metadata
            .skill_lines
            .acquisition_payload_like_cpp(skill_id)
        {
            SkillLineAcquisitionPayloadLikeCpp::Absent => {
                Err(SpellAcquisitionIndeterminateLikeCpp::MissingSkillLine { skill_id })
            }
            SkillLineAcquisitionPayloadLikeCpp::Incomplete => {
                Err(SpellAcquisitionIndeterminateLikeCpp::IncompleteSkillLine { skill_id })
            }
            SkillLineAcquisitionPayloadLikeCpp::Complete(fields) => Ok(fields),
        }
    }

    pub(super) fn set_skill_like_cpp(
        &mut self,
        skill_id: u32,
        step: u16,
        new_value: u16,
        maximum: u16,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        // Keep a final invariant at the shared mutation boundary as well as
        // the source-specific checks above. A later acquisition path must not
        // be able to manufacture a snapshot that `new` itself rejects.
        if usize::from(step) > wow_data::MAX_SKILL_STEP_LIKE_CPP {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id,
                step: i64::from(step),
            });
        }
        self.consume_work_like_cpp()?;
        self.set_skill_inner_like_cpp(skill_id, step, new_value, maximum, provenance)
    }

    fn set_skill_inner_like_cpp(
        &mut self,
        skill_id: u32,
        step: u16,
        new_value: u16,
        maximum: u16,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let _ = u16::try_from(skill_id).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillIdentifier {
                value: i64::from(skill_id),
                source: "SetSkill",
            }
        })?;
        let fields = self.skill_line_fields_like_cpp(skill_id)?;
        let before = self.skills.get(&skill_id).copied();

        if let Some(existing) = before {
            if new_value > 0 {
                self.activate_parent_skill_like_cpp(skill_id, fields)?;
                // C++ writes the skill fields before expanding rewards, but
                // deliberately keeps `SKILL_DELETED` until that expansion
                // finishes.  `HasSkill` therefore still observes a deleted
                // row as absent while recursively learning reward spells.
                let during_rewards = PlayerSkillAcquisitionRowLikeCpp {
                    skill_id,
                    step,
                    value: new_value,
                    maximum,
                    profession_association: existing.profession_association,
                    state: existing.state,
                };
                self.skills.insert(skill_id, during_rewards);
                self.record_skill_transition_like_cpp(
                    skill_id,
                    Some(existing),
                    during_rewards,
                    provenance.clone(),
                );
                self.learn_skill_rewards_like_cpp(skill_id, new_value)?;

                if skill_id == u32::from(SKILL_RIDING_LIKE_CPP) && new_value > existing.value {
                    self.post_commit_actions
                        .push(SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability);
                }
                self.record_skill_criteria_like_cpp(skill_id);

                let state = match existing.state {
                    PlayerSkillPersistenceStateLikeCpp::Unchanged if existing.value == 0 => {
                        PlayerSkillPersistenceStateLikeCpp::New
                    }
                    PlayerSkillPersistenceStateLikeCpp::Unchanged
                    | PlayerSkillPersistenceStateLikeCpp::Deleted => {
                        PlayerSkillPersistenceStateLikeCpp::Changed
                    }
                    PlayerSkillPersistenceStateLikeCpp::Changed => {
                        PlayerSkillPersistenceStateLikeCpp::Changed
                    }
                    PlayerSkillPersistenceStateLikeCpp::New => {
                        PlayerSkillPersistenceStateLikeCpp::New
                    }
                };
                let after = PlayerSkillAcquisitionRowLikeCpp {
                    state,
                    ..during_rewards
                };
                self.skills.insert(skill_id, after);
                self.record_skill_transition_like_cpp(
                    skill_id,
                    Some(during_rewards),
                    after,
                    provenance.clone(),
                );
                // In the existing-row branch C++ assigns the physical
                // ProfessionSkillLine slot only after reward recursion and
                // post-reward actions have completed.
                self.record_primary_profession_activation_like_cpp(fields, existing.value, after);
                return Ok(());
            }
            if existing.value > 0 {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::UnsupportedSkillDecrease {
                        skill_id,
                        old_value: existing.value,
                        new_value,
                    },
                );
            }
            return Ok(());
        }

        debug_assert!(!self.pending_skill_insertions.contains(&skill_id));
        self.pending_skill_insertions.push(skill_id);
        let result = (|| {
            if usize::from(self.occupied_skill_slots) >= MAX_PLAYER_SKILLS_LIKE_CPP {
                return Err(SpellAcquisitionIndeterminateLikeCpp::PlayerSkillCapacityExceeded);
            }
            let parent_is_being_created_by_this_root_child_expansion = matches!(
                &provenance,
                SpellAcquisitionProvenanceLikeCpp::RootChildSkill { parent_skill_id }
                    if *parent_skill_id == fields.parent_skill_line_id
                        && self.pending_skill_insertions.contains(parent_skill_id)
            );
            if fields.parent_skill_line_id != 0
                && !parent_is_being_created_by_this_root_child_expansion
            {
                self.activate_parent_skill_like_cpp(skill_id, fields)?;
            } else if fields.parent_skill_line_id == 0 {
                let children = self
                    .metadata
                    .skill_lines
                    .acquisition_children_for_parent_like_cpp(skill_id)
                    .collect::<Vec<_>>();
                for (child_skill_id, child_payload) in children {
                    match child_payload {
                        SkillLineAcquisitionPayloadLikeCpp::Complete(_) => {}
                        SkillLineAcquisitionPayloadLikeCpp::Incomplete => {
                            return Err(
                                SpellAcquisitionIndeterminateLikeCpp::IncompleteSkillLine {
                                    skill_id: child_skill_id,
                                },
                            );
                        }
                        SkillLineAcquisitionPayloadLikeCpp::Absent => {
                            return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSkillLine {
                                skill_id: child_skill_id,
                            });
                        }
                    }
                    // A child may be the outer insertion that activated this
                    // absent parent. Skip only pending insertions; structural
                    // parent-cycle detection remains independent.
                    if self.pending_skill_insertions.contains(&child_skill_id) {
                        continue;
                    }
                    if !self.has_skill_like_cpp(child_skill_id) {
                        self.set_skill_like_cpp(
                            child_skill_id,
                            0,
                            0,
                            0,
                            SpellAcquisitionProvenanceLikeCpp::RootChildSkill {
                                parent_skill_id: skill_id,
                            },
                        )?;
                    }
                }
            }
            if usize::from(self.occupied_skill_slots) >= MAX_PLAYER_SKILLS_LIKE_CPP {
                return Err(SpellAcquisitionIndeterminateLikeCpp::PlayerSkillCapacityExceeded);
            }

            let after = PlayerSkillAcquisitionRowLikeCpp {
                skill_id,
                step,
                value: new_value,
                maximum,
                profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
                state: PlayerSkillPersistenceStateLikeCpp::New,
            };
            self.skills.insert(skill_id, after);
            self.occupied_skill_slots = self.occupied_skill_slots.saturating_add(1);
            self.record_skill_transition_like_cpp(skill_id, None, after, provenance);
            self.record_primary_profession_activation_like_cpp(fields, 0, after);
            if new_value > 0 {
                self.learn_skill_rewards_like_cpp(skill_id, new_value)?;
                self.record_skill_criteria_like_cpp(skill_id);
            }
            Ok(())
        })();
        let popped = self.pending_skill_insertions.pop();
        debug_assert_eq!(popped, Some(skill_id));
        result
    }

    fn activate_parent_skill_like_cpp(
        &mut self,
        child_skill_id: u32,
        child_fields: SkillLineAcquisitionFieldsLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        if child_fields.parent_skill_line_id == 0 || child_fields.parent_tier_index <= 0 {
            return Ok(());
        }
        let parent_skill_id = child_fields.parent_skill_line_id;
        let required_step = u16::try_from(child_fields.parent_tier_index).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id: parent_skill_id,
                step: i64::from(child_fields.parent_tier_index),
            }
        })?;
        if usize::from(required_step) > wow_data::MAX_SKILL_STEP_LIKE_CPP {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id: parent_skill_id,
                step: i64::from(required_step),
            });
        }
        if self.skills.get(&parent_skill_id).is_some_and(|parent| {
            parent.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                && parent.value > 0
                && parent.step >= required_step
        }) {
            return Ok(());
        }
        let parent_skill_u16 = u16::try_from(parent_skill_id).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillIdentifier {
                value: i64::from(parent_skill_id),
                source: "SkillLine.ParentSkillLineID",
            }
        })?;
        let race_class = match self
            .metadata
            .skills
            .skill_race_class_info_coverage_for_player_like_cpp(
                parent_skill_u16,
                self.race,
                self.class,
            ) {
            SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero => {
                self.diagnostics.push(
                    SpellAcquisitionDiagnosticLikeCpp::SkillRaceClassNotApplicable {
                        skill_id: parent_skill_id,
                    },
                );
                return Ok(());
            }
            SkillRaceClassInfoMatchCoverageLikeCpp::Row(row) => row.clone(),
            SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                    spell_id: None,
                    skill_id: Some(parent_skill_id),
                    diagnostics: diagnostics.to_vec(),
                });
            }
        };
        self.skill_line_fields_like_cpp(parent_skill_id)?;
        let tier = u32::try_from(race_class.skill_tier_id)
            .ok()
            .and_then(|tier_id| self.metadata.skill_tiers.get_skill_tier_like_cpp(tier_id))
            .ok_or(SpellAcquisitionIndeterminateLikeCpp::MissingSkillTier {
                skill_id: parent_skill_id,
                skill_tier_id: race_class.skill_tier_id,
            })?;
        let maximum =
            tier.get_value_for_tier_index_like_cpp(u32::from(required_step.saturating_sub(1)));
        let maximum = u16::try_from(maximum).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                skill_id: parent_skill_id,
                value: maximum,
            }
        })?;
        let value = self
            .skills
            .get(&parent_skill_id)
            .filter(|parent| parent.state != PlayerSkillPersistenceStateLikeCpp::Deleted)
            .map(|parent| parent.value)
            .unwrap_or(0)
            .max(1);

        let owns_child = self.parent_skill_path.last().copied() != Some(child_skill_id);
        if owns_child {
            self.parent_skill_path.push(child_skill_id);
        }
        if let Some(cycle_start) = self
            .parent_skill_path
            .iter()
            .position(|visiting| *visiting == parent_skill_id)
        {
            let mut skill_ids = self.parent_skill_path[cycle_start..].to_vec();
            skill_ids.push(parent_skill_id);
            if owns_child {
                let popped = self.parent_skill_path.pop();
                debug_assert_eq!(popped, Some(child_skill_id));
            }
            return Err(SpellAcquisitionIndeterminateLikeCpp::SkillParentCycle { skill_ids });
        }
        self.parent_skill_path.push(parent_skill_id);
        let result = self.set_skill_like_cpp(
            parent_skill_id,
            required_step,
            value,
            maximum,
            SpellAcquisitionProvenanceLikeCpp::ParentSkill { child_skill_id },
        );
        let popped_parent = self.parent_skill_path.pop();
        debug_assert_eq!(popped_parent, Some(parent_skill_id));
        if owns_child {
            let popped_child = self.parent_skill_path.pop();
            debug_assert_eq!(popped_child, Some(child_skill_id));
        }
        result
    }

    fn record_primary_profession_activation_like_cpp(
        &mut self,
        fields: SkillLineAcquisitionFieldsLikeCpp,
        old_value: u16,
        after: PlayerSkillAcquisitionRowLikeCpp,
    ) {
        // The absent-row branch of C++ `Player::SetSkill` assigns an empty
        // root profession slot before writing the new skill value, including
        // a zero value. Existing rows reach this helper only while activating
        // a nonzero value; tier children never qualify as root professions.
        if old_value == 0
            && fields.category_id == SKILL_CATEGORY_PROFESSION_LIKE_CPP
            && fields.parent_skill_line_id == 0
            && !self
                .root_primary_profession_skill_ids
                .contains(&after.skill_id)
        {
            self.root_primary_profession_skill_ids.push(after.skill_id);
        }
    }

    fn record_skill_criteria_like_cpp(&mut self, skill_id: u32) {
        self.post_commit_actions
            .push(SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id });
        self.post_commit_actions.push(
            SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria { skill_id },
        );
    }

    fn consume_future_player_condition_resolution_like_cpp(
        &mut self,
        spell_id: u32,
        condition_id: u32,
    ) -> Result<bool, SpellAcquisitionIndeterminateLikeCpp> {
        let occurrence_index = self.future_player_condition_resolution_cursor;
        let resolution = self
            .future_player_condition_resolutions
            .get(occurrence_index)
            .copied()
            .ok_or(
                SpellAcquisitionIndeterminateLikeCpp::MissingFuturePlayerConditionResolution {
                    spell_id,
                    condition_id,
                    occurrence_index,
                },
            )?;
        if resolution.condition_id != condition_id {
            return Err(
                SpellAcquisitionIndeterminateLikeCpp::FuturePlayerConditionResolutionMismatch {
                    spell_id,
                    occurrence_index,
                    expected_condition_id: condition_id,
                    actual_condition_id: resolution.condition_id,
                },
            );
        }
        self.future_player_condition_resolution_cursor = occurrence_index + 1;
        Ok(resolution.allowed)
    }

    fn learn_skill_rewards_like_cpp(
        &mut self,
        skill_id: u32,
        skill_value: u16,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let skill_id_u16 = u16::try_from(skill_id).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillIdentifier {
                value: i64::from(skill_id),
                source: "LearnSkillRewardedSpells",
            }
        })?;
        let abilities = match self
            .metadata
            .skills
            .skill_line_ability_coverage_by_skill_like_cpp(skill_id_u16)
        {
            SkillLineAbilityCoverageLikeCpp::CoveredZero => return Ok(()),
            SkillLineAbilityCoverageLikeCpp::Rows(rows) => rows.to_vec(),
            SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                    spell_id: None,
                    skill_id: Some(skill_id),
                    diagnostics: diagnostics.to_vec(),
                });
            }
        };

        for ability in abilities {
            let relevant_method = matches!(
                ability.acquire_method,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP
                    | SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP
                    | SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP
            );
            if !relevant_method {
                self.diagnostics
                    .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                        skill_id,
                        record_id: ability.id,
                        gate: "acquire method",
                    });
                continue;
            }
            let spell_id = u32::try_from(ability.spell).map_err(|_| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "SkillLineAbility.Spell",
                    value: i128::from(ability.spell),
                }
            })?;
            if spell_id == 0 {
                return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                    field: "SkillLineAbility.Spell",
                    value: 0,
                });
            }
            if !self
                .metadata
                .catalog
                .contains_spell_difficulty_key_like_cpp(spell_id, DIFFICULTY_NONE_LIKE_CPP)
            {
                self.diagnostics
                    .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                        skill_id,
                        record_id: ability.id,
                        gate: "missing SpellInfo",
                    });
                continue;
            }

            if ability.acquire_method == SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP {
                if (ability.flags as u8)
                    & (SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP as u8)
                    == 0
                {
                    self.diagnostics
                        .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                            skill_id,
                            record_id: ability.id,
                            gate: "quest fallback flag",
                        });
                    continue;
                }
                let condition_id = match self
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
                    SpellAcquisitionMetadataLookupLikeCpp::Present(misc) => {
                        misc.future_player_condition_id_checked().map_err(|error| {
                            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                                record_id: misc.record_id,
                                field: error.field,
                                raw: error.raw,
                            }
                        })?
                    }
                };
                let Some(condition_id) = condition_id else {
                    self.diagnostics
                        .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                            skill_id,
                            record_id: ability.id,
                            gate: "missing future player condition",
                        });
                    continue;
                };
                let allowed = self
                    .consume_future_player_condition_resolution_like_cpp(spell_id, condition_id)?;
                if !allowed {
                    self.diagnostics
                        .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                            skill_id,
                            record_id: ability.id,
                            gate: "future player condition",
                        });
                    continue;
                }
            }

            if skill_id_u16 == SKILL_RIDING_LIKE_CPP
                && (ability.acquire_method != SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP
                    || ability.num_skill_ups != 1)
            {
                self.diagnostics
                    .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                        skill_id,
                        record_id: ability.id,
                        gate: "riding auto-learn",
                    });
                continue;
            }
            if !race_mask_matches_like_cpp(ability.race_mask, self.race)
                || !class_mask_matches_like_cpp(ability.class_mask, self.class)
            {
                self.diagnostics
                    .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                        skill_id,
                        record_id: ability.id,
                        gate: "race/class",
                    });
                continue;
            }

            let levels = match self
                .metadata
                .catalog
                .levels_for_spell_like_cpp(spell_id, DIFFICULTY_NONE_LIKE_CPP)
            {
                SpellAcquisitionMetadataLookupLikeCpp::MissingCoverage => {
                    return Err(SpellAcquisitionIndeterminateLikeCpp::MissingSpellCoverage {
                        spell_id,
                        table: SpellAcquisitionTableLikeCpp::SpellLevels,
                    });
                }
                SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(reasons) => {
                    return Err(
                        SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                            spell_id,
                            table: SpellAcquisitionTableLikeCpp::SpellLevels,
                            reasons: reasons.to_vec(),
                        },
                    );
                }
                SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow => {
                    wow_data::SpellAcquisitionLevelsLikeCpp {
                        record_id: 0,
                        spell_id_raw: i64::from(spell_id),
                        difficulty_id_raw: i64::from(DIFFICULTY_NONE_LIKE_CPP),
                        base_level_raw: 0,
                        spell_level_raw: 0,
                    }
                }
                SpellAcquisitionMetadataLookupLikeCpp::Present(levels) => levels.clone(),
            };
            let base_level = levels.base_level_checked().map_err(|error| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: levels.record_id,
                    field: error.field,
                    raw: error.raw,
                }
            })?;
            let spell_level = levels.spell_level_checked().map_err(|error| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: levels.record_id,
                    field: error.field,
                    raw: error.raw,
                }
            })?;
            let required_level = base_level.max(spell_level);
            if required_level < 0 {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                        record_id: levels.record_id,
                        field: "SpellLevels.BaseLevel/SpellLevel",
                        raw: i64::from(required_level),
                    },
                );
            }
            if u16::try_from(required_level).unwrap_or(u16::MAX) > u16::from(self.level) {
                self.diagnostics
                    .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                        skill_id,
                        record_id: ability.id,
                        gate: "player level",
                    });
                continue;
            }

            if i32::from(skill_value) < i32::from(ability.min_skill_line_rank)
                && ability.acquire_method == SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP
            {
                if self.spells.get(&spell_id).is_some_and(|spell| {
                    spell.state != PlayerSpellPersistenceStateLikeCpp::Removed
                        && spell.state != PlayerSpellPersistenceStateLikeCpp::Temporary
                }) {
                    return Err(
                        SpellAcquisitionIndeterminateLikeCpp::RewardSpellRemovalRequired {
                            skill_id,
                            spell_id,
                            record_id: ability.id,
                        },
                    );
                }
                self.diagnostics
                    .push(SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                        skill_id,
                        record_id: ability.id,
                        gate: "minimum skill rank",
                    });
                continue;
            }

            let provenance = SpellAcquisitionProvenanceLikeCpp::SkillReward {
                skill_id,
                record_id: ability.id,
            };
            let from_skill = u32::from(ability.skill_line);
            if self.lifecycle.is_in_world() {
                self.learn_spell_like_cpp(spell_id, true, from_skill, provenance)?;
            } else {
                self.add_spell_like_cpp(
                    spell_id, true, true, true, false, false, from_skill, false, provenance,
                )?;
            }
        }
        Ok(())
    }
}

fn race_mask_matches_like_cpp(mask: i64, race: u8) -> bool {
    mask == 0 || (mask & (1_i64 << (race - 1))) != 0
}

fn class_mask_matches_like_cpp(mask: i32, class: u8) -> bool {
    mask == 0 || (mask & (1_i32 << (class - 1))) != 0
}
