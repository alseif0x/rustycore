// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AddSpellAutocastResultLikeCpp {
    None,
    Cast,
    SkillStepReturned,
}

impl SpellAcquisitionPlannerLikeCpp<'_> {
    pub(super) fn project_add_spell_autocast_like_cpp(
        &mut self,
        spell_id: u32,
        projection: &EffectiveSpellProjectionLikeCpp,
    ) -> Result<AddSpellAutocastResultLikeCpp, SpellAcquisitionIndeterminateLikeCpp> {
        let has_learn_spell = projection
            .effects
            .iter()
            .any(|effect| effect.effect_type_checked() == Ok(SPELL_EFFECT_LEARN_SPELL));
        let has_skill_step = projection
            .effects
            .iter()
            .any(|effect| effect.effect_type_checked() == Ok(SPELL_EFFECT_SKILL_STEP));
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
        let cast_when_learned = projection
            .misc
            .as_ref()
            .map(|misc| {
                misc.cast_when_learned_checked().map_err(|error| {
                    SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                        record_id: misc.record_id,
                        field: error.field,
                        raw: error.raw,
                    }
                })
            })
            .transpose()?
            .unwrap_or(false);

        let mut refresh_passive_without_projected_acquisition = false;
        let reason = if projection.talent && has_learn_spell {
            Some(PlannedAcquisitionCastReasonLikeCpp::TalentLearnEffect)
        } else if passive {
            self.metadata
                .cast_authority
                .require_safe_like_cpp(spell_id)?;
            if projection
                .effects
                .iter()
                .any(effect_can_change_acquisition_like_cpp)
            {
                Some(PlannedAcquisitionCastReasonLikeCpp::PassiveLearn)
            } else {
                refresh_passive_without_projected_acquisition = true;
                None
            }
        } else if has_skill_step {
            Some(PlannedAcquisitionCastReasonLikeCpp::SkillStep)
        } else if cast_when_learned {
            Some(PlannedAcquisitionCastReasonLikeCpp::CastWhenLearned)
        } else {
            None
        };

        let Some(reason) = reason else {
            if refresh_passive_without_projected_acquisition {
                self.post_commit_actions
                    .push(SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id });
            }
            return Ok(AddSpellAutocastResultLikeCpp::None);
        };
        self.simulate_cast_like_cpp(spell_id, &projection.effects, reason, false)?;
        if has_skill_step {
            Ok(AddSpellAutocastResultLikeCpp::SkillStepReturned)
        } else {
            Ok(AddSpellAutocastResultLikeCpp::Cast)
        }
    }

    pub(super) fn simulate_cast_like_cpp(
        &mut self,
        spell_id: u32,
        effects: &[SpellAcquisitionEffectLikeCpp],
        reason: PlannedAcquisitionCastReasonLikeCpp,
        trainer_wrapper: bool,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        self.consume_work_like_cpp()?;
        self.metadata
            .cast_authority
            .require_safe_like_cpp(spell_id)?;
        // The acquisition effects of this cast are consumed by the immutable
        // plan below.  Emitting a generic post-commit CastSpell intent would
        // execute LearnSpell/SetSkill a second time when #158 applies it.
        self.diagnostics
            .push(SpellAcquisitionDiagnosticLikeCpp::AcquisitionCastProjected { spell_id, reason });

        // Fail the private projection before exposing a plan if any cast
        // branch can acquire through a runtime/script owner that is not part
        // of this immutable graph.
        for effect in effects {
            let effect_type = effect.effect_type_checked().map_err(|error| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                }
            })?;
            let effect_index = effect.effect_index_checked().map_err(|error| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                }
            })?;
            if effect_type == SPELL_EFFECT_SUMMON_LIKE_CPP {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::BattlePetOrSummonPath {
                        spell_id,
                        effect_index,
                    },
                );
            }
            if effect_type == SPELL_EFFECT_LEARN_PET_SPELL_LIKE_CPP {
                return Err(SpellAcquisitionIndeterminateLikeCpp::PetLearnPath {
                    spell_id,
                    effect_index,
                });
            }
            if runtime_dispatched_acquisition_effect_like_cpp(effect_type)
                || (effect_type != SPELL_EFFECT_LEARN_SPELL && effect.effect_trigger_spell_raw != 0)
            {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::UnsupportedRuntimeEffect {
                        spell_id,
                        effect_index,
                        effect_type,
                    },
                );
            }
            if effect_type != 0
                && !matches!(
                    effect_type,
                    SPELL_EFFECT_DUMMY_LIKE_CPP
                        | SPELL_EFFECT_LEARN_SPELL
                        | SPELL_EFFECT_SKILL_STEP
                        | SPELL_EFFECT_SKILL
                        | SPELL_EFFECT_DUAL_WIELD
                )
                && !wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(effect_type)
            {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::UnsupportedRuntimeEffect {
                        spell_id,
                        effect_index,
                        effect_type,
                    },
                );
            }
        }

        let needs_player_cast_resolution = effects.iter().any(|effect| {
            effect.effect_type_checked().is_ok_and(|effect_type| {
                matches!(
                    effect_type,
                    SPELL_EFFECT_LEARN_SPELL
                        | SPELL_EFFECT_SKILL_STEP
                        | SPELL_EFFECT_SKILL
                        | SPELL_EFFECT_DUAL_WIELD
                )
            })
        });
        let cast_resolution = if needs_player_cast_resolution {
            let resolution =
                self.cast_resolutions.get(&spell_id).copied().ok_or(
                    SpellAcquisitionIndeterminateLikeCpp::MissingCastResolution { spell_id },
                )?;
            let mut known_effect_mask = 0_u32;
            for effect in effects {
                let effect_index = effect.effect_index_checked().map_err(|error| {
                    SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    }
                })?;
                let Some(effect_bit) = 1_u32.checked_shl(u32::from(effect_index)) else {
                    return Err(
                        SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                            record_id: effect.record_id,
                            field: "SpellEffect.EffectIndex",
                            raw: i64::from(effect_index),
                        },
                    );
                };
                known_effect_mask |= effect_bit;
            }
            let unknown_effect_mask =
                resolution.executed_hit_target_effect_mask & !known_effect_mask;
            if unknown_effect_mask != 0 {
                return Err(
                    SpellAcquisitionIndeterminateLikeCpp::InvalidCastResolution {
                        spell_id,
                        effect_index: Some(unknown_effect_mask.trailing_zeros() as u8),
                    },
                );
            }
            if !resolution.reached_immediate_phase {
                self.diagnostics.push(
                    SpellAcquisitionDiagnosticLikeCpp::CastStoppedBeforeImmediatePhase { spell_id },
                );
                return Ok(());
            }
            Some(resolution)
        } else {
            None
        };

        // C++ `_handle_immediate_phase`: every HANDLE_HIT effect in
        // EffectIndex order. `SPELL_EFFECT_SKILL` reads the caster and is the
        // only graph-changing supported handler in this phase.
        for effect in effects {
            if effect.effect_type_checked().ok() != Some(SPELL_EFFECT_SKILL) {
                continue;
            }
            let provenance = effect_provenance_like_cpp(spell_id, effect, trainer_wrapper)?;
            self.apply_cast_skill_effect_like_cpp(spell_id, effect, provenance)?;
        }

        // C++ `DoProcessTargetContainer`: then every HANDLE_HIT_TARGET effect
        // in EffectIndex order. Each LearnSpell call completes recursively
        // before the next effect is visited.
        for effect in effects {
            let effect_type = effect.effect_type_checked().map_err(|error| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                }
            })?;
            if !matches!(
                effect_type,
                SPELL_EFFECT_LEARN_SPELL | SPELL_EFFECT_SKILL_STEP | SPELL_EFFECT_DUAL_WIELD
            ) {
                continue;
            }
            let effect_index = effect.effect_index_checked().map_err(|error| {
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                }
            })?;
            self.require_player_effect_target_like_cpp(spell_id, effect_index, effect)?;
            let effect_bit = 1_u32.checked_shl(u32::from(effect_index)).ok_or(
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: "SpellEffect.EffectIndex",
                    raw: i64::from(effect_index),
                },
            )?;
            let resolution = cast_resolution
                .expect("LEARN_SPELL, SKILL_STEP and DUAL_WIELD require a player cast resolution");
            if resolution.executed_hit_target_effect_mask & effect_bit == 0 {
                self.diagnostics.push(
                    SpellAcquisitionDiagnosticLikeCpp::HitTargetEffectSuppressed {
                        spell_id,
                        effect_index,
                    },
                );
                continue;
            }
            let provenance = effect_provenance_like_cpp(spell_id, effect, trainer_wrapper)?;
            match effect_type {
                SPELL_EFFECT_LEARN_SPELL => {
                    if effect.effect_trigger_spell_raw == 0 {
                        return Err(SpellAcquisitionIndeterminateLikeCpp::CastItemLearnPath {
                            spell_id,
                            effect_index,
                        });
                    }
                    let learned_spell_id = effect.trigger_spell_id_checked().map_err(|error| {
                        SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                            record_id: effect.record_id,
                            field: error.field,
                            raw: error.raw,
                        }
                    })?;
                    self.learn_spell_like_cpp(learned_spell_id, false, 0, provenance)?;
                }
                SPELL_EFFECT_SKILL_STEP => {
                    self.apply_cast_skill_effect_like_cpp(spell_id, effect, provenance)?;
                }
                SPELL_EFFECT_DUAL_WIELD => {
                    self.post_commit_actions.push(
                        SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                            source_spell_id: spell_id,
                        },
                    );
                }
                _ => unreachable!("filtered effect type"),
            }
        }
        Ok(())
    }

    fn require_player_effect_target_like_cpp(
        &self,
        spell_id: u32,
        effect_index: u8,
        effect: &SpellAcquisitionEffectLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        if effect
            .implicit_target_raw
            .contains(&TARGET_UNIT_PET_LIKE_CPP)
        {
            return Err(SpellAcquisitionIndeterminateLikeCpp::PetLearnPath {
                spell_id,
                effect_index,
            });
        }
        if effect.targets_player_like_cpp() {
            return Ok(());
        }
        Err(
            SpellAcquisitionIndeterminateLikeCpp::UnsupportedEffectTarget {
                spell_id,
                effect_index,
                targets: effect.implicit_target_raw,
            },
        )
    }

    fn apply_cast_skill_effect_like_cpp(
        &mut self,
        spell_id: u32,
        effect: &SpellAcquisitionEffectLikeCpp,
        provenance: SpellAcquisitionProvenanceLikeCpp,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        let effect_index = effect.effect_index_checked().map_err(|error| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                record_id: effect.record_id,
                field: error.field,
                raw: error.raw,
            }
        })?;
        let effect_type = effect.effect_type_checked().map_err(|error| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                record_id: effect.record_id,
                field: error.field,
                raw: error.raw,
            }
        })?;
        let skill_id = effect.misc_value_id_checked(0).map_err(|error| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                record_id: effect.record_id,
                field: error.field,
                raw: error.raw,
            }
        })?;
        let _ = u16::try_from(skill_id).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillIdentifier {
                value: i64::from(skill_id),
                source: "SpellEffect.EffectMiscValue",
            }
        })?;
        let dynamic_calc_fields = [
            ("SpellEffect.Coefficient", effect.effect_coefficient_bits),
            ("SpellEffect.Variance", effect.effect_variance_bits),
            (
                "SpellEffect.EffectPointsPerResource",
                effect.effect_points_per_resource_bits,
            ),
            (
                "SpellEffect.EffectRealPointsPerLevel",
                effect.effect_real_points_per_level_bits,
            ),
        ];
        for (field, bits) in dynamic_calc_fields {
            let value = f32::from_bits(bits);
            if !value.is_finite() || value != 0.0 {
                return Err(SpellAcquisitionIndeterminateLikeCpp::RuntimeCalcValue {
                    spell_id,
                    effect_index,
                    field,
                });
            }
        }
        if effect.effect_chain_targets_raw != 0 {
            return Err(SpellAcquisitionIndeterminateLikeCpp::RuntimeCalcValue {
                spell_id,
                effect_index,
                field: "SpellEffect.EffectChainTargets",
            });
        }
        let domain = effect
            .base_points_die_sides_domain_checked()
            .map_err(
                |error| SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                },
            )?;
        let Some(step_value) = domain.deterministic_value() else {
            return Err(
                SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                    record_id: effect.record_id,
                    field: "SpellEffect.CalcValue",
                    raw: effect.effect_base_points_raw,
                },
            );
        };
        if step_value < 1 {
            self.diagnostics.push(
                SpellAcquisitionDiagnosticLikeCpp::EffectHadNoRuntimeChange {
                    spell_id,
                    effect_index,
                    reason: "CalcValue < 1",
                },
            );
            return Ok(());
        }
        let step = u16::try_from(step_value).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id,
                step: i64::from(step_value),
            }
        })?;
        if usize::from(step) > wow_data::MAX_SKILL_STEP_LIKE_CPP {
            return Err(SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id,
                step: i64::from(step),
            });
        }
        if effect_type == SPELL_EFFECT_SKILL
            && self.skills.get(&skill_id).is_some_and(|skill| {
                skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    && skill.value > 0
                    && skill.step >= step
            })
        {
            self.diagnostics.push(
                SpellAcquisitionDiagnosticLikeCpp::EffectHadNoRuntimeChange {
                    spell_id,
                    effect_index,
                    reason: "SKILL effect did not raise the current step",
                },
            );
            return Ok(());
        }

        let skill_id_u16 = skill_id as u16;
        let race_class = match self
            .metadata
            .skills
            .skill_race_class_info_coverage_for_player_like_cpp(skill_id_u16, self.race, self.class)
        {
            SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero => {
                self.diagnostics.push(
                    SpellAcquisitionDiagnosticLikeCpp::SkillRaceClassNotApplicable { skill_id },
                );
                return Ok(());
            }
            SkillRaceClassInfoMatchCoverageLikeCpp::Row(row) => row.clone(),
            SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics) => {
                return Err(SpellAcquisitionIndeterminateLikeCpp::SkillLineAbility {
                    spell_id: None,
                    skill_id: Some(skill_id),
                    diagnostics: diagnostics.to_vec(),
                });
            }
        };
        self.skill_line_fields_like_cpp(skill_id)?;
        let tier = u32::try_from(race_class.skill_tier_id)
            .ok()
            .and_then(|tier_id| self.metadata.skill_tiers.get_skill_tier_like_cpp(tier_id))
            .ok_or(SpellAcquisitionIndeterminateLikeCpp::MissingSkillTier {
                skill_id,
                skill_tier_id: race_class.skill_tier_id,
            })?;
        let maximum = tier.get_value_for_tier_index_like_cpp(u32::from(step - 1));
        let maximum = u16::try_from(maximum).map_err(|_| {
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                skill_id,
                value: maximum,
            }
        })?;
        let mut value = self
            .skills
            .get(&skill_id)
            .filter(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted)
            .map(|skill| skill.value)
            .unwrap_or(0)
            .max(1);
        if race_class.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
            value = maximum;
        }
        self.set_skill_like_cpp(skill_id, step, value, maximum, provenance)
    }
}

fn effect_provenance_like_cpp(
    spell_id: u32,
    effect: &SpellAcquisitionEffectLikeCpp,
    trainer_wrapper: bool,
) -> Result<SpellAcquisitionProvenanceLikeCpp, SpellAcquisitionIndeterminateLikeCpp> {
    let effect_index = effect.effect_index_checked().map_err(|error| {
        SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
            record_id: effect.record_id,
            field: error.field,
            raw: error.raw,
        }
    })?;
    Ok(if trainer_wrapper {
        SpellAcquisitionProvenanceLikeCpp::WrapperEffect {
            wrapper_spell_id: spell_id,
            effect_index,
            record_id: effect.record_id,
        }
    } else {
        SpellAcquisitionProvenanceLikeCpp::AutocastEffect {
            source_spell_id: spell_id,
            effect_index,
            record_id: effect.record_id,
        }
    })
}

pub(super) fn effect_can_change_acquisition_like_cpp(
    effect: &SpellAcquisitionEffectLikeCpp,
) -> bool {
    effect.effect_type_checked().is_ok_and(|effect_type| {
        matches!(
            effect_type,
            SPELL_EFFECT_SUMMON_LIKE_CPP
                | SPELL_EFFECT_LEARN_SPELL
                | SPELL_EFFECT_SKILL_STEP
                | SPELL_EFFECT_SKILL
                | SPELL_EFFECT_DUAL_WIELD
                | SPELL_EFFECT_LEARN_PET_SPELL_LIKE_CPP
        ) || runtime_dispatched_acquisition_effect_like_cpp(effect_type)
            || effect.effect_trigger_spell_raw != 0
    })
}

fn runtime_dispatched_acquisition_effect_like_cpp(effect_type: u32) -> bool {
    matches!(
        effect_type,
        SPELL_EFFECT_TRIGGER_MISSILE_LIKE_CPP
            | SPELL_EFFECT_SUMMON_CHANGE_ITEM_LIKE_CPP
            | SPELL_EFFECT_SUMMON_PET_LIKE_CPP
            | SPELL_EFFECT_TRIGGER_SPELL_LIKE_CPP
            | SPELL_EFFECT_SCRIPT_EFFECT_LIKE_CPP
            | SPELL_EFFECT_TRIGGER_SPELL_WITH_VALUE_LIKE_CPP
            | SPELL_EFFECT_TRIGGER_MISSILE_SPELL_WITH_VALUE_LIKE_CPP
            | SPELL_EFFECT_TRIGGER_SPELL_2_LIKE_CPP
            | SPELL_EFFECT_UNLEARN_SPECIALIZATION_LIKE_CPP
            | SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS_LIKE_CPP
            | SPELL_EFFECT_TRIGGER_ACTION_SET_LIKE_CPP
    )
}
