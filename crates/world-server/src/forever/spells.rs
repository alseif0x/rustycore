//! Composition-only spell SQL DTO conversion; no learned Player state.
mod core;
mod costs;
mod custom_sources;
mod dependencies;
mod locales;
#[cfg(test)]
mod tests;
mod value_inputs;
use anyhow::Result;
use wow_data::forever_spells::{SpellLocaleRecords, SpellRecords};
use wow_persistence::forever::spells::{SpellLocaleRows, SpellRows};

pub(super) fn records(rows: SpellRows) -> Result<SpellRecords> {
    Ok(SpellRecords {
        spell_names: rows
            .spell_names
            .into_iter()
            .map(core::spell_name)
            .collect::<Result<Vec<_>>>()?,
        spell_effects: rows
            .spell_effects
            .into_iter()
            .map(core::spell_effect)
            .collect::<Result<Vec<_>>>()?,
        spell_misc: rows
            .spell_misc
            .into_iter()
            .map(core::spell_misc)
            .collect::<Result<Vec<_>>>()?,
        spell_aura_options: rows
            .spell_aura_options
            .into_iter()
            .map(core::spell_aura_options)
            .collect::<Result<Vec<_>>>()?,
        spell_aura_restrictions: rows
            .spell_aura_restrictions
            .into_iter()
            .map(core::spell_aura_restrictions)
            .collect::<Result<Vec<_>>>()?,
        spell_casting_requirements: rows
            .spell_casting_requirements
            .into_iter()
            .map(core::spell_casting_requirements)
            .collect::<Result<Vec<_>>>()?,
        spell_categories: rows
            .spell_categories
            .into_iter()
            .map(core::spell_categories)
            .collect::<Result<Vec<_>>>()?,
        spell_class_options: rows
            .spell_class_options
            .into_iter()
            .map(core::spell_class_options)
            .collect::<Result<Vec<_>>>()?,
        spell_cooldowns: rows
            .spell_cooldowns
            .into_iter()
            .map(core::spell_cooldowns)
            .collect::<Result<Vec<_>>>()?,
        spell_empowers: rows
            .spell_empowers
            .into_iter()
            .map(costs::spell_empower)
            .collect::<Result<Vec<_>>>()?,
        spell_empower_stages: rows
            .spell_empower_stages
            .into_iter()
            .map(costs::spell_empower_stage)
            .collect::<Result<Vec<_>>>()?,
        spell_equipped_items: rows
            .spell_equipped_items
            .into_iter()
            .map(core::spell_equipped_items)
            .collect::<Result<Vec<_>>>()?,
        spell_interrupts: rows
            .spell_interrupts
            .into_iter()
            .map(core::spell_interrupts)
            .collect::<Result<Vec<_>>>()?,
        spell_labels: rows
            .spell_labels
            .into_iter()
            .map(core::spell_label)
            .collect::<Result<Vec<_>>>()?,
        spell_levels: rows
            .spell_levels
            .into_iter()
            .map(core::spell_levels)
            .collect::<Result<Vec<_>>>()?,
        spell_powers: rows
            .spell_powers
            .into_iter()
            .map(costs::spell_power)
            .collect::<Result<Vec<_>>>()?,
        spell_power_difficulties: rows
            .spell_power_difficulties
            .into_iter()
            .map(costs::spell_power_difficulty)
            .collect::<Result<Vec<_>>>()?,
        spell_reagents: rows
            .spell_reagents
            .into_iter()
            .map(costs::spell_reagents)
            .collect::<Result<Vec<_>>>()?,
        spell_reagents_currencies: rows
            .spell_reagents_currencies
            .into_iter()
            .map(costs::spell_reagents_currency)
            .collect::<Result<Vec<_>>>()?,
        spell_scaling: rows
            .spell_scaling
            .into_iter()
            .map(costs::spell_scaling)
            .collect::<Result<Vec<_>>>()?,
        spell_shapeshifts: rows
            .spell_shapeshifts
            .into_iter()
            .map(costs::spell_shapeshift)
            .collect::<Result<Vec<_>>>()?,
        spell_target_restrictions: rows
            .spell_target_restrictions
            .into_iter()
            .map(costs::spell_target_restrictions)
            .collect::<Result<Vec<_>>>()?,
        spell_totems: rows
            .spell_totems
            .into_iter()
            .map(costs::spell_totems)
            .collect::<Result<Vec<_>>>()?,
        spell_x_spell_visuals: rows
            .spell_x_spell_visuals
            .into_iter()
            .map(costs::spell_x_spell_visual)
            .collect::<Result<Vec<_>>>()?,
        difficulties: rows
            .difficulties
            .into_iter()
            .map(dependencies::difficulty)
            .collect::<Result<Vec<_>>>()?,
        spell_cast_times: rows
            .spell_cast_times
            .into_iter()
            .map(dependencies::spell_cast_times)
            .collect::<Result<Vec<_>>>()?,
        spell_durations: rows
            .spell_durations
            .into_iter()
            .map(dependencies::spell_duration)
            .collect::<Result<Vec<_>>>()?,
        spell_ranges: rows
            .spell_ranges
            .into_iter()
            .map(dependencies::spell_range)
            .collect::<Result<Vec<_>>>()?,
        spell_radii: rows
            .spell_radii
            .into_iter()
            .map(dependencies::spell_radius)
            .collect::<Result<Vec<_>>>()?,
        spell_procs_per_minute: rows
            .spell_procs_per_minute
            .into_iter()
            .map(dependencies::spell_procs_per_minute)
            .collect::<Result<Vec<_>>>()?,
        spell_procs_per_minute_mods: rows
            .spell_procs_per_minute_mods
            .into_iter()
            .map(dependencies::spell_procs_per_minute_mod)
            .collect::<Result<Vec<_>>>()?,
        spell_learn_spells: rows
            .spell_learn_spells
            .into_iter()
            .map(dependencies::spell_learn_spell)
            .collect::<Result<Vec<_>>>()?,
        spell_shapeshift_forms: rows
            .spell_shapeshift_forms
            .into_iter()
            .map(dependencies::spell_shapeshift_form)
            .collect::<Result<Vec<_>>>()?,
        summon_properties: rows
            .summon_properties
            .into_iter()
            .map(dependencies::summon_properties)
            .collect::<Result<Vec<_>>>()?,
        battle_pet_species: rows
            .battle_pet_species
            .into_iter()
            .map(dependencies::battle_pet_species)
            .collect::<Result<Vec<_>>>()?,
        spell_category_definitions: rows
            .spell_category_definitions
            .into_iter()
            .map(dependencies::spell_category)
            .collect::<Result<Vec<_>>>()?,
        talents: rows
            .talents
            .into_iter()
            .map(custom_sources::talent)
            .collect::<Result<Vec<_>>>()?,
        spell_item_enchantments: rows
            .spell_item_enchantments
            .into_iter()
            .map(custom_sources::spell_item_enchantment)
            .collect::<Result<Vec<_>>>()?,
        spell_visuals: rows
            .spell_visuals
            .into_iter()
            .map(custom_sources::spell_visual)
            .collect::<Result<Vec<_>>>()?,
        spell_visual_missiles: rows
            .spell_visual_missiles
            .into_iter()
            .map(custom_sources::spell_visual_missile)
            .collect::<Result<Vec<_>>>()?,
        spell_visual_effect_names: rows
            .spell_visual_effect_names
            .into_iter()
            .map(custom_sources::spell_visual_effect_name)
            .collect::<Result<Vec<_>>>()?,
        liquid_types: rows
            .liquid_types
            .into_iter()
            .map(custom_sources::liquid_type)
            .collect::<Result<Vec<_>>>()?,
        expected_stats: rows
            .expected_stats
            .into_iter()
            .map(value_inputs::expected_stat)
            .collect::<Result<Vec<_>>>()?,
        expected_stat_mods: rows
            .expected_stat_mods
            .into_iter()
            .map(value_inputs::expected_stat_mod)
            .collect::<Result<Vec<_>>>()?,
        content_tunings: rows
            .content_tunings
            .into_iter()
            .map(value_inputs::content_tuning)
            .collect::<Result<Vec<_>>>()?,
        content_tuning_x_expected: rows
            .content_tuning_x_expected
            .into_iter()
            .map(value_inputs::content_tuning_x_expected)
            .collect::<Result<Vec<_>>>()?,
        rand_prop_points: rows
            .rand_prop_points
            .into_iter()
            .map(value_inputs::rand_prop_points)
            .collect::<Result<Vec<_>>>()?,
        mythic_plus_seasons: rows
            .mythic_plus_seasons
            .into_iter()
            .map(value_inputs::mythic_plus_season)
            .collect::<Result<Vec<_>>>()?,
        unit_conditions: rows
            .unit_conditions
            .into_iter()
            .map(custom_sources::unit_condition)
            .collect::<Result<Vec<_>>>()?,
        unknown_baseline_records: [0; 49],
        rand_prop_points_storage_last_index: None,
        rand_prop_points_sql_index_size: rows.rand_prop_points_sql_index_size,
    })
}
pub(super) fn locale_records(rows: SpellLocaleRows) -> SpellLocaleRecords {
    locales::records(rows)
}
