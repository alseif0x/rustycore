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
}
