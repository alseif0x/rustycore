// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Server-side spell overlays and corrections.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: i32,
    pub difficulty_id: u32,
    pub effect: i32,
    pub effect_aura: i32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura_period: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_base_points: f32,
    pub effect_misc_value_1: i32,
    pub effect_misc_value_2: i32,
    pub effect_radius_index_1: u32,
    pub effect_radius_index_2: u32,
    pub effect_spell_class_mask: [i32; 4],
    pub implicit_target_1: i32,
    pub implicit_target_2: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLikeCpp {
    pub effect_index: i32,
    pub difficulty_id: u32,
    pub effect: i32,
    pub effect_aura: i32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura_period: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_base_points: i32,
    pub effect_misc_value: [i32; 2],
    pub effect_radius_index: [u32; 2],
    pub effect_spell_class_mask: [i32; 4],
    pub implicit_target: [i32; 2],
}

impl ServersideSpellEffectRowLikeCpp {
    pub fn into_effect_like_cpp(self) -> Result<ServersideSpellEffectLikeCpp, Self> {
        let base_points = f64::from(self.effect_base_points);
        // `i32::MAX as f32` rounds to 2^31, so compare in f64 with an exclusive upper bound.
        if !base_points.is_finite()
            || base_points < f64::from(i32::MIN)
            || base_points >= f64::from(i32::MAX) + 1.0
        {
            return Err(self);
        }

        let effect_base_points = self.effect_base_points.trunc() as i32;
        Ok(ServersideSpellEffectLikeCpp {
            effect_index: self.effect_index,
            difficulty_id: self.difficulty_id,
            effect: self.effect,
            effect_aura: self.effect_aura,
            effect_amplitude: self.effect_amplitude,
            effect_attributes: self.effect_attributes,
            effect_aura_period: self.effect_aura_period,
            effect_bonus_coefficient: self.effect_bonus_coefficient,
            effect_chain_amplitude: self.effect_chain_amplitude,
            effect_chain_targets: self.effect_chain_targets,
            effect_item_type: self.effect_item_type,
            effect_mechanic: self.effect_mechanic,
            effect_points_per_resource: self.effect_points_per_resource,
            effect_pos_facing: self.effect_pos_facing,
            effect_real_points_per_level: self.effect_real_points_per_level,
            effect_trigger_spell: self.effect_trigger_spell,
            bonus_coefficient_from_ap: self.bonus_coefficient_from_ap,
            pvp_multiplier: self.pvp_multiplier,
            coefficient: self.coefficient,
            variance: self.variance,
            resource_coefficient: self.resource_coefficient,
            group_size_base_points_coefficient: self.group_size_base_points_coefficient,
            effect_base_points,
            effect_misc_value: [self.effect_misc_value_1, self.effect_misc_value_2],
            effect_radius_index: [self.effect_radius_index_1, self.effect_radius_index_2],
            effect_spell_class_mask: self.effect_spell_class_mask,
            implicit_target: [self.implicit_target_1, self.implicit_target_2],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServersideSpellEffectKeyLikeCpp {
    pub spell_id: u32,
    pub difficulty_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServersideSpellEffectLoadErrorKindLikeCpp {
    RegularSpellAlreadyLoaded,
    DifficultyMissing,
    EffectIndexOutOfRange,
    EffectTypeOutOfRange,
    AuraTypeOutOfRange,
    ImplicitTarget1OutOfRange,
    ImplicitTarget2OutOfRange,
    BasePointsOutOfRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLoadErrorLikeCpp {
    pub row: ServersideSpellEffectRowLikeCpp,
    pub kind: ServersideSpellEffectLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServersideSpellEffectLoadWarningKindLikeCpp {
    EffectRadius1Missing,
    EffectRadius2Missing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLoadWarningLikeCpp {
    pub row: ServersideSpellEffectRowLikeCpp,
    pub kind: ServersideSpellEffectLoadWarningKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServersideSpellEffectStoreLikeCpp {
    pub effects_by_spell_and_difficulty:
        BTreeMap<ServersideSpellEffectKeyLikeCpp, Vec<ServersideSpellEffectLikeCpp>>,
}

impl ServersideSpellEffectStoreLikeCpp {
    pub fn from_rows_like_cpp<I, RegularSpellExists, DifficultyExists, RadiusExists>(
        rows: I,
        mut regular_spell_exists: RegularSpellExists,
        mut difficulty_exists: DifficultyExists,
        mut radius_exists: RadiusExists,
    ) -> ServersideSpellEffectLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = ServersideSpellEffectRowLikeCpp>,
        RegularSpellExists: FnMut(u32) -> bool,
        DifficultyExists: FnMut(u32) -> bool,
        RadiusExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_effect_count = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for row in rows {
            if regular_spell_exists(row.spell_id) {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
                });
                continue;
            }

            let difficulty_id = u32::from(row.difficulty_id as u8);
            if difficulty_id != 0 && !difficulty_exists(difficulty_id) {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::DifficultyMissing,
                });
                continue;
            }

            // Negative indices are outside the source-valid input domain. The
            // C++ path would index before its vector, so reject this malformed
            // row defensively instead of claiming invalid-input parity.
            if row.effect_index < 0 || row.effect_index >= MAX_SPELL_EFFECTS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::EffectIndexOutOfRange,
                });
                continue;
            }

            if row.effect >= TOTAL_SPELL_EFFECTS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::EffectTypeOutOfRange,
                });
                continue;
            }

            if row.effect_aura >= TOTAL_AURAS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::AuraTypeOutOfRange,
                });
                continue;
            }

            if row.implicit_target_1 >= TOTAL_SPELL_TARGETS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget1OutOfRange,
                });
                continue;
            }

            if row.implicit_target_2 >= TOTAL_SPELL_TARGETS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget2OutOfRange,
                });
                continue;
            }

            if row.effect_radius_index_1 != 0 && !radius_exists(row.effect_radius_index_1) {
                warnings.push(ServersideSpellEffectLoadWarningLikeCpp {
                    row: row.clone(),
                    kind: ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius1Missing,
                });
            }

            if row.effect_radius_index_2 != 0 && !radius_exists(row.effect_radius_index_2) {
                warnings.push(ServersideSpellEffectLoadWarningLikeCpp {
                    row: row.clone(),
                    kind: ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius2Missing,
                });
            }

            let key = ServersideSpellEffectKeyLikeCpp {
                spell_id: row.spell_id,
                difficulty_id,
            };
            let mut effect = match row.into_effect_like_cpp() {
                Ok(effect) => effect,
                Err(row) => {
                    errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                        row,
                        kind: ServersideSpellEffectLoadErrorKindLikeCpp::BasePointsOutOfRange,
                    });
                    continue;
                }
            };
            effect.difficulty_id = difficulty_id;
            store
                .effects_by_spell_and_difficulty
                .entry(key)
                .or_default()
                .push(effect);
            loaded_effect_count += 1;
        }

        ServersideSpellEffectLoadOutcomeLikeCpp {
            store,
            loaded_effect_count,
            errors,
            warnings,
        }
    }

    pub fn effects_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> Option<&[ServersideSpellEffectLikeCpp]> {
        self.effects_by_spell_and_difficulty
            .get(&ServersideSpellEffectKeyLikeCpp {
                spell_id,
                difficulty_id,
            })
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLoadOutcomeLikeCpp {
    pub store: ServersideSpellEffectStoreLikeCpp,
    pub loaded_effect_count: usize,
    pub errors: Vec<ServersideSpellEffectLoadErrorLikeCpp>,
    pub warnings: Vec<ServersideSpellEffectLoadWarningLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellRowLikeCpp {
    pub spell_id: u32,
    pub difficulty_id: u32,
    pub category_id: u32,
    pub dispel: u32,
    pub mechanic: u32,
    pub attributes: u32,
    pub attributes_ex: [u32; 14],
    pub stances: u64,
    pub stances_not: u64,
    pub targets: u32,
    pub target_creature_type: u32,
    pub requires_spell_focus: u32,
    pub facing_caster_flags: u32,
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub exclude_caster_aura_state: u32,
    pub exclude_target_aura_state: u32,
    pub caster_aura_spell: u32,
    pub target_aura_spell: u32,
    pub exclude_caster_aura_spell: u32,
    pub exclude_target_aura_spell: u32,
    pub caster_aura_type: i32,
    pub target_aura_type: i32,
    pub exclude_caster_aura_type: i32,
    pub exclude_target_aura_type: i32,
    pub casting_time_index: u32,
    pub recovery_time: u32,
    pub category_recovery_time: u32,
    pub start_recovery_category: u32,
    pub start_recovery_time: u32,
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: [u32; 2],
    pub channel_interrupt_flags: [u32; 2],
    pub proc_flags: [u32; 2],
    pub proc_chance: u32,
    pub proc_charges: u32,
    pub proc_cooldown: u32,
    pub proc_base_ppm: f32,
    pub max_level: u32,
    pub base_level: u32,
    pub spell_level: u32,
    pub duration_index: u32,
    pub range_index: u32,
    pub speed: f32,
    pub launch_delay: f32,
    pub stack_amount: u32,
    pub equipped_item_class: i32,
    pub equipped_item_sub_class_mask: i32,
    pub equipped_item_inventory_type_mask: i32,
    pub content_tuning_id: u32,
    pub spell_name: String,
    pub cone_angle: f32,
    pub cone_width: f32,
    pub max_target_level: u32,
    pub max_affected_targets: u32,
    pub spell_family_name: u32,
    pub spell_family_flags: [u32; 4],
    pub dmg_class: u32,
    pub prevention_type: u32,
    pub area_group_id: i32,
    pub school_mask: u32,
    pub charge_category_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellInfoLikeCpp {
    pub row: ServersideSpellRowLikeCpp,
    pub effects: Vec<ServersideSpellEffectLikeCpp>,
}

impl ServersideSpellInfoLikeCpp {
    /// Port of C++ `SpellInfo::CheckShapeshift` (`SpellInfo.cpp`).
    pub fn check_shapeshift_like_cpp<'a, F>(&self, form: u32, mut lookup_form: F) -> SpellCastResult
    where
        F: FnMut(u32) -> Option<&'a crate::spell_db2::SpellShapeshiftFormEntry>,
    {
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);

        if stance_mask & self.row.stances_not != 0 {
            return SpellCastResult::NotShapeshift;
        }

        if stance_mask & self.row.stances != 0 {
            return SpellCastResult::Success;
        }

        let mut act_as_shifted = false;
        let mut form_flags = 0;
        if form > 0 {
            let Some(shape_info) = lookup_form(form) else {
                return SpellCastResult::Success;
            };
            form_flags = shape_info.flags;
            act_as_shifted = form_flags & shapeshift_form_flags::STANCE == 0;
        }

        if act_as_shifted {
            if self.row.attributes & attributes::SPELL_ATTR0_NOT_SHAPESHIFTED != 0
                || form_flags & shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS != 0
            {
                return SpellCastResult::NotShapeshift;
            }

            if self.row.stances != 0 {
                return SpellCastResult::OnlyShapeshift;
            }
        } else if self.row.attributes_ex[1]
            & attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM
            == 0
            && self.row.stances != 0
        {
            return SpellCastResult::OnlyShapeshift;
        }

        SpellCastResult::Success
    }
}

fn canonical_server_effect_slots(
    effects: &[ServersideSpellEffectLikeCpp],
    difficulty_id: u32,
) -> Vec<ServersideSpellEffectLikeCpp> {
    let Some(max_effect_index) = effects.iter().map(|effect| effect.effect_index).max() else {
        return Vec::new();
    };

    let mut canonical = (0..=max_effect_index)
        .map(|effect_index| ServersideSpellEffectLikeCpp {
            effect_index,
            difficulty_id,
            effect: 0,
            effect_aura: 0,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura_period: 0,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_item_type: 0,
            effect_mechanic: 0,
            effect_points_per_resource: 0.0,
            effect_pos_facing: 0.0,
            effect_real_points_per_level: 0.0,
            effect_trigger_spell: 0,
            bonus_coefficient_from_ap: 0.0,
            pvp_multiplier: 0.0,
            coefficient: 0.0,
            variance: 0.0,
            resource_coefficient: 0.0,
            group_size_base_points_coefficient: 0.0,
            effect_base_points: 0,
            effect_misc_value: [0; 2],
            effect_radius_index: [0; 2],
            effect_spell_class_mask: [0; 4],
            implicit_target: [0; 2],
        })
        .collect::<Vec<_>>();

    for effect in effects {
        let slot = &mut canonical[effect.effect_index as usize];
        *slot = effect.clone();
        slot.difficulty_id = difficulty_id;
    }

    canonical
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServersideSpellLoadErrorKindLikeCpp {
    RegularSpellAlreadyLoaded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellLoadErrorLikeCpp {
    pub row: ServersideSpellRowLikeCpp,
    pub kind: ServersideSpellLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServersideSpellStoreLikeCpp {
    pub spell_infos_by_spell_and_difficulty:
        BTreeMap<ServersideSpellEffectKeyLikeCpp, ServersideSpellInfoLikeCpp>,
    pub serverside_spell_names: Vec<(u32, String)>,
}

impl ServersideSpellStoreLikeCpp {
    pub fn from_rows_like_cpp<I, RegularSpellExists>(
        rows: I,
        effects: &ServersideSpellEffectStoreLikeCpp,
        mut regular_spell_exists: RegularSpellExists,
    ) -> ServersideSpellLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = ServersideSpellRowLikeCpp>,
        RegularSpellExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_spell_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if regular_spell_exists(row.spell_id) {
                errors.push(ServersideSpellLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
                });
                continue;
            }

            let difficulty_id = u32::from(row.difficulty_id as u8);
            let key = ServersideSpellEffectKeyLikeCpp {
                spell_id: row.spell_id,
                difficulty_id,
            };
            let staged_effects = effects
                .effects_for_spell_difficulty_like_cpp(row.spell_id, difficulty_id)
                .map(|effects| canonical_server_effect_slots(effects, difficulty_id))
                .unwrap_or_default();

            store
                .serverside_spell_names
                .push((row.spell_id, row.spell_name.clone()));
            let mut canonical_row = row;
            canonical_row.difficulty_id = difficulty_id;
            match store.spell_infos_by_spell_and_difficulty.entry(key) {
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    let first_name = entry.get().row.spell_name.clone();
                    entry.get_mut().row = canonical_row;
                    entry.get_mut().row.spell_name = first_name;
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(ServersideSpellInfoLikeCpp {
                        row: canonical_row,
                        effects: staged_effects,
                    });
                }
            }
            loaded_spell_count += 1;
        }

        ServersideSpellLoadOutcomeLikeCpp {
            store,
            loaded_spell_count,
            errors,
        }
    }

    pub fn get_serverside_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> Option<&ServersideSpellInfoLikeCpp> {
        self.spell_infos_by_spell_and_difficulty
            .get(&ServersideSpellEffectKeyLikeCpp {
                spell_id,
                difficulty_id,
            })
    }

    /// Whether an accepted server-side spell payload could override this
    /// cast's visual. This reports metadata potential, not active aura state.
    pub fn may_override_spell_visual(&self, target_spell_id: u32) -> bool {
        self.spell_infos_by_spell_and_difficulty
            .values()
            .any(|spell| {
                spell.effects.iter().any(|effect| {
                    u32::try_from(effect.effect).is_ok_and(|effect_type| {
                        super::visuals::is_visual_override_aura(effect_type, effect.effect_aura)
                            && effect.effect_misc_value[0] as u32 == target_spell_id
                    })
                })
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellLoadOutcomeLikeCpp {
    pub store: ServersideSpellStoreLikeCpp,
    pub loaded_spell_count: usize,
    pub errors: Vec<ServersideSpellLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCustomAttributeRowLikeCpp {
    pub spell_id: u32,
    pub attributes: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCustomAttributeSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub effects: Vec<SpellEffectInfo>,
}

impl SpellCustomAttributeSourceSpellInfoLikeCpp {
    fn into_source_variant_like_cpp(self) -> SpellCustomAttributeSourceVariantLikeCpp {
        SpellCustomAttributeSourceVariantLikeCpp {
            spell_id: self.spell_id,
            difficulty: self.difficulty,
            effect_types: Some(
                self.effects
                    .into_iter()
                    .map(|effect| effect.effect)
                    .collect(),
            ),
        }
    }
}

/// Exact spell variant used while composing SQL custom attributes.
///
/// `effect_types == None` means that the variant exists but its effect payload is not represented
/// by the current source. Attributes that do not depend on an effect type can still be composed;
/// effect-dependent validation must fail closed instead of treating missing coverage as an empty
/// effect list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCustomAttributeSourceVariantLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub effect_types: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpellCustomAttributeKeyLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCustomAttributeLoadErrorKindLikeCpp {
    SpellMissing,
    ShareDamageWithoutSchoolDamage,
    ShareDamageEffectCoverageUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCustomAttributeLoadErrorLikeCpp {
    pub spell_id: u32,
    pub difficulty: Option<u32>,
    /// Raw SQL bits from the rejected row. Consumers can keep uncertainty
    /// scoped to the attributes that were actually requested.
    pub attributes: u32,
    pub kind: SpellCustomAttributeLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellCustomAttributeStoreLikeCpp {
    pub attributes_by_spell_and_difficulty: BTreeMap<SpellCustomAttributeKeyLikeCpp, u32>,
}

impl SpellCustomAttributeStoreLikeCpp {
    pub fn from_sql_rows_like_cpp<I, SpellInfosById>(
        rows: I,
        mut spell_infos_by_id: SpellInfosById,
    ) -> SpellCustomAttributeLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellCustomAttributeRowLikeCpp>,
        SpellInfosById: FnMut(u32) -> Vec<SpellCustomAttributeSourceSpellInfoLikeCpp>,
    {
        Self::from_sql_rows_for_variants_like_cpp(rows, move |spell_id| {
            spell_infos_by_id(spell_id)
                .into_iter()
                .map(SpellCustomAttributeSourceSpellInfoLikeCpp::into_source_variant_like_cpp)
                .collect()
        })
    }

    pub fn from_sql_rows_for_variants_like_cpp<I, VariantsById>(
        rows: I,
        mut variants_by_id: VariantsById,
    ) -> SpellCustomAttributeLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellCustomAttributeRowLikeCpp>,
        VariantsById: FnMut(u32) -> Vec<SpellCustomAttributeSourceVariantLikeCpp>,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut applied_variant_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            let variants = variants_by_id(row.spell_id);
            if variants.is_empty() {
                errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                    spell_id: row.spell_id,
                    difficulty: None,
                    attributes: row.attributes,
                    kind: SpellCustomAttributeLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            for variant in variants {
                if row.attributes & SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP != 0 {
                    match variant.effect_types.as_ref() {
                        None => {
                            errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                                spell_id: row.spell_id,
                                difficulty: Some(variant.difficulty),
                                attributes: row.attributes,
                                kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
                            });
                            continue;
                        }
                        Some(effect_types)
                            if !effect_types
                                .contains(&spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE) =>
                        {
                            errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                                spell_id: row.spell_id,
                                difficulty: Some(variant.difficulty),
                                attributes: row.attributes,
                                kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageWithoutSchoolDamage,
                            });
                            continue;
                        }
                        Some(_) => {}
                    }
                }

                let key = SpellCustomAttributeKeyLikeCpp {
                    spell_id: variant.spell_id,
                    difficulty: variant.difficulty,
                };
                *store
                    .attributes_by_spell_and_difficulty
                    .entry(key)
                    .or_default() |= row.attributes;
                applied_variant_count += 1;
            }

            loaded_row_count += 1;
        }

        SpellCustomAttributeLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            applied_variant_count,
            errors,
        }
    }

    pub fn attributes_for_spell_difficulty_like_cpp(&self, spell_id: u32, difficulty: u32) -> u32 {
        self.attributes_by_spell_and_difficulty
            .get(&SpellCustomAttributeKeyLikeCpp {
                spell_id,
                difficulty,
            })
            .copied()
            .unwrap_or(0)
    }

    pub fn attributes_for_spell_any_difficulty_like_cpp(&self, spell_id: u32) -> u32 {
        self.attributes_by_spell_and_difficulty
            .range(
                SpellCustomAttributeKeyLikeCpp {
                    spell_id,
                    difficulty: 0,
                }..=SpellCustomAttributeKeyLikeCpp {
                    spell_id,
                    difficulty: u32::MAX,
                },
            )
            .fold(0, |attributes, (_, variant_attributes)| {
                attributes | variant_attributes
            })
    }

    pub fn has_attribute_any_difficulty_like_cpp(&self, spell_id: u32, attribute: u32) -> bool {
        self.attributes_for_spell_any_difficulty_like_cpp(spell_id) & attribute != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCustomAttributeLoadOutcomeLikeCpp {
    pub store: SpellCustomAttributeStoreLikeCpp,
    pub loaded_row_count: usize,
    pub applied_variant_count: usize,
    pub errors: Vec<SpellCustomAttributeLoadErrorLikeCpp>,
}
