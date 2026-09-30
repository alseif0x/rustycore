//! Mechanic masks from canonical applications and difficulty-selected metadata.

use super::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem};
use std::collections::{BTreeMap, HashMap};

impl AuraSubsystem {
    /// C++ Unit::HasAuraWithMechanic (Unit.cpp:4714-4729). Each Player
    /// application supplies its own difficulty.
    pub fn application_mechanic_mask<'catalog, E: 'catalog>(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        mut metadata: impl FnMut(i32, u8) -> Option<(i32, BTreeMap<u32, i32>)>,
        mut select: impl FnMut(i32, u8) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
    ) -> u64 {
        auras.values().fold(0_u64, |mask, aura| {
            mask | spell_mechanic_mask(
                aura.spell_id,
                aura.difficulty_id,
                aura.effect_mask,
                &mut metadata,
                &mut select,
                &fields,
            )
        })
    }

    /// Creature refs retain no difficulty; the canonical caller supplies it.
    pub fn applied_mechanic_mask<'catalog, E: 'catalog>(
        applied_auras: &[AppliedAuraRef],
        difficulty_id: u8,
        mut metadata: impl FnMut(i32, u8) -> Option<(i32, BTreeMap<u32, i32>)>,
        mut select: impl FnMut(i32, u8) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
    ) -> u64 {
        applied_auras.iter().fold(0_u64, |mask, aura| {
            mask | spell_mechanic_mask(
                i32::try_from(aura.spell_id).unwrap_or(0),
                difficulty_id,
                aura.effect_mask,
                &mut metadata,
                &mut select,
                &fields,
            )
        })
    }
}

/// The spell's own mechanic precedes row selection. The established exclusion
/// of effect index zero remains a known gap from the C++ loop.
fn spell_mechanic_mask<'catalog, E: 'catalog>(
    spell_id: i32,
    difficulty_id: u8,
    effect_mask: u32,
    metadata: &mut impl FnMut(i32, u8) -> Option<(i32, BTreeMap<u32, i32>)>,
    select: &mut impl FnMut(i32, u8) -> Option<&'catalog [E]>,
    fields: &impl Fn(&E) -> (u32, u32, i32, i32, i32),
) -> u64 {
    let Some((spell_mechanic, effect_mechanics)) = metadata(spell_id, difficulty_id) else {
        return 0;
    };
    let mut mask = mechanic_bit(spell_mechanic).unwrap_or(0);
    let effects = select(spell_id, difficulty_id);
    for (effect_index, mechanic) in effect_mechanics {
        if !(1..32).contains(&effect_index) || effect_mask & (1_u32 << effect_index) == 0 {
            continue;
        }
        let is_effect = effects.is_some_and(|effects| {
            effects
                .iter()
                .any(|effect| fields(effect).0 == effect_index && fields(effect).1 != 0)
        });
        if is_effect {
            mask |= mechanic_bit(mechanic).unwrap_or(0);
        }
    }
    mask
}

/// C++ UI64LIT(1) << mechanic for positive, in-range mechanics.
fn mechanic_bit(mechanic: i32) -> Option<u64> {
    (1..64).contains(&mechanic).then(|| 1_u64 << mechanic)
}
