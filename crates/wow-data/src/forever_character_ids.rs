//! Immutable presence catalog for build-70170 character admission.
//!
//! These are *all DB2 IDs*, not a list of playable race/class combinations.
//! Availability additionally requires the target world's SQL requirements.
//! Schema anchors: 02245dcd DB2Metadata.h::{ChrClassesMeta,ChrRacesMeta} and
//! DB2LoadInfo.h. Actual locally acquired 70170/esES tables match these hashes.
//! The old character_progression field mappings are not used here.

use std::{collections::BTreeSet, path::Path};

use anyhow::{Result, ensure};

use crate::wdc4::Wdc4Reader;

pub struct ForeverCharacterIds {
    classes: BTreeSet<u32>,
    races: BTreeSet<u32>,
}

impl ForeverCharacterIds {
    /// Read already acquired private tables; no CASC, network or DB mutation.
    /// A matching schema is not proof of the installation's client build.
    pub fn load(directory: &Path) -> Result<Self> {
        Ok(Self {
            classes: load_ids(
                &directory.join("ChrClasses.db2"),
                0xF588_9D8C,
                0xAFC9_B0C2,
                43,
                Some(29),
            )?,
            races: load_ids(
                &directory.join("ChrRaces.db2"),
                0x53F1_783C,
                0x4F44_C796,
                51,
                None,
            )?,
        })
    }

    pub fn classes(&self) -> &BTreeSet<u32> {
        &self.classes
    }

    pub fn races(&self) -> &BTreeSet<u32> {
        &self.races
    }
}

fn load_ids(
    path: &Path,
    table: u32,
    layout: u32,
    fields: usize,
    inline_id: Option<usize>,
) -> Result<BTreeSet<u32>> {
    let reader = Wdc4Reader::open(path)?;
    check_schema(
        (
            reader.format_version(),
            reader.table_hash(),
            reader.layout_hash(),
            reader.field_count(),
        ),
        (table, layout, fields),
    )?;
    ensure!(
        reader.inline_id_field() == inline_id,
        "Wrong character DB2 ID source"
    );
    let ids = unique_ids(reader.iter_records().map(|(id, _)| id))?;
    ensure!(
        ids.len() == reader.total_count(),
        "Unresolved character DB2 record copy"
    );
    Ok(ids)
}

fn check_schema(actual: (u32, u32, u32, usize), expected: (u32, u32, usize)) -> Result<()> {
    ensure!(actual.0 == 5, "Character DB2 must use WDC5");
    ensure!(actual.1 == expected.0, "Wrong character DB2 table hash");
    ensure!(actual.2 == expected.1, "Wrong character DB2 layout hash");
    ensure!(actual.3 == expected.2, "Wrong character DB2 field count");
    Ok(())
}

fn unique_ids(ids: impl Iterator<Item = u32>) -> Result<BTreeSet<u32>> {
    let mut result = BTreeSet::new();
    for id in ids {
        ensure!(
            id != 0 && result.insert(id),
            "Zero or duplicate character DB2 ID"
        );
    }
    ensure!(!result.is_empty(), "Empty character DB2 ID catalog");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_presence_only_and_sorted_without_playability_assumptions() {
        assert_eq!(
            unique_ids([96, 1, 95].into_iter()).unwrap(),
            BTreeSet::from([1, 95, 96])
        );
    }

    #[test]
    fn absent_zero_and_duplicate_ids_fail_closed() {
        for ids in [vec![], vec![0], vec![1, 1]] {
            assert!(unique_ids(ids.into_iter()).is_err());
        }
    }

    #[test]
    fn target_schema_is_not_an_old_version_or_an_unchecked_layout() {
        let target = (0xF588_9D8C, 0xAFC9_B0C2, 43);
        assert!(check_schema((5, target.0, target.1, target.2), target).is_ok());
        for actual in [
            (4, target.0, target.1, 43),
            (5, 0, target.1, 43),
            (5, target.0, 0, 43),
            (5, target.0, target.1, 42),
        ] {
            assert!(check_schema(actual, target).is_err());
        }
    }
}
