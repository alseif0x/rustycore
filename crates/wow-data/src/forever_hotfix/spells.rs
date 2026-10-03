//! 49 target spell serializers; one shared immutable effective record owner.
//! 02245dcd DB2StorageBase::WriteRecord: external ID excluded, full metadata
//! arrays/parents and inline IDs included; selected locale without fallback.
mod core;
mod costs;
mod custom_sources;
mod dependencies;
#[cfg(test)]
mod tests;
mod value_inputs;
use crate::forever_spells::{SPELL_TABLE_HASHES, SpellCatalog, SpellText};
use std::sync::Arc;

pub(super) struct SpellHotfixStores {
    pub data: Arc<SpellCatalog>,
}
impl SpellHotfixStores {
    pub(super) fn record(&self, hash: u32, id: u32, locale: u8) -> Option<Vec<u8>> {
        Some(
            match SPELL_TABLE_HASHES.iter().position(|&known| known == hash)? {
                0 => core::spell_name(self.data.spell_name(id)?, locale),
                1 => core::spell_effect(self.data.spell_effect(id)?, locale),
                2 => core::spell_misc(self.data.spell_misc(id)?, locale),
                3 => core::spell_aura_options(self.data.spell_aura_options(id)?, locale),
                4 => core::spell_aura_restrictions(self.data.spell_aura_restrictions(id)?, locale),
                5 => core::spell_casting_requirements(
                    self.data.spell_casting_requirements(id)?,
                    locale,
                ),
                6 => core::spell_categories(self.data.spell_categories(id)?, locale),
                7 => core::spell_class_options(self.data.spell_class_options(id)?, locale),
                8 => core::spell_cooldowns(self.data.spell_cooldowns(id)?, locale),
                9 => costs::spell_empower(self.data.spell_empower(id)?, locale),
                10 => costs::spell_empower_stage(self.data.spell_empower_stage(id)?, locale),
                11 => core::spell_equipped_items(self.data.spell_equipped_items(id)?, locale),
                12 => core::spell_interrupts(self.data.spell_interrupts(id)?, locale),
                13 => core::spell_label(self.data.spell_label(id)?, locale),
                14 => core::spell_levels(self.data.spell_levels(id)?, locale),
                15 => costs::spell_power(self.data.spell_power(id)?, locale),
                16 => costs::spell_power_difficulty(self.data.spell_power_difficulty(id)?, locale),
                17 => costs::spell_reagents(self.data.spell_reagents(id)?, locale),
                18 => {
                    costs::spell_reagents_currency(self.data.spell_reagents_currency(id)?, locale)
                }
                19 => costs::spell_scaling(self.data.spell_scaling(id)?, locale),
                20 => costs::spell_shapeshift(self.data.spell_shapeshift(id)?, locale),
                21 => costs::spell_target_restrictions(
                    self.data.spell_target_restrictions(id)?,
                    locale,
                ),
                22 => costs::spell_totems(self.data.spell_totems(id)?, locale),
                23 => costs::spell_x_spell_visual(self.data.spell_x_spell_visual(id)?, locale),
                24 => dependencies::difficulty(self.data.difficulty(id)?, locale),
                25 => dependencies::spell_cast_times(self.data.spell_cast_times(id)?, locale),
                26 => dependencies::spell_duration(self.data.spell_duration(id)?, locale),
                27 => dependencies::spell_range(self.data.spell_range(id)?, locale),
                28 => dependencies::spell_radius(self.data.spell_radius(id)?, locale),
                29 => dependencies::spell_procs_per_minute(
                    self.data.spell_procs_per_minute(id)?,
                    locale,
                ),
                30 => dependencies::spell_procs_per_minute_mod(
                    self.data.spell_procs_per_minute_mod(id)?,
                    locale,
                ),
                31 => dependencies::spell_learn_spell(self.data.spell_learn_spell(id)?, locale),
                32 => dependencies::spell_shapeshift_form(
                    self.data.spell_shapeshift_form(id)?,
                    locale,
                ),
                33 => dependencies::summon_properties(self.data.summon_properties(id)?, locale),
                34 => dependencies::battle_pet_species(self.data.battle_pet_species(id)?, locale),
                35 => dependencies::spell_category(self.data.spell_category(id)?, locale),
                36 => custom_sources::talent(self.data.talent(id)?, locale),
                37 => custom_sources::spell_item_enchantment(
                    self.data.spell_item_enchantment(id)?,
                    locale,
                ),
                38 => custom_sources::spell_visual(self.data.spell_visual(id)?, locale),
                39 => custom_sources::spell_visual_missile(
                    self.data.spell_visual_missile(id)?,
                    locale,
                ),
                40 => custom_sources::spell_visual_effect_name(
                    self.data.spell_visual_effect_name(id)?,
                    locale,
                ),
                41 => custom_sources::liquid_type(self.data.liquid_type(id)?, locale),
                42 => value_inputs::expected_stat(self.data.expected_stat(id)?, locale),
                43 => value_inputs::expected_stat_mod(self.data.expected_stat_mod(id)?, locale),
                44 => value_inputs::content_tuning(self.data.content_tuning(id)?, locale),
                45 => value_inputs::content_tuning_x_expected(
                    self.data.content_tuning_x_expected(id)?,
                    locale,
                ),
                46 => value_inputs::rand_prop_points(self.data.rand_prop_points(id)?, locale),
                47 => value_inputs::mythic_plus_season(self.data.mythic_plus_season(id)?, locale),
                48 => custom_sources::unit_condition(self.data.unit_condition(id)?),
                _ => unreachable!(),
            },
        )
    }
}
fn text(bytes: &mut Vec<u8>, value: &SpellText, locale: u8) {
    // Common.h::LocalizedString::operator[] returns exactly Str[locale].
    // Missing slots are source nullStr, not an implicit enUS fallback.
    let selected = value.at(locale).unwrap_or(&[]);
    let end = selected
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(selected.len());
    bytes.extend_from_slice(&selected[..end]);
    bytes.push(0);
}
