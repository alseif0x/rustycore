//! 02245dcd SpellInfo.cpp:3482-3653 / SpellMgr.cpp:5364-5420.
//! Startup metadata only; applying/removing immunities belongs to the live Unit.
mod creatures;
#[cfg(test)]
mod tests;
use super::{Definition, SpellDefinitionSeeds, SpellEffectValues};
use std::collections::{BTreeMap, BTreeSet};
use wow_persistence::forever::spells::CreatureImmunityRow;

// SharedDefines.h:2912-2918. Its old hex comment omits disarm/silence;
// retain the actual expression, not the comment's obsolete 0x49967ca6.
const LOSS_CONTROL: u64 = (1 << 1)
    | (1 << 2)
    | (1 << 5)
    | (1 << 7)
    | (1 << 10)
    | (1 << 11)
    | (1 << 12)
    | (1 << 13)
    | (1 << 9)
    | (1 << 3)
    | (1 << 14)
    | (1 << 17)
    | (1 << 18)
    | (1 << 20)
    | (1 << 23)
    | (1 << 24)
    | (1 << 27)
    | (1 << 30);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct EffectImmunityInfo {
    pub school_mask: u32,
    pub harmful_aura_school_mask: u32,
    pub mechanic_mask: u64,
    pub dispel_mask: u32,
    pub damage_school_mask: u32,
    pub other_mask: u8,
    pub remove_effects_with_mechanic: bool,
    pub aura_types: BTreeSet<u32>,
    pub effect_types: BTreeSet<u32>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CreatureImmunityInfo {
    pub school_mask: u8,
    pub dispel_mask: u16,
    pub mechanic_mask: u64,
    pub other_mask: u8,
    pub effect_types: Vec<u32>,
    pub aura_types: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImmunityCounts {
    pub input_rows: usize,
    pub creatures: usize,
    pub truncated_masks: usize,
    pub invalid_effect_tokens: usize,
    pub invalid_aura_tokens: usize,
    pub definitions: usize,
    pub effect_slots: usize,
    pub effects_with_info: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellImmunityError {
    RequiresDiminishing,
    AlreadyApplied,
    UndefinedMechanicShift,
    UndefinedDispelShift,
    UndefinedDurationAbs,
}

// Derived metadata only, canonical under its Definition. Index is physical
// effect slot; neither raw effect values nor a second definition are copied.
#[derive(Default)]
pub(super) struct ImmunityState {
    pub(super) effects: BTreeMap<usize, EffectImmunityInfo>,
    pub(super) allowed_mechanics: u64,
}
pub(super) struct LoadedImmunities {
    pub(super) creatures: BTreeMap<i32, CreatureImmunityInfo>,
    pub(super) counts: ImmunityCounts,
}

impl SpellDefinitionSeeds {
    pub fn with_immunity_info(
        mut self,
        rows: Vec<CreatureImmunityRow>,
    ) -> Result<Self, SpellImmunityError> {
        if self.immunities.is_some() {
            return Err(SpellImmunityError::AlreadyApplied);
        }
        if self.diminishing.is_none() {
            return Err(SpellImmunityError::RequiresDiminishing);
        }
        let mut counts = ImmunityCounts::default();
        let creatures = creatures::load(rows, &mut counts);
        let keys = &self
            .source_order
            .as_ref()
            .expect("diminishing admitted source order")
            .primary;
        for key in keys {
            let definition = self.definitions.get_mut(key).expect("admitted definition");
            let mut state = ImmunityState::default();
            for (index, effect) in definition.effects.iter().enumerate() {
                // No IsEffect/IsAura filter: source visits even blank gap slots.
                let maximum = if effect.aura == 77 {
                    match definition
                        .duration
                        .and_then(|id| self.catalog.spell_duration(id))
                    {
                        Some(row) if row.max_duration == -1 => -1,
                        Some(row) => row
                            .max_duration
                            .checked_abs()
                            .ok_or(SpellImmunityError::UndefinedDurationAbs)?,
                        None if definition.fields.attributes[0] & 0x40 != 0 => -1,
                        None => 0,
                    }
                } else {
                    0
                };
                let info = effect_info(key.0, effect, maximum, &creatures)?;
                state.allowed_mechanics |= info.mechanic_mask;
                counts.effect_slots += 1;
                if has_info(&info) {
                    state.effects.insert(index, info);
                    counts.effects_with_info += 1;
                }
            }
            state.allowed_mechanics |= attribute_mechanics(key.0, definition);
            definition.immunity = state;
            counts.definitions += 1;
        }
        counts.creatures = creatures.len();
        self.immunities = Some(LoadedImmunities { creatures, counts });
        Ok(self)
    }
    pub fn immunity_counts(&self) -> Option<ImmunityCounts> {
        self.immunities.as_ref().map(|loaded| loaded.counts)
    }
    pub fn creature_immunity(&self, id: i32) -> Option<&CreatureImmunityInfo> {
        self.immunities.as_ref()?.creatures.get(&id)
    }
}

fn has_info(info: &EffectImmunityInfo) -> bool {
    info.school_mask != 0
        || info.harmful_aura_school_mask != 0
        || info.mechanic_mask != 0
        || info.dispel_mask != 0
        || info.damage_school_mask != 0
        || info.other_mask != 0
        || !info.aura_types.is_empty()
        || !info.effect_types.is_empty()
    // RemoveEffectsWithMechanic alone does not allocate source immunity info.
}
fn effect_info(
    id: u32,
    effect: &SpellEffectValues,
    maximum: i32,
    creatures: &BTreeMap<i32, CreatureImmunityInfo>,
) -> Result<EffectImmunityInfo, SpellImmunityError> {
    let mut info = EffectImmunityInfo::default();
    let misc = effect.misc_values[0];
    match effect.aura {
        147 => {
            if let Some(creature) = creatures.get(&misc) {
                info.school_mask = u32::from(creature.school_mask);
                info.dispel_mask = u32::from(creature.dispel_mask);
                info.mechanic_mask = creature.mechanic_mask;
                info.other_mask = creature.other_mask;
                info.effect_types
                    .extend(creature.effect_types.iter().copied());
                info.aura_types.extend(creature.aura_types.iter().copied());
            }
        }
        77 => {
            match id {
                42292 | 59752 => {
                    info.mechanic_mask = LOSS_CONTROL;
                    info.aura_types.insert(191);
                    info.remove_effects_with_mechanic = true;
                }
                34471 | 19574 | 46227 | 53490 | 65547 | 134946 | 134956 | 195710 | 208683 => {
                    info.mechanic_mask = LOSS_CONTROL;
                    info.remove_effects_with_mechanic = true;
                }
                54508 => info.mechanic_mask = (1 << 11) | (1 << 7) | (1 << 12),
                _ if misc >= 1 => {
                    info.mechanic_mask = 1u64
                        .checked_shl(misc as u32)
                        .ok_or(SpellImmunityError::UndefinedMechanicShift)?
                }
                _ => {}
            }
            if maximum == 100 {
                info.remove_effects_with_mechanic = true;
            }
        }
        37 => {
            info.effect_types.insert(misc as u32);
        }
        38 => {
            info.aura_types.insert(misc as u32);
        }
        39 => info.school_mask = misc as u32,
        267 => info.harmful_aura_school_mask = misc as u32,
        40 => info.damage_school_mask = misc as u32,
        41 => {
            info.dispel_mask = 1u32
                .checked_shl(misc as u32)
                .ok_or(SpellImmunityError::UndefinedDispelShift)?
        }
        _ => {}
    }
    Ok(info)
}
fn attribute_mechanics(id: u32, definition: &Definition) -> u64 {
    let attributes = definition.fields.attributes[5];
    let mut mask = 0;
    if attributes & 0x8 != 0 {
        mask |= match id {
            22812 | 47585 => (1 << 12) | (1 << 13) | (1 << 14) | (1 << 10),
            49039 => 0,
            _ => 1 << 12,
        };
    }
    if attributes & 0x40000 != 0 {
        mask |= 1 << 2;
    }
    if attributes & 0x20000 != 0 {
        mask |= if matches!(id, 22812 | 47585) {
            (1 << 5) | (1 << 24)
        } else {
            1 << 5
        };
    }
    mask
}
