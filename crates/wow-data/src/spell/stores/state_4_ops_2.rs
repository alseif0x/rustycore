//! C++-shaped spell stores state definitions, part 4 of 4. operations, part 2 of 2.
//!
//! The inherent impl is divided by responsibility under #646; every
//! method keeps its original body.

use super::*;

impl SpellStore {
    pub fn aura_interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 2]> {
        self.interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .map(|(aura, _)| aura)
    }
    pub fn has_aura_interrupt_flag_like_cpp(&self, spell_id: i32, flags: u32, flags2: u32) -> bool {
        self.aura_interrupt_flags_like_cpp(spell_id)
            .is_some_and(|known| {
                (flags != 0 && known[0] & flags != 0) || (flags2 != 0 && known[1] & flags2 != 0)
            })
    }
    /// C++ `SpellInfo::HasChannelInterruptFlag` for the two
    /// `SpellAuraInterruptFlags` words loaded from difficulty zero.
    pub fn channel_interrupt_flags_like_cpp(&self, spell_id: i32) -> Option<[u32; 2]> {
        self.channel_interrupt_flags_for_difficulty_like_cpp(spell_id, 0, None)
    }
    pub fn channel_interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 2]> {
        self.interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .map(|(_, channel)| channel)
    }
    pub fn has_channel_interrupt_flag_like_cpp(
        &self,
        spell_id: i32,
        flags: u32,
        flags2: u32,
    ) -> bool {
        self.channel_interrupt_flags_like_cpp(spell_id)
            .is_some_and(|known| {
                (flags != 0 && known[0] & flags != 0) || (flags2 != 0 && known[1] & flags2 != 0)
            })
    }
    /// Port of C++ `SpellInfo::CheckShapeshift` for regular `SpellInfo`
    /// entries composed by `SpellMgr::LoadSpellInfoStore`.
    pub fn check_shapeshift_like_cpp<'a, F>(
        &self,
        spell_id: i32,
        form: u32,
        mut lookup_form: F,
    ) -> Option<SpellCastResult>
    where
        F: FnMut(u32) -> Option<&'a crate::spell_db2::SpellShapeshiftFormEntry>,
    {
        self.spells.get(&spell_id)?;

        let (stances, stances_not) = self
            .spell_shapeshift_masks
            .get(&spell_id)
            .copied()
            .unwrap_or((0, 0));
        let attributes = self
            .spell_misc_attributes
            .get(&spell_id)
            .copied()
            .unwrap_or([0; 15]);
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);

        if stance_mask & stances_not != 0 {
            return Some(SpellCastResult::NotShapeshift);
        }

        if stance_mask & stances != 0 {
            return Some(SpellCastResult::Success);
        }

        let mut act_as_shifted = false;
        let mut form_flags = 0;
        if form > 0 {
            let Some(shape_info) = lookup_form(form) else {
                return Some(SpellCastResult::Success);
            };
            form_flags = shape_info.flags;
            act_as_shifted = form_flags & shapeshift_form_flags::STANCE == 0;
        }

        if act_as_shifted {
            if attributes[0] & attributes::SPELL_ATTR0_NOT_SHAPESHIFTED != 0
                || form_flags & shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS != 0
            {
                return Some(SpellCastResult::NotShapeshift);
            }

            if stances != 0 {
                return Some(SpellCastResult::OnlyShapeshift);
            }
        } else if attributes[2] & attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM
            == 0
            && stances != 0
        {
            return Some(SpellCastResult::OnlyShapeshift);
        }

        Some(SpellCastResult::Success)
    }
    pub fn iter(&self) -> impl Iterator<Item = &SpellInfo> {
        self.spells.values()
    }
    pub fn implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<&ConditionsReference> {
        self.implicit_target_conditions
            .get(&(spell_id, effect_index))
    }
    pub fn attach_spell_implicit_target_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> usize {
        let mut attached = 0;
        let Some(entries) = conditions.entries_for_source_type_like_cpp(
            wow_constants::ConditionSourceType::SpellImplicitTarget,
        ) else {
            return attached;
        };

        self.implicit_target_conditions.clear();
        for (id, bucket) in entries {
            let Some(spell) = self.spells.get(&id.source_entry) else {
                continue;
            };

            for effect in &spell.effects {
                let bit = 1_u32.checked_shl(effect.effect_index).unwrap_or(0);
                if bit == 0 || (id.source_group & bit) == 0 {
                    continue;
                }

                self.implicit_target_conditions.insert(
                    (id.source_entry, effect.effect_index),
                    ConditionsReference::new(bucket),
                );
                attached += bucket.len();
            }
        }

        attached
    }
    /// Insert a spell into the store (for testing or dynamic registration).
    #[allow(dead_code)]
    pub fn insert(&mut self, spell_id: i32, info: SpellInfo) {
        self.spells.insert(spell_id, info);
    }
    #[allow(dead_code)]
    pub fn insert_spell_misc_attributes_like_cpp(&mut self, spell_id: i32, attributes: [u32; 15]) {
        self.spell_misc_attributes.insert(spell_id, attributes);
        self.spell_misc_attributes_by_difficulty
            .insert((spell_id, 0), attributes);
    }
    #[allow(dead_code)]
    pub fn insert_spell_misc_attributes_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        attributes: [u32; 15],
    ) {
        self.spell_misc_attributes_by_difficulty
            .insert((spell_id, difficulty_id), attributes);
        if difficulty_id == 0 {
            self.spell_misc_attributes.insert(spell_id, attributes);
        }
    }
    /// Insert one synthetic hit-metadata projection for focused tests or
    /// dynamic registration without widening `SpellInfo`/`SpellEffectInfo`.
    #[allow(dead_code)]
    pub fn insert_spell_hit_metadata_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        metadata: SpellHitMetadataLikeCpp,
    ) {
        let SpellHitMetadataLikeCpp {
            category_id,
            charge_category_id,
            defense_type,
            spell_mechanic,
            school_mask,
            effect_mechanics,
        } = metadata;
        self.spell_hit_categories_by_difficulty.insert(
            (spell_id, difficulty_id),
            SpellHitCategoriesRowLikeCpp {
                record_id: u32::MAX,
                category_id,
                charge_category_id,
                defense_type,
                spell_mechanic,
            },
        );
        self.spell_hit_misc_by_difficulty.insert(
            (spell_id, difficulty_id),
            SpellHitMiscRowLikeCpp {
                record_id: u32::MAX,
                school_mask,
            },
        );
        self.spell_hit_effect_mechanics_by_difficulty.insert(
            (spell_id, difficulty_id),
            effect_mechanics
                .into_iter()
                .filter(|(effect_index, _)| *effect_index < MAX_SPELL_EFFECTS_LIKE_CPP as u32)
                .map(|(effect_index, mechanic)| {
                    (
                        effect_index,
                        SpellHitEffectMechanicRowLikeCpp {
                            record_id: u32::MAX,
                            mechanic,
                        },
                    )
                })
                .collect(),
        );
    }
    #[allow(dead_code)]
    pub fn insert_spell_interrupt_flags_like_cpp(
        &mut self,
        spell_id: i32,
        aura_interrupt_flags: [u32; 2],
        channel_interrupt_flags: [u32; 2],
    ) {
        self.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            0,
            aura_interrupt_flags,
            channel_interrupt_flags,
        );
    }
    #[allow(dead_code)]
    pub fn insert_spell_interrupt_flags_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        aura_interrupt_flags: [u32; 2],
        channel_interrupt_flags: [u32; 2],
    ) {
        self.spell_interrupt_flags.insert(
            (spell_id, difficulty_id),
            (aura_interrupt_flags, channel_interrupt_flags),
        );
    }
    #[allow(dead_code)]
    pub fn insert_spell_shapeshift_masks_like_cpp(
        &mut self,
        spell_id: i32,
        stances: u64,
        stances_not: u64,
    ) {
        self.spell_shapeshift_masks
            .insert(spell_id, (stances, stances_not));
    }
    /// Get the total number of loaded spells.
    pub fn len(&self) -> usize {
        self.spells.len()
    }
    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}
