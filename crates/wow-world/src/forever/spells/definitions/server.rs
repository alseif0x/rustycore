//! SpellMgr.cpp:2749-2993: effects before main rows, internal-only namespace.
use super::{Definition, Key, ServerSpellCounts, SpellDefinitionError, SpellEffectValues};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use wow_data::forever_spells::SpellCatalog;
use wow_persistence::forever::spells::server::{
    ServerSpellEffectRow, ServerSpellRow, ServerSpellRows,
};

pub(super) fn load(
    catalog: &SpellCatalog,
    definitions: &mut BTreeMap<Key, Definition>,
    rows: ServerSpellRows,
    requests: &mut Vec<Key>,
) -> Result<ServerSpellCounts, SpellDefinitionError> {
    let mut counts = ServerSpellCounts {
        input_spells: rows.spells.len(),
        input_effects: rows.effects.len(),
        ..Default::default()
    };
    let regular_spells: BTreeSet<u32> = definitions.keys().map(|key| key.0).collect();
    let mut effects = BTreeMap::<Key, Vec<SpellEffectValues>>::new();
    for row in rows.effects {
        // _GetSpellInfo checks any constructed regular difficulty, not just
        // DIFFICULTY_NONE and not name-only raw SpellName identities.
        if regular_spells.contains(&row.spell_id) {
            counts.skipped_regular_effects += 1;
            continue;
        }
        // Actual SQL is int32; source UInt32 -> enum Difficulty:int16 keeps
        // the same low 16 bits, including negative signed keys.
        let difficulty = row.difficulty_id as i16;
        if difficulty != 0 && catalog.difficulty(difficulty as i32 as u32).is_none() {
            counts.skipped_missing_difficulty_effects += 1;
            continue;
        }
        // These are the source's upper-bound skip conditions in source order.
        if row.effect_index >= 32
            || row.effect as u32 >= 361
            || row.effect_aura >= 665
            || row.implicit_target.iter().any(|&target| target >= 153)
        {
            counts.skipped_invalid_effects += 1;
            continue;
        }
        // Only otherwise-admitted rows would reach source-undefined indexing.
        if row.effect_index < 0 {
            return Err(SpellDefinitionError::EffectIndex);
        }
        if row.effect_aura < 0 {
            return Err(SpellDefinitionError::NegativeAura);
        }
        if row.implicit_target.iter().any(|&target| target < 0) {
            return Err(SpellDefinitionError::NegativeImplicitTarget);
        }
        counts.missing_radius_warnings += row
            .effect_radius_index
            .iter()
            .filter(|&&id| id != 0 && catalog.spell_radius(id).is_none())
            .count();
        // The source logs missing nonzero radii but does not rewrite them.
        // Constructor pointer resolution gives None, never an invented ID zero.
        effects
            .entry((row.spell_id, difficulty))
            .or_default()
            .push(effect(catalog, &row));
    }
    for mut row in rows.spells {
        // Raw SpellName presence bans server overriding, including name-only
        // records that did not construct a regular SpellInfo.
        if catalog.spell_name(row.id).is_some() {
            counts.rejected_client_names += 1;
            continue;
        }
        let key = (row.id, row.difficulty_id as i16);
        // Replay every admitted emplace request, including duplicate keys.
        requests.push(key);
        // Main server rows have no Difficulty-presence check in this source.
        let definition = match definitions.entry(key) {
            Entry::Vacant(entry) => {
                counts.definitions_added += 1;
                let mut definition = Definition::empty_server(std::mem::take(&mut row.spell_name));
                definition.effects = assemble_effects(effects.remove(&key).unwrap_or_default());
                entry.insert(definition)
            }
            Entry::Occupied(entry) => {
                // hashed_unique::emplace keeps the original object/name/effects;
                // the source then reapplies every selected scalar/link field.
                counts.duplicate_spell_rows += 1;
                entry.into_mut()
            }
        };
        apply_fields(catalog, definition, &row);
    }
    counts.orphan_effect_groups = effects.len();
    Ok(counts)
}

fn assemble_effects(rows: Vec<SpellEffectValues>) -> Vec<SpellEffectValues> {
    let mut effects = Vec::new();
    for row in rows {
        let index = row.index as usize;
        if effects.len() <= index {
            effects.resize_with(index + 1, SpellEffectValues::default);
        }
        effects[index] = row;
    }
    for (index, effect) in effects.iter_mut().enumerate() {
        effect.index = index as u32;
    }
    effects
}

fn effect(catalog: &SpellCatalog, row: &ServerSpellEffectRow) -> SpellEffectValues {
    SpellEffectValues {
        index: row.effect_index as u32,
        effect: row.effect as u32,
        aura: row.effect_aura as u32,
        aura_period: row.effect_aura_period as u32,
        base_points: row.effect_base_points,
        real_points_per_level: row.effect_real_points_per_level,
        points_per_resource: row.effect_points_per_resource,
        amplitude: row.effect_amplitude,
        chain_amplitude: row.effect_chain_amplitude,
        bonus_coefficient: row.effect_bonus_coefficient,
        misc_values: row.effect_misc_value,
        mechanic: row.effect_mechanic as u32,
        position_facing: row.effect_pos_facing,
        implicit_targets: row.implicit_target.map(|target| target as u32),
        chain_targets: row.effect_chain_targets,
        item_type: row.effect_item_type as u32,
        trigger_spell: row.effect_trigger_spell as u32,
        class_mask: row.effect_spell_class_mask.map(|word| word as u32),
        bonus_coefficient_from_ap: row.bonus_coefficient_from_ap,
        scaling_class: 0,
        scaling_coefficient: row.coefficient,
        scaling_variance: row.variance,
        scaling_resource_coefficient: row.resource_coefficient,
        attributes: row.effect_attributes as u32,
        radius_ids: row
            .effect_radius_index
            .map(|id| catalog.spell_radius(id).map(|row| row.id)),
    }
}

fn apply_fields(catalog: &SpellCatalog, definition: &mut Definition, row: &ServerSpellRow) {
    definition.fields.attributes = row.attributes;
    definition.fields.speed = row.speed;
    definition.fields.launch_delay = row.launch_delay;
    definition.fields.school_mask = row.school_mask;
    definition.fields.content_tuning_id = row.content_tuning_id;
    definition.fields.proc_flags = row.proc_flags;
    definition.fields.proc_chance = row.proc_chance;
    definition.fields.proc_charges = row.proc_charges;
    definition.fields.proc_cooldown = row.proc_cooldown;
    definition.fields.stack_amount = row.stack_amount;
    definition.fields.caster_aura_state = row.caster_aura_state;
    definition.fields.target_aura_state = row.target_aura_state;
    definition.fields.exclude_caster_aura_state = row.exclude_caster_aura_state;
    definition.fields.exclude_target_aura_state = row.exclude_target_aura_state;
    definition.fields.caster_aura_spell = row.caster_aura_spell;
    definition.fields.target_aura_spell = row.target_aura_spell;
    definition.fields.exclude_caster_aura_spell = row.exclude_caster_aura_spell;
    definition.fields.exclude_target_aura_spell = row.exclude_target_aura_spell;
    definition.fields.caster_aura_type = row.caster_aura_type as u32;
    definition.fields.target_aura_type = row.target_aura_type as u32;
    definition.fields.exclude_caster_aura_type = row.exclude_caster_aura_type as u32;
    definition.fields.exclude_target_aura_type = row.exclude_target_aura_type as u32;
    definition.fields.requires_spell_focus = row.requires_spell_focus;
    definition.fields.facing_caster_flags = row.facing_caster_flags;
    definition.fields.required_areas_id = row.area_group_id;
    definition.fields.category_id = row.category_id;
    definition.fields.dispel = row.dispel;
    definition.fields.mechanic = row.mechanic;
    definition.fields.start_recovery_category = row.start_recovery_category;
    definition.fields.damage_class = row.dmg_class;
    definition.fields.prevention_type = row.prevention_type;
    definition.fields.charge_category_id = row.charge_category_id;
    definition.fields.spell_family_name = row.spell_family_name;
    definition.fields.spell_family_flags = row.spell_family_flags;
    definition.fields.recovery_time = row.recovery_time;
    definition.fields.category_recovery_time = row.category_recovery_time;
    definition.fields.start_recovery_time = row.start_recovery_time;
    definition.fields.equipped_item_class = row.equipped_item_class;
    definition.fields.equipped_item_subclass_mask = row.equipped_item_sub_class_mask;
    definition.fields.equipped_item_inventory_type_mask = row.equipped_item_inventory_type_mask;
    definition.fields.interrupt_flags = row.interrupt_flags;
    definition.fields.aura_interrupt_flags = row.aura_interrupt_flags;
    definition.fields.channel_interrupt_flags = row.channel_interrupt_flags;
    definition.fields.max_level = row.max_level;
    definition.fields.base_level = row.base_level;
    definition.fields.spell_level = row.spell_level;
    definition.fields.stances = row.stances;
    definition.fields.stances_not = row.stances_not;
    definition.fields.cone_angle = row.cone_angle;
    definition.fields.width = row.cone_width;
    definition.fields.targets = row.targets;
    definition.fields.target_creature_type = row.target_creature_type;
    definition.fields.max_affected_targets = row.max_affected_targets;
    definition.fields.max_target_level = row.max_target_level;
    definition.fields.proc_base_ppm = row.proc_base_ppm;
    definition.cast_time = catalog
        .spell_cast_times(row.casting_time_index)
        .map(|row| row.id);
    definition.duration = catalog.spell_duration(row.duration_index).map(|row| row.id);
    definition.range = catalog.spell_range(row.range_index).map(|row| row.id);
}
