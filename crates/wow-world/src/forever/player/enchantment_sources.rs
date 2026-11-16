use std::convert::Infallible;
use wow_data::forever_game_tables::SpellValueGameTables;
use wow_entities::{SkillEnchantmentItemTemplate, SkillEnchantmentRecord, SkillEnchantmentSource};

use crate::forever::{creation::NumericItemTemplates, spells::SpellDefinitionSeeds};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEnchantmentSourcesError {
    MissingValueGameTables,
}

/// Immutable lookups over the currently admitted World definitions and item
/// templates. Absence is scoped to those effective sources; upstream unknown
/// baseline accounting remains owned by the catalog admission path.
/// The game tables are borrowed from the same definition owner as enchant rows.
pub struct SkillEnchantmentSources<'a> {
    definitions: &'a SpellDefinitionSeeds,
    item_templates: &'a NumericItemTemplates,
    value_game_tables: &'a SpellValueGameTables,
}

impl<'a> SkillEnchantmentSources<'a> {
    pub fn new(
        definitions: &'a SpellDefinitionSeeds,
        item_templates: &'a NumericItemTemplates,
    ) -> Result<Self, SkillEnchantmentSourcesError> {
        let value_game_tables = definitions
            .value_game_tables()
            .ok_or(SkillEnchantmentSourcesError::MissingValueGameTables)?;
        Ok(Self {
            definitions,
            item_templates,
            value_game_tables,
        })
    }
}

impl SkillEnchantmentSource for SkillEnchantmentSources<'_> {
    type Error = Infallible;

    fn enchantment(&self, id: u32) -> Result<Option<SkillEnchantmentRecord>, Self::Error> {
        Ok(self
            .definitions
            .spell_item_enchantment_record(id)
            .map(|row| {
                SkillEnchantmentRecord::new(
                    row.id,
                    row.effect.map(|value| value as u32),
                    row.effect_points_min,
                    row.effect_arg,
                    row.flags,
                    row.effect_scaling_points,
                    row.scaling_class,
                    row.scaling_class_restricted,
                    row.condition_id as u32,
                    row.required_skill_id as u32,
                    row.required_skill_rank as u32,
                    row.min_level as u32,
                    row.max_level as u32,
                )
            }))
    }

    fn item_template(&self, id: u32) -> Result<Option<SkillEnchantmentItemTemplate>, Self::Error> {
        Ok(self.item_templates.template(id).map(|row| {
            SkillEnchantmentItemTemplate::new(
                row.extended.inventory_type as u8 as i32,
                u32::from(row.extended.required_skill),
                u32::from(row.extended.required_skill_rank),
            )
        }))
    }

    fn spell_scaling_row_count(&self) -> Result<usize, Self::Error> {
        Ok(self.value_game_tables.counts()[0])
    }

    fn spell_scaling_column(&self, level: u8, class: i32) -> Result<Option<f32>, Self::Error> {
        Ok(self
            .value_game_tables
            .scaling(u32::from(level))
            .map(|row| row.for_class(class)))
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/forever/player/enchantment_sources.rs"]
mod tests;
