//! Pure fresh-initialization result for current owned fields, not phase admission.
//! SpellInfo.cpp:4576-4613 / DBCEnums.h:2438 / SharedDefines.h:932 (02245dcd).
#[cfg(test)]
mod tests;
use super::{Definition, SpellDefinitionView};
use crate::forever::spells::{
    EffectTargetType,
    effect_targets::{CORPSE_MASK, UNIT_MASK},
};
use wow_data::forever_spells::SpellCatalog;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExplicitTargetMasks {
    pub available: u32,
    pub required: u32,
}
impl SpellDefinitionView<'_> {
    /// Derive the source constructor-to-initialization result from this current
    /// readonly definition. Does NOT cache/mark complete custom attributes,
    /// positivity, immunity, initialized SpellInfo or Player admission.
    pub fn derive_explicit_target_masks(&self) -> ExplicitTargetMasks {
        derive(self.catalog, self.definition)
    }
}

pub(super) fn derive(catalog: &SpellCatalog, spell: &Definition) -> ExplicitTargetMasks {
    let mut result = ExplicitTargetMasks::default();
    let (mut source_set, mut destination_set) = (false, false);
    let zero_max_range = spell
        .range
        .and_then(|id| catalog.spell_range(id))
        .is_none_or(|range| range.range_max[0] == 0.0 && range.range_max[1] == 0.0);
    for effect in &spell.effects {
        if effect.effect == 0 {
            continue; // blank slots do not update location state
        }
        let [a, b] = effect.target_metadata();
        let mut mask = a.explicit_target_mask(&mut source_set, &mut destination_set);
        mask |= b.explicit_target_mask(&mut source_set, &mut destination_set);
        let info = effect.effect_target_info();
        if info.implicit_type() == EffectTargetType::Explicit {
            let provided = a.object_type().flag_mask() | b.object_type().flag_mask() | mask;
            let mut missing = info.missing_target_mask(source_set, destination_set, provided);
            if zero_max_range {
                // Source does NOT strip ITEM, GAMEOBJECT_ITEM or SOURCE here.
                missing &= !(UNIT_MASK | 0x800 | CORPSE_MASK | 0x40);
            }
            mask |= missing;
        }
        result.available |= mask;
        if effect.attributes & 0x0010_0000 == 0 {
            result.required |= mask;
        }
    }
    result.available |= spell.fields.targets;
    if spell.fields.attributes[13] & 0x8000 == 0 {
        result.required |= spell.fields.targets;
    }
    result
}
