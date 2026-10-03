//! 02245dcd Player.cpp:25678-25725::LearnSkillRewardedSpells.
//! Complete ordered admission/dispatch, not actual ConditionMgr/AddSpell effects.
use super::{PlayerSkills, SkillSetEffects, SkillSetError, SkillSetInputError, SkillSetSources};
use wow_data::forever_birth::race_in_mask;

impl PlayerSkills {
    pub fn learn_skill_rewards<E: SkillSetEffects>(
        &mut self,
        skill: u32,
        value: u32,
        race: u8,
        sources: &SkillSetSources<'_>,
        effects: &mut E,
    ) -> Result<(), SkillSetError<E::Error>> {
        // Source captures class mask once; level/world remain live per entry.
        let class_mask = 1_u32 << (sources.class - 1);
        for ability in sources.birth.abilities_for_skill(skill) {
            let Some(spell) = sources
                .spells
                .get(ability.spell as u32, 0)
                .map_err(|error| {
                    SkillSetError::Source(SkillSetInputError::DefinitionLookup(error))
                })?
            else {
                continue;
            };
            match ability.acquire_method {
                1 | 2 => {}
                4 => {
                    let condition = spell.fields().show_future_spell_player_condition_id;
                    if condition != 0
                        && !effects
                            .meets_player_condition(self, condition)
                            .map_err(SkillSetError::Effect)?
                    {
                        continue;
                    }
                    if !effects
                        .meets_skill_ability_conditions(self, ability.id)
                        .map_err(SkillSetError::Effect)?
                    {
                        continue;
                    }
                }
                _ => continue,
            }
            // Conditions precede race/class/level filters; moving them later
            // changes observable admission/reentrant effects and query order.
            if ability.race_mask != 0 && !race_in_mask(ability.race_mask, u32::from(race)) {
                continue;
            }
            if ability.class_mask != 0 && ability.class_mask as u32 & class_mask == 0 {
                continue;
            }
            if spell.fields().spell_level.max(spell.fields().base_level)
                > u32::from(effects.player_level())
            {
                continue;
            }
            if (value as i32) < i32::from(ability.min_skill_rank) && ability.acquire_method == 1 {
                // Unlike SetSkill deactivation this removes the EXACT ability
                // spell, not the first-in-chain ID. Preserve signed ID bits.
                effects
                    .remove_spell(self, ability.spell as u32)
                    .map_err(SkillSetError::Effect)?;
            } else if !effects.is_in_world() {
                let _ = effects
                    .add_reward_spell(self, ability.spell as u32, u32::from(ability.skill_line))
                    .map_err(SkillSetError::Effect)?;
            } else {
                effects
                    .learn_reward_spell(self, ability.spell as u32, u32::from(ability.skill_line))
                    .map_err(SkillSetError::Effect)?;
            }
        }
        Ok(())
    }
}
