//! Selected immutable answers only. Real admission, indexing and default recipes stay in World/Data.
use super::*;
use crate::player::skills::test_catalog::Catalog;
pub(super) use crate::player::skills::test_catalog::line;
use crate::{
    SkillAbilityMetadata, SkillCatalog, SkillLineMetadata, SkillRaceClassMetadata,
    SkillRewardSpellMetadata,
};
use std::collections::BTreeMap;

pub(super) struct Fixture {
    pub birth: Catalog,
    pub selected_rc: BTreeMap<u32, SkillRaceClassMetadata>,
    pub tiers: BTreeMap<(u32, u32), u32>,
    pub defaults: Vec<SkillRaceClassMetadata>,
    pub default_answers: BTreeMap<(u32, u8), SkillUpdate>,
    pub spells: BTreeMap<u32, SkillRewardSpellMetadata>,
    pub first_spells: BTreeMap<u32, u32>,
}
impl Fixture {
    pub fn new(lines: Vec<SkillLineMetadata>, abilities: Vec<SkillAbilityMetadata>) -> Self {
        let spells = abilities
            .iter()
            .map(|ability| {
                (
                    ability.spell as u32,
                    SkillRewardSpellMetadata {
                        future_condition_id: 0,
                        spell_level: 0,
                        base_level: 0,
                    },
                )
            })
            .collect();
        Self {
            birth: Catalog::new(lines, abilities),
            selected_rc: BTreeMap::new(),
            tiers: BTreeMap::new(),
            defaults: vec![],
            default_answers: BTreeMap::new(),
            spells,
            first_spells: BTreeMap::new(),
        }
    }
    pub fn sources(&self) -> &Self {
        self
    }
}
impl SkillCatalog for Fixture {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        self.birth.skill_line(id)
    }
    fn child_lines(&self, parent: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        self.birth.child_lines(parent)
    }
    fn abilities_for_skill(&self, skill: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        self.birth.abilities_for_skill(skill)
    }
}
impl SkillSetSource for Fixture {
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
        Ok(self.defaults.iter().copied())
    }
    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::Error> {
        Ok(self.selected_rc.get(&skill).copied())
    }
    fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32> {
        self.tiers.get(&(tier, index)).copied()
    }
    fn default_skill_request(&self, rc: SkillRaceClassMetadata, level: u8) -> Option<SkillUpdate> {
        self.default_answers.get(&(rc.id, level)).copied()
    }
    fn reward_spell(&self, id: u32) -> Result<Option<SkillRewardSpellMetadata>, Self::Error> {
        Ok(self.spells.get(&id).copied())
    }
    fn first_spell_in_chain(&self, id: u32) -> u32 {
        self.first_spells.get(&id).copied().unwrap_or(id)
    }
    fn race_in_mask(&self, mask: u64, race: u32) -> bool {
        // These domain cases supply race 1; target race-ID mapping is covered by the real adapter.
        assert_eq!(race, 1);
        mask & 1 != 0
    }
}
pub(super) fn rc(id: u32, skill: u16, tier: i16) -> SkillRaceClassMetadata {
    SkillRaceClassMetadata {
        id,
        skill,
        flags: 0,
        min_level: 1,
        tier,
    }
}
pub(super) fn ability(id: u32, skill_line: u16, spell: u32) -> SkillAbilityMetadata {
    SkillAbilityMetadata {
        id,
        skill_line,
        spell: spell as i32,
        min_skill_rank: 0,
        class_mask: 0,
        acquire_method: 0,
        race_mask: 0,
    }
}
pub(super) fn rewards(skills: &[u16]) -> Vec<SkillAbilityMetadata> {
    skills
        .iter()
        .enumerate()
        .map(|(id, &skill)| {
            let mut row = ability(id as u32 + 1, skill, u32::from(skill) + 1000);
            row.acquire_method = 2;
            row
        })
        .collect()
}
pub(super) fn spell_fields(fixture: &mut Fixture, spell: u32, fields: Option<(u32, u32, u32)>) {
    if let Some((base_level, spell_level, future_condition_id)) = fields {
        fixture.spells.insert(
            spell,
            SkillRewardSpellMetadata {
                base_level,
                spell_level,
                future_condition_id,
            },
        );
    } else {
        fixture.spells.remove(&spell);
    }
}
pub(super) fn update(skill: u32, step: u16, value: u16, maximum: u16) -> SkillUpdate {
    SkillUpdate {
        skill,
        step,
        value,
        maximum,
    }
}
pub(super) fn allocated(ids: impl IntoIterator<Item = u32>) -> PlayerSkills {
    PlayerSkills::initialize(ids)
}
pub(super) fn activate(
    skills: &mut PlayerSkills,
    skill: u32,
    value: u16,
    maximum: u16,
    state: SkillUpdateState,
) {
    let status = skills.status.get_mut(&skill).unwrap();
    status.state = state;
    let field = &mut skills.fields[usize::from(status.slot)];
    field.step = 1;
    field.rank = value;
    field.maximum = maximum;
}
