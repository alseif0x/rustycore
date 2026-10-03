//! 02245dcd SpellInfo.cpp:4614-5107, not a sorted/global graph reduction.
//! Called per definition by the admitted ordered custom phase. Recursive
//! queries observe the live canonical negative bits, never a cloned snapshot.
use super::{Key, SpellDefinitionSeeds, SpellEffectValues, SpellValueError};
use crate::forever::spells::TargetCheck;
use std::collections::BTreeSet;
use wow_data::forever_birth::item_records::ItemCatalog;
#[cfg(test)]
mod tests;

type Visited = BTreeSet<(Key, usize)>;

fn positive_target(effect: &SpellEffectValues) -> bool {
    effect.effect == 0
        || effect
            .target_metadata()
            .iter()
            .all(|target| target.check_type() != TargetCheck::Enemy)
}

// Full custom startup owns this call's position and one-shot admission.
pub(super) fn initialize(
    seeds: &mut SpellDefinitionSeeds,
    key: Key,
    items: &ItemCatalog,
    draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
) -> Result<usize, SpellValueError> {
    let count = seeds
        .definitions
        .get(&key)
        .ok_or(SpellValueError::InvalidCatalogInputs)?
        .effects
        .len();
    let mut visited = Visited::new();
    let mut changed = 0;
    for index in 0..count {
        if !positive(seeds, key, index, items, draw, &mut visited)? {
            let negative = &mut seeds
                .definitions
                .get_mut(&key)
                .expect("same authority")
                .negative_effects[index];
            changed += usize::from(!*negative);
            *negative = true;
        }
    }
    // Source checks ONLY later slots, not every negative effect or a fixpoint.
    // It does not require those later slots to be active/aura effects.
    for index in 0..count {
        let definition = &seeds.definitions[&key];
        let effect = &definition.effects[index];
        if effect.effect == 0
            || definition.negative_effects[index]
            || !matches!(effect.aura, 4 | 12 | 7 | 11 | 56 | 9 | 33)
        {
            continue;
        }
        let negative = (index + 1..count).any(|later| {
            definition.negative_effects[later]
                && effect.implicit_targets == definition.effects[later].implicit_targets
        });
        if negative {
            seeds
                .definitions
                .get_mut(&key)
                .expect("same authority")
                .negative_effects[index] = true;
            changed += 1;
        }
    }
    Ok(changed)
}

fn positive(
    seeds: &SpellDefinitionSeeds,
    key: Key,
    index: usize,
    items: &ItemCatalog,
    draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
    visited: &mut Visited,
) -> Result<bool, SpellValueError> {
    let definition = &seeds.definitions[&key];
    let effect = &definition.effects[index];
    if effect.effect == 0 {
        return Ok(true);
    }
    if definition.negative_effects[index] {
        return Ok(false);
    }
    let fields = &definition.fields;
    if fields.attributes[0] & 0x40 != 0 {
        return Ok(true);
    } // passive
    if fields.attributes[0] & 0x0400_0000 != 0 {
        return Ok(false);
    }
    if fields.attributes[4] & 0x1000 != 0 {
        return Ok(true);
    }
    if effect.attributes & 0x1000 != 0 {
        return Ok(false);
    } // IsHarmful
    visited.insert((key, index));
    // This draw precedes even a family/ID, immunity, heal or aura early return.
    let bp = seeds
        .calculate_startup_value(key.0, key.1, index, items, None, draw)?
        .ok_or(SpellValueError::InvalidCatalogInputs)?
        .value;
    match fields.spell_family_name {
        0 => match key.0 {
            40268 | 61987 | 61988 | 64412 | 72410 | 71204 => return Ok(false),
            24732 | 30877 | 61716 | 61734 | 62344 | 50344 | 61819 | 61834 | 73523 => {
                return Ok(true);
            }
            _ => {}
        },
        8 => match key.0 {
            32645 => return Ok(true),
            40251 => return Ok(false),
            _ => {}
        },
        4 if fields.spell_family_flags[0] & 0x2020_0000 != 0 => return Ok(false),
        _ => {}
    }
    if fields.mechanic == 29 {
        return Ok(true);
    }
    if fields.attributes[1] & 0x800 != 0
        && definition
            .effects
            .iter()
            .any(|other| !positive_target(other))
    {
        return Ok(false);
    }
    // Keep the source's per-slot interleaving: a later heal cannot override an
    // earlier same-target instakill or a whole-spell negative aura return.
    for other in &definition.effects {
        match other.effect {
            10 | 36 | 44 | 136 => return Ok(true),
            1 if other.index != effect.index
                && other.implicit_targets == effect.implicit_targets =>
            {
                return Ok(false);
            }
            _ => {}
        }
        if other.is_aura() {
            match other.aura {
                16 | 93 => return Ok(true),
                301 | 121 | 271 | 92 => return Ok(false),
                _ => {}
            }
        }
    }
    let target_positive = positive_target(effect);
    match effect.effect {
        58 | 17 | 121 | 31 | 2 | 7 | 9 | 1 | 8 | 126 | 68 | 71 | 87 | 111 | 115 | 129 | 55 | 69 => {
            return Ok(false);
        }
        30 | 137 | 136 | 67 | 75 => return Ok(true),
        98 | 96 | 27 | 114 | 62 if !target_positive => return Ok(false),
        38 if matches!(effect.misc_values[0], 5 | 6 | 9) || !target_positive => return Ok(false),
        108 if !target_positive && matches!(effect.misc_values[0], 16 | 19 | 21 | 25) => {
            return Ok(false);
        }
        63 | 125 if !target_positive && bp > 0.0 => return Ok(false),
        _ => {}
    }
    if effect.is_aura() {
        match effect.aura {
            29 | 30 | 400 | 49 | 135 | 59 | 20 | 21 | 290 | 54 | 55 | 57 | 140 | 192 | 65 | 216
            | 143 | 91 | 133 | 137 | 58 | 80 | 34 | 129
                if bp < 0.0 || effect.real_points_per_level < 0.0 =>
            {
                return Ok(false);
            }
            9 | 138 | 13 | 22 | 101 | 189 | 99 | 124 | 79 | 252 | 193 | 166 | 136 | 118
                if !target_positive || bp < 0.0 =>
            {
                return Ok(false);
            }
            14 | 125 | 126 | 73 | 72 | 255 if bp > 0.0 => return Ok(false),
            87 if !target_positive && bp > 0.0 => return Ok(false),
            88 if !target_positive && bp < 0.0 => return Ok(false),
            109 => return Ok(true),
            227 | 48 => {
                if !triggered_positive(
                    seeds,
                    key.1,
                    effect.trigger_spell,
                    true,
                    items,
                    draw,
                    visited,
                )? {
                    return Ok(false);
                }
            }
            23 | 12 | 56 | 33 | 7 | 11 | 25 | 60 | 67 | 254 | 278 | 6 | 177 | 2 | 75 | 15 | 42
            | 184 | 185 | 186 | 187 | 197 | 4 | 226 | 115 | 52 | 162 | 196 | 205 | 31 | 47
            | 296 | 24 | 37 | 112 | 36 | 10 | 231
                if !target_positive =>
            {
                return Ok(false);
            }
            5 | 86 | 26 | 455 | 27 | 221 | 95 | 53 | 64 | 68 | 314 | 3 | 70 | 89 | 165 | 127 => {
                return Ok(false);
            }
            77 if matches!(effect.misc_values[0], 16 | 19 | 21 | 25) => return Ok(false),
            107 | 108 | 219 | 218 => {
                let spell_positive = definition.negative_effects.iter().all(|negative| !negative);
                // Source SpellModOp is uint8, including unknown/negative misc bits.
                match effect.misc_values[0] as u8 {
                    10 | 19 | 30 | 21 if bp > 0.0 => return Ok(false),
                    11 | 14 | 34 | 39 if !spell_positive && bp > 0.0 => return Ok(false),
                    3 | 12 | 23 | 32 | 33 | 8 | 2 | 20 | 27 => return Ok(true),
                    1 | 7 | 0 | 17 if !spell_positive && bp < 0.0 => return Ok(false),
                    // Matched ops with false conditions must NOT hit default.
                    10 | 19 | 30 | 21 | 11 | 14 | 34 | 39 | 1 | 7 | 0 | 17 => {}
                    _ if bp < 0.0 => return Ok(false),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if effect.aura == 0
        && effect.trigger_spell != 0
        && !triggered_positive(
            seeds,
            key.1,
            effect.trigger_spell,
            false,
            items,
            draw,
            visited,
        )?
    {
        return Ok(false);
    }
    Ok(true)
}

fn triggered_positive(
    seeds: &SpellDefinitionSeeds,
    difficulty: i16,
    spell: u32,
    only_positive_targets: bool,
    items: &ItemCatalog,
    draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
    visited: &mut Visited,
) -> Result<bool, SpellValueError> {
    let Some(triggered) = seeds
        .get(spell, difficulty)
        .map_err(SpellValueError::DefinitionLookup)?
    else {
        return Ok(true);
    };
    for (index, effect) in triggered.definition.effects.iter().enumerate() {
        if visited.contains(&(triggered.key, index))
            || effect.effect == 0
            || (only_positive_targets && !positive_target(effect))
        {
            continue;
        }
        if !positive(seeds, triggered.key, index, items, draw, visited)? {
            return Ok(false);
        }
    }
    Ok(true)
}
