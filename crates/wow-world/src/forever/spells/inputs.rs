//! Read-only target constructor inputs, borrowed from the canonical raw store.
use super::{EFFECT_SLOTS, Helper, POWER_SLOTS};
use std::collections::BTreeMap;
use wow_data::forever_spells::*;

pub struct SpellInputs<'a> {
    pub(super) catalog: &'a SpellCatalog,
    pub(super) ppm_modifiers_by_rate: &'a BTreeMap<u32, Vec<u32>>,
    pub(super) helper: &'a Helper,
    pub(super) spell: u32,
    pub(super) difficulty: i16,
}

impl<'a> SpellInputs<'a> {
    pub(super) fn ppm_modifiers_for(
        &self,
        rate: u32,
    ) -> impl Iterator<Item = &'a SpellProcsPerMinuteModRecord> + 'a {
        let catalog = self.catalog;
        self.ppm_modifiers_by_rate
            .get(&rate)
            .into_iter()
            .flatten()
            .map(move |&id| {
                catalog
                    .spell_procs_per_minute_mod(id)
                    .expect("selected PPM modifier")
            })
    }
    pub fn spell_id(&self) -> u32 {
        self.spell
    }
    pub fn difficulty(&self) -> i16 {
        self.difficulty
    }
    pub fn name(&self) -> &'a SpellNameRecord {
        self.catalog
            .spell_name(self.spell)
            .expect("named spell helper")
    }
    pub fn aura_options(&self) -> Option<&'a SpellAuraOptionsRecord> {
        self.helper
            .aura_options
            .and_then(|id| self.catalog.spell_aura_options(id))
    }
    pub fn aura_restrictions(&self) -> Option<&'a SpellAuraRestrictionsRecord> {
        self.helper
            .aura_restrictions
            .and_then(|id| self.catalog.spell_aura_restrictions(id))
    }
    pub fn casting_requirements(&self) -> Option<&'a SpellCastingRequirementsRecord> {
        self.helper
            .casting_requirements
            .and_then(|id| self.catalog.spell_casting_requirements(id))
    }
    pub fn categories(&self) -> Option<&'a SpellCategoriesRecord> {
        self.helper
            .categories
            .and_then(|id| self.catalog.spell_categories(id))
    }
    pub fn class_options(&self) -> Option<&'a SpellClassOptionsRecord> {
        self.helper
            .class_options
            .and_then(|id| self.catalog.spell_class_options(id))
    }
    pub fn cooldowns(&self) -> Option<&'a SpellCooldownsRecord> {
        self.helper
            .cooldowns
            .and_then(|id| self.catalog.spell_cooldowns(id))
    }
    pub fn equipped_items(&self) -> Option<&'a SpellEquippedItemsRecord> {
        self.helper
            .equipped_items
            .and_then(|id| self.catalog.spell_equipped_items(id))
    }
    pub fn interrupts(&self) -> Option<&'a SpellInterruptsRecord> {
        self.helper
            .interrupts
            .and_then(|id| self.catalog.spell_interrupts(id))
    }
    pub fn levels(&self) -> Option<&'a SpellLevelsRecord> {
        self.helper
            .levels
            .and_then(|id| self.catalog.spell_levels(id))
    }
    pub fn misc(&self) -> Option<&'a SpellMiscRecord> {
        self.helper.misc.and_then(|id| self.catalog.spell_misc(id))
    }
    pub fn reagents(&self) -> Option<&'a SpellReagentsRecord> {
        self.helper
            .reagents
            .and_then(|id| self.catalog.spell_reagents(id))
    }
    pub fn scaling(&self) -> Option<&'a SpellScalingRecord> {
        self.helper
            .scaling
            .and_then(|id| self.catalog.spell_scaling(id))
    }
    pub fn shapeshift(&self) -> Option<&'a SpellShapeshiftRecord> {
        self.helper
            .shapeshift
            .and_then(|id| self.catalog.spell_shapeshift(id))
    }
    pub fn target_restrictions(&self) -> Option<&'a SpellTargetRestrictionsRecord> {
        self.helper
            .target_restrictions
            .and_then(|id| self.catalog.spell_target_restrictions(id))
    }
    pub fn totems(&self) -> Option<&'a SpellTotemsRecord> {
        self.helper
            .totems
            .and_then(|id| self.catalog.spell_totems(id))
    }
    pub fn effect_slots(&self) -> [Option<&'a SpellEffectRecord>; EFFECT_SLOTS] {
        self.helper
            .effects
            .map(|id| id.and_then(|id| self.catalog.spell_effect(id)))
    }
    pub fn power_slots(&self) -> [Option<&'a SpellPowerRecord>; POWER_SLOTS] {
        self.helper
            .powers
            .map(|id| id.and_then(|id| self.catalog.spell_power(id)))
    }
    pub fn empower_stages(&self) -> impl Iterator<Item = &'a SpellEmpowerStageRecord> + 'a {
        let catalog = self.catalog;
        self.helper.empower_stages.iter().map(move |&id| {
            catalog
                .spell_empower_stage(id)
                .expect("selected empower stage")
        })
    }
    pub fn labels(&self) -> impl Iterator<Item = &'a SpellLabelRecord> + 'a {
        let catalog = self.catalog;
        self.helper
            .labels
            .iter()
            .map(move |&id| catalog.spell_label(id).expect("selected spell label"))
    }
    pub fn reagent_currencies(&self) -> impl Iterator<Item = &'a SpellReagentsCurrencyRecord> + 'a {
        let catalog = self.catalog;
        self.helper.reagent_currencies.iter().map(move |&id| {
            catalog
                .spell_reagents_currency(id)
                .expect("selected reagent currency")
        })
    }
    pub fn visuals(&self) -> impl Iterator<Item = &'a SpellXSpellVisualRecord> + 'a {
        let catalog = self.catalog;
        self.helper.visuals.iter().map(move |&id| {
            catalog
                .spell_x_spell_visual(id)
                .expect("selected spell visual")
        })
    }
}
