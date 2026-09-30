//! Validated player stat rows and the ObjectMgr level-stat loader.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use tracing::info;

use super::PlayerLevelStats;
use crate::BaseMpGameTableLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRaceStatsRowLikeCpp {
    pub race: u8,
    pub stat_modifiers: [i16; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerClassLevelStatsRowLikeCpp {
    pub class: u8,
    pub level: u8,
    pub primary_stats: [u16; 5],
}

pub struct PlayerRaceStatsRowsLikeCpp(Vec<PlayerRaceStatsRowLikeCpp>);

impl PlayerRaceStatsRowsLikeCpp {
    pub fn try_from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerRaceStatsRowLikeCpp>,
    ) -> Result<Self> {
        let rows: Vec<_> = rows.into_iter().collect();
        if rows.is_empty() {
            bail!("Loaded 0 race stats definitions: player_racestats is empty");
        }
        Ok(Self(rows))
    }
}

pub struct PlayerClassLevelStatsRowsLikeCpp(Vec<PlayerClassLevelStatsRowLikeCpp>);

impl PlayerClassLevelStatsRowsLikeCpp {
    pub fn try_from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerClassLevelStatsRowLikeCpp>,
    ) -> Result<Self> {
        let rows: Vec<_> = rows.into_iter().collect();
        if rows.is_empty() {
            bail!("Loaded 0 level stats definitions: player_classlevelstats is empty");
        }
        Ok(Self(rows))
    }
}

/// In-memory C++ player level information keyed by `(race, class, level)`.
pub struct PlayerStatsStore {
    stats: HashMap<(u8, u8, u8), PlayerLevelStats>,
}

impl PlayerStatsStore {
    /// Compose the exact C++ sources used by `ObjectMgr::LoadPlayerInfo`.
    pub fn load_from_validated_rows_like_cpp(
        data_dir: impl AsRef<Path>,
        max_player_level: u8,
        valid_race_classes: &[(u8, u8)],
        race_rows: PlayerRaceStatsRowsLikeCpp,
        class_rows: PlayerClassLevelStatsRowsLikeCpp,
    ) -> Result<Self> {
        let race_rows: Vec<_> = race_rows
            .0
            .into_iter()
            .map(|row| (row.race, row.stat_modifiers))
            .collect();

        let class_rows: Vec<_> = class_rows
            .0
            .into_iter()
            .map(|row| (row.class, row.level, row.primary_stats))
            .collect();

        let base_mp = BaseMpGameTableLikeCpp::load(data_dir)
            .context("Failed to load gt/BaseMp.txt for player class-level stats")?;
        let store = Self::from_cpp_sources(
            valid_race_classes.iter().copied(),
            race_rows,
            class_rows,
            &base_mp,
            max_player_level,
        )?;
        info!(
            "Loaded {} C++ player race/class/level stat entries",
            store.len()
        );
        Ok(store)
    }

    /// Build the same combined rows as C++ `ObjectMgr::LoadPlayerInfo`.
    ///
    /// Missing class-level rows after level 1 inherit the previous level.
    /// A class represented in the input without level-1 data is rejected,
    /// matching C++'s fatal integrity check for playable combinations.
    pub fn from_cpp_sources(
        valid_race_classes: impl IntoIterator<Item = (u8, u8)>,
        race_rows: impl IntoIterator<Item = (u8, [i16; 5])>,
        class_rows: impl IntoIterator<Item = (u8, u8, [u16; 5])>,
        base_mp: &BaseMpGameTableLikeCpp,
        max_player_level: u8,
    ) -> Result<Self> {
        if max_player_level == 0 {
            bail!("CONFIG_MAX_PLAYER_LEVEL must be at least 1");
        }

        let valid_race_classes: HashSet<(u8, u8)> = valid_race_classes.into_iter().collect();
        if valid_race_classes.is_empty() {
            bail!("playercreateinfo has no valid race/class combinations");
        }

        let race_modifiers: HashMap<u8, [i16; 5]> = race_rows.into_iter().collect();
        if race_modifiers.is_empty() {
            bail!("player_racestats is empty");
        }

        let mut class_level_stats = HashMap::new();
        for (class, level, primary_stats) in class_rows {
            if level == 0 || level > max_player_level {
                continue;
            }
            class_level_stats.insert((class, level), primary_stats);
        }
        if class_level_stats.is_empty() {
            bail!("player_classlevelstats has no rows within the configured level range");
        }

        let required_classes: HashSet<u8> =
            valid_race_classes.iter().map(|&(_, class)| class).collect();
        for &class in &required_classes {
            let Some(mut previous) = class_level_stats.get(&(class, 1)).copied() else {
                bail!("Class {class} level 1 does not have stats data");
            };
            for level in 2..=max_player_level {
                match class_level_stats.get(&(class, level)).copied() {
                    Some(stats) if stats[0] != 0 => previous = stats,
                    _ => {
                        class_level_stats.insert((class, level), previous);
                    }
                }
            }
        }

        let mut stats = HashMap::new();
        for (race, class) in valid_race_classes {
            let race_modifiers = race_modifiers.get(&race).copied().unwrap_or([0; 5]);
            for level in 1..=max_player_level {
                let Some(class_stats) = class_level_stats.get(&(class, level)) else {
                    continue;
                };
                let combined: [u16; 5] = std::array::from_fn(|index| {
                    // C++ assigns the promoted `uint16 + int16` result back
                    // to `uint16`. Valid world rows remain non-negative.
                    (i32::from(class_stats[index]) + i32::from(race_modifiers[index])) as u16
                });
                if level == 1 && combined[0] == 0 {
                    bail!("Race {race} Class {class} Level 1 does not have stats data");
                }
                stats.insert(
                    (race, class, level),
                    PlayerLevelStats {
                        strength: combined[0],
                        agility: combined[1],
                        stamina: combined[2],
                        intellect: combined[3],
                        spirit: combined[4],
                        base_mana: base_mp.base_mana_like_cpp(class, level).unwrap_or(0),
                    },
                );
            }
        }

        Ok(Self { stats })
    }

    pub fn get(&self, race: u8, class: u8, level: u8) -> Option<&PlayerLevelStats> {
        self.stats.get(&(race, class, level))
    }

    /// Test/fixture constructor for already-combined C++ rows.
    pub fn from_entries(
        entries: impl IntoIterator<Item = ((u8, u8, u8), PlayerLevelStats)>,
    ) -> Self {
        Self {
            stats: entries.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.stats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }
}
