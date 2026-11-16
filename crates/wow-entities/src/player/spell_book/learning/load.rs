use super::{AddPlayerSpell, LearningError, PlayerSpellBook, SpellLearningEffects};
use crate::SpellLearningSource;

/// One decoded row from the native `_LoadSpells` query, in query order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLoadRow {
    pub spell: u32,
    pub active: bool,
    pub disabled: bool,
}

impl PlayerSpellBook {
    /// Apply native spell rows through the existing AddSpell operation, then
    /// mark favorites on physical entries only. Errors retain their completed
    /// prefix; the caller must discard an unadmitted construction.
    pub fn load_spells<S, E>(
        &mut self,
        rows: &[SpellLoadRow],
        favorites: &[u32],
        sources: &S,
        effects: &mut E,
    ) -> Result<(), LearningError<E::Error, S>>
    where
        S: SpellLearningSource,
        E: SpellLearningEffects<S>,
    {
        for row in rows {
            let _ = self.add_spell(
                AddPlayerSpell {
                    spell: row.spell,
                    active: row.active,
                    learning: false,
                    dependent: false,
                    disabled: row.disabled,
                    loading: true,
                    from_skill: 0,
                    favorite: false,
                    trait_data: None,
                },
                sources,
                effects,
            )?;
        }

        for &spell in favorites {
            if let Some(entry) = self.entries.get_mut(&spell) {
                entry.favorite = true;
            }
        }

        Ok(())
    }
}
