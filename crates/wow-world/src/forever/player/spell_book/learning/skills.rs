use super::{
    AddPlayerSpell, PlayerSpellBook, SpellLearningEffects, SpellLearningError,
    SpellLearningSourceError, SpellLearningSources, effect, source,
};
use crate::forever::player::SkillUpdate;
impl PlayerSpellBook {
    pub(super) fn add_spell_skills<E: SpellLearningEffects>(
        &mut self,
        request: AddPlayerSpell,
        sources: &SpellLearningSources<'_>,
        effects: &mut E,
    ) -> Result<(), SpellLearningError<E::Error>> {
        if let Some(node) = sources.spells.spell_learn_skill(request.spell) {
            if i32::from(node.skill) == request.from_skill {
                return Ok(());
            }
            let skill = u32::from(node.skill);
            let pure_value = effects.pure_skill_value(skill);
            let skill_maximum = effects.pure_skill_maximum(skill);
            let (value, new_maximum) = sources
                .learned_skill_range(
                    node.skill,
                    node.step,
                    pure_value.max(node.value).max(1),
                    node.max_value,
                    || effects.player_level(),
                )
                .map_err(source)?;
            let maximum = skill_maximum.max(new_maximum);
            effect(effects.set_skill(
                self,
                SkillUpdate {
                    skill,
                    step: node.step,
                    value,
                    maximum,
                },
            ))?;
        } else {
            for ability in sources
                .spells
                .skill_line_abilities(request.spell)
                .expect("admitted ability map")
            {
                let skill = u32::from(ability.skill_line);
                if sources.birth.skill_line(skill).is_none() || skill as i32 == request.from_skill {
                    continue;
                }
                if (ability.acquire_method == 2 && !effects.has_skill(skill))
                    || (skill == 960 && ability.trivial_rank_high == 0)
                {
                    if let Some(rc) = sources
                        .world
                        .skill_race_class_info(skill, sources.race, sources.class)
                        .map_err(|error| source(SpellLearningSourceError::World(error)))?
                    {
                        if let Some(input) = sources.world.default_skill_request(
                            rc,
                            sources.class,
                            effects.player_level(),
                        ) {
                            effect(effects.set_skill(
                                self,
                                SkillUpdate {
                                    skill: u32::from(input.skill()),
                                    step: input.step(),
                                    value: input.rank(),
                                    maximum: input.maximum(),
                                },
                            ))?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
