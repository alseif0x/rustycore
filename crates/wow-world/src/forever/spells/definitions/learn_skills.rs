//! 02245dcd SpellMgr.cpp:958-999, first Skill/DualWield effect in source order.
#[cfg(test)]
mod tests;
use super::{SpellDefinitionSeeds, SpellValueError};
use std::collections::BTreeMap;
use wow_data::forever_birth::item_records::ItemCatalog;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearnSkillNode {
    pub skill: u16,
    pub step: u16,
    pub value: u16,
    pub max_value: u16,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LearnSkillCounts {
    pub regular_definitions: usize,
    pub nodes: usize,
    pub skill_values: usize,
    pub dual_wield: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellLearnSkillError {
    RequiresRequiredSpells,
    AlreadyApplied,
    Value(SpellValueError),
}
pub(super) struct LoadedLearnSkills {
    nodes: BTreeMap<u32, SpellLearnSkillNode>,
    counts: LearnSkillCounts,
}
impl SpellDefinitionSeeds {
    pub fn with_learn_skills(
        mut self,
        items: &ItemCatalog,
        draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
    ) -> Result<Self, SpellLearnSkillError> {
        if self.learn_skills.is_some() {
            return Err(SpellLearnSkillError::AlreadyApplied);
        }
        if self.required.is_none() {
            return Err(SpellLearnSkillError::RequiresRequiredSpells);
        }
        let mut loaded = LoadedLearnSkills {
            nodes: BTreeMap::new(),
            counts: Default::default(),
        };
        for &key in &self
            .source_order
            .as_ref()
            .expect("required-spell admission follows source order")
            .primary
        {
            if key.1 != 0 {
                continue;
            }
            loaded.counts.regular_definitions += 1;
            let definition = &self.definitions[&key];
            for (slot, effect) in definition.effects.iter().enumerate() {
                let node = match effect.effect {
                    118 => {
                        let value = self
                            .calculate_startup_value(key.0, key.1, slot, items, None, draw)
                            .map_err(SpellLearnSkillError::Value)?
                            .expect("known definition/physical effect")
                            .as_int()
                            .map_err(SpellLearnSkillError::Value)?;
                        loaded.counts.skill_values += 1;
                        SpellLearnSkillNode {
                            skill: effect.misc_values[0] as u16,
                            step: value as u16,
                            value: 0,
                            max_value: 0,
                        }
                    }
                    40 => {
                        loaded.counts.dual_wield += 1;
                        SpellLearnSkillNode {
                            skill: 118,
                            step: 1,
                            value: 1,
                            max_value: 1,
                        }
                    }
                    _ => continue,
                };
                loaded.nodes.insert(key.0, node);
                loaded.counts.nodes += 1;
                break;
            }
        }
        self.learn_skills = Some(loaded);
        Ok(self)
    }
    pub fn learn_skill_counts(&self) -> Option<LearnSkillCounts> {
        self.learn_skills.as_ref().map(|l| l.counts)
    }
    pub fn spell_learn_skill(&self, id: u32) -> Option<&SpellLearnSkillNode> {
        self.learn_skills.as_ref()?.nodes.get(&id)
    }
}
