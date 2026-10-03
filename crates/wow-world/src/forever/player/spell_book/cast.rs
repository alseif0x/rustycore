//! 02245dcd Player.cpp:30999-31008; Unit.cpp:14584-14643 / Unit.h:1490-1505.
//! Read-only cast definition resolution, not a cast/Unit/creation executor.
#[cfg(test)]
mod tests;
use super::{PlayerSpellBook, SpellBookMutation, SpellBookOrderError};
use crate::forever::spells::{SpellDefinitionError, SpellDefinitionSeeds, SpellDefinitionView};

const IGNORE_POWER: u32 = 0x4;
const IGNORE_CAST_TIME: u32 = 0x40;
const IGNORE_SHAPESHIFT: u32 = 0x400;

/// Borrowed projection of an actual aura's immutable SpellInfo/effect and live
/// amount. No Unit/Aura copy or retained mutable mirror. The caller supplies
/// GetAuraEffectsByType order; it must not sort, reconstruct from SpellInfo's
/// base points, or substitute startup/passive requests for live applied auras.
pub struct CastOverrideAura<'a> {
    definition: SpellDefinitionView<'a>,
    slot: usize,
    amount: &'a f32,
}
impl<'a> CastOverrideAura<'a> {
    pub fn new(definition: SpellDefinitionView<'a>, slot: usize, amount: &'a f32) -> Self {
        Self {
            definition,
            slot,
            amount,
        }
    }
}

/// Both lists are required, borrowed and source ordered. Empty means an
/// admitted Unit has no applied auras of that type, not unavailable Unit data.
/// No default or production producer exists yet; Create remains disabled.
pub struct CastSpellAuras<'a> {
    regular: &'a [CastOverrideAura<'a>],
    triggered: &'a [CastOverrideAura<'a>],
}
impl<'a> CastSpellAuras<'a> {
    pub fn new(regular: &'a [CastOverrideAura<'a>], triggered: &'a [CastOverrideAura<'a>]) -> Self {
        Self { regular, triggered }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CastSpellError<E> {
    Order(SpellBookOrderError<E>),
    Definition(SpellDefinitionError),
    MismatchedDefinitionOwner,
    InvalidAuraEffect,
    UndefinedAmountNarrowing,
}
pub struct CastSpellResult<'a> {
    definition: SpellDefinitionView<'a>,
    trigger_flags: u32,
}
impl<'a> CastSpellResult<'a> {
    pub fn definition(&self) -> &SpellDefinitionView<'a> {
        &self.definition
    }
    pub fn trigger_flags(&self) -> u32 {
        self.trigger_flags
    }
}

#[derive(Default)]
struct Context {
    visited: [u32; 5],
}
impl Context {
    fn add(&mut self, spell: u32) -> bool {
        for slot in &mut self.visited {
            // Equality precedes empty: ID zero is rejected at an empty slot.
            if *slot == spell {
                return false;
            }
            if *slot == 0 {
                *slot = spell;
                return true;
            }
        }
        false
    }
}

impl PlayerSpellBook {
    /// Source wrapper starts with zero trigger flags and five empty slots; the
    /// initial definition is NOT previsited. Missing replacement definitions
    /// still consume a slot. Success follows the first present replacement all
    /// the way down: no sibling backtracking after recursion. Unit replacement
    /// reenters the virtual Player override phase with the SAME context/flags.
    ///
    /// Caller owns the actual map difficulty and admitted applied-aura lists.
    /// This synchronous read-only operation excludes membership/aura mutation;
    /// no pointer, Unit guard, native container or history copy survives return.
    pub fn cast_spell_info<'a, E>(
        &self,
        mut definition: SpellDefinitionView<'a>,
        spells: &'a SpellDefinitionSeeds,
        map_difficulty: i16,
        auras: &CastSpellAuras<'a>,
        mut order: impl FnMut(&[SpellBookMutation], usize) -> Result<Vec<u32>, E>,
    ) -> Result<CastSpellResult<'a>, CastSpellError<E>> {
        if !definition.belongs_to(spells) {
            return Err(CastSpellError::MismatchedDefinitionOwner);
        }
        let mut context = Context::default();
        let mut flags = 0;
        loop {
            let mut replacement = None;
            for id in self
                .source_override_spells(definition.spell_id(), &mut order)
                .map_err(CastSpellError::Order)?
            {
                if context.add(id) {
                    if let Some(next) = spells
                        .get(id, map_difficulty)
                        .map_err(CastSpellError::Definition)?
                    {
                        replacement = Some(next);
                        break;
                    }
                }
            }
            if let Some(next) = replacement {
                definition = next;
                continue;
            }
            if let Some(next) = find_aura(
                &definition,
                spells,
                map_difficulty,
                auras.regular,
                332,
                &mut context,
                &mut flags,
            )? {
                flags &= !IGNORE_CAST_TIME;
                definition = next;
                continue;
            }
            if let Some(next) = find_aura(
                &definition,
                spells,
                map_difficulty,
                auras.triggered,
                333,
                &mut context,
                &mut flags,
            )? {
                flags |= IGNORE_CAST_TIME;
                definition = next;
                continue;
            }
            return Ok(CastSpellResult {
                definition,
                trigger_flags: flags,
            });
        }
    }
}

fn find_aura<'a, E>(
    current: &SpellDefinitionView<'_>,
    spells: &'a SpellDefinitionSeeds,
    difficulty: i16,
    auras: &[CastOverrideAura<'a>],
    kind: u32,
    context: &mut Context,
    flags: &mut u32,
) -> Result<Option<SpellDefinitionView<'a>>, CastSpellError<E>> {
    for aura in auras {
        if !aura.definition.belongs_to(spells) {
            return Err(CastSpellError::MismatchedDefinitionOwner);
        }
        let effect = aura
            .definition
            .effect(aura.slot)
            .filter(|effect| effect.is_aura_kind(kind))
            .ok_or(CastSpellError::InvalidAuraEffect)?;
        let misc = effect.values().misc_values[0];
        let matches = if misc != 0 {
            misc as u32 == current.spell_id()
        } else {
            affected(
                current,
                aura.definition.fields().spell_family_name,
                effect.values().class_mask,
            )
        };
        if !matches {
            continue;
        }
        // Source float -> int32 is undefined outside this finite interval. Do
        // not let Rust's saturating float cast invent admission. Truncation of
        // valid negative fractions, followed by uint32 promotion, is retained.
        let amount = *aura.amount;
        if !amount.is_finite() || !(-2147483648.0..2147483648.0).contains(&amount) {
            return Err(CastSpellError::UndefinedAmountNarrowing);
        }
        let id = (amount as i32) as u32;
        if context.add(id) {
            if let Some(next) = spells
                .get(id, difficulty)
                .map_err(CastSpellError::Definition)?
            {
                set_flag(
                    flags,
                    IGNORE_POWER,
                    aura.definition.fields().attributes[8] & 0x40000 != 0,
                );
                set_flag(
                    flags,
                    IGNORE_SHAPESHIFT,
                    aura.definition.fields().attributes[11] & 0x200 != 0,
                );
                return Ok(Some(next));
            }
        }
    }
    Ok(None)
}

fn affected(spell: &SpellDefinitionView<'_>, family: u32, mask: [u32; 4]) -> bool {
    // SpellInfo.cpp:1975-1986: family zero bypasses BOTH family and flags;
    // nonzero matching family with zero effect mask affects all family spells.
    family == 0
        || (family == spell.fields().spell_family_name
            && (mask == [0; 4]
                || mask
                    .iter()
                    .zip(spell.fields().spell_family_flags)
                    .any(|(a, b)| *a & b != 0)))
}
fn set_flag(flags: &mut u32, bit: u32, enabled: bool) {
    if enabled {
        *flags |= bit;
    } else {
        *flags &= !bit;
    }
}
