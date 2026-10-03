use super::{SpellDefinitionView, SpellLearningSourceError as Error};
use crate::forever::{
    creation::{NumericItemTemplates, WorldSources},
    spells::SpellDefinitionSeeds,
};
use wow_data::forever_birth::BirthCatalog;

pub struct SpellLearningSources<'a> {
    pub(super) world: &'a WorldSources,
    pub(super) spells: &'a SpellDefinitionSeeds,
    pub(super) items: &'a NumericItemTemplates,
    pub(super) birth: &'a BirthCatalog,
    pub(super) race: u8,
    pub(super) class: u8,
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
    pub(super) fn spell(&self, id: u32) -> Result<Option<SpellDefinitionView<'a>>, Error> {
        self.spells.get(id, 0).map_err(Error::Definition)
    }
    pub(super) fn passive(&self, id: u32) -> Result<bool, Error> {
        Ok(self
            .spell(id)?
            .is_some_and(|view| view.fields().attributes[0] & 0x40 != 0))
    }
    pub(super) fn primary_profession_first(&self, view: &SpellDefinitionView<'_>) -> bool {
        // SpellInfo.cpp:1666-1685; SpellMgr.cpp:107-111, regular ChainEntry.
        self.spells
            .spell_rank_node(view.spell_id())
            .map_or(1, |node| node.rank)
            == 1
            && view.effects().any(|effect| {
                effect.values().effect == 118
                    && self
                        .birth
                        .skill_line(effect.values().misc_values[0] as u32)
                        .is_some_and(|line| line.category == 9 && line.parent_skill == 0)
            })
    }
    /// Shared AddSpell/RemoveSpell range phase only. Callers own their distinct
    /// initial value/minimum and final cap rules. Keep lazy GetLevel ordering.
    pub(super) fn learned_skill_range(
        &self,
        skill: u16,
        step: u16,
        mut value: u16,
        mut maximum: u16,
        level: impl FnOnce() -> u8,
    ) -> Result<(u16, u16), Error> {
        if maximum == 0 {
            if let Some(rc) = self
                .world
                .skill_race_class_info(u32::from(skill), self.race, self.class)
                .map_err(Error::World)?
            {
                if let Some(line) = self.birth.skill_line(u32::from(skill)) {
                    if self
                        .world
                        .skill_tier_value(rc.tier as i32 as u32, 0)
                        .is_some()
                    {
                        maximum = self
                            .world
                            .skill_tier_value(rc.tier as i32 as u32, (i32::from(step) - 1) as u32)
                            .expect("same existing tier") as u16;
                    } else if skill == 960 || line.category == 8 {
                        maximum = 1;
                    } else if line.category == 10 {
                        value = 300;
                        maximum = 300;
                    } else {
                        maximum = u16::from(level()) * 5;
                    }
                }
                if rc.flags & 0x10 != 0 {
                    value = maximum;
                }
            }
        }
        Ok((value, maximum))
    }
}
