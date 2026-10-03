//! 02245dcd Player.cpp:25544-25599 / ObjectMgr.cpp:9076-9100.
//! Live ordered default learning, never a bulk application of input requests.
use super::{
    PlayerSkills, SkillSetEffects, SkillSetError, SkillSetInputError, SkillSetSources, SkillUpdate,
};

impl PlayerSkills {
    pub fn learn_default_skills<E: SkillSetEffects>(
        &mut self,
        sources: &SkillSetSources<'_>,
        effects: &mut E,
    ) -> Result<(), SkillSetError<E::Error>> {
        let records = sources
            .world
            .default_skill_records(sources.race, sources.class)
            .map_err(|error| SkillSetError::Source(SkillSetInputError::World(error)))?;
        for rc in records {
            // Check HasSkill immediately before the individual call, after all
            // prior reward/child effects; never deduplicate a startup request list.
            if self.has_skill(u32::from(rc.skill)) {
                continue;
            }
            if i16::from(rc.min_level) > i16::from(effects.player_level()) {
                continue;
            }
            if let Some(request) =
                sources
                    .world
                    .default_skill_request(rc, sources.class, effects.player_level())
            {
                // Source SetSkill is void: ordinary no-slot/missing/inventory
                // returns do not end this loop. Required-effect errors do abort
                // the unadmitted construction; no Create/save success is implied.
                let _ = self.set_skill(
                    SkillUpdate {
                        skill: u32::from(request.skill()),
                        step: request.step(),
                        value: request.rank(),
                        maximum: request.maximum(),
                    },
                    sources,
                    effects,
                )?;
            }
        }
        Ok(())
    }
}
