use super::{
    PlayerSpellBook, SpellLearningEffects, SpellLearningError, SpellLearningSources, effect, source,
};
use crate::forever::player::SkillUpdate;

impl PlayerSpellBook {
    pub(super) fn remove_spell_skills<E: SpellLearningEffects>(
        &mut self,
        spell: u32,
        sources: &SpellLearningSources<'_>,
        effects: &mut E,
    ) -> Result<(), SpellLearningError<E::Error>> {
        let Some(node) = sources.spells.spell_learn_skill(spell) else {
            return Ok(());
        };
        let mut previous = sources.spells.previous_spell_in_chain(spell);
        let previous_skill = if previous == 0 {
            None
        } else {
            let mut skill = sources.spells.spell_learn_skill(previous);
            while skill.is_none() && previous != 0 {
                previous = sources.spells.previous_spell_in_chain(previous);
                // Source quirk: query FIRST of the newly visited previous rank,
                // including first_spell_in_chain(0), not that rank directly.
                skill = sources
                    .spells
                    .spell_learn_skill(sources.spells.first_spell_in_chain(previous));
            }
            skill
        };
        let Some(prior) = previous_skill else {
            return effect(effects.set_skill(
                self,
                SkillUpdate {
                    skill: u32::from(node.skill),
                    step: 0,
                    value: 0,
                    maximum: 0,
                },
            ));
        };
        let pure_value = effects.pure_skill_value(u32::from(prior.skill));
        let pure_maximum = effects.pure_skill_maximum(u32::from(prior.skill));
        let (mut value, maximum) = sources
            .learned_skill_range(prior.skill, prior.step, pure_value, prior.max_value, || {
                effects.player_level()
            })
            .map_err(source)?;
        if prior.max_value != 0 && value > prior.value {
            value = prior.value;
        }
        value = value.min(maximum);
        effect(effects.set_skill(
            self,
            SkillUpdate {
                skill: u32::from(prior.skill),
                step: prior.step,
                value,
                maximum: pure_maximum.min(maximum),
            },
        ))
    }
}
