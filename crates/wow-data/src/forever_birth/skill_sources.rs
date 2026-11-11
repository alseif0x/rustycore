//! Actual final BirthCatalog adapter, without full SetSkill readiness requirements.
use super::BirthCatalog;
use wow_entities::{SkillAbilityMetadata, SkillCatalog, SkillLineMetadata};

impl SkillCatalog for BirthCatalog {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        BirthCatalog::skill_line(self, id).map(|line| SkillLineMetadata {
            id: line.id,
            category: line.category,
            parent_skill: line.parent_skill,
            parent_tier_index: line.parent_tier_index,
        })
    }

    fn child_lines(&self, parent: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        BirthCatalog::child_lines(self, parent).map(|line| SkillLineMetadata {
            id: line.id,
            category: line.category,
            parent_skill: line.parent_skill,
            parent_tier_index: line.parent_tier_index,
        })
    }

    fn abilities_for_skill(&self, skill: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        BirthCatalog::abilities_for_skill(self, skill).map(|ability| SkillAbilityMetadata {
            id: ability.id,
            skill_line: ability.skill_line,
            spell: ability.spell,
            min_skill_rank: ability.min_skill_rank,
            class_mask: ability.class_mask,
            acquire_method: ability.acquire_method,
            race_mask: ability.race_mask,
        })
    }
}
