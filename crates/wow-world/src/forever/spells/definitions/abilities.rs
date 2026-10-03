//! SpellMgr.cpp:1952-1963 skill-line multimap and :119-127 membership query.
//! 02245dcd: all final relations, not only admitted SpellInfo or race/class rows.
#[cfg(test)]
mod tests;
use super::{SpellDefinitionError, SpellDefinitionSeeds};
use std::{collections::BTreeMap, sync::Arc};
use wow_data::forever_birth::{BirthCatalog, SkillAbilityRecord};

pub(super) struct SkillLineAbilityIndex {
    catalog: Arc<BirthCatalog>,
    by_spell: BTreeMap<u32, Vec<u32>>,
    counts: SkillLineAbilityCounts,
}
impl SkillLineAbilityIndex {
    pub(super) fn catalog(&self) -> &BirthCatalog {
        &self.catalog
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillLineAbilityCounts {
    pub relations: usize,
    pub spell_ids: usize,
    /// Baseline uncertainty, not a count of missing final spell identities.
    pub unavailable_baseline_abilities: usize,
}

impl SpellDefinitionSeeds {
    pub(crate) fn skill_birth_catalog(&self) -> Option<&BirthCatalog> {
        self.skill_line_abilities
            .as_ref()
            .map(|index| index.catalog())
    }

    /// Source World.cpp:1390-1396 startup phase: complete corrections, then
    /// this map, then custom attributes. No record clone, SQL or Player owner.
    pub fn with_skill_line_abilities(
        mut self,
        catalog: Arc<BirthCatalog>,
    ) -> Result<Self, SpellDefinitionError> {
        if self.global_corrections.is_none() || self.source_order.is_none() {
            return Err(SpellDefinitionError::SkillLineAbilitiesRequireGlobalCorrections);
        }
        if self.skill_line_abilities.is_some() {
            return Err(SpellDefinitionError::SkillLineAbilitiesAlreadyLoaded);
        }
        let mut by_spell = BTreeMap::<u32, Vec<u32>>::new();
        let mut counts = SkillLineAbilityCounts::default();
        for row in catalog.skill_ability_records() {
            // Source signed Spell field -> uint32 multimap key. No existence,
            // class/race/rank/availability or nonzero-spell filtering here.
            by_spell.entry(row.spell as u32).or_default().push(row.id);
            counts.relations += 1;
        }
        counts.spell_ids = by_spell.len();
        counts.unavailable_baseline_abilities = catalog.counts()[5];
        self.skill_line_abilities = Some(SkillLineAbilityIndex {
            catalog,
            by_spell,
            counts,
        });
        Ok(self)
    }

    pub fn skill_line_ability_counts(&self) -> Option<SkillLineAbilityCounts> {
        self.skill_line_abilities.as_ref().map(|index| index.counts)
    }

    /// None before map admission, Some(empty) for an absent key afterwards.
    /// std::multimap preserves storage insertion order among equal Spell keys.
    /// These are borrowed final DB2 relations, not learned Player spells.
    pub fn skill_line_abilities(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = &SkillAbilityRecord>> {
        let index = self.skill_line_abilities.as_ref()?;
        Some(
            index
                .by_spell
                .get(&spell)
                .into_iter()
                .flatten()
                .map(move |id| {
                    index
                        .catalog
                        .skill_ability(*id)
                        .expect("admitted immutable skill ability")
                }),
        )
    }

    /// Source IsPartOfSkillLine compares SkillLine, NOT SkillupSkillLineID.
    pub fn is_part_of_skill_line(&self, skill: u32, spell: u32) -> Option<bool> {
        Some(
            self.skill_line_abilities(spell)?
                .any(|row| u32::from(row.skill_line) == skill),
        )
    }
}
