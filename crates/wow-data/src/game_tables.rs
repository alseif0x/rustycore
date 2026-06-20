// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3

//! C++ `DataStores/GameTables.*` text-table readers.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// C++ `GtBattlePetXPEntry`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BattlePetXpEntryLikeCpp {
    pub wins: f32,
    pub xp: f32,
}

/// C++ `sBattlePetXPGameTable`.
///
/// GameTables are indexed by row position, not by the explicit first column.
/// Row 0 is a default unused entry, matching `LoadGameTable`.
#[derive(Debug, Clone, PartialEq)]
pub struct BattlePetXpGameTableLikeCpp {
    rows: Vec<BattlePetXpEntryLikeCpp>,
}

/// C++ `GtShieldBlockRegularEntry`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShieldBlockRegularEntryLikeCpp {
    pub poor: f32,
    pub standard: f32,
    pub good: f32,
    pub superior: f32,
    pub epic: f32,
    pub legendary: f32,
    pub artifact: f32,
    pub scaling_stat: f32,
}

/// C++ `sShieldBlockRegularGameTable`.
///
/// GameTables are indexed by row position, not by the explicit first column.
/// Row 0 is a default unused entry, matching `LoadGameTable`.
#[derive(Debug, Clone, PartialEq)]
pub struct ShieldBlockRegularGameTableLikeCpp {
    rows: Vec<ShieldBlockRegularEntryLikeCpp>,
}

impl BattlePetXpGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "BattlePetXP.txt";
    pub const VALUE_COLUMN_COUNT: usize = 2;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = BattlePetXpEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(BattlePetXpEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, level: u16) -> Option<&BattlePetXpEntryLikeCpp> {
        self.rows.get(usize::from(level))
    }

    pub fn xp_per_level_like_cpp(&self, level: u16) -> Option<u16> {
        self.row(level)
            .map(|row| battle_pet_xp_per_level_like_cpp(row) as u16)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn parse_like_cpp(content: &str, path: &Path) -> Result<Self> {
        let mut lines = content.lines();
        let Some(headers) = lines.next() else {
            bail!("GameTable file {} is empty.", path.display());
        };

        let column_defs: Vec<&str> = headers
            .split('\t')
            .filter(|part| !part.is_empty())
            .collect();
        if column_defs.len().saturating_sub(1) != Self::VALUE_COLUMN_COUNT {
            bail!(
                "GameTable '{}' has different count of columns {} than expected by size of C++ structure ({}).",
                path.display(),
                column_defs.len().saturating_sub(1),
                Self::VALUE_COLUMN_COUNT
            );
        }

        let mut rows = vec![BattlePetXpEntryLikeCpp::default()];
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

            rows.push(BattlePetXpEntryLikeCpp {
                wins: parse_float_like_cpp(values[1]),
                xp: parse_float_like_cpp(values[2]),
            });
        }

        Ok(Self { rows })
    }
}

pub fn battle_pet_xp_per_level_like_cpp(row: &BattlePetXpEntryLikeCpp) -> f32 {
    row.wins * row.xp
}

impl ShieldBlockRegularGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "ShieldBlockRegular.txt";
    pub const VALUE_COLUMN_COUNT: usize = 8;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = ShieldBlockRegularEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(ShieldBlockRegularEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, item_level: u32) -> Option<&ShieldBlockRegularEntryLikeCpp> {
        self.rows.get(usize::try_from(item_level).ok()?)
    }

    pub fn shield_block_for_quality_like_cpp(&self, item_level: u32, quality: u32) -> Option<i16> {
        self.row(item_level)
            .map(|row| shield_block_regular_column_for_quality_like_cpp(row, quality) as i16)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn parse_like_cpp(content: &str, path: &Path) -> Result<Self> {
        let mut lines = content.lines();
        let Some(headers) = lines.next() else {
            bail!("GameTable file {} is empty.", path.display());
        };

        let column_defs: Vec<&str> = headers
            .split('\t')
            .filter(|part| !part.is_empty())
            .collect();
        if column_defs.len().saturating_sub(1) != Self::VALUE_COLUMN_COUNT {
            bail!(
                "GameTable '{}' has different count of columns {} than expected by size of C++ structure ({}).",
                path.display(),
                column_defs.len().saturating_sub(1),
                Self::VALUE_COLUMN_COUNT
            );
        }

        let mut rows = vec![ShieldBlockRegularEntryLikeCpp::default()];
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

            rows.push(ShieldBlockRegularEntryLikeCpp {
                poor: parse_float_like_cpp(values[1]),
                standard: parse_float_like_cpp(values[2]),
                good: parse_float_like_cpp(values[3]),
                superior: parse_float_like_cpp(values[4]),
                epic: parse_float_like_cpp(values[5]),
                legendary: parse_float_like_cpp(values[6]),
                artifact: parse_float_like_cpp(values[7]),
                scaling_stat: parse_float_like_cpp(values[8]),
            });
        }

        Ok(Self { rows })
    }
}

pub fn shield_block_regular_column_for_quality_like_cpp(
    row: &ShieldBlockRegularEntryLikeCpp,
    quality: u32,
) -> f32 {
    match quality {
        0 => row.poor,
        1 => row.standard,
        2 => row.good,
        3 => row.superior,
        4 => row.epic,
        5 => row.legendary,
        6 => row.artifact,
        7 => row.scaling_stat,
        _ => 0.0,
    }
}

fn parse_float_like_cpp(value: &str) -> f32 {
    value.parse::<f32>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_battle_pet_xp(content: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        dir.push(format!(
            "rustycore-battle-pet-xp-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(dir.join("gt")).expect("create temp gt dir");
        let path = dir.join("gt").join(BattlePetXpGameTableLikeCpp::FILE_NAME);
        fs::write(&path, content).expect("write temp BattlePetXP");
        dir
    }

    fn write_temp_shield_block_regular(content: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        dir.push(format!(
            "rustycore-shield-block-regular-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(dir.join("gt")).expect("create temp gt dir");
        let path = dir
            .join("gt")
            .join(ShieldBlockRegularGameTableLikeCpp::FILE_NAME);
        fs::write(&path, content).expect("write temp ShieldBlockRegular");
        dir
    }

    #[test]
    fn battle_pet_xp_game_table_loads_rows_by_position_not_id_like_cpp() {
        let dir = write_temp_battle_pet_xp("ID\tWins\tXp\r\n23\t2\t50\r\n99\t3\t40\r\n\r\n");
        let table = BattlePetXpGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.len(), 3);
        assert_eq!(table.xp_per_level_like_cpp(0), Some(0));
        assert_eq!(table.xp_per_level_like_cpp(1), Some(100));
        assert_eq!(table.xp_per_level_like_cpp(2), Some(120));
        assert_eq!(table.xp_per_level_like_cpp(23), None);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn battle_pet_xp_game_table_invalid_float_defaults_to_zero_like_cpp() {
        let dir = write_temp_battle_pet_xp("ID\tWins\tXp\n1\tbad\t50\n2\t3\tbad\n");
        let table = BattlePetXpGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.xp_per_level_like_cpp(1), Some(0));
        assert_eq!(table.xp_per_level_like_cpp(2), Some(0));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn battle_pet_xp_game_table_rejects_wrong_column_count_like_cpp() {
        let dir = write_temp_battle_pet_xp("ID\tWins\n1\t2\n");
        let err = BattlePetXpGameTableLikeCpp::load(&dir).expect_err("column mismatch");

        assert!(err.to_string().contains("different count of columns"));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn shield_block_regular_game_table_loads_rows_by_position_and_quality_like_cpp() {
        let dir = write_temp_shield_block_regular(
            "ID\tPoor\tStandard\tGood\tSuperior\tEpic\tLegendary\tArtifact\tScalingStat\r\n\
             20\t1\t2\t3\t4\t5\t6\t7\t8\r\n\
             40\t10\t20\t30\t40\t50\t60\t70\t80\r\n",
        );
        let table = ShieldBlockRegularGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.len(), 3);
        assert_eq!(table.shield_block_for_quality_like_cpp(0, 3), Some(0));
        assert_eq!(table.shield_block_for_quality_like_cpp(1, 0), Some(1));
        assert_eq!(table.shield_block_for_quality_like_cpp(1, 3), Some(4));
        assert_eq!(table.shield_block_for_quality_like_cpp(1, 7), Some(8));
        assert_eq!(table.shield_block_for_quality_like_cpp(1, 8), Some(0));
        assert_eq!(table.shield_block_for_quality_like_cpp(20, 3), None);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn shield_block_regular_game_table_invalid_float_defaults_to_zero_like_cpp() {
        let dir = write_temp_shield_block_regular(
            "ID\tPoor\tStandard\tGood\tSuperior\tEpic\tLegendary\tArtifact\tScalingStat\n\
             1\tbad\t2\t3\t4\t5\t6\t7\t8\n",
        );
        let table = ShieldBlockRegularGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.shield_block_for_quality_like_cpp(1, 0), Some(0));
        assert_eq!(table.shield_block_for_quality_like_cpp(1, 1), Some(2));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn shield_block_regular_game_table_rejects_wrong_column_count_like_cpp() {
        let dir = write_temp_shield_block_regular("ID\tPoor\n1\t2\n");
        let err = ShieldBlockRegularGameTableLikeCpp::load(&dir).expect_err("column mismatch");

        assert!(err.to_string().contains("different count of columns"));

        fs::remove_dir_all(dir).ok();
    }
}
