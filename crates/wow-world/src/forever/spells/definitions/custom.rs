//! Complete ordered derived portion of LoadSpellInfoCustomAttributes, after
//! its separately admitted SQL prefix. 02245dcd SpellMgr.cpp:3039-3375.
//! No raw mutation, Player/learned state or fully executable SpellInfo claim.
mod binary;
mod effect_flags;
mod tail;
#[cfg(test)]
mod tests;
use super::{Key, SpellDefinitionSeeds, SpellValueError, positivity, target_masks};
use std::collections::BTreeSet;
use wow_data::forever_birth::item_records::ItemCatalog;

const ENCHANT_PROC: u32 = 0x1;
const CONE_LINE: u32 = 0x4;
const NO_INITIAL_THREAT: u32 = 0x10;
const AURA_CC: u32 = 0x20;
const CAN_CRIT: u32 = 0x80;
const DIRECT_DAMAGE: u32 = 0x100;
const CHARGE: u32 = 0x200;
const PICKPOCKET: u32 = 0x400;
const IGNORE_ARMOR: u32 = 0x8000;
const NEEDS_AMMO: u32 = 0x0008_0000;
const BINARY: u32 = 0x0010_0000;
const MIXED_SCHOOL: u32 = 0x0020_0000;
const TALENT: u32 = 0x0080_0000;
const CANNOT_SAVE: u32 = 0x0100_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellCustomAttributeError {
    RequiresSqlPrefixAndSourceOrder,
    RequiresValueGameTables,
    AlreadyApplied,
    IncompleteDependencies,
    UndefinedMechanicShift,
    Value(SpellValueError),
}
impl From<SpellValueError> for SpellCustomAttributeError {
    fn from(value: SpellValueError) -> Self {
        Self::Value(value)
    }
}

/// Source branch/write counts for this once-only phase, not global readiness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CustomAttributeCounts {
    pub definitions: usize,
    pub effect_slots: usize,
    pub enchant_proc_assignments: usize,
    pub binary_assignments: usize,
    pub new_negative_effects: usize,
    pub normal_school_clears: usize,
    pub explicit_masks_initialized: usize,
    pub ammo_assignments: usize,
    pub crit_clears: usize,
    pub liquid_assignments: usize,
}

impl SpellDefinitionSeeds {
    /// Consume the canonical startup owner. A failure drops it rather than
    /// returning partial definitions; no Arc publication/await/delivery here.
    /// Source primary and equal_range order were admitted by the native replay.
    pub fn with_custom_attributes(
        mut self,
        items: &ItemCatalog,
        draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
    ) -> Result<Self, SpellCustomAttributeError> {
        if self.custom_attributes.is_some() {
            return Err(SpellCustomAttributeError::AlreadyApplied);
        }
        if self.sql_custom_attributes.is_none()
            || self.source_order.is_none()
            || self.skill_line_abilities.is_none()
        {
            return Err(SpellCustomAttributeError::RequiresSqlPrefixAndSourceOrder);
        }
        if self.value_game_tables.is_none() {
            return Err(SpellCustomAttributeError::RequiresValueGameTables);
        }
        if self.catalog.counts()[36..42]
            .iter()
            .any(|(_, _, unknown)| *unknown != 0)
        {
            return Err(SpellCustomAttributeError::IncompleteDependencies);
        }
        let talent_spells: BTreeSet<_> = self
            .catalog
            .talent_records()
            .map(|row| row.spell_id)
            .collect();
        let mut counts = CustomAttributeCounts::default();
        let length = self
            .source_order
            .as_ref()
            .expect("admitted order")
            .primary
            .len();
        for position in 0..length {
            let key = self.source_order.as_ref().expect("admitted order").primary[position];
            counts.definitions += 1;
            let slots = self.definitions[&key].effects.len();
            for index in 0..slots {
                counts.effect_slots += 1;
                effect_flags::apply(&mut self, key, index, &mut counts)?;
            }
            binary::initialize(&mut self, key, items, draw, &mut counts)?;
            let definition = self.definitions.get_mut(&key).expect("admitted key");
            if definition.fields.school_mask & 1 != 0 && definition.fields.school_mask & 0x7e != 0 {
                definition.fields.school_mask &= !1;
                definition.custom_attributes |= MIXED_SCHOOL;
                counts.normal_school_clears += 1;
            }
            counts.new_negative_effects += positivity::initialize(&mut self, key, items, draw)?;
            tail::local(&mut self, key, talent_spells.contains(&key.0), &mut counts);
            let masks = target_masks::derive(&self.catalog, &self.definitions[&key]);
            self.definitions
                .get_mut(&key)
                .expect("admitted key")
                .explicit_target_masks = masks;
            counts.explicit_masks_initialized += 1;
            tail::ammo_and_leave_world(&mut self, key, &mut counts);
        }
        tail::second_pass_and_liquids(&mut self, &mut counts)?;
        self.custom_attributes = Some(counts);
        Ok(self)
    }
    pub fn custom_attribute_counts(&self) -> Option<CustomAttributeCounts> {
        self.custom_attributes
    }
}
