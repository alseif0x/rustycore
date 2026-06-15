// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal `BattlemasterList.db2` store for GameEvent holiday world-state lookup.
//!
//! C++ anchors:
//! - `GameEventMgr.cpp:1501-1507` uses `BattlemasterListEntry::HolidayWorldState`.
//! - `BattlegroundMgr.cpp:529-566` validates battleground queues from
//!   `BattlemasterListEntry::InstanceType` and queue metadata.
//! - `BattlegroundMgr.cpp:677-690` maps weekend holidays to `BattlegroundTypeId`.
//! - `DB2Structure.h:484-503` and `DB2LoadInfo.h:705-714` place
//!   `InstanceType`, `HolidayWorldState`, and `Flags` in `BattlemasterListEntry`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const HOLIDAY_NONE_LIKE_CPP: u32 = 0;
pub const HOLIDAY_CALL_TO_ARMS_BG_LIKE_CPP: u32 = 435;
pub const HOLIDAY_CALL_TO_ARMS_TP_LIKE_CPP: u32 = 436;
pub const HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP: u32 = 490;
pub const HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP: u32 = 491;
pub const HOLIDAY_CALL_TO_ARMS_ES_LIKE_CPP: u32 = 492;
pub const HOLIDAY_CALL_TO_ARMS_IC_LIKE_CPP: u32 = 493;
pub const HOLIDAY_CALL_TO_ARMS_SA_LIKE_CPP: u32 = 495;
pub const HOLIDAY_CALL_TO_ARMS_WG_LIKE_CPP: u32 = 499;

pub const BATTLEGROUND_TYPE_NONE_LIKE_CPP: u32 = 0;
pub const BATTLEGROUND_AV_LIKE_CPP: u32 = 1;
pub const BATTLEGROUND_WS_LIKE_CPP: u32 = 2;
pub const BATTLEGROUND_AB_LIKE_CPP: u32 = 3;
pub const BATTLEGROUND_AA_LIKE_CPP: u32 = 6;
pub const BATTLEGROUND_EY_LIKE_CPP: u32 = 7;
pub const BATTLEGROUND_SA_LIKE_CPP: u32 = 9;
pub const BATTLEGROUND_IC_LIKE_CPP: u32 = 30;
pub const BATTLEGROUND_TP_LIKE_CPP: u32 = 108;
pub const BATTLEGROUND_BFG_LIKE_CPP: u32 = 120;

pub const MAP_BATTLEGROUND_LIKE_CPP: i8 = 3;
pub const MAP_ARENA_LIKE_CPP: i8 = 4;
pub const BATTLEMASTER_LIST_FLAG_INTERNAL_ONLY_LIKE_CPP: i8 = 0x01;

const BATTLEMASTER_LIST_INSTANCE_TYPE_FIELD_LIKE_CPP: usize = 0;
const BATTLEMASTER_LIST_HOLIDAY_WORLD_STATE_FIELD_LIKE_CPP: usize = 8;
const BATTLEMASTER_LIST_FLAGS_FIELD_LIKE_CPP: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlemasterListEntry {
    pub id: u32,
    pub instance_type: i8,
    pub holiday_world_state: i16,
    pub flags: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolidayWorldStateLookupLikeCpp {
    HolidayNone,
    HolidayNotWeekendBattleground {
        holiday_id: u32,
    },
    BattlemasterListMissing {
        holiday_id: u32,
        battleground_type_id: u32,
    },
    HolidayWorldStateZero {
        holiday_id: u32,
        battleground_type_id: u32,
    },
    SetValueRepresented {
        holiday_id: u32,
        battleground_type_id: u32,
        world_state_id: i16,
    },
}

#[derive(Debug, Clone, Default)]
pub struct BattlemasterListStore {
    entries: HashMap<u32, BattlemasterListEntry>,
}

impl BattlemasterListStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let filename = "BattlemasterList.db2";
        let path = Path::new(data_dir).join("dbc").join(locale).join(filename);
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, record_idx) in reader.iter_records() {
            entries.insert(
                id,
                BattlemasterListEntry {
                    id,
                    instance_type: reader
                        .get_field_i8(record_idx, BATTLEMASTER_LIST_INSTANCE_TYPE_FIELD_LIKE_CPP),
                    holiday_world_state: reader.get_field_i16(
                        record_idx,
                        BATTLEMASTER_LIST_HOLIDAY_WORLD_STATE_FIELD_LIKE_CPP,
                    ),
                    flags: reader.get_field_i8(record_idx, BATTLEMASTER_LIST_FLAGS_FIELD_LIKE_CPP),
                },
            );
        }

        info!("Loaded {} rows from {}", entries.len(), path.display());
        Ok(Self { entries })
    }

    pub fn from_entries(entries: impl IntoIterator<Item = BattlemasterListEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    pub fn get(&self, id: u32) -> Option<&BattlemasterListEntry> {
        self.entries.get(&id)
    }

    pub fn is_internal_only_like_cpp(&self, id: u32) -> bool {
        self.get(id)
            .map(|entry| (entry.flags & BATTLEMASTER_LIST_FLAG_INTERNAL_ONLY_LIKE_CPP) != 0)
            .unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn holiday_world_state_for_weekend_holiday_like_cpp(
        &self,
        holiday_id: u32,
    ) -> HolidayWorldStateLookupLikeCpp {
        if holiday_id == HOLIDAY_NONE_LIKE_CPP {
            return HolidayWorldStateLookupLikeCpp::HolidayNone;
        }

        let battleground_type_id = weekend_holiday_id_to_bg_type_like_cpp(holiday_id);
        if battleground_type_id == BATTLEGROUND_TYPE_NONE_LIKE_CPP {
            return HolidayWorldStateLookupLikeCpp::HolidayNotWeekendBattleground { holiday_id };
        }

        let Some(entry) = self.get(battleground_type_id) else {
            return HolidayWorldStateLookupLikeCpp::BattlemasterListMissing {
                holiday_id,
                battleground_type_id,
            };
        };

        if entry.holiday_world_state == 0 {
            return HolidayWorldStateLookupLikeCpp::HolidayWorldStateZero {
                holiday_id,
                battleground_type_id,
            };
        }

        HolidayWorldStateLookupLikeCpp::SetValueRepresented {
            holiday_id,
            battleground_type_id,
            world_state_id: entry.holiday_world_state,
        }
    }
}

pub fn weekend_holiday_id_to_bg_type_like_cpp(holiday_id: u32) -> u32 {
    match holiday_id {
        HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP => BATTLEGROUND_AV_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_ES_LIKE_CPP => BATTLEGROUND_EY_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_WG_LIKE_CPP => BATTLEGROUND_WS_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_SA_LIKE_CPP => BATTLEGROUND_SA_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP => BATTLEGROUND_AB_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_IC_LIKE_CPP => BATTLEGROUND_IC_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_TP_LIKE_CPP => BATTLEGROUND_TP_LIKE_CPP,
        HOLIDAY_CALL_TO_ARMS_BG_LIKE_CPP => BATTLEGROUND_BFG_LIKE_CPP,
        _ => BATTLEGROUND_TYPE_NONE_LIKE_CPP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekend_holiday_id_to_bg_type_matches_cpp_mapping() {
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP),
            BATTLEGROUND_AV_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_ES_LIKE_CPP),
            BATTLEGROUND_EY_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_WG_LIKE_CPP),
            BATTLEGROUND_WS_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_SA_LIKE_CPP),
            BATTLEGROUND_SA_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP),
            BATTLEGROUND_AB_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_IC_LIKE_CPP),
            BATTLEGROUND_IC_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_TP_LIKE_CPP),
            BATTLEGROUND_TP_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(HOLIDAY_CALL_TO_ARMS_BG_LIKE_CPP),
            BATTLEGROUND_BFG_LIKE_CPP
        );
        assert_eq!(
            weekend_holiday_id_to_bg_type_like_cpp(283),
            BATTLEGROUND_TYPE_NONE_LIKE_CPP
        );
    }

    #[test]
    fn nonzero_holiday_world_state_returns_represented_set_value() {
        let store = BattlemasterListStore::from_entries([BattlemasterListEntry {
            id: BATTLEGROUND_AV_LIKE_CPP,
            instance_type: MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 1234,
            flags: 0,
        }]);

        assert_eq!(
            store
                .holiday_world_state_for_weekend_holiday_like_cpp(HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP),
            HolidayWorldStateLookupLikeCpp::SetValueRepresented {
                holiday_id: HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                battleground_type_id: BATTLEGROUND_AV_LIKE_CPP,
                world_state_id: 1234,
            }
        );
    }

    #[test]
    fn missing_battlemaster_row_returns_explicit_skip() {
        let store = BattlemasterListStore::from_entries([]);

        assert_eq!(
            store
                .holiday_world_state_for_weekend_holiday_like_cpp(HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP),
            HolidayWorldStateLookupLikeCpp::BattlemasterListMissing {
                holiday_id: HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                battleground_type_id: BATTLEGROUND_AV_LIKE_CPP,
            }
        );
    }

    #[test]
    fn zero_holiday_world_state_returns_no_set_value_skip() {
        let store = BattlemasterListStore::from_entries([BattlemasterListEntry {
            id: BATTLEGROUND_AB_LIKE_CPP,
            instance_type: MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 0,
            flags: 0,
        }]);

        assert_eq!(
            store
                .holiday_world_state_for_weekend_holiday_like_cpp(HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP),
            HolidayWorldStateLookupLikeCpp::HolidayWorldStateZero {
                holiday_id: HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP,
                battleground_type_id: BATTLEGROUND_AB_LIKE_CPP,
            }
        );
    }

    #[test]
    fn non_weekend_holiday_returns_explicit_skip() {
        let store = BattlemasterListStore::from_entries([BattlemasterListEntry {
            id: BATTLEGROUND_AV_LIKE_CPP,
            instance_type: MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 1234,
            flags: 0,
        }]);

        assert_eq!(
            store.holiday_world_state_for_weekend_holiday_like_cpp(283),
            HolidayWorldStateLookupLikeCpp::HolidayNotWeekendBattleground { holiday_id: 283 }
        );
    }
}
