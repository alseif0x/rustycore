use crate::player::skill_sources::{
    SkillAbilityMetadata, SkillCatalog, SkillLineMetadata, SkillLoadSource, SkillRaceClassMetadata,
    SkillRewardSpellMetadata, SkillSetSource,
};
use crate::player::skills::{
    PlayerSkills, ProfessionItemMove, SkillCriteria, SkillFields, SkillLoadRow, SkillSetEffects,
    SkillUpdate, SkillUpdateState,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Sources {
    pub lines: BTreeMap<u32, SkillLineMetadata>,
    pub children: BTreeMap<u32, Vec<SkillLineMetadata>>,
    pub abilities: BTreeMap<u32, Vec<SkillAbilityMetadata>>,
    pub race_class: BTreeMap<u32, SkillRaceClassMetadata>,
    pub tiers: BTreeMap<u32, [u32; 16]>,
    pub rc_error: Option<u32>,
    pub reward_error: Option<u32>,
    pub missing_spells: BTreeSet<u32>,
    spell_metadata: BTreeMap<u32, SkillRewardSpellMetadata>,
}

impl Sources {
    pub fn add_line(&mut self, id: u32, category: i8, parent_skill: u32, parent_tier_index: i32) {
        self.lines.insert(
            id,
            SkillLineMetadata {
                id,
                category,
                parent_skill,
                parent_tier_index,
            },
        );
    }
    pub fn add_rc(&mut self, skill: u16, tier: i16) {
        let id = u32::from(skill);
        self.race_class.insert(
            id,
            SkillRaceClassMetadata {
                id,
                skill,
                flags: 0,
                min_level: 1,
                tier,
            },
        );
    }
    pub fn add_child(&mut self, parent: u32, line: SkillLineMetadata) {
        self.lines.insert(line.id, line);
        self.children.entry(parent).or_default().push(line);
    }
    pub fn add_ability(&mut self, skill: u32, spell: u32, minimum_rank: i16) {
        self.abilities
            .entry(skill)
            .or_default()
            .push(SkillAbilityMetadata {
                id: spell,
                skill_line: skill as u16,
                spell: spell as i32,
                min_skill_rank: minimum_rank,
                class_mask: 0,
                acquire_method: 2,
                race_mask: 0,
            });
    }
    pub fn spell_metadata(&mut self) -> &mut BTreeMap<u32, SkillRewardSpellMetadata> {
        &mut self.spell_metadata
    }
    fn tier_value(&self, tier: u32, index: u32) -> Option<u32> {
        let values = self.tiers.get(&tier)?;
        let mut index = index.min(15) as usize;
        while values[index] == 0 && index > 0 {
            index -= 1;
        }
        Some(values[index])
    }
}

impl SkillCatalog for Sources {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        self.lines.get(&id).copied()
    }
    fn child_lines(&self, parent: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        self.children.get(&parent).into_iter().flatten().copied()
    }
    fn abilities_for_skill(&self, skill: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        self.abilities.get(&skill).into_iter().flatten().copied()
    }
}

impl SkillSetSource for Sources {
    type Error = &'static str;
    fn race(&self) -> u8 {
        1
    }
    fn class(&self) -> u8 {
        1
    }
    fn default_skill_records(
        &self,
    ) -> Result<impl Iterator<Item = SkillRaceClassMetadata> + '_, Self::Error> {
        Ok(std::iter::empty())
    }
    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::Error> {
        if self.rc_error == Some(skill) {
            return Err("skill race/class source failed");
        }
        Ok(self.race_class.get(&skill).copied())
    }
    fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32> {
        self.tier_value(tier, index)
    }
    fn default_skill_request(&self, _: SkillRaceClassMetadata, _: u8) -> Option<SkillUpdate> {
        None
    }
    fn reward_spell(&self, id: u32) -> Result<Option<SkillRewardSpellMetadata>, Self::Error> {
        if self.reward_error == Some(id) {
            return Err("reward source failed");
        }
        if self.missing_spells.contains(&id) {
            return Ok(None);
        }
        Ok(Some(self.spell_metadata.get(&id).copied().unwrap_or(
            SkillRewardSpellMetadata {
                future_condition_id: 0,
                spell_level: 1,
                base_level: 1,
            },
        )))
    }
    fn first_spell_in_chain(&self, id: u32) -> u32 {
        id
    }
    fn race_in_mask(&self, mask: u64, race: u32) -> bool {
        race == 1 && mask & 1 != 0
    }
}

impl SkillLoadSource for Sources {
    fn skill_tier_values(&self, tier: u32) -> Option<&[u32; 16]> {
        self.tiers.get(&tier)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Learned(u32, u32),
    Added(u32, u32),
    Removed(u32),
}

pub(super) struct Effects<'a> {
    pub level: u8,
    pub in_world: bool,
    pub events: Vec<Event>,
    pub observe_skill: Option<u32>,
    pub observed_skill: Option<bool>,
    pub level_after_reward: Option<u8>,
    pub reenter: Option<SkillUpdate>,
    pub fail_learning: bool,
    pub requests: usize,
    sources: &'a Sources,
}

impl<'a> Effects<'a> {
    pub fn new(sources: &'a Sources, level: u8) -> Self {
        Self {
            level,
            in_world: true,
            events: Vec::new(),
            observe_skill: None,
            observed_skill: None,
            level_after_reward: None,
            reenter: None,
            fail_learning: false,
            requests: 0,
            sources,
        }
    }
}

impl SkillSetEffects for Effects<'_> {
    type Error = &'static str;
    fn request_client_update(&mut self) {
        self.requests += 1;
    }
    fn player_level(&self) -> u8 {
        self.level
    }
    fn is_in_world(&self) -> bool {
        self.in_world
    }
    fn update_enchantments(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: u16,
        _: u16,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn meets_player_condition(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn meets_skill_ability_conditions(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn add_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        from_skill: u32,
    ) -> Result<bool, Self::Error> {
        self.events.push(Event::Added(from_skill, spell));
        self.observe(skills);
        Ok(true)
    }
    fn learn_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        from_skill: u32,
    ) -> Result<(), Self::Error> {
        self.events.push(Event::Learned(from_skill, spell));
        self.observe(skills);
        if let Some(level) = self.level_after_reward.take() {
            self.level = level;
        }
        if let Some(update) = self.reenter.take() {
            let sources = self.sources;
            let _ = skills
                .set_skill(update, sources, self)
                .map_err(|_| "reentrant SetSkill failed")?;
        }
        if self.fail_learning {
            return Err("reward effect failed");
        }
        Ok(())
    }
    fn update_mount_capability(&mut self, _: &mut PlayerSkills) -> Result<(), Self::Error> {
        Ok(())
    }
    fn update_criteria(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: SkillCriteria,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn refresh_bonus_auras(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn store_profession_item(
        &mut self,
        _: &mut PlayerSkills,
        _: u8,
    ) -> Result<ProfessionItemMove, Self::Error> {
        Ok(ProfessionItemMove::StoredOrAbsent)
    }
    fn display_inventory_full(&mut self, _: &mut PlayerSkills) -> Result<(), Self::Error> {
        Ok(())
    }
    fn remove_spell(&mut self, _: &mut PlayerSkills, spell: u32) -> Result<(), Self::Error> {
        self.events.push(Event::Removed(spell));
        Ok(())
    }
}

impl Effects<'_> {
    fn observe(&mut self, skills: &PlayerSkills) {
        if let Some(skill) = self.observe_skill {
            self.observed_skill = Some(skills.has_skill(skill));
        }
    }
}

pub(super) fn row(skill: u16, value: u16, maximum: u16, profession_slot: i8) -> SkillLoadRow {
    SkillLoadRow {
        skill,
        value,
        maximum,
        profession_slot,
    }
}

pub(super) fn ordered(history: &[u32], count: usize) -> Result<Vec<u32>, &'static str> {
    assert_eq!(history.len(), count);
    Ok(history.to_vec())
}

pub(super) fn seed(
    skills: &mut PlayerSkills,
    skill: u32,
    step: u16,
    rank: u16,
    maximum: u16,
    temporary_bonus: i16,
    permanent_bonus: u16,
    state: SkillUpdateState,
) {
    let slot = if let Some(status) = skills.status.get(&skill) {
        usize::from(status.slot)
    } else {
        let slot = skills.status.len();
        skills.try_insert_status(
            skill,
            super::super::Status {
                slot: slot as u16,
                state,
            },
        );
        slot
    };
    skills.status.get_mut(&skill).unwrap().state = state;
    skills.fields[slot] = SkillFields {
        line: skill as u16,
        step,
        rank,
        starting_rank: 1,
        maximum,
        temporary_bonus,
        permanent_bonus,
    };
}
