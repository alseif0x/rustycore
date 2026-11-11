//! Explicit admitted facts in the order presented to the domain, not a DB2 index builder.
use crate::{SkillAbilityMetadata, SkillCatalog, SkillLineMetadata};

pub(super) struct Catalog {
    lines: Vec<SkillLineMetadata>,
    abilities: Vec<SkillAbilityMetadata>,
}
impl Catalog {
    pub(super) fn new(lines: Vec<SkillLineMetadata>, abilities: Vec<SkillAbilityMetadata>) -> Self {
        Self { lines, abilities }
    }
}
impl SkillCatalog for Catalog {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        self.lines.iter().find(|line| line.id == id).copied()
    }
    fn child_lines(&self, parent: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        self.lines
            .iter()
            .filter(move |line| line.parent_skill == parent)
            .copied()
    }
    fn abilities_for_skill(&self, skill: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        self.abilities
            .iter()
            .filter(move |ability| u32::from(ability.skill_line) == skill)
            .copied()
    }
}
pub(super) fn line(
    id: u32,
    category: i8,
    parent_skill: u32,
    parent_tier_index: i32,
) -> SkillLineMetadata {
    SkillLineMetadata {
        id,
        category,
        parent_skill,
        parent_tier_index,
    }
}
