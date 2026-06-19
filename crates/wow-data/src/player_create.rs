// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadPlayerInfo` player-create cast spell data.

use std::collections::HashMap;

use anyhow::{Context, Result};
use wow_database::{WorldDatabase, WorldStatements};

pub const PLAYER_CREATE_MODE_NORMAL_LIKE_CPP: u8 = 0;
pub const PLAYER_CREATE_MODE_NPE_LIKE_CPP: u8 = 1;
pub const PLAYER_CREATE_MODE_MAX_LIKE_CPP: u8 = 2;

const RACE_HUMAN_LIKE_CPP: u8 = 1;
const MAX_RACES_LIKE_CPP: u8 = 78;
const CLASS_WARRIOR_LIKE_CPP: u8 = 1;
const MAX_CLASSES_LIKE_CPP: u8 = 15;

const RACEMASK_ALL_PLAYABLE_LIKE_CPP: u64 = 0x0003_007F_FFFF;
const CLASSMASK_ALL_PLAYABLE_LIKE_CPP: u32 = 0x1FFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCreateInfoCastSpellRowLikeCpp {
    pub race_mask: u64,
    pub class_mask: u32,
    pub spell_id: u32,
    pub create_mode: i8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerCreateInfoCastSpellLoadReportLikeCpp {
    pub loaded_assignments: usize,
    pub skipped_invalid_race_mask: usize,
    pub skipped_invalid_class_mask: usize,
    pub skipped_invalid_create_mode: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerCreateInfoCastSpellStoreLikeCpp {
    spells_by_key: HashMap<(u8, u8, u8), Vec<u32>>,
    load_report: PlayerCreateInfoCastSpellLoadReportLikeCpp,
}

impl PlayerCreateInfoCastSpellStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerCreateInfoCastSpellRowLikeCpp>,
    ) -> Self {
        let mut spells_by_key = HashMap::<(u8, u8, u8), Vec<u32>>::new();
        let mut load_report = PlayerCreateInfoCastSpellLoadReportLikeCpp::default();

        for row in rows {
            if row.race_mask != 0 && row.race_mask & RACEMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                load_report.skipped_invalid_race_mask += 1;
                continue;
            }

            if row.class_mask != 0 && row.class_mask & CLASSMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                load_report.skipped_invalid_class_mask += 1;
                continue;
            }

            let Ok(create_mode) = u8::try_from(row.create_mode) else {
                load_report.skipped_invalid_create_mode += 1;
                continue;
            };
            if create_mode >= PLAYER_CREATE_MODE_MAX_LIKE_CPP {
                load_report.skipped_invalid_create_mode += 1;
                continue;
            }

            for race in RACE_HUMAN_LIKE_CPP..MAX_RACES_LIKE_CPP {
                if row.race_mask != 0 && row.race_mask & race_mask_bit_like_cpp(race) == 0 {
                    continue;
                }

                for class in CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP {
                    if row.class_mask != 0 && row.class_mask & class_mask_bit_like_cpp(class) == 0 {
                        continue;
                    }

                    spells_by_key
                        .entry((race, class, create_mode))
                        .or_default()
                        .push(row.spell_id);
                    load_report.loaded_assignments += 1;
                }
            }
        }

        Self {
            spells_by_key,
            load_report,
        }
    }

    pub async fn load_like_cpp(world_db: &WorldDatabase) -> Result<Self> {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO_CAST_SPELL);
        let mut result = world_db
            .query(&stmt)
            .await
            .context("Failed to query playercreateinfo_cast_spell")?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(PlayerCreateInfoCastSpellRowLikeCpp {
                    race_mask: result.try_read::<u64>(0).unwrap_or(0),
                    class_mask: result.try_read::<u32>(1).unwrap_or(0),
                    spell_id: result.try_read::<u32>(2).unwrap_or(0),
                    create_mode: result.try_read::<i8>(3).unwrap_or(-1),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows))
    }

    pub fn cast_spells_like_cpp(&self, race: u8, class: u8, create_mode: u8) -> &[u32] {
        self.spells_by_key
            .get(&(race, class, create_mode))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn load_report_like_cpp(&self) -> &PlayerCreateInfoCastSpellLoadReportLikeCpp {
        &self.load_report
    }
}

fn race_mask_bit_like_cpp(race: u8) -> u64 {
    let bit = match race {
        1..=10 | 22 | 24..=30 => Some(race - 1),
        34 => Some(11),
        35 => Some(12),
        36 => Some(13),
        37 => Some(14),
        70 => Some(16),
        52 => Some(15),
        _ => None,
    };
    bit.map(|bit| 1_u64 << bit).unwrap_or(0)
}

fn class_mask_bit_like_cpp(class: u8) -> u32 {
    if class == 0 || class >= 33 {
        0
    } else {
        1_u32 << (class - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_create_cast_spell_expands_masks_and_modes_like_cpp() {
        let store = PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp([
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: race_mask_bit_like_cpp(1) | race_mask_bit_like_cpp(2),
                class_mask: class_mask_bit_like_cpp(1),
                spell_id: 100,
                create_mode: PLAYER_CREATE_MODE_NORMAL_LIKE_CPP as i8,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 200,
                create_mode: PLAYER_CREATE_MODE_NPE_LIKE_CPP as i8,
            },
        ]);

        assert_eq!(store.cast_spells_like_cpp(1, 1, 0), &[100]);
        assert_eq!(store.cast_spells_like_cpp(2, 1, 0), &[100]);
        assert!(store.cast_spells_like_cpp(3, 1, 0).is_empty());
        assert_eq!(store.cast_spells_like_cpp(1, 1, 1), &[200]);
        assert_eq!(store.cast_spells_like_cpp(77, 13, 1), &[200]);
    }

    #[test]
    fn player_create_cast_spell_rejects_invalid_rows_like_cpp() {
        let store = PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp([
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 1_u64 << 62,
                class_mask: 0,
                spell_id: 100,
                create_mode: 0,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 1_u32 << 31,
                spell_id: 101,
                create_mode: 0,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 102,
                create_mode: 2,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 103,
                create_mode: -1,
            },
        ]);

        assert_eq!(
            *store.load_report_like_cpp(),
            PlayerCreateInfoCastSpellLoadReportLikeCpp {
                skipped_invalid_race_mask: 1,
                skipped_invalid_class_mask: 1,
                skipped_invalid_create_mode: 2,
                ..PlayerCreateInfoCastSpellLoadReportLikeCpp::default()
            }
        );
        assert!(store.cast_spells_like_cpp(1, 1, 0).is_empty());
    }
}
