use super::{
    AddPlayerSpell, LearnPlayerSpell, LearningError, PlayerSpellBook, SpellBookMessage,
    SpellLearningEffects, SpellLearningSource, effect,
};

impl PlayerSpellBook {
    pub fn learn_spell<S: SpellLearningSource, E: SpellLearningEffects<S>>(
        &mut self,
        request: LearnPlayerSpell,
        sources: &S,
        effects: &mut E,
    ) -> Result<(), LearningError<E::Error, S>> {
        let before = self.spell(request.spell).copied();
        let disabled = before.is_some_and(|entry| entry.disabled);
        let active = if disabled {
            before.unwrap().active
        } else {
            true
        };
        let favorite = before.is_some_and(|entry| entry.favorite);
        let mut add = AddPlayerSpell::learned(request.spell, active, request.dependent);
        add.from_skill = request.from_skill;
        add.favorite = favorite;
        add.trait_data = request.trait_data;
        let learning = self.add_spell(add, sources, effects)?;
        if learning && effects.is_in_world() {
            effect(effects.publish(
                self,
                SpellBookMessage::Learned {
                    spell: request.spell,
                    favorite,
                    trait_definition: request.trait_data.map(|data| data.definition_id()),
                    suppress_messaging: request.suppress_messaging,
                },
            ))?;
        }
        if disabled {
            let next = sources.next_spell_in_chain(request.spell);
            if next != 0 && self.spell(next).is_some_and(|entry| entry.disabled) {
                self.learn_spell(
                    LearnPlayerSpell::new(next, false, request.from_skill),
                    sources,
                    effects,
                )?;
            }
            for id in sources
                .spells_requiring(request.spell)
                .expect("admitted required phase")
            {
                if self.spell(id).is_some_and(|entry| entry.disabled) {
                    self.learn_spell(
                        LearnPlayerSpell::new(id, false, request.from_skill),
                        sources,
                        effects,
                    )?;
                }
            }
        } else {
            effect(effects.quest_learn_spell(self, request.spell))?;
        }
        Ok(())
    }
}
