// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! QuestXP.db2 loader — provides XP reward values per quest level and difficulty tier.
//!
//! C# ref: QuestXPRecord, Quest::XPValue(), Quest::RoundXPValue()

use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use tracing::{info, warn};
use crate::wdc4::Wdc4Reader;

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
                    reader.get_field_u32(idx, 0),
                    reader.get_field_u32(idx, 1),
                    reader.get_field_u32(idx, 2),
                    reader.get_field_u32(idx, 3),
                    reader.get_field_u32(idx, 4),
                    reader.get_field_u32(idx, 5),
                    reader.get_field_u32(idx, 6),
                    reader.get_field_u32(idx, 7),
                    reader.get_field_u32(idx, 8),
                    reader.get_field_u32(idx, 9),
                ],
            };
            rows.insert(id, row);
        }

        info!("Loaded {} QuestXP rows from {}", rows.len(), path.display());
        Ok(Self { rows })
    }

    /// Calculate XP reward for a quest.
    ///
    /// Formula (C# ref: Quest::XPValue):
    ///   quest_level = quest.QuestLevel (or player.level if -1)
    ///   diffFactor  = clamp(2*(questLevel - playerLevel) + 20, 1, 10)
    ///   xp          = round(diffFactor * difficulty[xpDifficulty] / 10)
    ///
    /// `xp_difficulty` is `QuestTemplate.reward_xp_difficulty` (0–9).
    pub fn calculate_xp(&self, quest_level: i32, player_level: u8, xp_difficulty: u32) -> u32 {
        if xp_difficulty >= 10 {
            return 0;
        }

        // quest_level == -1 → use player level
        let ql = if quest_level == -1 { player_level as i32 } else { quest_level };

        let row = match self.rows.get(&(ql as u32)) {
            Some(r) => r,
            None => {
                // Grey quest or level out of range → nearest available
                if let Some(r) = self.nearest(ql as u32) { r } else { return 0; }
            }
        };

        let base_xp = row.difficulty[xp_difficulty as usize];
        if base_xp == 0 {
            return 0;
        }

        // diffFactor — reduces XP for grey quests, boosts for high-level quests
        let diff_factor = (2 * (ql - player_level as i32) + 20).clamp(1, 10) as u32;

        // RoundXPValue: round to nearest 5 (WotLK uses /5 rounding)
        let xp = diff_factor * base_xp / 10;
        round_xp(xp)
    }

    fn nearest(&self, target: u32) -> Option<&QuestXpRow> {
        self.rows.values().min_by_key(|r| (r.level as i64 - target as i64).unsigned_abs())
    }
}

/// C# ref: Quest::RoundXPValue — rounds to nearest 5.
fn round_xp(xp: u32) -> u32 {
    if xp <= 10 { return xp; }
    // Round to nearest 5
    ((xp + 2) / 5) * 5
}

impl Default for QuestXpStore {
    fn default() -> Self {
        Self { rows: HashMap::new() }
    }
}
