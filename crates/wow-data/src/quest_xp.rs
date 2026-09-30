// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! QuestXP.db2 loader — provides XP reward values per quest level and difficulty tier.
//!
//! Reward valuation is owned by wow-progression; this module only loads rows.

use crate::wdc4::Wdc4Reader;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

/// One row from QuestXP.db2.
/// ID = quest level; difficulty[0..9] = XP per difficulty tier.
/// Preserves the loader's u16-to-u32 representation of all ten difficulties.
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

    /// Borrow one immutable row; reward math belongs to wow-progression.
    pub fn get(&self, level: u32) -> Option<&QuestXpRow> {
        self.rows.get(&level)
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
