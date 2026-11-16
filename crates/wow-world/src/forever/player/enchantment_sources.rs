use std::convert::Infallible;
use wow_data::forever_birth::item_specs::ItemSpecCatalog;
use wow_data::forever_game_tables::SpellValueGameTables;
use wow_entities::{
    EnchantmentConditionFacts, EnchantmentConditionSource, SkillEnchantmentItemTemplate,
    SkillEnchantmentRecord, SkillEnchantmentSource,
};

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

/// Borrowed current catalogs used by native item-gem condition evaluation.
/// Absence is scoped to these admitted views; upstream unknown accounting
/// remains owned by the existing catalog admission path.
pub struct EnchantmentConditionSources<'a> {
    definitions: &'a SpellDefinitionSeeds,
    item_templates: &'a NumericItemTemplates,
    item_specs: &'a ItemSpecCatalog,
}

impl<'a> EnchantmentConditionSources<'a> {
    pub fn new(
        definitions: &'a SpellDefinitionSeeds,
        item_templates: &'a NumericItemTemplates,
        item_specs: &'a ItemSpecCatalog,
    ) -> Self {
        Self {
            definitions,
            item_templates,
            item_specs,
        }
    }
}

impl EnchantmentConditionSource for EnchantmentConditionSources<'_> {
    type Error = Infallible;

    fn condition(&self, id: u32) -> Result<Option<EnchantmentConditionFacts>, Self::Error> {
        Ok(self
            .definitions
            .spell_item_enchantment_condition_record(id)
            .map(|row| {
                EnchantmentConditionFacts::new(
                    row.id,
                    row.lt_operand_type,
                    row.operator,
                    row.rt_operand_type,
                    row.rt_operand,
                )
            }))
    }

    fn gem_color_mask(&self, item_entry: u32) -> Result<Option<u32>, Self::Error> {
        let Some(template) = self.item_templates.template(item_entry) else {
            return Ok(None);
        };
        let Some(gem) = self
            .item_specs
            .gem(u32::from(template.extended.gem_properties))
        else {
            return Ok(None);
        };
        Ok(Some(gem.kind as u32))
    }
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

#[cfg(test)]
#[path = "../../../unit_tests/forever/player/enchantment_condition_sources.rs"]
mod condition_tests;
