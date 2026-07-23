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

/// C++ `GtBaseMPEntry`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BaseMpEntryLikeCpp {
    columns: [f32; BaseMpGameTableLikeCpp::VALUE_COLUMN_COUNT],
}

/// C++ `sBaseMPGameTable`.
///
/// GameTables are indexed by row position, not by the explicit first column.
/// Row 0 is a default unused entry, matching `LoadGameTable`.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseMpGameTableLikeCpp {
    rows: Vec<BaseMpEntryLikeCpp>,
}

/// C++ `GtCombatRatingsEntry`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CombatRatingsEntryLikeCpp {
    columns: [f32; CombatRatingsGameTableLikeCpp::VALUE_COLUMN_COUNT],
}

/// C++ `sCombatRatingsGameTable`.
///
/// GameTables are indexed by row position, not by the explicit first column.
/// Row 0 is a default unused entry, matching `LoadGameTable`.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatRatingsGameTableLikeCpp {
    rows: Vec<CombatRatingsEntryLikeCpp>,
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

impl BaseMpEntryLikeCpp {
    pub fn from_columns(columns: [f32; BaseMpGameTableLikeCpp::VALUE_COLUMN_COUNT]) -> Self {
        Self { columns }
    }

    /// C++ `GetGameTableColumnForClass`.
    pub fn mana_for_class_like_cpp(&self, class: u8) -> f32 {
        let column = match class {
            4 => 0,   // Rogue
            11 => 1,  // Druid
            3 => 2,   // Hunter
            8 => 3,   // Mage
            2 => 4,   // Paladin
            5 => 5,   // Priest
            7 => 6,   // Shaman
            9 => 7,   // Warlock
            1 => 8,   // Warrior
            6 => 9,   // Death Knight
            10 => 10, // Monk
            12 => 11, // Demon Hunter
            _ => return 0.0,
        };
        self.columns[column]
    }
}

impl BaseMpGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "BaseMp.txt";
    pub const VALUE_COLUMN_COUNT: usize = 12;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = BaseMpEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(BaseMpEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, level: u16) -> Option<&BaseMpEntryLikeCpp> {
        self.rows.get(usize::from(level))
    }

    /// C++ `ObjectMgr::GetPlayerClassLevelInfo`.
    pub fn base_mana_like_cpp(&self, class: u8, level: u8) -> Option<u32> {
        self.row(u16::from(level))
            .map(|row| row.mana_for_class_like_cpp(class) as u32)
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

        let mut rows = vec![BaseMpEntryLikeCpp::default()];
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

            let mut columns = [0.0f32; Self::VALUE_COLUMN_COUNT];
            for (column, raw_value) in columns.iter_mut().zip(values.iter().skip(1)) {
                *column = parse_float_like_cpp(raw_value);
            }
            rows.push(BaseMpEntryLikeCpp { columns });
        }

        Ok(Self { rows })
    }
}

pub fn battle_pet_xp_per_level_like_cpp(row: &BattlePetXpEntryLikeCpp) -> f32 {
    row.wins * row.xp
}

impl CombatRatingsEntryLikeCpp {
    pub const WEAPON_SKILL: usize = 0;
    pub const DEFENSE_SKILL: usize = 1;
    pub const DODGE: usize = 2;
    pub const PARRY: usize = 3;
    pub const BLOCK: usize = 4;
    pub const HIT_MELEE: usize = 5;
    pub const HIT_RANGED: usize = 6;
    pub const HIT_SPELL: usize = 7;
    pub const CRIT_MELEE: usize = 8;
    pub const CRIT_RANGED: usize = 9;
    pub const CRIT_SPELL: usize = 10;
    pub const HIT_TAKEN_MELEE: usize = 11;
    pub const HIT_TAKEN_RANGED: usize = 12;
    pub const HIT_TAKEN_SPELL: usize = 13;
    pub const HASTE_MELEE: usize = 17;
    pub const HASTE_RANGED: usize = 18;
    pub const HASTE_SPELL: usize = 19;

    pub fn from_columns(columns: [f32; CombatRatingsGameTableLikeCpp::VALUE_COLUMN_COUNT]) -> Self {
        Self { columns }
    }

    pub fn column(&self, index: usize) -> f32 {
        self.columns.get(index).copied().unwrap_or(0.0)
    }
}

impl CombatRatingsGameTableLikeCpp {
    pub const FILE_NAME: &'static str = "CombatRatings.txt";
    pub const VALUE_COLUMN_COUNT: usize = 32;

    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_from_path(data_dir.as_ref().join("gt").join(Self::FILE_NAME))
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("GameTable file {} cannot be opened.", path.display()))?;
        Self::parse_like_cpp(&content, path)
    }

    pub fn from_rows(rows: impl IntoIterator<Item = CombatRatingsEntryLikeCpp>) -> Self {
        let mut stored = Vec::with_capacity(1);
        stored.push(CombatRatingsEntryLikeCpp::default());
        stored.extend(rows);
        Self { rows: stored }
    }

    pub fn row(&self, level: u16) -> Option<&CombatRatingsEntryLikeCpp> {
        self.rows.get(usize::from(level))
    }

    pub fn rating_multiplier_like_cpp(&self, level: u16, rating: u32) -> f32 {
        self.row(level)
            .map(|row| combat_rating_multiplier_like_cpp(row, rating))
            .unwrap_or(1.0)
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

        let mut rows = vec![CombatRatingsEntryLikeCpp::default()];
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

            let mut columns = [0.0f32; Self::VALUE_COLUMN_COUNT];
            for (column, raw_value) in columns.iter_mut().zip(values.iter().skip(1)) {
                *column = parse_float_like_cpp(raw_value);
            }
            rows.push(CombatRatingsEntryLikeCpp { columns });
        }

        Ok(Self { rows })
    }
}

pub fn combat_rating_column_for_rating_like_cpp(
    row: &CombatRatingsEntryLikeCpp,
    rating: u32,
) -> f32 {
    match rating {
        0 => row.column(CombatRatingsEntryLikeCpp::WEAPON_SKILL),
        1 => row.column(CombatRatingsEntryLikeCpp::DEFENSE_SKILL),
        2 => row.column(CombatRatingsEntryLikeCpp::DODGE),
        3 => row.column(CombatRatingsEntryLikeCpp::PARRY),
        4 => row.column(CombatRatingsEntryLikeCpp::BLOCK),
        5 => row.column(CombatRatingsEntryLikeCpp::HIT_MELEE),
        6 => row.column(CombatRatingsEntryLikeCpp::HIT_RANGED),
        7 => row.column(CombatRatingsEntryLikeCpp::HIT_SPELL),
        8 => row.column(CombatRatingsEntryLikeCpp::CRIT_MELEE),
        9 => row.column(CombatRatingsEntryLikeCpp::CRIT_RANGED),
        10 => row.column(CombatRatingsEntryLikeCpp::CRIT_SPELL),
        11 => row.column(CombatRatingsEntryLikeCpp::HIT_TAKEN_MELEE),
        12 => row.column(CombatRatingsEntryLikeCpp::HIT_TAKEN_RANGED),
        13 => row.column(CombatRatingsEntryLikeCpp::HIT_TAKEN_SPELL),
        // Mirrors C++ `GetGameTableColumnForCombatRating`, including the
        // crit-taken cases intentionally selecting `HitTakenMelee`.
        14..=16 => row.column(CombatRatingsEntryLikeCpp::HIT_TAKEN_MELEE),
        17 => row.column(CombatRatingsEntryLikeCpp::HASTE_MELEE),
        18 => row.column(CombatRatingsEntryLikeCpp::HASTE_RANGED),
        19 => row.column(CombatRatingsEntryLikeCpp::HASTE_SPELL),
        _ => 1.0,
    }
}

pub fn combat_rating_multiplier_like_cpp(row: &CombatRatingsEntryLikeCpp, rating: u32) -> f32 {
    let value = combat_rating_column_for_rating_like_cpp(row, rating);
    if value == 0.0 { 1.0 } else { 1.0 / value }
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

    fn write_temp_base_mp(content: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        dir.push(format!(
            "rustycore-base-mp-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(dir.join("gt")).expect("create temp gt dir");
        let path = dir.join("gt").join(BaseMpGameTableLikeCpp::FILE_NAME);
        fs::write(&path, content).expect("write temp BaseMp");
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

    fn write_temp_combat_ratings(content: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        dir.push(format!(
            "rustycore-combat-ratings-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(dir.join("gt")).expect("create temp gt dir");
        let path = dir
            .join("gt")
            .join(CombatRatingsGameTableLikeCpp::FILE_NAME);
        fs::write(&path, content).expect("write temp CombatRatings");
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
    fn base_mp_game_table_maps_file_columns_to_cpp_class_ids() {
        let dir = write_temp_base_mp(
            "Level\tRogue\tDruid\tHunter\tMage\tPaladin\tPriest\tShaman\tWarlock\tWarrior\tDeath Knight\tMonk\tDemon Hunter\r\n\
             1\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\t20\t21\r\n",
        );
        let table = BaseMpGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.base_mana_like_cpp(4, 1), Some(10));
        assert_eq!(table.base_mana_like_cpp(11, 1), Some(11));
        assert_eq!(table.base_mana_like_cpp(1, 1), Some(18));
        assert_eq!(table.base_mana_like_cpp(6, 1), Some(19));
        assert_eq!(table.base_mana_like_cpp(13, 1), Some(0));
        assert_eq!(table.base_mana_like_cpp(5, 2), None);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn base_mp_fixture_matches_cpp_level_and_class_lookup() {
        let data_dir = Path::new("/home/server/woltk-server-core/Data");
        let path = data_dir.join("gt").join(BaseMpGameTableLikeCpp::FILE_NAME);
        if !path.exists() {
            eprintln!(
                "Skipping test: BaseMp fixture not found at {}",
                path.display()
            );
            return;
        }

        let table = BaseMpGameTableLikeCpp::load(data_dir).expect("load BaseMp");
        assert_eq!(table.base_mana_like_cpp(5, 1), Some(155));
        assert_eq!(table.base_mana_like_cpp(11, 1), Some(31));
        assert_eq!(table.base_mana_like_cpp(1, 80), Some(0));
    }

    #[test]
    fn combat_ratings_game_table_loads_rows_by_level_position_like_cpp() {
        let dir = write_temp_combat_ratings(
            "Level\tWeaponSkill\tDefenseSkill\tDodge\tParry\tBlock\tHitMelee\tHitRanged\tHitSpell\tCritMelee\tCritRanged\tCritSpell\tHitTakenMelee\tHitTakenRanged\tHitTakenSpell\tCritTakenMelee\tCritTakenRanged\tCritTakenSpell\tHasteMelee\tHasteRanged\tHasteSpell\tUnknown0\tUnknown1\tUnknown2\tUnknown3\tUnknown4\tUnknown5\tUnknown6\tUnknown7\tUnknown8\tUnknown9\tUnknown10\tUnknown11\r\n\
             80\t8.197496\t4.918498\t45.250187\t45.250187\t16.394995\t32.789989\t32.789989\t26.231993\t45.905987\t45.905987\t45.905987\t32.789989\t32.789989\t26.231993\t94.271225\t94.271225\t94.271225\t32.789989\t32.789989\t32.789989\t8.197496\t8.197496\t8.197496\t8.197496\t15.395300\t0\t0\t0\t0\t0\t0\t0\r\n",
        );
        let table = CombatRatingsGameTableLikeCpp::load(&dir).expect("load table");

        assert_eq!(table.len(), 2);
        assert_eq!(table.row(0), Some(&CombatRatingsEntryLikeCpp::default()));
        assert_eq!(table.row(80), None);
        let row = table.row(1).expect("first data row is positional");
        assert_eq!(combat_rating_column_for_rating_like_cpp(row, 8), 45.905987);
        assert_eq!(
            combat_rating_column_for_rating_like_cpp(row, 14),
            32.789989,
            "C++ maps CR_CRIT_TAKEN_* to HitTakenMelee in GetGameTableColumnForCombatRating"
        );
        assert!((combat_rating_multiplier_like_cpp(row, 8) - (1.0 / 45.905987)).abs() < 0.00001);
        assert_eq!(
            combat_rating_multiplier_like_cpp(row, 23),
            1.0,
            "ratings without a C++ table column use the default multiplier"
        );

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn combat_ratings_fixture_level_80_matches_cpp_columns() {
        let data_dir = Path::new("/home/server/woltk-server-core/Data");
        let path = data_dir
            .join("gt")
            .join(CombatRatingsGameTableLikeCpp::FILE_NAME);
        if !path.exists() {
            eprintln!(
                "Skipping test: CombatRatings fixture not found at {}",
                path.display()
            );
            return;
        }

        let table = CombatRatingsGameTableLikeCpp::load(data_dir).expect("load CombatRatings");
        let row = table.row(80).expect("level 80 row");
        assert!((combat_rating_column_for_rating_like_cpp(row, 8) - 45.905987).abs() < 0.00001);
        assert!((table.rating_multiplier_like_cpp(80, 2) - (1.0 / 45.250187)).abs() < 0.00001);
        assert!((table.rating_multiplier_like_cpp(80, 4) - (1.0 / 16.394995)).abs() < 0.00001);
    }

    #[test]
    fn combat_ratings_fixture_level_80_rating_bonus_is_table_derived_like_cpp() {
        let data_dir = Path::new("/home/server/woltk-server-core/Data");
        let path = data_dir
            .join("gt")
            .join(CombatRatingsGameTableLikeCpp::FILE_NAME);
        if !path.exists() {
            eprintln!(
                "Skipping test: CombatRatings fixture not found at {}",
                path.display()
            );
            return;
        }

        let table = CombatRatingsGameTableLikeCpp::load(data_dir).expect("load CombatRatings");
        let crit_rating = 207.0f32;
        let crit_bonus = crit_rating * table.rating_multiplier_like_cpp(80, 8);

        assert!((crit_bonus - (207.0 / 45.905987)).abs() < 0.00001);
        assert_eq!(
            crit_rating * table.rating_multiplier_like_cpp(80, 23),
            207.0,
            "C++ ratings without a CombatRatings column keep the default 1.0 multiplier"
        );
    }

    #[test]
    fn combat_ratings_game_table_rejects_wrong_column_count_like_cpp() {
        let dir = write_temp_combat_ratings("Level\tWeaponSkill\n1\t1\n");
        let err = CombatRatingsGameTableLikeCpp::load(&dir).expect_err("column mismatch");

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
