//! Immutable presence catalog for build-70170 character admission.
//!
//! These are readable DB2 IDs, not a list of playable race/class combinations.
//! Availability additionally requires the target world's SQL requirements.
//! Schema anchors: 02245dcd DB2Metadata.h::{ChrClassesMeta,ChrRacesMeta,
//! AchievementMeta} and DB2LoadInfo.h. Actual locally acquired 70170/esES
//! character tables match their pinned hashes. Achievement's readable prefix
//! excludes nine unknown-key records, matching C++'s unknown TACT section skip.
//! The old character_progression field mappings are not used here.

use std::{collections::BTreeSet, path::Path};

use anyhow::{Result, ensure};

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, Copy)]
struct IdCatalogSchema {
    name: &'static str,
    table_hash: Option<u32>,
    layout_hash: u32,
    field_count: usize,
    inline_id_field: Option<usize>,
}

const CHR_CLASSES_SCHEMA: IdCatalogSchema = IdCatalogSchema {
    name: "ChrClasses",
    table_hash: Some(0xF588_9D8C),
    layout_hash: 0xAFC9_B0C2,
    field_count: 43,
    inline_id_field: Some(29),
};

const CHR_RACES_SCHEMA: IdCatalogSchema = IdCatalogSchema {
    name: "ChrRaces",
    table_hash: Some(0x53F1_783C),
    layout_hash: 0x4F44_C796,
    field_count: 51,
    inline_id_field: None,
};

/// `AchievementMeta::Instance` at DB2Metadata.h:110-118.
///
/// `Wdc4Reader` currently validates the WDC5 container and ID source, but it
/// does not expose the DB2 metadata's parent-field index. Keep this source
/// anchor explicit without pretending that `load` validates parent links.
pub const ACHIEVEMENT_FILE_DATA_ID: u32 = 1_260_179;
pub const ACHIEVEMENT_LAYOUT_HASH: u32 = 0x6FC5_281B;
pub const ACHIEVEMENT_FIELD_COUNT: usize = 19;
pub const ACHIEVEMENT_INLINE_ID_FIELD: usize = 3;
pub const ACHIEVEMENT_PARENT_INDEX_FIELD: usize = 11;

const ACHIEVEMENT_SCHEMA: IdCatalogSchema = IdCatalogSchema {
    name: "Achievement",
    // Observed in the actual local 70170/esES header, not guessed from retail.
    table_hash: Some(0xD2EE_2CA7),
    layout_hash: ACHIEVEMENT_LAYOUT_HASH,
    field_count: ACHIEVEMENT_FIELD_COUNT,
    inline_id_field: Some(ACHIEVEMENT_INLINE_ID_FIELD),
};

pub struct ForeverCharacterIds {
    classes: BTreeSet<u32>,
    races: BTreeSet<u32>,
}

/// Immutable Achievement.db2 ID presence, separate from race/class admission.
/// This is only a DB2 presence catalog; it does not validate achievement
/// semantics or make SQL unlock requirements playable.
pub struct ForeverAchievementIds {
    achievements: BTreeSet<u32>,
}

impl ForeverCharacterIds {
    /// Read already acquired private tables; no CASC, network or DB mutation.
    /// A matching schema is not proof of the installation's client build.
    pub fn load(directory: &Path) -> Result<Self> {
        Ok(Self {
            classes: load_ids(&directory.join("ChrClasses.db2"), CHR_CLASSES_SCHEMA)?,
            races: load_ids(&directory.join("ChrRaces.db2"), CHR_RACES_SCHEMA)?,
        })
    }

    pub fn classes(&self) -> &BTreeSet<u32> {
        &self.classes
    }

    pub fn races(&self) -> &BTreeSet<u32> {
        &self.races
    }
}

impl ForeverAchievementIds {
    /// Read the explicitly acquired available prefix, not a fake complete DB2.
    /// Unknown-key IDs remain absent, matching target DB2FileLoader's Skip path.
    pub fn load(directory: &Path) -> Result<Self> {
        Ok(Self {
            achievements: reader_ids(
                Wdc4Reader::open_available_achievement(
                    &directory.join("Achievement.available.db2"),
                )?,
                ACHIEVEMENT_SCHEMA,
            )?,
        })
    }

    pub fn contains(&self, achievement_id: u32) -> bool {
        self.achievements.contains(&achievement_id)
    }

    pub fn available_count(&self) -> usize {
        self.achievements.len()
    }
}

fn load_ids(path: &Path, schema: IdCatalogSchema) -> Result<BTreeSet<u32>> {
    reader_ids(Wdc4Reader::open(path)?, schema)
}

fn reader_ids(reader: Wdc4Reader, schema: IdCatalogSchema) -> Result<BTreeSet<u32>> {
    check_schema(
        (
            reader.format_version(),
            reader.table_hash(),
            reader.layout_hash(),
            reader.field_count(),
        ),
        schema,
    )?;
    ensure!(
        reader.inline_id_field() == schema.inline_id_field,
        "Wrong {} DB2 ID source",
        schema.name
    );
    let ids = unique_ids(reader.iter_records().map(|(id, _)| id), schema.name)?;
    ensure!(
        ids.len() == reader.total_count(),
        "Unresolved {} DB2 record copy",
        schema.name
    );
    Ok(ids)
}

fn check_schema(actual: (u32, u32, u32, usize), expected: IdCatalogSchema) -> Result<()> {
    ensure!(actual.0 == 5, "{} DB2 must use WDC5", expected.name);
    if let Some(table_hash) = expected.table_hash {
        ensure!(
            actual.1 == table_hash,
            "Wrong {} DB2 table hash",
            expected.name
        );
    }
    ensure!(
        actual.2 == expected.layout_hash,
        "Wrong {} DB2 layout hash",
        expected.name
    );
    ensure!(
        actual.3 == expected.field_count,
        "Wrong {} DB2 field count",
        expected.name
    );
    Ok(())
}

fn unique_ids(ids: impl Iterator<Item = u32>, table_name: &str) -> Result<BTreeSet<u32>> {
    let mut result = BTreeSet::new();
    for id in ids {
        ensure!(
            id != 0 && result.insert(id),
            "Zero or duplicate {} DB2 ID",
            table_name
        );
    }
    ensure!(!result.is_empty(), "Empty {} DB2 ID catalog", table_name);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_presence_only_and_sorted_without_playability_assumptions() {
        assert_eq!(
            unique_ids([96, 1, 95].into_iter(), "test").unwrap(),
            BTreeSet::from([1, 95, 96])
        );
    }

    #[test]
    fn absent_zero_and_duplicate_ids_fail_closed() {
        for ids in [vec![], vec![0], vec![1, 1]] {
            assert!(unique_ids(ids.into_iter(), "test").is_err());
        }
    }

    #[test]
    fn achievement_presence_is_separate_from_character_catalogs() {
        let catalog = ForeverAchievementIds {
            achievements: BTreeSet::from([1260179, 9001]),
        };
        assert!(catalog.contains(1260179));
        assert!(!catalog.contains(1260180));
    }

    #[test]
    fn target_schema_is_not_an_old_version_or_an_unchecked_layout() {
        let target = CHR_CLASSES_SCHEMA;
        assert!(
            check_schema(
                (5, 0xF588_9D8C, target.layout_hash, target.field_count),
                target
            )
            .is_ok()
        );
        for actual in [
            (4, 0xF588_9D8C, target.layout_hash, 43),
            (5, 0, target.layout_hash, 43),
            (5, 0xF588_9D8C, 0, 43),
            (5, 0xF588_9D8C, target.layout_hash, 42),
        ] {
            assert!(check_schema(actual, target).is_err());
        }
    }

    #[test]
    fn achievement_schema_pins_the_observed_table_hash() {
        assert_eq!(ACHIEVEMENT_SCHEMA.table_hash, Some(0xD2EE_2CA7));
        assert_eq!(ACHIEVEMENT_SCHEMA.layout_hash, 0x6FC5_281B);
        assert_eq!(ACHIEVEMENT_SCHEMA.field_count, 19);
        assert_eq!(ACHIEVEMENT_SCHEMA.inline_id_field, Some(3));
        assert_eq!(ACHIEVEMENT_PARENT_INDEX_FIELD, 11);
        assert!(
            check_schema(
                (
                    5,
                    0xD2EE_2CA7,
                    ACHIEVEMENT_LAYOUT_HASH,
                    ACHIEVEMENT_FIELD_COUNT
                ),
                ACHIEVEMENT_SCHEMA
            )
            .is_ok()
        );
        assert!(
            check_schema(
                (5, 0xDEAD_BEEF, 0, ACHIEVEMENT_FIELD_COUNT),
                ACHIEVEMENT_SCHEMA
            )
            .is_err()
        );
    }
}
