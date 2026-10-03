//! 02245dcd SpellMgr.cpp:143-232, AddSpell's msg=false admission query.
//! A valid definition is not an executed effect or learned Player spell.
#[cfg(test)]
mod tests;
use super::{Key, SpellDefinitionError, SpellDefinitionSeeds};
use crate::forever::creation::NumericItemTemplates;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellValidityError {
    DefinitionLookup(SpellDefinitionError),
    RecursiveLearnCycle,
}
struct Frame {
    key: Key,
    next_effect: usize,
    check_reagents: bool,
}
impl SpellDefinitionSeeds {
    /// Exact/fallback parent lookup, then physical effects and regular child
    /// lookup; only crafting effects require positive reagent-item checks.
    /// Uses the same admitted template authority as creation/inventory, never
    /// an Item.db2-only presence guess. No diagnostic chat/SQL side effects.
    pub fn spell_is_valid(
        &self,
        spell: u32,
        difficulty: i16,
        items: &NumericItemTemplates,
    ) -> Result<bool, SpellValidityError> {
        valid(self, spell, difficulty, &mut |item| {
            items.template(item).is_some()
        })
    }
}

fn valid(
    seeds: &SpellDefinitionSeeds,
    spell: u32,
    difficulty: i16,
    item_exists: &mut impl FnMut(u32) -> bool,
) -> Result<bool, SpellValidityError> {
    let Some(root) = seeds
        .get(spell, difficulty)
        .map_err(SpellValidityError::DefinitionLookup)?
    else {
        return Ok(false);
    };
    let key = (root.spell_id(), root.difficulty());
    let mut active = BTreeSet::from([key]);
    let mut frames = vec![Frame {
        key,
        next_effect: 0,
        check_reagents: false,
    }];
    // An explicit stack preserves source depth-first early-return order without
    // consuming the native stack or caching/reordering duplicate child queries.
    while let Some(frame) = frames.last_mut() {
        let definition = &seeds.definitions[&frame.key];
        if let Some(effect) = definition.effects.get(frame.next_effect) {
            frame.next_effect += 1;
            match effect.effect {
                24 | 157 => {
                    if effect.item_type == 0 {
                        if !definition.has_effect(59) && !definition.has_effect(157) {
                            return Ok(false);
                        }
                    } else if !item_exists(effect.item_type) {
                        return Ok(false);
                    }
                    frame.check_reagents = true;
                }
                36 => {
                    let Some(child) = seeds
                        .get(effect.trigger_spell, 0)
                        .map_err(SpellValidityError::DefinitionLookup)?
                    else {
                        return Ok(false);
                    };
                    let key = (child.spell_id(), child.difficulty());
                    if !active.insert(key) {
                        return Err(SpellValidityError::RecursiveLearnCycle);
                    }
                    frames.push(Frame {
                        key,
                        next_effect: 0,
                        check_reagents: false,
                    });
                }
                _ => {}
            }
        } else {
            if frame.check_reagents {
                for &reagent in &definition.fields.reagents {
                    if reagent > 0 && !item_exists(reagent as u32) {
                        return Ok(false);
                    }
                }
            }
            active.remove(&frame.key);
            frames.pop();
        }
    }
    Ok(true)
}
