// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadCreatureModelInfo` world-database store.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::WorldDatabase;

use crate::CreatureDisplayInfoStore;

const DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP: f32 = 1.5;

fn normalize_combat_reach_like_cpp(combat_reach: f32) -> f32 {
    if combat_reach < 0.1 {
        DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP
    } else {
        combat_reach
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureModelInfoLikeCpp {
    pub display_id: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub display_id_other_gender: u32,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureModelInfoStoreLikeCpp {
    entries: HashMap<u32, CreatureModelInfoLikeCpp>,
}

impl CreatureModelInfoStoreLikeCpp {
    pub fn from_entries(entries: impl IntoIterator<Item = CreatureModelInfoLikeCpp>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| (entry.display_id, entry))
                .collect(),
        }
    }

    /// Mirrors C++ `ObjectMgr::LoadCreatureModelInfo`.
    ///
    /// C++ validates `DisplayID` against `CreatureDisplayInfo` and clears
    /// `DisplayID_Other_Gender` when that display id does not exist.
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        display_store: &CreatureDisplayInfoStore,
    ) -> Result<Self> {
        let mut result = db
            .direct_query(
                "SELECT DisplayID, BoundingRadius, CombatReach, DisplayID_Other_Gender FROM creature_model_info",
            )
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut entries = Vec::new();
        loop {
            let display_id = result.try_read::<u32>(0).unwrap_or(0);
            if display_store.get(display_id).is_none() {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let mut display_id_other_gender = result.try_read::<u32>(3).unwrap_or(0);
            if display_id_other_gender != 0 && display_store.get(display_id_other_gender).is_none()
            {
                display_id_other_gender = 0;
            }

            let combat_reach =
                normalize_combat_reach_like_cpp(result.try_read::<f32>(2).unwrap_or(0.0));

            entries.push(CreatureModelInfoLikeCpp {
                display_id,
                bounding_radius: result.try_read::<f32>(1).unwrap_or(0.0),
                combat_reach,
                display_id_other_gender,
            });

            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_entries(entries))
    }

    pub fn get(&self, display_id: u32) -> Option<&CreatureModelInfoLikeCpp> {
        self.entries.get(&display_id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_combat_reach_like_cpp;

    #[test]
    fn combat_reach_below_cpp_floor_uses_player_default() {
        assert_eq!(normalize_combat_reach_like_cpp(0.0), 1.5);
        assert_eq!(normalize_combat_reach_like_cpp(0.099), 1.5);
        assert_eq!(normalize_combat_reach_like_cpp(0.1), 0.1);
        assert_eq!(normalize_combat_reach_like_cpp(2.25), 2.25);
    }
}
