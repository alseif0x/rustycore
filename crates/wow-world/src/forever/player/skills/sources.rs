//! Concrete World/Data readiness and lazy source projections for SetSkill.
use super::{
    SkillAbilityMetadata, SkillCatalog, SkillLineMetadata, SkillRaceClassMetadata,
    SkillRewardSpellMetadata, SkillSetSource, SkillUpdate,
};
use crate::forever::{
    creation::{SourceError, WorldSources},
    spells::{SpellDefinitionError, SpellDefinitionSeeds},
};
use wow_data::forever_birth::{BirthCatalog, race_in_mask};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSetInputError {
    World(SourceError),
    MissingRanks,
    MismatchedBirthCatalog,
    DefinitionLookup(SpellDefinitionError),
}

/// Scoped immutable dependencies, not a general Player/Session context. Birth
/// is borrowed from the same WorldSources, never separately supplied or copied.
pub struct SkillSetSources<'a> {
    world: &'a WorldSources,
    birth: &'a BirthCatalog,
    spells: &'a SpellDefinitionSeeds,
    race: u8,
    class: u8,
}
impl<'a> SkillSetSources<'a> {
    pub fn new(
        world: &'a WorldSources,
        spells: &'a SpellDefinitionSeeds,
        race: u8,
        class: u8,
    ) -> Result<Self, SkillSetInputError> {
        // Validate class and the admitted RC lookup before any mutable operation.
        world
            .skill_race_class_info(0, race, class)
            .map_err(SkillSetInputError::World)?;
        if spells.spell_rank_counts().is_none() {
            return Err(SkillSetInputError::MissingRanks);
        }
        let birth = world
            .skill_birth_catalog()
            .map_err(SkillSetInputError::World)?;
        if !spells
            .skill_birth_catalog()
            .is_some_and(|catalog| std::ptr::eq(catalog, birth))
        {
            return Err(SkillSetInputError::MismatchedBirthCatalog);
        }
        Ok(Self {
            world,
            birth,
            spells,
            race,
            class,
        })
    }
}

impl SkillCatalog for SkillSetSources<'_> {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        SkillCatalog::skill_line(self.birth, id)
    }
    fn child_lines(&self, parent: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        SkillCatalog::child_lines(self.birth, parent)
    }
    fn abilities_for_skill(&self, skill: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        SkillCatalog::abilities_for_skill(self.birth, skill)
    }
}

impl SkillSetSource for SkillSetSources<'_> {
    type Error = SkillSetInputError;
    fn race(&self) -> u8 {
        self.race
    }
    fn class(&self) -> u8 {
        self.class
    }

    fn default_skill_records(
        &self,
    ) -> Result<impl Iterator<Item = SkillRaceClassMetadata> + '_, Self::Error> {
        self.world
            .default_skill_records(self.race, self.class)
            .map(|rows| {
                rows.map(|rc| SkillRaceClassMetadata {
                    id: rc.id,
                    skill: rc.skill,
                    flags: rc.flags,
                    min_level: rc.min_level,
                    tier: rc.tier,
                })
            })
            .map_err(SkillSetInputError::World)
    }

    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::Error> {
        self.world
            .skill_race_class_info(skill, self.race, self.class)
            .map(|row| {
                row.map(|rc| SkillRaceClassMetadata {
                    id: rc.id,
                    skill: rc.skill,
                    flags: rc.flags,
                    min_level: rc.min_level,
                    tier: rc.tier,
                })
            })
            .map_err(SkillSetInputError::World)
    }

    fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32> {
        self.world.skill_tier_value(tier, index)
    }

    fn default_skill_request(&self, rc: SkillRaceClassMetadata, level: u8) -> Option<SkillUpdate> {
        let original = self.birth.race_class_record(rc.id)?;
        self.world
            .default_skill_request(original, self.class, level)
            .map(|request| SkillUpdate {
                skill: u32::from(request.skill()),
                step: request.step(),
                value: request.rank(),
                maximum: request.maximum(),
            })
    }

    fn reward_spell(&self, id: u32) -> Result<Option<SkillRewardSpellMetadata>, Self::Error> {
        self.spells
            .get(id, 0)
            .map(|spell| {
                spell.map(|spell| SkillRewardSpellMetadata {
                    future_condition_id: spell.fields().show_future_spell_player_condition_id,
                    spell_level: spell.fields().spell_level,
                    base_level: spell.fields().base_level,
                })
            })
            .map_err(SkillSetInputError::DefinitionLookup)
    }

    fn first_spell_in_chain(&self, id: u32) -> u32 {
        self.spells.first_spell_in_chain(id)
    }

    fn race_in_mask(&self, mask: u64, race: u32) -> bool {
        race_in_mask(mask, race)
    }
}
