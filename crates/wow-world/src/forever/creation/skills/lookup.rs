//! 02245dcd DB2Stores.cpp:445,1546-1548,3038-3059.
//! The first matching RC record follows source unordered_multimap equal_range,
//! not storage ID or specificity. The composition adapter supplies IDs only.
use super::{SkillRaceClassRecord, SourceError, WorldSources, matches};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillLookupCounts {
    pub records: usize,
    pub skills: usize,
    pub orphan_records: usize,
}
pub(super) struct RaceClassLookup {
    by_skill: BTreeMap<u32, Vec<u32>>,
    counts: SkillLookupCounts,
}

impl WorldSources {
    /// Consumes an exact-set source replay. Only equal-key ordering is source
    /// observable; groups must be contiguous and ascending by skill in this ABI.
    /// No raw rows are cloned or inferred. Admission checks identity/membership,
    /// not independent proof of the native producer's toolchain/order contract.
    pub fn with_birth_skill_lookup(mut self, order: Vec<u32>) -> Result<Self, SourceError> {
        let sources = self
            .skill_sources
            .as_mut()
            .ok_or(SourceError::MissingBirthSkillSources)?;
        if sources.lookup.is_some() {
            return Err(SourceError::BirthSkillLookupAlreadyLoaded);
        }
        let eligible = sources
            .birth
            .race_class_records()
            .filter(|row| sources.birth.skill_line(u32::from(row.skill)).is_some())
            .count();
        if order.len() != eligible {
            return Err(SourceError::InvalidBirthSkillLookup);
        }
        let mut seen = BTreeSet::new();
        let mut by_skill = BTreeMap::<u32, Vec<u32>>::new();
        let mut previous = None;
        for id in order {
            let row = sources
                .birth
                .race_class_record(id)
                .ok_or(SourceError::InvalidBirthSkillLookup)?;
            let skill = u32::from(row.skill);
            if sources.birth.skill_line(skill).is_none()
                || !seen.insert(id)
                || previous.is_some_and(|before| before > skill)
            {
                return Err(SourceError::InvalidBirthSkillLookup);
            }
            by_skill.entry(skill).or_default().push(id);
            previous = Some(skill);
        }
        let counts = SkillLookupCounts {
            records: eligible,
            skills: by_skill.len(),
            orphan_records: sources.birth.counts()[1] - eligible,
        };
        sources.lookup = Some(RaceClassLookup { by_skill, counts });
        Ok(self)
    }

    /// Source query: first matching race/class row, with no availability,
    /// MinLevel, tier, rank or "most specific" preference added by the reader.
    /// Class validation protects the same source shift used by Player callers.
    pub fn skill_race_class_info(
        &self,
        skill: u32,
        race: u8,
        class: u8,
    ) -> Result<Option<&SkillRaceClassRecord>, SourceError> {
        if !(1..=15).contains(&class) {
            return Err(SourceError::InvalidClass);
        }
        let sources = self
            .skill_sources
            .as_ref()
            .ok_or(SourceError::MissingBirthSkillSources)?;
        let lookup = sources
            .lookup
            .as_ref()
            .ok_or(SourceError::MissingBirthSkillLookup)?;
        Ok(lookup
            .by_skill
            .get(&skill)
            .into_iter()
            .flatten()
            .map(|id| {
                sources
                    .birth
                    .race_class_record(*id)
                    .expect("admitted immutable RC identity")
            })
            .find(|row| matches(row, race, class)))
    }

    pub fn birth_skill_lookup_counts(&self) -> Option<SkillLookupCounts> {
        self.skill_sources
            .as_ref()?
            .lookup
            .as_ref()
            .map(|lookup| lookup.counts)
    }
}
