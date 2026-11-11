use super::SpellLearningSource;
use super::{LearningError, PlayerSpellBook, SpellLearningEffects, effect, source};
use crate::{SpellBookDefinition, SpellBookEffect, SpellLearningError, SpellLearningSourceError};
impl PlayerSpellBook {
    pub(super) fn passive_learn<'a, S: SpellLearningSource + 'a, E: SpellLearningEffects<S>>(
        &mut self,
        view: &S::Definition<'a>,
        effects: &mut E,
    ) -> Result<bool, LearningError<E::Error, S>> {
        let form = effects.shapeshift_form();
        let need_cast = if view.stances() == 0 {
            true
        } else if form != 0 {
            if form > 64 {
                return Err(source(SpellLearningSourceError::UndefinedShapeshiftMask));
            }
            view.stances() & (1u64 << (form - 1)) != 0
        } else {
            view.attributes()[2] & 0x00080000 != 0
        };
        if view.equipped_item_class() >= 0 && view.effects().any(|effect| effect.is_aura()) {
            if !effects.has_aura(view.spell_id())
                && effects
                    .item_fits_spell(view)
                    .map_err(SpellLearningError::Effect)?
            {
                effect(effects.add_aura(self, view.spell_id()))?;
            }
            return Ok(false); // Even if aura is absent/item does not fit.
        }
        Ok(need_cast
            && (view.caster_aura_state() == 0 || effects.has_aura_state(view.caster_aura_state())))
    }
}
