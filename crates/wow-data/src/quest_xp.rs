// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! QuestXP.db2 loader — provides XP reward values per quest level and difficulty tier.
//!
//! C# ref: QuestXPRecord, Quest::XPValue(), Quest::RoundXPValue()

use crate::wdc4::Wdc4Reader;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

/// One row from QuestXP.db2.
/// ID = quest level; difficulty[0..9] = XP per difficulty tier.
/// C# ref: QuestXPRecord { int Id; ushort[] Difficulty = new ushort[10]; }
#[derive(Debug, Clone)]
pub struct QuestXpRow {
    pub level: u32,
    pub difficulty: [u32; 10],
}

/// In-memory table of QuestXP values, keyed by quest level.
pub struct QuestXpStore {
    rows: HashMap<u32, QuestXpRow>,
}

impl QuestXpStore {
    /// Load QuestXP.db2 from the given DBC data directory.
    /// path: e.g. "/home/server/woltk-server-core/Data/dbc/esES"
    pub fn load(dbc_dir: &str) -> Result<Self> {
        let path = Path::new(dbc_dir).join("QuestXP.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut rows = HashMap::with_capacity(reader.total_count());

        for (id, idx) in reader.iter_records() {
            let row = QuestXpRow {
                level: id,
                difficulty: [
                    u32::from(reader.get_array_u16(idx, 0, 0)),
                    u32::from(reader.get_array_u16(idx, 0, 1)),
                    u32::from(reader.get_array_u16(idx, 0, 2)),
                    u32::from(reader.get_array_u16(idx, 0, 3)),
                    u32::from(reader.get_array_u16(idx, 0, 4)),
                    u32::from(reader.get_array_u16(idx, 0, 5)),
                    u32::from(reader.get_array_u16(idx, 0, 6)),
                    u32::from(reader.get_array_u16(idx, 0, 7)),
                    u32::from(reader.get_array_u16(idx, 0, 8)),
                    u32::from(reader.get_array_u16(idx, 0, 9)),
                ],
            };
            rows.insert(id, row);
        }

        info!("Loaded {} QuestXP rows from {}", rows.len(), path.display());
        Ok(Self { rows })
    }

    /// Calculate XP reward for a quest.
    ///
    /// Formula (C++ `Quest::XPValue`):
    ///   quest_level = quest.QuestLevel (or player.level if -1)
    ///   diffFactor  = clamp(2*(questLevel - playerLevel) + 20, 1, 10)
    ///   xp          = round(diffFactor * difficulty[xpDifficulty] / 10)
    ///
    /// `xp_difficulty` is `QuestTemplate.reward_xp_difficulty` (0–9).
    pub fn calculate_xp(
        &self,
        quest_level: i32,
        player_level: u8,
        xp_difficulty: u32,
        xp_multiplier: f32,
        min_quest_scaled_xp_ratio: u32,
    ) -> u32 {
        if xp_difficulty >= 10 {
            return 0;
        }

        // quest_level == -1 → use player level
        let ql = if quest_level == -1 {
            player_level as i32
        } else {
            quest_level
        };

        let row = match self.rows.get(&(ql as u32)) {
            Some(r) => r,
            None => return 0,
        };

        let base_xp = row.difficulty[xp_difficulty as usize];
        if base_xp == 0 {
            return 0;
        }

        // diffFactor — reduces XP for grey quests, boosts for high-level quests
        let diff_factor = (2 * (ql - player_level as i32) + 20).clamp(1, 10) as u32;

        // RoundXPValue: round to nearest 5 (WotLK uses /5 rounding)
        let xp = round_xp(diff_factor * base_xp / 10);
        if min_quest_scaled_xp_ratio != 0 {
            xp.max(
                round_xp((base_xp as f32 * xp_multiplier) as u32)
                    * min_quest_scaled_xp_ratio
                    / 100,
            )
        } else {
            xp
        }
    }

    /// C++ `QuestXPEntry const* questXp = sQuestXPStore.LookupEntry(player->GetLevel())`
    /// followed by `Quest::RoundXPValue(questXp->Difficulty[xpDifficulty])`.
    pub fn player_level_difficulty_xp_like_cpp(&self, player_level: u8, xp_difficulty: u32) -> u32 {
        if xp_difficulty >= 10 {
            return 0;
        }

        self.rows
            .get(&(player_level as u32))
            .map(|row| round_xp(row.difficulty[xp_difficulty as usize]))
            .unwrap_or(0)
    }

}

/// C++ `Quest::RoundXPValue`.
fn round_xp(xp: u32) -> u32 {
    if xp <= 100 {
        5 * ((xp + 2) / 5)
    } else if xp <= 500 {
        10 * ((xp + 5) / 10)
    } else if xp <= 1000 {
        25 * ((xp + 12) / 25)
    } else {
        50 * ((xp + 25) / 50)
    }
}

impl Default for QuestXpStore {
    fn default() -> Self {
        Self {
            rows: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_xp_value_matches_cpp_thresholds() {
        assert_eq!(round_xp(1), 0);
        assert_eq!(round_xp(3), 5);
        assert_eq!(round_xp(102), 100);
        assert_eq!(round_xp(106), 110);
        assert_eq!(round_xp(511), 500);
        assert_eq!(round_xp(513), 525);
        assert_eq!(round_xp(1024), 1000);
        assert_eq!(round_xp(1026), 1050);
    }

    #[test]
    fn player_level_difficulty_xp_uses_raw_player_level_row_like_cpp() {
        let mut rows = HashMap::new();
        rows.insert(
            42,
            QuestXpRow {
                level: 42,
                difficulty: [0, 1, 11, 101, 511, 1026, 0, 0, 0, 0],
            },
        );
        let store = QuestXpStore { rows };

        assert_eq!(store.player_level_difficulty_xp_like_cpp(42, 1), 0);
        assert_eq!(store.player_level_difficulty_xp_like_cpp(42, 2), 10);
        assert_eq!(store.player_level_difficulty_xp_like_cpp(42, 3), 100);
        assert_eq!(store.player_level_difficulty_xp_like_cpp(42, 4), 500);
        assert_eq!(store.player_level_difficulty_xp_like_cpp(42, 5), 1050);
        assert_eq!(store.player_level_difficulty_xp_like_cpp(41, 5), 0);
        assert_eq!(store.player_level_difficulty_xp_like_cpp(42, 10), 0);
    }

    #[test]
    fn calculate_xp_missing_quest_level_returns_zero_like_cpp() {
        let mut rows = HashMap::new();
        rows.insert(
            42,
            QuestXpRow {
                level: 42,
                difficulty: [0, 100, 0, 0, 0, 0, 0, 0, 0, 0],
            },
        );
        let store = QuestXpStore { rows };

        assert_eq!(store.calculate_xp(41, 42, 1, 1.0, 0), 0);
    }

    #[test]
    fn calculate_xp_min_scaled_ratio_raises_grey_quest_like_cpp() {
        let mut rows = HashMap::new();
        rows.insert(
            42,
            QuestXpRow {
                level: 42,
                difficulty: [0, 1000, 0, 0, 0, 0, 0, 0, 0, 0],
            },
        );
        let store = QuestXpStore { rows };

        assert_eq!(store.calculate_xp(42, 80, 1, 2.0, 0), 100);
        assert_eq!(store.calculate_xp(42, 80, 1, 2.0, 50), 1000);
    }

    #[test]
    fn load_real_quest_xp_level_80_pallet_array_like_cpp() {
        let dbc_dir = "/home/server/woltk-server-core/Data/dbc/esES";
        if !Path::new(dbc_dir).join("QuestXP.db2").exists() {
            return;
        }

        let store = QuestXpStore::load(dbc_dir).expect("load QuestXP.db2");
        let row = store.rows.get(&80).expect("level 80 QuestXP row");
        assert_eq!(
            row.difficulty,
            [0, 2200, 5500, 11050, 16550, 22050, 27550, 33100, 44100, 0]
        );
    }
}
