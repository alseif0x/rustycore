//! Concrete immutable World/Data admission and source projections.
use crate::forever::{
    creation::{NumericItemTemplates, SourceError, WorldSources},
    spells::{SpellDefinitionError, SpellDefinitionSeeds, SpellDefinitionView, SpellValidityError},
};
use wow_data::forever_birth::BirthCatalog;
use wow_entities::{
    SkillCatalog, SkillLineMetadata, SkillRaceClassMetadata, SkillUpdate, SpellDefinitionSource,
    SpellLearningAbility, SpellLearningNode, SpellLearningRank, SpellLearningSkill,
    SpellLearningSource,
};

pub type SpellLearningSourceError =
    wow_entities::SpellLearningSourceError<SourceError, SpellDefinitionError, SpellValidityError>;
pub type SpellLearningError<E> = wow_entities::SpellLearningError<E, SpellLearningSourceError>;
use SpellLearningSourceError as Error;

pub struct SpellLearningSources<'a> {
    world: &'a WorldSources,
    spells: &'a SpellDefinitionSeeds,
    items: &'a NumericItemTemplates,
    birth: &'a BirthCatalog,
    race: u8,
    class: u8,
}
impl<'a> SpellLearningSources<'a> {
    pub fn new(
        world: &'a WorldSources,
        spells: &'a SpellDefinitionSeeds,
        items: &'a NumericItemTemplates,
        race: u8,
        class: u8,
    ) -> Result<Self, Error> {
        world
            .skill_race_class_info(0, race, class)
            .map_err(Error::World)?;
        if spells.spell_rank_counts().is_none()
            || spells.required_spell_counts().is_none()
            || spells.learn_skill_counts().is_none()
            || spells.learn_spell_counts().is_none()
        {
            return Err(Error::MissingLearningPhases);
        }
        let birth = world.skill_birth_catalog().map_err(Error::World)?;
        if !spells
            .skill_birth_catalog()
            .is_some_and(|other| std::ptr::eq(birth, other))
        {
            return Err(Error::MismatchedBirthCatalog);
        }
        Ok(Self {
            world,
            spells,
            items,
            birth,
            race,
            class,
        })
    }
}

impl SpellDefinitionSource for SpellLearningSources<'_> {
    type Error = SpellDefinitionError;
    type Definition<'a>
        = SpellDefinitionView<'a>
    where
        Self: 'a;
    fn definition(
        &self,
        spell: u32,
        difficulty: i16,
    ) -> Result<Option<Self::Definition<'_>>, Self::Error> {
        self.spells.definition(spell, difficulty)
    }
    fn owns<'a>(&self, definition: &Self::Definition<'a>) -> bool
    where
        Self: 'a,
    {
        self.spells.owns(definition)
    }
}
impl SpellLearningSource for SpellLearningSources<'_> {
    type WorldError = SourceError;
    type ValidityError = SpellValidityError;
    fn spell_is_valid(&self, spell: u32) -> Result<bool, Self::ValidityError> {
        self.spells.spell_is_valid(spell, 0, self.items)
    }
    fn spell_rank_node(&self, spell: u32) -> Option<SpellLearningRank> {
        self.spells
            .spell_rank_node(spell)
            .map(|node| SpellLearningRank {
                first: node.first,
                rank: node.rank,
            })
    }
    fn previous_spell_in_chain(&self, spell: u32) -> u32 {
        self.spells.previous_spell_in_chain(spell)
    }
    fn next_spell_in_chain(&self, spell: u32) -> u32 {
        self.spells.next_spell_in_chain(spell)
    }
    fn first_spell_in_chain(&self, spell: u32) -> u32 {
        self.spells.first_spell_in_chain(spell)
    }
    fn spells_requiring(&self, spell: u32) -> Option<impl Iterator<Item = u32> + '_> {
        self.spells.spells_requiring(spell)
    }
    fn spell_learn_nodes(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = SpellLearningNode> + '_> {
        self.spells.spell_learn_nodes(spell).map(|rows| {
            rows.map(|node| SpellLearningNode {
                source: node.source,
                spell: node.spell,
                overrides_spell: node.overrides_spell,
                active: node.active,
                auto_learned: node.auto_learned,
            })
        })
    }
    fn spell_learned_by(&self, spell: u32) -> Option<impl Iterator<Item = SpellLearningNode> + '_> {
        self.spells.spell_learned_by(spell).map(|rows| {
            rows.map(|node| SpellLearningNode {
                source: node.source,
                spell: node.spell,
                overrides_spell: node.overrides_spell,
                active: node.active,
                auto_learned: node.auto_learned,
            })
        })
    }
    fn skill_line_abilities(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = SpellLearningAbility> + '_> {
        self.spells.skill_line_abilities(spell).map(|rows| {
            rows.map(|ability| SpellLearningAbility {
                skill_line: ability.skill_line,
                acquire_method: ability.acquire_method,
                trivial_rank_high: ability.trivial_rank_high,
            })
        })
    }
    fn spell_learn_skill(&self, spell: u32) -> Option<SpellLearningSkill> {
        self.spells
            .spell_learn_skill(spell)
            .map(|node| SpellLearningSkill {
                skill: node.skill,
                step: node.step,
                value: node.value,
                max_value: node.max_value,
            })
    }
    fn skill_line(&self, skill: u32) -> Option<SkillLineMetadata> {
        SkillCatalog::skill_line(self.birth, skill)
    }
    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::WorldError> {
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
}
