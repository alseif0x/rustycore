use super::*;
use crate::player::skills::test_catalog::Catalog;
use std::cell::Cell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Enchant(u32, u16, u16, u16),
    Learn(u32, u16, Option<SkillUpdateState>),
    Mount,
    Criteria(u32, SkillCriteria, u16),
    Aura(u32, u32, Option<SkillUpdateState>),
    Store(u8),
    Full,
    Remove(u32),
    PlayerCondition(u32),
    AbilityConditions(u32),
}
#[derive(Default)]
pub(super) struct Effects<'a> {
    pub events: Vec<Event>,
    pub full_at: Option<u8>,
    pub fail_at: Option<usize>,
    pub bonus: Option<(&'a Catalog, i32)>,
    pub reenter: Option<(&'a Fixture, SkillUpdate)>,
    pub level: Option<u8>,
    pub in_world: bool,
    pub deny_player: Option<u32>,
    pub deny_ability: Option<u32>,
    pub add_false: bool,
    pub flip_world_after_grant: bool,
    pub grants: Vec<(u32, u32, bool)>,
    pub level_queries: Cell<usize>,
    pub world_queries: Cell<usize>,
    pub level_after_grant: Option<u8>,
}
impl Effects<'_> {
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len() - 1) {
            Err("unavailable required effect")
        } else {
            Ok(())
        }
    }
    fn grant(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        skill: u32,
        live: bool,
    ) -> Result<(), &'static str> {
        self.record(Event::Learn(
            skill,
            skills.pure_value(skill),
            skills.status(skill).map(|(_, state)| state),
        ))?;
        self.grants.push((spell, skill, live));
        if let Some((sources, input)) = self.reenter.take() {
            skills
                .set_skill(input, sources, self)
                .map_err(|_| "nested required effect failed")?;
        }
        if self.flip_world_after_grant {
            self.in_world = !self.in_world;
        }
        if let Some(level) = self.level_after_grant {
            self.level = Some(level);
        }
        Ok(())
    }
}
impl SkillSetEffects for Effects<'_> {
    type Error = &'static str;
    fn player_level(&self) -> u8 {
        self.level_queries.set(self.level_queries.get() + 1);
        self.level.unwrap_or(1)
    }
    fn is_in_world(&self) -> bool {
        self.world_queries.set(self.world_queries.get() + 1);
        self.in_world
    }
    fn update_enchantments(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        old: u16,
        new: u16,
    ) -> Result<(), Self::Error> {
        self.record(Event::Enchant(skill, old, new, skills.pure_value(skill)))
    }
    fn meets_player_condition(
        &mut self,
        _skills: &mut PlayerSkills,
        condition: u32,
    ) -> Result<bool, Self::Error> {
        self.record(Event::PlayerCondition(condition))?;
        Ok(self.deny_player != Some(condition))
    }
    fn meets_skill_ability_conditions(
        &mut self,
        _skills: &mut PlayerSkills,
        ability: u32,
    ) -> Result<bool, Self::Error> {
        self.record(Event::AbilityConditions(ability))?;
        Ok(self.deny_ability != Some(ability))
    }
    fn add_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        skill: u32,
    ) -> Result<bool, Self::Error> {
        self.grant(skills, spell, skill, false)?;
        Ok(!self.add_false)
    }
    fn learn_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        skill: u32,
    ) -> Result<(), Self::Error> {
        self.grant(skills, spell, skill, true)
    }
    fn update_mount_capability(&mut self, _skills: &mut PlayerSkills) -> Result<(), Self::Error> {
        self.record(Event::Mount)
    }
    fn update_criteria(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        criterion: SkillCriteria,
    ) -> Result<(), Self::Error> {
        self.record(Event::Criteria(skill, criterion, skills.pure_value(skill)))
    }
    fn refresh_bonus_auras(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        aura_type: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Aura(
            skill,
            aura_type,
            skills.status(skill).map(|(_, state)| state),
        ))?;
        if let Some((birth, bonus)) = self.bonus
            && aura_type == 30
        {
            skills
                .modify_bonus(birth, skill, bonus, false)
                .map_err(|_| "bonus mutation failed")?;
        }
        Ok(())
    }
    fn store_profession_item(
        &mut self,
        _skills: &mut PlayerSkills,
        slot: u8,
    ) -> Result<ProfessionItemMove, Self::Error> {
        self.record(Event::Store(slot))?;
        Ok(if self.full_at == Some(slot) {
            ProfessionItemMove::InventoryFull
        } else {
            ProfessionItemMove::StoredOrAbsent
        })
    }
    fn display_inventory_full(&mut self, _skills: &mut PlayerSkills) -> Result<(), Self::Error> {
        self.record(Event::Full)
    }
    fn remove_spell(&mut self, _skills: &mut PlayerSkills, spell: u32) -> Result<(), Self::Error> {
        self.record(Event::Remove(spell))
    }
}
