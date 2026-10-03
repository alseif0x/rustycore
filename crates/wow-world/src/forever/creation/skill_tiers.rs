//! Sole immutable owner of source tier values, not learned Player skills.
//! ObjectMgr.cpp:8016-8031,9005-9031 at 02245dcd.
use super::SourceError;
use std::collections::BTreeMap;
use wow_persistence::forever::creation::SkillTierRow;

pub(super) struct SkillTiers(BTreeMap<u32, [u32; 16]>);

impl SkillTiers {
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn load(rows: Vec<SkillTierRow>) -> Result<Self, SourceError> {
        let mut tiers = BTreeMap::new();
        for row in rows {
            // SQL primary-key uniqueness; duplicate startup DTO identities
            // indicate broken composition, not an arbitrary last-row policy.
            if tiers.insert(row.id, row.values).is_some() {
                return Err(SourceError::DuplicateIdentity);
            }
        }
        Ok(Self(tiers))
    }

    pub(super) fn value(&self, id: u32, index: u32) -> Option<u32> {
        let values = self.0.get(&id)?;
        let mut index = index.min(15) as usize;
        while values[index] == 0 && index > 0 {
            index -= 1;
        }
        Some(values[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_backtracking_clamps_before_search_and_does_not_narrow_u32() {
        let mut values = [0; 16];
        values[0] = 75;
        values[3] = 300;
        values[14] = u32::MAX;
        let tiers = SkillTiers::load(vec![SkillTierRow { id: 1, values }]).unwrap();
        for (index, expected) in [
            (0, 75),
            (2, 75),
            (3, 300),
            (13, 300),
            (14, u32::MAX),
            (15, u32::MAX),
            (u32::MAX, u32::MAX),
        ] {
            assert_eq!(tiers.value(1, index), Some(expected));
        }
    }

    #[test]
    fn empty_missing_and_known_zero_tiers_are_distinct() {
        let tiers = SkillTiers::load(vec![]).unwrap();
        assert_eq!(tiers.value(1, 0), None);
        let tiers = SkillTiers::load(vec![SkillTierRow {
            id: 0,
            values: [0; 16],
        }])
        .unwrap();
        assert_eq!(tiers.value(0, u32::MAX), Some(0));
        assert_eq!(tiers.value(1, 0), None);
    }

    #[test]
    fn duplicate_identity_is_not_a_silent_override() {
        let row = SkillTierRow {
            id: 1,
            values: [75; 16],
        };
        assert!(matches!(
            SkillTiers::load(vec![row, row]),
            Err(SourceError::DuplicateIdentity)
        ));
    }
}
