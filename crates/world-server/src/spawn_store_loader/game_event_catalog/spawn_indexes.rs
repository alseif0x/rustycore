//! Per-event spawn, pool, model, flag, quest-relation, and vendor indexes.

use wow_map::{SpawnId, SpawnObjectType};

use super::event_state::GameEventSizingLikeCpp;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventPoolIdsLikeCpp {
    game_event_size: i32,
    pool_ids_by_internal_event_id: Vec<Vec<u32>>,
}

impl GameEventPoolIdsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        Self {
            game_event_size: sizing.game_event_size_like_cpp(),
            pool_ids_by_internal_event_id: vec![Vec::new(); sizing.slot_count_like_cpp()],
        }
    }

    pub fn game_event_size_like_cpp(&self) -> i32 {
        self.game_event_size
    }

    pub fn internal_event_id_like_cpp(&self, event_id: i16) -> Option<usize> {
        let internal_event_id = self.game_event_size + i32::from(event_id) - 1;
        let index = usize::try_from(internal_event_id).ok()?;
        (index < self.pool_ids_by_internal_event_id.len()).then_some(index)
    }

    pub fn pool_ids_like_cpp(&self, event_id: i16) -> Option<&[u32]> {
        self.internal_event_id_like_cpp(event_id)
            .and_then(|index| self.pool_ids_by_internal_event_id.get(index))
            .map(Vec::as_slice)
    }

    #[cfg(test)]
    pub fn with_pool_ids_for_event_like_cpp(
        mut self,
        event_id: i16,
        pool_ids: impl IntoIterator<Item = u32>,
    ) -> Self {
        if let Some(index) = self.internal_event_id_like_cpp(event_id) {
            self.pool_ids_by_internal_event_id[index].extend(pool_ids);
        }
        self
    }

    pub(in crate::spawn_store_loader) fn push_pool_id_like_cpp(
        &mut self,
        event_id: i16,
        pool_id: u32,
    ) -> bool {
        let Some(index) = self.internal_event_id_like_cpp(event_id) else {
            return false;
        };
        self.pool_ids_by_internal_event_id[index].push(pool_id);
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventSpawnGuidsLikeCpp {
    game_event_size: i32,
    creature_guids_by_internal_event_id: Vec<Vec<SpawnId>>,
    gameobject_guids_by_internal_event_id: Vec<Vec<SpawnId>>,
}

impl GameEventSpawnGuidsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        Self {
            game_event_size: sizing.game_event_size_like_cpp(),
            creature_guids_by_internal_event_id: vec![Vec::new(); sizing.slot_count_like_cpp()],
            gameobject_guids_by_internal_event_id: vec![Vec::new(); sizing.slot_count_like_cpp()],
        }
    }

    pub fn game_event_size_like_cpp(&self) -> i32 {
        self.game_event_size
    }

    pub fn internal_event_id_like_cpp(&self, event_id: i16) -> Option<usize> {
        let internal_event_id = self.game_event_size + i32::from(event_id) - 1;
        let index = usize::try_from(internal_event_id).ok()?;
        (index < self.creature_guids_by_internal_event_id.len()).then_some(index)
    }

    pub fn creature_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.internal_event_id_like_cpp(event_id)
            .and_then(|index| self.creature_guids_by_internal_event_id.get(index))
            .map(Vec::as_slice)
    }

    pub fn gameobject_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.internal_event_id_like_cpp(event_id)
            .and_then(|index| self.gameobject_guids_by_internal_event_id.get(index))
            .map(Vec::as_slice)
    }

    pub(crate) fn push_guid_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        event_id: i16,
        guid: SpawnId,
    ) -> bool {
        let Some(index) = self.internal_event_id_like_cpp(event_id) else {
            return false;
        };
        match object_type {
            SpawnObjectType::Creature => self.creature_guids_by_internal_event_id[index].push(guid),
            SpawnObjectType::GameObject => {
                self.gameobject_guids_by_internal_event_id[index].push(guid);
            }
            SpawnObjectType::AreaTrigger => return false,
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn truncate_gameobject_guid_buckets_for_test_like_cpp(
        mut self,
        bucket_count: usize,
    ) -> Self {
        self.gameobject_guids_by_internal_event_id
            .truncate(bucket_count);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameEventModelEquipRecordLikeCpp {
    pub spawn_id: SpawnId,
    pub model_id: u32,
    pub model_id_prev: u32,
    pub equipment_id: u8,
    /// C++ member is spelled `equipement_id_prev`; Rust keeps the corrected field name.
    pub equipment_id_prev: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventModelEquipLikeCpp {
    records_by_event_id: Vec<Vec<GameEventModelEquipRecordLikeCpp>>,
}

impl GameEventModelEquipLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        Self {
            records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn records_like_cpp(&self, event_id: u16) -> Option<&[GameEventModelEquipRecordLikeCpp]> {
        self.records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn records_mut_like_cpp(
        &mut self,
        event_id: u16,
    ) -> Option<&mut [GameEventModelEquipRecordLikeCpp]> {
        self.records_by_event_id
            .get_mut(usize::from(event_id))
            .map(Vec::as_mut_slice)
    }

    pub(in crate::spawn_store_loader) fn push_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventModelEquipRecordLikeCpp,
    ) -> bool {
        let Some(records) = self.records_by_event_id.get_mut(usize::from(event_id)) else {
            return false;
        };
        records.push(record);
        true
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameEventNpcFlagRecordLikeCpp {
    pub spawn_id: SpawnId,
    pub npcflag: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcFlagsLikeCpp {
    pub(in crate::spawn_store_loader) records_by_event_id: Vec<Vec<GameEventNpcFlagRecordLikeCpp>>,
}

#[allow(dead_code)]
impl GameEventNpcFlagsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        Self {
            records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn records_like_cpp(&self, event_id: u16) -> Option<&[GameEventNpcFlagRecordLikeCpp]> {
        self.records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn push_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventNpcFlagRecordLikeCpp,
    ) -> bool {
        let Some(records) = self.records_by_event_id.get_mut(usize::from(event_id)) else {
            return false;
        };
        records.push(record);
        true
    }

    pub fn game_event_npc_flag_mask_like_cpp(
        &self,
        spawn_id: SpawnId,
        active_event_ids: &[u16],
    ) -> u64 {
        let mut mask = 0_u64;
        for event_id in active_event_ids {
            let Some(records) = self.records_like_cpp(*event_id) else {
                continue;
            };
            for record in records {
                if record.spawn_id == spawn_id {
                    mask |= record.npcflag;
                }
            }
        }
        mask
    }
}

/// C++ `GameEventMgr.h` `QuestRelation(id, quest)` metadata for GameEvent quest givers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventQuestRelationRecordLikeCpp {
    pub giver_id: u32,
    pub quest_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationsLikeCpp {
    pub(in crate::spawn_store_loader) creature_records_by_event_id:
        Vec<Vec<GameEventQuestRelationRecordLikeCpp>>,
    pub(in crate::spawn_store_loader) gameobject_records_by_event_id:
        Vec<Vec<GameEventQuestRelationRecordLikeCpp>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationCacheUpdateSummaryLikeCpp {
    pub event_id: u16,
    pub activate: bool,
    pub creature_records_seen: usize,
    pub gameobject_records_seen: usize,
    pub creature_inserted: usize,
    pub gameobject_inserted: usize,
    pub creature_removed: usize,
    pub gameobject_removed: usize,
    pub creature_remove_misses: usize,
    pub gameobject_remove_misses: usize,
    pub creature_no_match: usize,
    pub gameobject_no_match: usize,
    pub creature_missing_event_bucket: bool,
    pub gameobject_missing_event_bucket: bool,
    pub creature_skipped_active_other_event: usize,
    pub gameobject_skipped_active_other_event: usize,
}

#[allow(dead_code)]
impl GameEventQuestRelationsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        Self {
            creature_records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
            gameobject_records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn creature_records_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.creature_records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn gameobject_records_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.gameobject_records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub(crate) fn push_creature_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventQuestRelationRecordLikeCpp,
    ) -> bool {
        let Some(records) = self
            .creature_records_by_event_id
            .get_mut(usize::from(event_id))
        else {
            return false;
        };
        records.push(record);
        true
    }

    pub(crate) fn push_gameobject_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventQuestRelationRecordLikeCpp,
    ) -> bool {
        let Some(records) = self
            .gameobject_records_by_event_id
            .get_mut(usize::from(event_id))
        else {
            return false;
        };
        records.push(record);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEventNpcVendorRecordLikeCpp {
    pub spawn_id: SpawnId,
    pub guid: SpawnId,
    pub entry: u32,
    pub item: u32,
    pub maxcount: u32,
    pub incrtime: u32,
    pub extended_cost: u32,
    pub vendor_type: u8,
    pub item_type: u8,
    pub bonus_list_ids: Vec<i32>,
    pub player_condition_id: u32,
    pub ignore_filtering: bool,
    pub event_npc_flag_low32: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcVendorsLikeCpp {
    records_by_event_id: Vec<Vec<GameEventNpcVendorRecordLikeCpp>>,
}

#[allow(dead_code)]
impl GameEventNpcVendorsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        Self {
            records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn records_like_cpp(&self, event_id: u16) -> Option<&[GameEventNpcVendorRecordLikeCpp]> {
        self.records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn records_for_entry_like_cpp(
        &self,
        event_id: u16,
        entry: u32,
    ) -> Option<Vec<&GameEventNpcVendorRecordLikeCpp>> {
        self.records_like_cpp(event_id).map(|records| {
            records
                .iter()
                .filter(|record| record.entry == entry)
                .collect()
        })
    }

    pub(crate) fn push_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventNpcVendorRecordLikeCpp,
    ) -> bool {
        let Some(records) = self.records_by_event_id.get_mut(usize::from(event_id)) else {
            return false;
        };
        records.push(record);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventModelEquipBaselineRecordOutcomeLikeCpp {
    Applied {
        spawn_id: SpawnId,
        model_id_prev: u32,
        equipment_id_prev: u8,
        model_id_after: u32,
        equipment_id_after: u8,
    },
    MissingSpawnMetadata {
        spawn_id: SpawnId,
    },
    MissingCreatureRuntimeRow {
        spawn_id: SpawnId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventModelEquipBaselineChangeSummaryLikeCpp {
    pub event_id: u16,
    pub activate: bool,
    pub records_seen: usize,
    pub records_applied: usize,
    pub missing_event_bucket: bool,
    pub missing_spawn_metadata: usize,
    pub missing_creature_runtime_rows: usize,
    pub record_outcomes: Vec<GameEventModelEquipBaselineRecordOutcomeLikeCpp>,
}
