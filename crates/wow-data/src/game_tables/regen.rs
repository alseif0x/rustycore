// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3

//! C++ `DataStores/GameTables.*` regeneration tables.
//!
//! This module owns the `RegenMPPerSpt`, `RegenHPPerSpt` and `OCTRegenHP`
//! tables used by `Player::OCTRegenMPPerSpirit` and
//! `Player::OCTRegenHPPerSpirit`.  All three share the same 11 class columns
//! and are indexed by row position rather than by the explicit Level column,
//! matching `LoadGameTable`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::parse_float_like_cpp;

/// Class-column count shared by every regeneration game table.
///
/// C++ `GtOCTRegenHPEntry`, `GtRegenHPPerSptEntry` and `GtRegenMPPerSptEntry`
/// each carry one `float` for playable classes 1..11.
pub const REGEN_CLASS_COLUMN_COUNT: usize = 11;

/// C++ `GetRegenGameTableColumnForClass` column index for a class id.
///
/// Classes 1..11 map to columns 0..10; any other class has no column.
fn regen_column_index_for_class_like_cpp(class: u8) -> Option<usize> {
    match class {
        1 => Some(0),   // Warrior
        2 => Some(1),   // Paladin
        3 => Some(2),   // Hunter
        4 => Some(3),   // Rogue
        5 => Some(4),   // Priest
        6 => Some(5),   // Death Knight
        7 => Some(6),   // Shaman
        8 => Some(7),   // Mage
        9 => Some(8),   // Warlock
        10 => Some(9),  // Monk
        11 => Some(10), // Druid
        _ => None,
    }
}

/// Shared parser for the regeneration tables.
///
/// Mirrors C++ `LoadGameTable`: the explicit first column is discarded, row 0
/// is a default unused entry and the row position is the lookup key.  The
/// tables are identical apart from their expected value column count, which is
/// always [`REGEN_CLASS_COLUMN_COUNT`].
fn parse_regen_rows_like_cpp(
    content: &str,
    path: &Path,
    value_column_count: usize,
) -> Result<Vec<[f32; REGEN_CLASS_COLUMN_COUNT]>> {
    let mut lines = content.lines();
    let Some(headers) = lines.next() else {
        bail!("GameTable file {} is empty.", path.display());
    };

    let column_defs: Vec<&str> = headers
        .split('\t')
        .filter(|part| !part.is_empty())
        .collect();
    if column_defs.len().saturating_sub(1) != value_column_count {
        bail!(
            "GameTable '{}' has different count of columns {} than expected by size of C++ structure ({}).",
            path.display(),
            column_defs.len().saturating_sub(1),
            value_column_count
        );
    }

    let mut rows = vec![[0.0f32; REGEN_CLASS_COLUMN_COUNT]];
    for raw_line in lines {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let mut values: Vec<&str> = line.split('\t').collect();
        if values.is_empty() || (values.len() == 1 && values[0].is_empty()) {
            break;
        }
        while values.len() > 1 && values.last().is_some_and(|value| value.is_empty()) {
            values.pop();
        }
        if values.len() <= 1 {
            break;
        }
        if values.len() != column_defs.len() {
            bail!("{} == {}", values.len(), column_defs.len());
        }
        let mut columns = [0.0f32; REGEN_CLASS_COLUMN_COUNT];
        for (column, raw_value) in columns.iter_mut().zip(values.iter().skip(1)) {
            *column = parse_float_like_cpp(raw_value);
        }
        rows.push(columns);
    }

    Ok(rows)
}

/// C++ `GtRegenMPPerSptEntry`.
///
/// The columns follow the order in TrinityCore's `GameTables.h`, while the
/// explicit Level column is discarded just like `LoadGameTable` does.  The
/// row position, rather than the value in that column, is the lookup key.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RegenMpPerSptEntryLikeCpp {
    columns: [f32; REGEN_CLASS_COLUMN_COUNT],
}

impl RegenMpPerSptEntryLikeCpp {
    pub fn from_columns(columns: [f32; REGEN_CLASS_COLUMN_COUNT]) -> Self {
        Self { columns }
    }

    /// C++ `GetRegenGameTableColumnForClass` for the MP-per-spirit table.
    pub fn mana_regen_ratio_for_class_like_cpp(&self, class: u8) -> f32 {
        regen_column_index_for_class_like_cpp(class)
            .map(|column| self.columns[column])
            .unwrap_or(0.0)
    }
}

/// C++ `sRegenMPPerSptGameTable` used by `Player::OCTRegenMPPerSpirit`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegenMpPerSptGameTableLikeCpp {
    rows: Vec<RegenMpPerSptEntryLikeCpp>,
}

impl RegenMpPerSptGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "RegenMPPerSpt.txt";
    pub const VALUE_COLUMN_COUNT: usize = REGEN_CLASS_COLUMN_COUNT;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = RegenMpPerSptEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(RegenMpPerSptEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, level: u16) -> Option<&RegenMpPerSptEntryLikeCpp> {
        self.rows.get(usize::from(level))
    }

    pub fn mana_regen_ratio_like_cpp(&self, level: u16, class: u8) -> f32 {
        self.row(level)
            .map(|row| row.mana_regen_ratio_for_class_like_cpp(class))
            .unwrap_or(0.0)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn parse_like_cpp(content: &str, path: &Path) -> Result<Self> {
        let rows = parse_regen_rows_like_cpp(content, path, Self::VALUE_COLUMN_COUNT)?;
        Ok(Self {
            rows: rows
                .into_iter()
                .map(RegenMpPerSptEntryLikeCpp::from_columns)
                .collect(),
        })
    }
}

/// C++ `GtRegenHPPerSptEntry`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RegenHpPerSptEntryLikeCpp {
    columns: [f32; REGEN_CLASS_COLUMN_COUNT],
}

impl RegenHpPerSptEntryLikeCpp {
    pub fn from_columns(columns: [f32; REGEN_CLASS_COLUMN_COUNT]) -> Self {
        Self { columns }
    }

    /// C++ `GetRegenGameTableColumnForClass` for the HP-per-spirit table.
    pub fn health_regen_ratio_for_class_like_cpp(&self, class: u8) -> f32 {
        regen_column_index_for_class_like_cpp(class)
            .map(|column| self.columns[column])
            .unwrap_or(0.0)
    }
}

/// C++ `sRegenHPPerSptGameTable` used by `Player::OCTRegenHPPerSpirit`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegenHpPerSptGameTableLikeCpp {
    rows: Vec<RegenHpPerSptEntryLikeCpp>,
}

impl RegenHpPerSptGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "RegenHPPerSpt.txt";
    pub const VALUE_COLUMN_COUNT: usize = REGEN_CLASS_COLUMN_COUNT;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = RegenHpPerSptEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(RegenHpPerSptEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, level: u16) -> Option<&RegenHpPerSptEntryLikeCpp> {
        self.rows.get(usize::from(level))
    }

    pub fn health_regen_ratio_like_cpp(&self, level: u16, class: u8) -> f32 {
        self.row(level)
            .map(|row| row.health_regen_ratio_for_class_like_cpp(class))
            .unwrap_or(0.0)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn parse_like_cpp(content: &str, path: &Path) -> Result<Self> {
        let rows = parse_regen_rows_like_cpp(content, path, Self::VALUE_COLUMN_COUNT)?;
        Ok(Self {
            rows: rows
                .into_iter()
                .map(RegenHpPerSptEntryLikeCpp::from_columns)
                .collect(),
        })
    }
}

/// C++ `GtOCTRegenHPEntry`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OctRegenHpEntryLikeCpp {
    columns: [f32; REGEN_CLASS_COLUMN_COUNT],
}

impl OctRegenHpEntryLikeCpp {
    pub fn from_columns(columns: [f32; REGEN_CLASS_COLUMN_COUNT]) -> Self {
        Self { columns }
    }

    /// C++ `GetRegenGameTableColumnForClass` for the out-of-combat HP table.
    pub fn oct_regen_hp_ratio_for_class_like_cpp(&self, class: u8) -> f32 {
        regen_column_index_for_class_like_cpp(class)
            .map(|column| self.columns[column])
            .unwrap_or(0.0)
    }
}

/// C++ `sOCTRegenHPGameTable` used by `Player::OCTRegenHPPerSpirit`.
#[derive(Debug, Clone, PartialEq)]
pub struct OctRegenHpGameTableLikeCpp {
    rows: Vec<OctRegenHpEntryLikeCpp>,
}

impl OctRegenHpGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "OCTRegenHP.txt";
    pub const VALUE_COLUMN_COUNT: usize = REGEN_CLASS_COLUMN_COUNT;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = OctRegenHpEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(OctRegenHpEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, level: u16) -> Option<&OctRegenHpEntryLikeCpp> {
        self.rows.get(usize::from(level))
    }

    pub fn oct_regen_hp_ratio_like_cpp(&self, level: u16, class: u8) -> f32 {
        self.row(level)
            .map(|row| row.oct_regen_hp_ratio_for_class_like_cpp(class))
            .unwrap_or(0.0)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn parse_like_cpp(content: &str, path: &Path) -> Result<Self> {
        let rows = parse_regen_rows_like_cpp(content, path, Self::VALUE_COLUMN_COUNT)?;
        Ok(Self {
            rows: rows
                .into_iter()
                .map(OctRegenHpEntryLikeCpp::from_columns)
                .collect(),
        })
    }
}

/// Bundle of the three regeneration tables used by `Player::OCTRegenHPPerSpirit`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegenGameTablesLikeCpp {
    pub regen_mp_per_spt: RegenMpPerSptGameTableLikeCpp,
    pub regen_hp_per_spt: RegenHpPerSptGameTableLikeCpp,
    pub oct_regen_hp: OctRegenHpGameTableLikeCpp,
}

impl RegenGameTablesLikeCpp {
    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            regen_mp_per_spt: RegenMpPerSptGameTableLikeCpp::load(&data_dir)?,
            regen_hp_per_spt: RegenHpPerSptGameTableLikeCpp::load(&data_dir)?,
            oct_regen_hp: OctRegenHpGameTableLikeCpp::load(&data_dir)?,
        })
    }

    pub fn from_tables(
        regen_mp_per_spt: RegenMpPerSptGameTableLikeCpp,
        regen_hp_per_spt: RegenHpPerSptGameTableLikeCpp,
        oct_regen_hp: OctRegenHpGameTableLikeCpp,
    ) -> Self {
        Self {
            regen_mp_per_spt,
            regen_hp_per_spt,
            oct_regen_hp,
        }
    }

    pub fn mana_regen_ratio_like_cpp(&self, level: u16, class: u8) -> f32 {
        self.regen_mp_per_spt
            .mana_regen_ratio_like_cpp(level, class)
    }

    /// C++ `Player::OCTRegenHPPerSpirit`.
    pub fn oct_regen_hp_per_spirit_like_cpp(&self, level: u16, class: u8, spirit: f32) -> f32 {
        let (Some(base_gt), Some(more_gt)) = (
            self.oct_regen_hp.row(level),
            self.regen_hp_per_spt.row(level),
        ) else {
            return 0.0;
        };

        let base_ratio = base_gt.oct_regen_hp_ratio_for_class_like_cpp(class);
        let more_ratio = more_gt.health_regen_ratio_for_class_like_cpp(class);

        // Formula from PaperDollFrame script.
        let base_spirit = spirit.min(50.0);
        let more_spirit = spirit - base_spirit;
        base_spirit * base_ratio + more_spirit * more_ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_regen_table(prefix: &str, file_name: &str, content: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        dir.push(format!(
            "rustycore-{prefix}-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(dir.join("gt")).expect("create temp gt dir");
        let path = dir.join("gt").join(file_name);
        fs::write(&path, content).expect("write temp regen table");
        dir
    }

    #[test]
    fn regen_hp_per_spt_game_table_maps_level_and_class_like_cpp() {
        let dir = write_temp_regen_table(
            "regen-hp-per-spt",
            RegenHpPerSptGameTableLikeCpp::FILE_NAME,
            "Level\tWarrior\tPaladin\tHunter\tRogue\tPriest\tDeath Knight\tShaman\tMage\tWarlock\tMonk\tDruid\r\n\
             80\t0.01\t0.02\t0.03\t0.04\t0.05\t0.06\t0.07\t0.08\t0.09\t0.10\t0.11\r\n",
        );
        let table = RegenHpPerSptGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.len(), 2);
        assert_eq!(table.health_regen_ratio_like_cpp(0, 5), 0.0);
        assert_eq!(table.health_regen_ratio_like_cpp(1, 1), 0.01);
        assert_eq!(table.health_regen_ratio_like_cpp(1, 5), 0.05);
        assert_eq!(table.health_regen_ratio_like_cpp(1, 11), 0.11);
        assert_eq!(table.health_regen_ratio_like_cpp(1, 12), 0.0);
        assert_eq!(table.health_regen_ratio_like_cpp(80, 5), 0.0);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn regen_hp_per_spt_game_table_rejects_wrong_column_count_like_cpp() {
        let dir = write_temp_regen_table(
            "regen-hp-per-spt-bad",
            RegenHpPerSptGameTableLikeCpp::FILE_NAME,
            "Level\tWarrior\n1\t0.1\n",
        );
        let err = RegenHpPerSptGameTableLikeCpp::load(&dir).expect_err("column mismatch");

        assert!(err.to_string().contains("different count of columns"));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn oct_regen_hp_game_table_maps_level_and_class_like_cpp() {
        let dir = write_temp_regen_table(
            "oct-regen-hp",
            OctRegenHpGameTableLikeCpp::FILE_NAME,
            "Level\tWarrior\tPaladin\tHunter\tRogue\tPriest\tDeath Knight\tShaman\tMage\tWarlock\tMonk\tDruid\r\n\
             80\t0.01\t0.02\t0.03\t0.04\t0.05\t0.06\t0.07\t0.08\t0.09\t0.10\t0.11\r\n",
        );
        let table = OctRegenHpGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.len(), 2);
        assert_eq!(table.oct_regen_hp_ratio_like_cpp(0, 5), 0.0);
        assert_eq!(table.oct_regen_hp_ratio_like_cpp(1, 1), 0.01);
        assert_eq!(table.oct_regen_hp_ratio_like_cpp(1, 5), 0.05);
        assert_eq!(table.oct_regen_hp_ratio_like_cpp(1, 11), 0.11);
        assert_eq!(table.oct_regen_hp_ratio_like_cpp(1, 12), 0.0);
        assert_eq!(table.oct_regen_hp_ratio_like_cpp(80, 5), 0.0);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn oct_regen_hp_game_table_rejects_wrong_column_count_like_cpp() {
        let dir = write_temp_regen_table(
            "oct-regen-hp-bad",
            OctRegenHpGameTableLikeCpp::FILE_NAME,
            "Level\tWarrior\n1\t0.1\n",
        );
        let err = OctRegenHpGameTableLikeCpp::load(&dir).expect_err("column mismatch");

        assert!(err.to_string().contains("different count of columns"));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn regen_game_tables_fixture_oct_regen_hp_per_spirit_level_80_like_cpp() {
        let data_dir = Path::new("/home/server/woltk-server-core/Data");
        let oct_path = data_dir
            .join("gt")
            .join(OctRegenHpGameTableLikeCpp::FILE_NAME);
        let hp_path = data_dir
            .join("gt")
            .join(RegenHpPerSptGameTableLikeCpp::FILE_NAME);
        if !oct_path.exists() || !hp_path.exists() {
            eprintln!(
                "Skipping test: regen HP fixtures not found at {} or {}",
                oct_path.display(),
                hp_path.display()
            );
            return;
        }

        let tables = RegenGameTablesLikeCpp::load(data_dir).expect("load regen tables");
        let base = tables
            .oct_regen_hp
            .row(80)
            .expect("OCTRegenHP level 80 row");
        let more = tables
            .regen_hp_per_spt
            .row(80)
            .expect("RegenHPPerSpt level 80 row");
        let base_ratio = base.oct_regen_hp_ratio_for_class_like_cpp(5);
        let more_ratio = more.health_regen_ratio_for_class_like_cpp(5);

        let regen = tables.oct_regen_hp_per_spirit_like_cpp(80, 5, 60.0);
        assert!(regen > 0.0, "level 80 Priest spirit regen must be positive");
        assert!(
            (regen - (50.0 * base_ratio + 10.0 * more_ratio)).abs() < 1.0e-4,
            "spirit <= 50 uses the OCTRegenHP column and the excess uses RegenHPPerSpt"
        );

        assert_eq!(
            tables.oct_regen_hp_per_spirit_like_cpp(255, 5, 60.0),
            0.0,
            "a missing level row returns the C++ default"
        );
        assert_eq!(
            tables.oct_regen_hp_per_spirit_like_cpp(80, 12, 60.0),
            0.0,
            "a class outside 1..11 has no regen column"
        );
    }

    #[test]
    fn regen_game_tables_from_tables_delegates_mana_regen_like_cpp() {
        let mut columns = [0.0f32; REGEN_CLASS_COLUMN_COUNT];
        columns[4] = 0.123; // Priest is class 5.
        let mp =
            RegenMpPerSptGameTableLikeCpp::from_rows([RegenMpPerSptEntryLikeCpp::from_columns(
                columns,
            )]);

        let tables = RegenGameTablesLikeCpp::from_tables(
            mp,
            RegenHpPerSptGameTableLikeCpp::from_rows([]),
            OctRegenHpGameTableLikeCpp::from_rows([]),
        );

        assert_eq!(tables.mana_regen_ratio_like_cpp(1, 5), 0.123);
        assert_eq!(tables.mana_regen_ratio_like_cpp(1, 1), 0.0);
        assert_eq!(tables.mana_regen_ratio_like_cpp(2, 5), 0.0);
    }
}
