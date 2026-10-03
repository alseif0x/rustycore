use super::{
    PlayerSpellBook, SpellDefinitionView, SpellLearningEffects, SpellLearningError,
    SpellLearningSourceError, effect, source,
};
impl PlayerSpellBook {
    pub(super) fn passive_learn<E: SpellLearningEffects>(
        &mut self,
        view: &SpellDefinitionView<'_>,
        effects: &mut E,
    ) -> Result<bool, SpellLearningError<E::Error>> {
        let form = effects.shapeshift_form();
        let fields = view.fields();
        let need_cast = if fields.stances == 0 {
            true
        } else if form != 0 {
            if form > 64 {
                return Err(source(SpellLearningSourceError::UndefinedShapeshiftMask));
            }
            fields.stances & (1u64 << (form - 1)) != 0
        } else {
            fields.attributes[2] & 0x00080000 != 0
        };
        if fields.equipped_item_class >= 0 && view.effects().any(|effect| effect.is_aura()) {
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
            && (fields.caster_aura_state == 0 || effects.has_aura_state(fields.caster_aura_state)))
    }
}
