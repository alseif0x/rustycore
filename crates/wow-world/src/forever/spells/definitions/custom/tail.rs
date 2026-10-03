//! Source local tail, second primary pass and final liquid-store pass.
use super::*;

pub(super) fn local(
    seeds: &mut SpellDefinitionSeeds,
    key: Key,
    talent: bool,
    _counts: &mut CustomAttributeCounts,
) {
    let definition = seeds.definitions.get_mut(&key).expect("admitted key");
    if talent {
        definition.custom_attributes |= TALENT;
    }
    if fuzzy_ne_zero(definition.fields.width) {
        definition.custom_attributes |= CONE_LINE;
    }
    match definition.fields.spell_family_name {
        4 if definition.fields.spell_family_flags[0] & 0x20000 != 0 => {
            definition.custom_attributes |= AURA_CC
        }
        7 if definition.fields.spell_family_flags[0] & 8 != 0 => {
            definition.custom_attributes |= AURA_CC
        }
        0 if key.0 == 5729 => definition.custom_attributes |= AURA_CC,
        _ => {}
    }
}

// g3dmath.h:133,825-837,858-864: fuzzyNe has ONLY a double overload.
// Width/0.0f promote before epsilon arithmetic, unlike fuzzyEq's float overload.
fn fuzzy_ne_zero(width: f32) -> bool {
    let value = f64::from(width);
    let magnitude = value.abs() + 1.0;
    let epsilon = if magnitude == f64::INFINITY {
        0.0000005
    } else {
        0.0000005 * magnitude
    };
    !(value == 0.0 || value.abs() <= epsilon)
}

pub(super) fn ammo_and_leave_world(
    seeds: &mut SpellDefinitionSeeds,
    key: Key,
    counts: &mut CustomAttributeCounts,
) {
    let definition = &seeds.definitions[&key];
    let needs_ammo = definition.fields.speed > 0.0
        && definition.visuals.iter().any(|id| {
            let Some(relation) = seeds.catalog.spell_x_spell_visual(*id) else {
                return false;
            };
            let Some(visual) = seeds.catalog.spell_visual(relation.spell_visual_id) else {
                return false;
            };
            seeds
                .catalog
                .spell_visual_missiles_for_set(u32::from(visual.spell_visual_missile_set_id))
                .any(|missile| {
                    seeds
                        .catalog
                        .spell_visual_effect_name(u32::from(missile.spell_visual_effect_name_id))
                        .is_some_and(|name| matches!(name.r#type, 6 | 7))
                })
        });
    let leave_world = definition.fields.aura_interrupt_flags[0] & 0x0008_0000 != 0;
    let definition = seeds.definitions.get_mut(&key).expect("admitted key");
    if needs_ammo {
        definition.custom_attributes |= NEEDS_AMMO;
        counts.ammo_assignments += 1;
    }
    if leave_world {
        definition.custom_attributes |= CANNOT_SAVE;
    }
}

pub(super) fn second_pass_and_liquids(
    seeds: &mut SpellDefinitionSeeds,
    counts: &mut CustomAttributeCounts,
) -> Result<(), SpellCustomAttributeError> {
    let length = seeds
        .source_order
        .as_ref()
        .expect("admitted order")
        .primary
        .len();
    for position in 0..length {
        let key = seeds.source_order.as_ref().expect("admitted order").primary[position];
        let definition = &seeds.definitions[&key];
        // Preserve source's unusual NOT-binary condition: this is not a repair
        // to run the branch on binary spells or a propagated binary fixpoint.
        if definition.custom_attributes & BINARY == 0 {
            let mut all_non_binary = true;
            let mut override_attribute = false;
            for effect in &definition.effects {
                if !effect.is_aura()
                    || effect.trigger_spell == 0
                    || !matches!(effect.aura, 23 | 48 | 227)
                {
                    continue;
                }
                if let Some(triggered) = seeds
                    .get(effect.trigger_spell, 0)
                    .map_err(SpellValueError::DefinitionLookup)?
                {
                    override_attribute = true;
                    if triggered.custom_attributes() & BINARY != 0 {
                        all_non_binary = false;
                    }
                }
            }
            if override_attribute && all_non_binary {
                seeds
                    .definitions
                    .get_mut(&key)
                    .expect("admitted key")
                    .custom_attributes &= !BINARY;
            }
        }
        let definition = seeds.definitions.get_mut(&key).expect("admitted key");
        if definition.custom_attributes & CAN_CRIT != 0
            && definition.fields.attributes[2] & 0x2000_0000 != 0
        {
            definition.custom_attributes &= !CAN_CRIT;
            counts.crit_clears += 1;
        }
    }
    for liquid in seeds.catalog.liquid_type_records() {
        if liquid.spell_id == 0 {
            continue;
        }
        let Some(keys) = seeds
            .source_order
            .as_ref()
            .expect("admitted order")
            .by_spell
            .get(&liquid.spell_id)
        else {
            continue;
        };
        for key in keys {
            seeds
                .definitions
                .get_mut(key)
                .expect("admitted foreign key")
                .custom_attributes |= CANNOT_SAVE;
            counts.liquid_assignments += 1;
        }
    }
    Ok(())
}
