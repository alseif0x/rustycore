//! Indexed spell-effect storage and difficulty-aware reads.

use super::*;

impl SpellStore {
    /// Whether known regular spell metadata could override this cast's visual.
    ///
    /// This is metadata potential only. Aura application and caster/aura state
    /// remain runtime facts. Dynamic and corrected payloads are inspected from
    /// the canonical spell map on each query so no second mutable index exists.
    pub fn may_override_spell_visual(&self, target_spell_id: u32) -> bool {
        if self
            .regular_visual_override_targets
            .contains(&target_spell_id)
        {
            return true;
        }

        if self
            .spell_effects_by_difficulty
            .values()
            .flatten()
            .any(|effect| {
                super::super::visuals::is_visual_override_aura(effect.effect, effect.effect_aura)
                    && effect.effect_misc_value_1 as u32 == target_spell_id
            })
        {
            return true;
        }

        self.spells.values().any(|spell| {
            if spell.effects.is_empty() {
                return super::super::visuals::is_visual_override_aura(
                    spell.effect_type,
                    spell.aura_type.unwrap_or_default(),
                );
            }

            spell.effects.iter().any(|effect| {
                super::super::visuals::is_visual_override_aura(effect.effect, effect.effect_aura)
                    && effect.effect_misc_value_1 as u32 == target_spell_id
            })
        })
    }

    fn has_raw_effect_rows_for_spell(&self, spell_id: i32) -> bool {
        (u8::MIN..=u8::MAX).any(|difficulty_id| {
            self.spell_effects_by_difficulty
                .contains_key(&(spell_id, difficulty_id))
        })
    }

    /// Return the canonical rows stored for this exact difficulty.
    ///
    /// A dynamically registered base spell can provide its explicit effect
    /// vector only when no raw effect rows exist for that spell at any
    /// difficulty.
    pub fn effect_rows_for_exact_difficulty(
        &self,
        spell_id: i32,
        difficulty_id: u8,
    ) -> Option<&[SpellEffectInfo]> {
        if let Some(rows) = self
            .spell_effects_by_difficulty
            .get(&(spell_id, difficulty_id))
        {
            return Some(rows);
        }

        if difficulty_id != 0 || self.has_raw_effect_rows_for_spell(spell_id) {
            return None;
        }

        self.spells.get(&spell_id).map(SpellInfo::effects)
    }

    /// Return the active prefix along the existing C++ difficulty fallback.
    /// Inert-only buckets do not terminate the legacy projection.
    pub fn effects_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<&[SpellEffectInfo]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(effects) = self
                .spell_effects_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                let active_count = effects
                    .iter()
                    .take_while(|effect| effect.effect != 0)
                    .count();
                if active_count > 0 {
                    return Some(&effects[..active_count]);
                }
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spells.get(&spell_id).map(SpellInfo::effects)
    }

    pub(super) fn hydrate_spell_effect_rows(
        &mut self,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
    ) {
        let mut visual_effect_winners =
            BTreeMap::<(u32, u8, i32), &crate::spell_db2::SpellEffectDb2Entry>::new();
        for row in spell_effect_store.entries_like_cpp() {
            visual_effect_winners
                .entry((row.spell_id, row.difficulty_id as u8, row.effect_index))
                .and_modify(|winner| {
                    if row.id > winner.id {
                        *winner = row;
                    }
                })
                .or_insert(row);
        }

        self.regular_visual_override_targets.clear();
        for row in visual_effect_winners.values() {
            if row.effect != 0
                && super::super::visuals::is_visual_override_aura(
                    row.effect,
                    i32::from(row.effect_aura),
                )
            {
                self.regular_visual_override_targets
                    .insert(row.effect_misc_value[0] as u32);
            }
        }

        let mut winners_by_difficulty = HashMap::<(i32, u8), Vec<(u32, SpellEffectInfo)>>::new();

        // SpellMgr.cpp:2500-2508 assigns each RecordID-ordered row into its
        // Effects[EffectIndex] slot, so the greatest surviving ID wins.
        for row in spell_effect_store.entries_like_cpp() {
            let spell_id = row.spell_id as i32;
            let difficulty_id = row.difficulty_id as u8;
            let effect = Self::spell_effect_from_db2(row);
            let slot_rows = winners_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default();
            if let Some((record_id, current)) = slot_rows
                .iter_mut()
                .find(|(_, current)| current.effect_index == effect.effect_index)
            {
                if row.id > *record_id {
                    *record_id = row.id;
                    *current = effect;
                }
            } else {
                slot_rows.push((row.id, effect));
            }
        }

        for ((spell_id, difficulty_id), mut rows) in winners_by_difficulty {
            rows.sort_by_key(|(_, effect)| (effect.effect == 0, effect.effect_index));
            let canonical_rows = rows
                .into_iter()
                .map(|(_, effect)| effect)
                .collect::<Vec<_>>();

            if difficulty_id == 0 {
                let active_effects = canonical_rows
                    .iter()
                    .filter(|effect| effect.effect != 0)
                    .cloned()
                    .collect::<Vec<_>>();
                if !active_effects.is_empty() {
                    let spell = self
                        .spells
                        .entry(spell_id)
                        .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
                    spell.effects = active_effects;
                }
            }

            self.spell_effects_by_difficulty
                .insert((spell_id, difficulty_id), canonical_rows);
        }

        for spell in self.spells.values_mut() {
            Self::hydrate_primary_effect(spell);
        }
    }

    fn spell_effect_from_db2(row: &crate::spell_db2::SpellEffectDb2Entry) -> SpellEffectInfo {
        SpellEffectInfo {
            effect_index: row.effect_index as u32,
            effect: row.effect,
            effect_aura: i32::from(row.effect_aura),
            effect_base_points: row.effect_base_points,
            effect_mechanic: row.effect_mechanic,
            effect_amplitude: row.effect_amplitude,
            effect_die_sides: row.effect_die_sides,
            effect_bonus_coefficient_from_ap: row.bonus_coefficient_from_ap,
            effect_spell_class_mask: row.effect_spell_class_mask,
            effect_misc_value_1: row.effect_misc_value[0],
            effect_misc_value_2: row.effect_misc_value[1],
            effect_trigger_spell: row.effect_trigger_spell,
            effect_radius_index_1: row.effect_radius_index[0],
            position_facing: row.effect_pos_facing,
            chain_targets: row.effect_chain_targets,
            implicit_target_1: u32::try_from(row.implicit_target[0]).unwrap_or(0),
            implicit_target_2: u32::try_from(row.implicit_target[1]).unwrap_or(0),
            polarity_metadata: Some(SpellEffectPolarityMetadata {
                attributes: row.effect_attributes as u32,
                real_points_per_level: row.effect_real_points_per_level,
                scaling_coefficient: row.coefficient,
                scaling_variance: row.variance,
            }),
        }
    }

    fn hydrate_primary_effect(info: &mut SpellInfo) {
        info.effects.sort_by_key(|effect| effect.effect_index);
        if let Some(primary) = info.effects.iter().find(|effect| effect.effect != 0) {
            info.effect_type = primary.effect;
            info.effect_base_points = primary.effect_base_points;
            info.effect_bonus_coefficient = 0.0;
            info.aura_type = Some(primary.effect_aura);
        }
    }
}

#[cfg(test)]
#[path = "effects_visual_tests.rs"]
mod tests;
