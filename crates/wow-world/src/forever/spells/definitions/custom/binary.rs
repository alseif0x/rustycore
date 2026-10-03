//! Source's first binary pass, before school changes and positivity/draws.
use super::*;

pub(super) fn initialize(
    seeds: &mut SpellDefinitionSeeds,
    key: Key,
    items: &ItemCatalog,
    draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
    counts: &mut CustomAttributeCounts,
) -> Result<(), SpellCustomAttributeError> {
    if seeds.definitions[&key].fields.attributes[3] & 0x0004_0000 != 0 {
        return Ok(());
    }
    for index in 0..seeds.definitions[&key].effects.len() {
        let definition = &seeds.definitions[&key];
        let effect = &definition.effects[index];
        if effect.effect == 0 || matches!(effect.effect, 2 | 58 | 17 | 121 | 31 | 64 | 142) {
            continue;
        }
        if matches!(
            effect.effect,
            27 | 6 | 35 | 65 | 128 | 129 | 119 | 143 | 174 | 202 | 271
        ) && matches!(effect.aura, 3 | 89 | 4 | 53 | 62 | 226)
        {
            continue;
        }
        // Always calculate/cast BEFORE CC qualification and exception IDs.
        let amount = seeds
            .calculate_startup_value(key.0, key.1, index, items, None, draw)?
            .ok_or(SpellValueError::InvalidCatalogInputs)?
            .as_int()?;
        let zero_control = (effect.effect == 68 || definition.custom_attributes & AURA_CC != 0)
            && definition.fields.attributes[0] & 0x2000_0000 == 0;
        if amount == 0 && !zero_control {
            continue;
        }
        if matches!(
            key.0,
            69649 | 71056 | 71057 | 71058 | 73061 | 73062 | 73063 | 73064 | 55095
        ) || (definition.fields.spell_family_name == 3
            && definition.fields.spell_family_flags[0] & 0x20 != 0)
            || (definition.fields.spell_family_name == 5
                && definition.fields.spell_family_flags[1] & 0x40000 != 0)
        {
            continue;
        }
        seeds
            .definitions
            .get_mut(&key)
            .expect("admitted key")
            .custom_attributes |= BINARY;
        counts.binary_assignments += 1;
        break;
    }
    Ok(())
}
