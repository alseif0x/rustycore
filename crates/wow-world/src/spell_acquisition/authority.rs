// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SpellAcquisitionCraftValidityAuthorityLikeCpp {
    proven_valid_spell_ids: BTreeSet<u32>,
    reasons_by_spell: BTreeMap<u32, Vec<SpellAcquisitionCraftValidityIndeterminateReasonLikeCpp>>,
}

impl SpellAcquisitionCraftValidityAuthorityLikeCpp {
    pub(crate) fn from_audited_rows_like_cpp(
        proven_valid_spell_ids: impl IntoIterator<Item = u32>,
        indeterminate: impl IntoIterator<
            Item = (
                u32,
                Vec<SpellAcquisitionCraftValidityIndeterminateReasonLikeCpp>,
            ),
        >,
    ) -> Self {
        let mut authority = Self {
            proven_valid_spell_ids: proven_valid_spell_ids.into_iter().collect(),
            reasons_by_spell: indeterminate.into_iter().collect(),
        };
        for spell_id in authority.reasons_by_spell.keys() {
            authority.proven_valid_spell_ids.remove(spell_id);
        }
        authority
    }

    pub(super) fn require_valid_like_cpp(
        &self,
        spell_id: u32,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        if self.proven_valid_spell_ids.contains(&spell_id) {
            return Ok(());
        }
        Err(
            SpellAcquisitionIndeterminateLikeCpp::CraftSpellValidityAuthority {
                spell_id,
                reasons: self
                    .reasons_by_spell
                    .get(&spell_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        vec![
                            SpellAcquisitionCraftValidityIndeterminateReasonLikeCpp::IncompleteAuthority,
                        ]
                    }),
            },
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SpellAcquisitionCastAuthorityLikeCpp {
    proven_safe_spell_ids: BTreeSet<u32>,
    reasons_by_spell: BTreeMap<u32, Vec<SpellAcquisitionCastIndeterminateReasonLikeCpp>>,
}

impl SpellAcquisitionCastAuthorityLikeCpp {
    pub(crate) fn from_evidence_like_cpp(
        evidence: impl IntoIterator<Item = SpellAcquisitionCastAuditEvidenceLikeCpp>,
    ) -> Self {
        let mut proven_safe_spell_ids = BTreeSet::new();
        let mut reasons_by_spell = BTreeMap::new();
        for row in evidence {
            let mut reasons = Vec::new();
            if !row.all_sources_complete {
                reasons.push(SpellAcquisitionCastIndeterminateReasonLikeCpp::IncompleteAuthority);
            }
            let blockers = [
                (
                    row.has_script_binding,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::ScriptBinding,
                ),
                (
                    row.has_legacy_spell_script_command,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::LegacySpellScriptCommand,
                ),
                (
                    row.has_spell_pet_aura,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::SpellPetAura,
                ),
                (
                    row.has_linked_cast,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::LinkedCast,
                ),
                (
                    row.has_linked_hit,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::LinkedHit,
                ),
                (
                    row.has_linked_aura,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::LinkedAura,
                ),
                (
                    row.has_cast_condition,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::CastCondition,
                ),
                (
                    row.has_target_condition,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::TargetCondition,
                ),
                (
                    row.has_spell_modifier_class_options,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::SpellModifierClassOptions,
                ),
                (
                    row.has_spell_modifier_label,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::SpellModifierLabel,
                ),
                (
                    row.has_aura_learn_spell,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::AuraLearnSpell,
                ),
                (
                    row.has_runtime_calc_value,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::RuntimeCalcValue,
                ),
                (
                    row.is_disabled,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::DisabledSpell,
                ),
                (
                    row.has_hardcoded_dummy_handler,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::HardcodedDummyHandler,
                ),
                (
                    row.is_delayed_or_channeled,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::DelayedOrChanneled,
                ),
                (
                    row.has_unsupported_target_selection,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::UnsupportedTargetSelection,
                ),
                (
                    row.has_unmodelled_check_cast,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::UnmodelledCheckCast,
                ),
                (
                    row.has_runtime_state_mutation_before_closure,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::RuntimeStateMutationBeforeClosure,
                ),
                (
                    row.is_passive_cast && !row.passive_cast_prerequisites_proven,
                    SpellAcquisitionCastIndeterminateReasonLikeCpp::PassiveCastPrerequisites,
                ),
            ];
            for (blocked, reason) in blockers {
                if blocked {
                    reasons.push(reason);
                }
            }
            reasons.sort();
            reasons.dedup();
            if reasons.is_empty() {
                proven_safe_spell_ids.insert(row.spell_id);
            } else {
                reasons_by_spell
                    .entry(row.spell_id)
                    .or_insert_with(Vec::new)
                    .extend(reasons);
            }
        }
        for reasons in reasons_by_spell.values_mut() {
            reasons.sort();
            reasons.dedup();
        }
        // Multiple independent evidence producers may report the same spell.
        // A single blocker must dominate every positive row.
        for spell_id in reasons_by_spell.keys() {
            proven_safe_spell_ids.remove(spell_id);
        }
        Self {
            proven_safe_spell_ids,
            reasons_by_spell,
        }
    }

    pub(crate) fn from_audited_rows_like_cpp(
        proven_safe_spell_ids: impl IntoIterator<Item = u32>,
        indeterminate: impl IntoIterator<
            Item = (u32, Vec<SpellAcquisitionCastIndeterminateReasonLikeCpp>),
        >,
    ) -> Self {
        let mut authority = Self {
            proven_safe_spell_ids: proven_safe_spell_ids.into_iter().collect(),
            reasons_by_spell: indeterminate.into_iter().collect(),
        };
        for spell_id in authority.reasons_by_spell.keys() {
            authority.proven_safe_spell_ids.remove(spell_id);
        }
        authority
    }

    pub(super) fn require_safe_like_cpp(
        &self,
        spell_id: u32,
    ) -> Result<(), SpellAcquisitionIndeterminateLikeCpp> {
        if self.proven_safe_spell_ids.contains(&spell_id) {
            return Ok(());
        }
        Err(SpellAcquisitionIndeterminateLikeCpp::CastAuthority {
            spell_id,
            reasons: self
                .reasons_by_spell
                .get(&spell_id)
                .cloned()
                .unwrap_or_else(|| {
                    vec![SpellAcquisitionCastIndeterminateReasonLikeCpp::IncompleteAuthority]
                }),
        })
    }
}
