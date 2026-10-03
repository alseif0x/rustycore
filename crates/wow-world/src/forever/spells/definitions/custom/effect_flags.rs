//! Source effect-slot traversal, including blank slots and live foreign writes.
use super::*;

pub(super) fn apply(
    seeds: &mut SpellDefinitionSeeds,
    key: Key,
    index: usize,
    counts: &mut CustomAttributeCounts,
) -> Result<(), SpellCustomAttributeError> {
    let definition = &seeds.definitions[&key];
    let effect = &definition.effects[index];
    // Source shifts the parent's mechanic even for a blank effect, but the
    // effect mechanic only if active. Do not silently mask undefined shifts.
    if definition.fields.mechanic >= 64 || (effect.effect != 0 && effect.mechanic >= 64) {
        return Err(SpellCustomAttributeError::UndefinedMechanicShift);
    }
    let mut flags = 0;
    if definition.fields.mechanic == 15 || (effect.effect != 0 && effect.mechanic == 15) {
        flags |= IGNORE_ARMOR;
    }
    if matches!(effect.aura, 2 | 5 | 6 | 177 | 7 | 12) {
        flags |= AURA_CC;
    }
    if matches!(effect.aura, 292 | 236 | 1 | 2 | 378 | 6 | 177 | 398 | 397) {
        flags |= CANNOT_SAVE;
    }
    if matches!(
        effect.effect,
        2 | 9 | 10 | 17 | 31 | 58 | 62 | 75 | 121 | 136 | 165
    ) {
        flags |= CAN_CRIT;
    }
    match effect.effect {
        2 | 58 | 17 | 121 | 31 | 10 => flags |= DIRECT_DAMAGE,
        8 | 62 | 67 | 9 | 136 | 137 | 30 | 75 => flags |= NO_INITIAL_THREAT,
        96 | 149 | 41 | 42 | 138 => flags |= CHARGE,
        71 => flags |= PICKPOCKET,
        _ => {}
    }
    let enchant = matches!(effect.effect, 53 | 54 | 360 | 156 | 92);
    let enchant_id = effect.misc_values[0] as u32;
    seeds
        .definitions
        .get_mut(&key)
        .expect("admitted key")
        .custom_attributes |= flags;
    if !enchant
        || !seeds
            .is_part_of_skill_line(333, key.0)
            .expect("admitted skill map")
    {
        return Ok(());
    }
    for slot in 0..3 {
        let Some(enchantment) = seeds.catalog.spell_item_enchantment(enchant_id) else {
            break;
        };
        let proc_id = enchantment.effect_arg[slot];
        if enchantment.effect[slot] != 1 {
            continue;
        }
        let Some(keys) = seeds
            .source_order
            .as_ref()
            .expect("admitted order")
            .by_spell
            .get(&proc_id)
        else {
            continue;
        };
        for key in keys {
            let definition = seeds
                .definitions
                .get_mut(key)
                .expect("admitted foreign key");
            if definition
                .effects
                .iter()
                .any(|effect| effect.is_aura_kind(42))
            {
                continue;
            }
            definition.custom_attributes |= ENCHANT_PROC;
            counts.enchant_proc_assignments += 1;
        }
    }
    Ok(())
}
