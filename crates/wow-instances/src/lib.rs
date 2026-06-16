// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `game/Instances` foundation.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock, Weak},
};

use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{DungeonEncounterEntry, DungeonEncounterStore};
use wow_database::{
    CharStatements, CharacterDatabase, DatabaseError, PreparedStatement, SqlTransaction,
    StatementDef,
};

const INSTANCE_SCRIPT_HEADER_KEY: &str = "Header";
const INSTANCE_SCRIPT_BOSS_STATES_KEY: &str = "BossStates";
const INSTANCE_SCRIPT_ADDITIONAL_DATA_KEY: &str = "AdditionalData";

/// C++ `MAX_DUNGEON_ENCOUNTERS_PER_BOSS`.
pub const MAX_DUNGEON_ENCOUNTERS_PER_BOSS: usize = 4;
/// C++ `INSTANCE_ID_HIGH_MASK`.
pub const INSTANCE_ID_HIGH_MASK: u32 = 0x1F44_0000;
/// C++ `INSTANCE_ID_LFG_MASK`.
pub const INSTANCE_ID_LFG_MASK: u32 = 0x0000_0001;
/// C++ `INSTANCE_ID_NORMAL_MASK`.
pub const INSTANCE_ID_NORMAL_MASK: u32 = 0x0001_0000;
/// C++ `InstanceLockKey = pair<MapDifficultyEntry::MapID, MapDifficultyEntry::LockID>`.
pub type InstanceLockKey = (u32, u32);
/// Unix timestamp seconds used by C++ `system_clock::time_point` lock expiry.
pub type InstanceResetTime = u64;

/// C++ `EncounterState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EncounterState {
    NotStarted = 0,
    InProgress = 1,
    Fail = 2,
    Done = 3,
    Special = 4,
    ToBeDecided = 5,
}

/// C++ `MAP_DIFFICULTY_RESET_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MapDifficultyResetInterval {
    Anytime = 0,
    Daily = 1,
    Weekly = 2,
}

impl MapDifficultyResetInterval {
    pub const fn raid_duration_secs(self) -> u64 {
        match self {
            Self::Daily => 86_400,
            Self::Weekly => 604_800,
            Self::Anytime => 0,
        }
    }
}

/// Minimal C++ `TransferAbortReason` values used by `InstanceLockMgr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TransferAbortReason {
    None = 0,
    LockedToDifferentInstance = 18,
    AlreadyCompletedEncounter = 19,
}

/// C++ `InstanceLockData`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstanceLockData {
    pub data: String,
    pub completed_encounters_mask: u32,
    pub entrance_world_safe_loc_id: u32,
}

/// C++ `SharedInstanceLockData`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SharedInstanceLockData {
    pub instance_id: u32,
    pub data: InstanceLockData,
}

/// C++ `InstanceLock` plus optional `SharedInstanceLock` data.
#[derive(Debug, Clone)]
pub struct InstanceLock {
    pub map_id: u32,
    pub difficulty_id: u8,
    pub instance_id: u32,
    pub expiry_time: InstanceResetTime,
    pub extended: bool,
    pub data: InstanceLockData,
    pub is_in_use: bool,
    pub is_new: bool,
    pub shared_data: Option<Arc<RwLock<SharedInstanceLockData>>>,
}

impl InstanceLock {
    pub fn new(
        map_id: u32,
        difficulty_id: u8,
        expiry_time: InstanceResetTime,
        instance_id: u32,
    ) -> Self {
        Self {
            map_id,
            difficulty_id,
            instance_id,
            expiry_time,
            extended: false,
            data: InstanceLockData::default(),
            is_in_use: false,
            is_new: false,
            shared_data: None,
        }
    }

    pub fn new_shared(
        map_id: u32,
        difficulty_id: u8,
        expiry_time: InstanceResetTime,
        instance_id: u32,
        shared_data: Arc<RwLock<SharedInstanceLockData>>,
    ) -> Self {
        Self {
            shared_data: Some(shared_data),
            ..Self::new(map_id, difficulty_id, expiry_time, instance_id)
        }
    }

    /// C++ `InstanceLock::IsExpired`.
    pub const fn is_expired_at(&self, now: InstanceResetTime) -> bool {
        self.expiry_time < now
    }

    /// C++ `InstanceLock::GetEffectiveExpiryTime`.
    pub fn effective_expiry_time_at(
        &self,
        entries: &MapDb2Entries,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> InstanceResetTime {
        if !self.extended {
            return self.expiry_time;
        }

        if self.is_expired_at(now) {
            return next_reset_time_at(entries, schedule, now);
        }

        self.expiry_time + entries.reset_interval.raid_duration_secs()
    }

    pub fn instance_initialization_data(&self) -> InstanceLockData {
        self.shared_data
            .as_ref()
            .map(|shared| shared.read().unwrap().data.clone())
            .unwrap_or_else(|| self.data.clone())
    }
}

/// Rust-owned view of C++ `MapEntry` + `MapDifficultyEntry` needed by locks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapDb2Entries {
    pub map_id: u32,
    pub difficulty_id: u8,
    pub lock_id: u32,
    pub reset_interval: MapDifficultyResetInterval,
    pub max_players: u32,
    pub is_flex_locking: bool,
    pub is_using_encounter_locks: bool,
}

impl MapDb2Entries {
    pub fn from_stores_like_cpp(
        map_store: &wow_data::MapStore,
        map_difficulty_store: &wow_data::MapDifficultyStore,
        map_id: u32,
        difficulty_id: u8,
    ) -> Option<Self> {
        let map = map_store.get(map_id)?;
        let map_difficulty = map_difficulty_store.get(map_id, difficulty_id)?;

        Some(Self {
            map_id,
            difficulty_id,
            lock_id: u32::from(map_difficulty.lock_id),
            reset_interval: match map_difficulty.reset_interval {
                1 => MapDifficultyResetInterval::Daily,
                2 => MapDifficultyResetInterval::Weekly,
                _ => MapDifficultyResetInterval::Anytime,
            },
            max_players: map_difficulty.max_players,
            is_flex_locking: map.is_flex_locking(),
            is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
        })
    }

    pub fn from_downscaled_stores_like_cpp(
        map_store: &wow_data::MapStore,
        map_difficulty_store: &wow_data::MapDifficultyStore,
        difficulty_store: &wow_data::DifficultyStore,
        map_id: u32,
        difficulty_id: u8,
    ) -> Option<Self> {
        let map = map_store.get(map_id)?;
        let (map_difficulty, effective_difficulty_id) = map_difficulty_store
            .downscaled_for_map_like_cpp(map_id, difficulty_id, difficulty_store)?;

        Some(Self {
            map_id,
            difficulty_id: effective_difficulty_id,
            lock_id: u32::from(map_difficulty.lock_id),
            reset_interval: match map_difficulty.reset_interval {
                1 => MapDifficultyResetInterval::Daily,
                2 => MapDifficultyResetInterval::Weekly,
                _ => MapDifficultyResetInterval::Anytime,
            },
            max_players: map_difficulty.max_players,
            is_flex_locking: map.is_flex_locking(),
            is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
        })
    }

    /// C++ null-guarded `MapDb2Entries::GetKey`.
    pub const fn key(&self) -> InstanceLockKey {
        (self.map_id, self.lock_id)
    }

    /// C++ `MapDb2Entries::IsInstanceIdBound`.
    pub const fn is_instance_id_bound(&self) -> bool {
        !self.is_flex_locking && !self.is_using_encounter_locks
    }

    pub const fn has_reset_schedule(&self) -> bool {
        !matches!(self.reset_interval, MapDifficultyResetInterval::Anytime)
    }
}

/// C++ world reset config values consumed by `GetNextResetTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetSchedule {
    /// C++ `CONFIG_RESET_SCHEDULE_HOUR`, 0..23.
    pub hour: u8,
    /// C++ `CONFIG_RESET_SCHEDULE_WEEK_DAY`, `tm_wday` compatible: Sunday=0.
    pub week_day: u8,
}

impl Default for ResetSchedule {
    fn default() -> Self {
        Self {
            hour: 8,
            week_day: 2,
        }
    }
}

/// C++ `InstanceLockUpdateEvent`, with the completed encounter reduced to its bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceLockUpdateEvent {
    pub instance_id: u32,
    pub new_data: String,
    pub instance_completed_encounters_mask: u32,
    pub completed_encounter_bit: Option<u8>,
    pub entrance_world_safe_loc_id: Option<u32>,
}

/// Row shape loaded from C++ `instance` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedInstanceLockRow {
    pub instance_id: u32,
    pub data: String,
    pub completed_encounters_mask: u32,
    pub entrance_world_safe_loc_id: u32,
}

/// Row shape loaded from C++ `character_instance_lock` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInstanceLockRow {
    pub player_guid_counter: u64,
    pub map_id: u32,
    pub lock_id: u32,
    pub instance_id: u32,
    pub difficulty_id: u8,
    pub data: String,
    pub completed_encounters_mask: u32,
    pub entrance_world_safe_loc_id: u32,
    pub expiry_time: InstanceResetTime,
    pub extended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceLockLoadIssue {
    MissingSharedInstanceData {
        player_guid_counter: u64,
        instance_id: u32,
    },
}

/// C++ `InstanceLocksStatistics`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstanceLocksStatistics {
    pub instance_count: u32,
    pub player_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct InstanceLockResetResult {
    pub reset: Vec<InstanceLock>,
    pub failed_to_reset: Vec<InstanceLock>,
}

/// C++ `WorldPackets::Instance::InstanceLock` data produced by `Player::SendRaidInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRaidInfoLock {
    pub instance_id: u64,
    pub map_id: u32,
    pub difficulty_id: u32,
    pub time_remaining: i32,
    pub completed_mask: u32,
    pub locked: bool,
    pub extended: bool,
}

/// In-memory C++ `InstanceLockMgr` core. DB persistence is intentionally left to
/// the later database wiring step; lock semantics mirror the C++ methods here.
#[derive(Debug, Default)]
pub struct InstanceLockMgr {
    temporary_instance_locks_by_player: HashMap<ObjectGuid, HashMap<InstanceLockKey, InstanceLock>>,
    instance_locks_by_player: HashMap<ObjectGuid, HashMap<InstanceLockKey, InstanceLock>>,
    instance_lock_data_by_id: HashMap<u32, Weak<RwLock<SharedInstanceLockData>>>,
    loaded_character_instance_ids_like_cpp: Vec<u32>,
}

impl InstanceLockMgr {
    pub fn player_lock_map_difficulties(&self, player_guid: ObjectGuid) -> Vec<(u32, u8)> {
        let Some(player_locks) = self.instance_locks_by_player.get(&player_guid) else {
            return Vec::new();
        };

        let mut map_difficulties = player_locks
            .values()
            .map(|lock| (lock.map_id, lock.difficulty_id))
            .collect::<Vec<_>>();
        map_difficulties.sort_unstable();
        map_difficulties.dedup();
        map_difficulties
    }

    pub async fn load_from_database_like_cpp(
        &mut self,
        character_db: &CharacterDatabase,
        entries_for: impl FnMut(u32, u8) -> Option<MapDb2Entries>,
    ) -> Result<Vec<InstanceLockLoadIssue>, DatabaseError> {
        let mut shared_result = character_db
            .query(&character_db.prepare(CharStatements::SEL_INSTANCE))
            .await?;
        let mut shared_rows = Vec::new();
        if !shared_result.is_empty() {
            loop {
                shared_rows.push(SharedInstanceLockRow {
                    instance_id: shared_result.try_read::<u32>(0).unwrap_or(0),
                    data: shared_result.read_string(1),
                    completed_encounters_mask: shared_result.try_read::<u32>(2).unwrap_or(0),
                    entrance_world_safe_loc_id: shared_result.try_read::<u32>(3).unwrap_or(0),
                });
                if !shared_result.next_row() {
                    break;
                }
            }
        }

        let mut character_result = character_db
            .query(&character_db.prepare(CharStatements::SEL_CHARACTER_INSTANCE_LOCK))
            .await?;
        let mut character_rows = Vec::new();
        if !character_result.is_empty() {
            loop {
                let guid = character_result
                    .try_read::<u64>(0)
                    .or_else(|| {
                        character_result
                            .try_read::<i64>(0)
                            .map(|value| value as u64)
                    })
                    .unwrap_or(0);
                character_rows.push(CharacterInstanceLockRow {
                    player_guid_counter: guid,
                    map_id: character_result.try_read::<u32>(1).unwrap_or(0),
                    lock_id: character_result.try_read::<u32>(2).unwrap_or(0),
                    instance_id: character_result.try_read::<u32>(3).unwrap_or(0),
                    difficulty_id: character_result.try_read::<u8>(4).unwrap_or(0),
                    data: character_result.read_string(5),
                    completed_encounters_mask: character_result.try_read::<u32>(6).unwrap_or(0),
                    entrance_world_safe_loc_id: character_result.try_read::<u32>(7).unwrap_or(0),
                    expiry_time: character_result
                        .try_read::<u64>(8)
                        .or_else(|| {
                            character_result
                                .try_read::<i64>(8)
                                .map(|value| value as u64)
                        })
                        .unwrap_or(0),
                    extended: character_result.try_read::<u8>(9).unwrap_or(0) != 0,
                });
                if !character_result.next_row() {
                    break;
                }
            }
        }

        Ok(self.load_from_rows_like_cpp(shared_rows, character_rows, entries_for))
    }

    pub fn load_from_rows_like_cpp(
        &mut self,
        shared_rows: impl IntoIterator<Item = SharedInstanceLockRow>,
        character_rows: impl IntoIterator<Item = CharacterInstanceLockRow>,
        mut entries_for: impl FnMut(u32, u8) -> Option<MapDb2Entries>,
    ) -> Vec<InstanceLockLoadIssue> {
        self.temporary_instance_locks_by_player.clear();
        self.instance_locks_by_player.clear();
        self.instance_lock_data_by_id.clear();
        self.loaded_character_instance_ids_like_cpp.clear();

        let mut shared_data_by_id = HashMap::new();
        for row in shared_rows {
            let shared_data = Arc::new(RwLock::new(SharedInstanceLockData {
                instance_id: row.instance_id,
                data: InstanceLockData {
                    data: row.data,
                    completed_encounters_mask: row.completed_encounters_mask,
                    entrance_world_safe_loc_id: row.entrance_world_safe_loc_id,
                },
            }));
            self.instance_lock_data_by_id
                .insert(row.instance_id, Arc::downgrade(&shared_data));
            shared_data_by_id.insert(row.instance_id, shared_data);
        }

        let mut issues = Vec::new();
        for row in character_rows {
            self.loaded_character_instance_ids_like_cpp
                .push(row.instance_id);
            let entries = entries_for(row.map_id, row.difficulty_id).unwrap_or(MapDb2Entries {
                map_id: row.map_id,
                difficulty_id: row.difficulty_id,
                lock_id: row.lock_id,
                reset_interval: MapDifficultyResetInterval::Anytime,
                max_players: 0,
                is_flex_locking: true,
                is_using_encounter_locks: true,
            });
            let player_guid =
                ObjectGuid::create_global(HighGuid::Player, 0, row.player_guid_counter as i64);
            let data = InstanceLockData {
                data: row.data,
                completed_encounters_mask: row.completed_encounters_mask,
                entrance_world_safe_loc_id: row.entrance_world_safe_loc_id,
            };

            let mut lock = if entries.is_instance_id_bound() {
                let Some(shared_data) = shared_data_by_id.get(&row.instance_id) else {
                    issues.push(InstanceLockLoadIssue::MissingSharedInstanceData {
                        player_guid_counter: row.player_guid_counter,
                        instance_id: row.instance_id,
                    });
                    continue;
                };
                InstanceLock::new_shared(
                    row.map_id,
                    row.difficulty_id,
                    row.expiry_time,
                    row.instance_id,
                    Arc::clone(shared_data),
                )
            } else {
                InstanceLock::new(
                    row.map_id,
                    row.difficulty_id,
                    row.expiry_time,
                    row.instance_id,
                )
            };
            lock.data = data;
            lock.extended = row.extended;

            self.instance_locks_by_player
                .entry(player_guid)
                .or_default()
                .insert((row.map_id, row.lock_id), lock);
        }

        issues
    }

    pub fn get_instance_locks_for_player(&self, player_guid: ObjectGuid) -> Vec<&InstanceLock> {
        self.instance_locks_by_player
            .get(&player_guid)
            .map(|locks| locks.values().collect())
            .unwrap_or_default()
    }

    pub fn get_raid_info_locks_for_player_at(
        &self,
        player_guid: ObjectGuid,
        now: InstanceResetTime,
        schedule: ResetSchedule,
        mut entries_for: impl FnMut(u32, u8) -> Option<MapDb2Entries>,
    ) -> Vec<InstanceRaidInfoLock> {
        self.get_instance_locks_for_player(player_guid)
            .into_iter()
            .map(|lock| {
                let effective_expiry_time = if lock.extended {
                    entries_for(lock.map_id, lock.difficulty_id)
                        .map(|entries| lock.effective_expiry_time_at(&entries, schedule, now))
                        .unwrap_or(lock.expiry_time)
                } else {
                    lock.expiry_time
                };
                let seconds_remaining = effective_expiry_time
                    .saturating_sub(now)
                    .min(i32::MAX as u64);

                InstanceRaidInfoLock {
                    instance_id: u64::from(lock.instance_id),
                    map_id: lock.map_id,
                    difficulty_id: u32::from(lock.difficulty_id),
                    time_remaining: seconds_remaining as i32,
                    completed_mask: lock.data.completed_encounters_mask,
                    locked: !lock.is_expired_at(now),
                    extended: lock.extended,
                }
            })
            .collect()
    }

    pub fn find_active_instance_lock_at(
        &self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        now: InstanceResetTime,
    ) -> Option<&InstanceLock> {
        self.find_active_instance_lock_inner(player_guid, entries, now, false, true)
    }

    pub fn set_active_instance_lock_instance_id_at(
        &mut self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        now: InstanceResetTime,
        instance_id: u32,
    ) -> bool {
        if !entries.has_reset_schedule() {
            return false;
        }

        let key = entries.key();
        if let Some(lock) = self
            .instance_locks_by_player
            .get_mut(&player_guid)
            .and_then(|locks| locks.get_mut(&key))
        {
            if !lock.is_expired_at(now) || lock.extended {
                lock.instance_id = instance_id;
                return true;
            }
        }

        self.temporary_instance_locks_by_player
            .get_mut(&player_guid)
            .and_then(|locks| locks.get_mut(&key))
            .map(|lock| {
                lock.instance_id = instance_id;
            })
            .is_some()
    }

    pub fn create_instance_lock_for_new_instance_at(
        &mut self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        instance_id: u32,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> Option<&InstanceLock> {
        if !entries.has_reset_schedule() {
            return None;
        }

        let expiry_time = next_reset_time_at(entries, schedule, now);
        let mut instance_lock = if entries.is_instance_id_bound() {
            let shared_data = Arc::new(RwLock::new(SharedInstanceLockData::default()));
            self.instance_lock_data_by_id
                .insert(instance_id, Arc::downgrade(&shared_data));
            InstanceLock::new_shared(
                entries.map_id,
                entries.difficulty_id,
                expiry_time,
                instance_id,
                shared_data,
            )
        } else {
            InstanceLock::new(
                entries.map_id,
                entries.difficulty_id,
                expiry_time,
                instance_id,
            )
        };
        instance_lock.is_new = true;

        self.temporary_instance_locks_by_player
            .entry(player_guid)
            .or_default()
            .insert(entries.key(), instance_lock);
        self.temporary_instance_locks_by_player
            .get(&player_guid)?
            .get(&entries.key())
    }

    pub fn update_instance_lock_for_player_at(
        &mut self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        update_event: InstanceLockUpdateEvent,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> Option<&InstanceLock> {
        if !entries.has_reset_schedule() {
            return None;
        }

        let key = entries.key();
        if !self
            .instance_locks_by_player
            .get(&player_guid)
            .and_then(|locks| locks.get(&key))
            .is_some_and(|lock| !lock.is_expired_at(now) || lock.extended)
        {
            let mut promoted_temporary = false;
            if let Some(temp) = self
                .temporary_instance_locks_by_player
                .get_mut(&player_guid)
                .and_then(|locks| locks.remove(&key))
            {
                self.instance_locks_by_player
                    .entry(player_guid)
                    .or_default()
                    .insert(key, temp);
                promoted_temporary = true;
            }
            if self
                .temporary_instance_locks_by_player
                .get(&player_guid)
                .is_some_and(HashMap::is_empty)
            {
                self.temporary_instance_locks_by_player.remove(&player_guid);
            }
            if !promoted_temporary {
                if let Some(player_locks) = self.instance_locks_by_player.get_mut(&player_guid) {
                    player_locks.remove(&key);
                }
            }
        }

        if !self
            .instance_locks_by_player
            .get(&player_guid)
            .is_some_and(|locks| locks.contains_key(&key))
        {
            let expiry_time = next_reset_time_at(entries, schedule, now);
            let instance_lock = if entries.is_instance_id_bound() {
                let shared_data = self
                    .instance_lock_data_by_id
                    .get(&update_event.instance_id)
                    .and_then(Weak::upgrade)
                    .unwrap_or_else(|| {
                        let shared_data = Arc::new(RwLock::new(SharedInstanceLockData {
                            instance_id: update_event.instance_id,
                            data: InstanceLockData::default(),
                        }));
                        self.instance_lock_data_by_id
                            .insert(update_event.instance_id, Arc::downgrade(&shared_data));
                        shared_data
                    });
                InstanceLock::new_shared(
                    entries.map_id,
                    entries.difficulty_id,
                    expiry_time,
                    update_event.instance_id,
                    shared_data,
                )
            } else {
                InstanceLock::new(
                    entries.map_id,
                    entries.difficulty_id,
                    expiry_time,
                    update_event.instance_id,
                )
            };
            self.instance_locks_by_player
                .entry(player_guid)
                .or_default()
                .insert(key, instance_lock);
        }

        let instance_lock = self
            .instance_locks_by_player
            .get_mut(&player_guid)?
            .get_mut(&key)?;
        instance_lock.instance_id = update_event.instance_id;
        instance_lock.is_new = false;
        instance_lock.data.data = update_event.new_data;
        if let Some(bit) = update_event.completed_encounter_bit {
            instance_lock.data.completed_encounters_mask |= 1_u32 << bit;
        }
        if !entries.is_using_encounter_locks {
            instance_lock.data.completed_encounters_mask |=
                update_event.instance_completed_encounters_mask;
        }
        if let Some(entrance_id) = update_event.entrance_world_safe_loc_id {
            instance_lock.data.entrance_world_safe_loc_id = entrance_id;
        }
        if instance_lock.is_expired_at(now) {
            instance_lock.expiry_time = next_reset_time_at(entries, schedule, now);
            instance_lock.extended = false;
        }

        self.instance_locks_by_player
            .get(&player_guid)?
            .get(&entries.key())
    }

    /// C++ `InstanceLockMgr::UpdateInstanceLockForPlayer(trans, ...)`.
    ///
    /// Mutates the in-memory lock first, then appends
    /// `CHAR_DEL_CHARACTER_INSTANCE_LOCK` + `CHAR_INS_CHARACTER_INSTANCE_LOCK`
    /// to the caller-owned transaction.
    pub fn update_instance_lock_for_player_tx_at(
        &mut self,
        tx: &mut SqlTransaction,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        update_event: InstanceLockUpdateEvent,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> Option<InstanceLock> {
        let lock = self
            .update_instance_lock_for_player_at(player_guid, entries, update_event, schedule, now)?
            .clone();

        tx.append(Self::delete_character_instance_lock_statement(
            player_guid,
            entries,
        ));
        tx.append(Self::insert_character_instance_lock_statement(
            player_guid,
            entries,
            &lock,
        ));

        Some(lock)
    }

    pub fn can_join_instance_lock_at(
        &self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        instance_lock: &InstanceLock,
        now: InstanceResetTime,
    ) -> TransferAbortReason {
        let Some(player_instance_lock) =
            self.find_active_instance_lock_at(player_guid, entries, now)
        else {
            return TransferAbortReason::None;
        };

        if entries.is_flex_locking {
            if player_instance_lock.data.completed_encounters_mask
                & !instance_lock.data.completed_encounters_mask
                != 0
            {
                return TransferAbortReason::AlreadyCompletedEncounter;
            }
            return TransferAbortReason::None;
        }

        if !entries.is_using_encounter_locks
            && !player_instance_lock.is_new
            && player_instance_lock.instance_id != instance_lock.instance_id
        {
            return TransferAbortReason::LockedToDifferentInstance;
        }

        TransferAbortReason::None
    }

    pub fn update_shared_instance_lock(
        &mut self,
        update_event: InstanceLockUpdateEvent,
    ) -> Option<SharedInstanceLockData> {
        let shared_data = self
            .instance_lock_data_by_id
            .get(&update_event.instance_id)
            .and_then(Weak::upgrade)?;
        let mut data = shared_data.write().unwrap();
        data.instance_id = update_event.instance_id;
        data.data.data = update_event.new_data;
        if let Some(bit) = update_event.completed_encounter_bit {
            data.data.completed_encounters_mask |= 1_u32 << bit;
        }
        if let Some(entrance_id) = update_event.entrance_world_safe_loc_id {
            data.data.entrance_world_safe_loc_id = entrance_id;
        }
        Some(data.clone())
    }

    /// C++ `InstanceLockMgr::UpdateSharedInstanceLock(trans, ...)`.
    ///
    /// Mutates shared lock data first, then appends `CHAR_DEL_INSTANCE` +
    /// `CHAR_INS_INSTANCE` to the caller-owned transaction.
    pub fn update_shared_instance_lock_tx(
        &mut self,
        tx: &mut SqlTransaction,
        update_event: InstanceLockUpdateEvent,
    ) -> Option<SharedInstanceLockData> {
        let shared_data = self.update_shared_instance_lock(update_event)?;

        tx.append(Self::delete_instance_statement(shared_data.instance_id));
        tx.append(Self::insert_instance_statement(&shared_data));

        Some(shared_data)
    }

    pub fn cleanup_unreferenced_shared_instance_lock_data_like_cpp(
        &mut self,
        instance_id: u32,
    ) -> Option<PreparedStatement> {
        let weak_data = self.instance_lock_data_by_id.get(&instance_id)?;
        if weak_data.upgrade().is_some() {
            return None;
        }

        self.instance_lock_data_by_id.remove(&instance_id);
        Some(Self::delete_instance_statement(instance_id))
    }

    pub fn update_instance_lock_extension_for_player_at(
        &mut self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        extended: bool,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> Option<(InstanceResetTime, InstanceResetTime)> {
        let key = entries.key();
        let lock = self
            .instance_locks_by_player
            .get_mut(&player_guid)?
            .get_mut(&key)?;
        let active = !lock.is_expired_at(now) || lock.extended;
        if !active {
            return None;
        }

        let old_expiry = lock.effective_expiry_time_at(entries, schedule, now);
        lock.extended = extended;
        let new_expiry = lock.effective_expiry_time_at(entries, schedule, now);
        Some((old_expiry, new_expiry))
    }

    /// C++ `InstanceLockMgr::UpdateInstanceLockExtensionForPlayer`.
    ///
    /// Mutates the active lock extension flag and appends the matching
    /// `CHAR_UPD_CHARACTER_INSTANCE_LOCK_EXTENSION` statement.
    pub fn update_instance_lock_extension_for_player_tx_at(
        &mut self,
        tx: &mut SqlTransaction,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        extended: bool,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> Option<(InstanceResetTime, InstanceResetTime)> {
        let expiry_times = self.update_instance_lock_extension_for_player_at(
            player_guid,
            entries,
            extended,
            schedule,
            now,
        )?;

        tx.append(Self::update_character_instance_lock_extension_statement(
            player_guid,
            entries,
            extended,
        ));

        Some(expiry_times)
    }

    pub fn reset_instance_locks_for_player_at(
        &mut self,
        player_guid: ObjectGuid,
        map_id: Option<u32>,
        difficulty_id: Option<u8>,
        entries_by_key: &HashMap<InstanceLockKey, MapDb2Entries>,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> InstanceLockResetResult {
        let mut result = InstanceLockResetResult::default();
        let Some(player_locks) = self.instance_locks_by_player.get_mut(&player_guid) else {
            return result;
        };

        for (key, lock) in player_locks.iter_mut() {
            if map_id.is_some_and(|expected| expected != lock.map_id)
                || difficulty_id.is_some_and(|expected| expected != lock.difficulty_id)
                || lock.is_expired_at(now)
            {
                continue;
            }

            if lock.is_in_use {
                result.failed_to_reset.push(lock.clone());
                continue;
            }

            let Some(entries) = entries_by_key.get(key) else {
                continue;
            };
            lock.expiry_time = next_reset_time_at(entries, schedule, now)
                - entries.reset_interval.raid_duration_secs();
            lock.extended = false;
            result.reset.push(lock.clone());
        }

        result
    }

    /// C++ `InstanceLockMgr::ResetInstanceLocksForPlayer`.
    ///
    /// Mutates resettable locks and appends one
    /// `CHAR_UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE` per reset lock.
    pub fn reset_instance_locks_for_player_tx_at(
        &mut self,
        tx: &mut SqlTransaction,
        player_guid: ObjectGuid,
        map_id: Option<u32>,
        difficulty_id: Option<u8>,
        entries_by_key: &HashMap<InstanceLockKey, MapDb2Entries>,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> InstanceLockResetResult {
        let result = self.reset_instance_locks_for_player_at(
            player_guid,
            map_id,
            difficulty_id,
            entries_by_key,
            schedule,
            now,
        );

        for lock in &result.reset {
            if let Some(entries) = entries_by_key.values().find(|entries| {
                entries.map_id == lock.map_id && entries.difficulty_id == lock.difficulty_id
            }) {
                tx.append(Self::force_expire_character_instance_lock_statement(
                    player_guid,
                    entries,
                    lock.expiry_time,
                ));
            }
        }

        result
    }

    pub fn statistics(&self) -> InstanceLocksStatistics {
        InstanceLocksStatistics {
            instance_count: self.instance_lock_data_by_id.len() as u32,
            player_count: self.instance_locks_by_player.len() as u32,
        }
    }

    /// Instance ids currently referenced by loaded permanent locks, sorted like
    /// C++ `character_instance_lock ORDER BY instanceId` before
    /// `MapManager::RegisterInstanceId`.
    pub fn registered_instance_ids_like_cpp_order(&self) -> Vec<u32> {
        if !self.loaded_character_instance_ids_like_cpp.is_empty() {
            let mut instance_ids = self.loaded_character_instance_ids_like_cpp.clone();
            instance_ids.sort_unstable();
            instance_ids.dedup();
            return instance_ids;
        }

        let mut instance_ids = self
            .instance_locks_by_player
            .values()
            .flat_map(|locks| locks.values().map(|lock| lock.instance_id))
            .collect::<Vec<_>>();
        instance_ids.sort_unstable();
        instance_ids.dedup();
        instance_ids
    }

    pub fn delete_character_instance_lock_statement(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
    ) -> PreparedStatement {
        let mut stmt = PreparedStatement::new(CharStatements::DEL_CHARACTER_INSTANCE_LOCK.sql());
        stmt.set_u64(0, player_guid.counter() as u64);
        stmt.set_u32(1, entries.map_id);
        stmt.set_u32(2, entries.lock_id);
        stmt
    }

    pub fn delete_character_instance_locks_by_guid_statement(
        player_guid: ObjectGuid,
    ) -> PreparedStatement {
        let mut stmt =
            PreparedStatement::new(CharStatements::DEL_CHARACTER_INSTANCE_LOCK_BY_GUID.sql());
        stmt.set_u64(0, player_guid.counter() as u64);
        stmt
    }

    pub fn insert_character_instance_lock_statement(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        lock: &InstanceLock,
    ) -> PreparedStatement {
        let mut stmt = PreparedStatement::new(CharStatements::INS_CHARACTER_INSTANCE_LOCK.sql());
        stmt.set_u64(0, player_guid.counter() as u64);
        stmt.set_u32(1, entries.map_id);
        stmt.set_u32(2, entries.lock_id);
        stmt.set_u32(3, lock.instance_id);
        stmt.set_u8(4, entries.difficulty_id);
        stmt.set_string(5, &lock.data.data);
        stmt.set_u32(6, lock.data.completed_encounters_mask);
        stmt.set_u32(7, lock.data.entrance_world_safe_loc_id);
        stmt.set_u64(8, lock.expiry_time);
        stmt.set_u8(9, u8::from(lock.extended));
        stmt
    }

    pub fn update_character_instance_lock_extension_statement(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        extended: bool,
    ) -> PreparedStatement {
        let mut stmt =
            PreparedStatement::new(CharStatements::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION.sql());
        stmt.set_u8(0, u8::from(extended));
        stmt.set_u64(1, player_guid.counter() as u64);
        stmt.set_u32(2, entries.map_id);
        stmt.set_u32(3, entries.lock_id);
        stmt
    }

    pub fn force_expire_character_instance_lock_statement(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        expiry_time: InstanceResetTime,
    ) -> PreparedStatement {
        let mut stmt =
            PreparedStatement::new(CharStatements::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE.sql());
        stmt.set_u64(0, expiry_time);
        stmt.set_u64(1, player_guid.counter() as u64);
        stmt.set_u32(2, entries.map_id);
        stmt.set_u32(3, entries.lock_id);
        stmt
    }

    pub fn delete_instance_statement(instance_id: u32) -> PreparedStatement {
        let mut stmt = PreparedStatement::new(CharStatements::DEL_INSTANCE.sql());
        stmt.set_u32(0, instance_id);
        stmt
    }

    pub fn insert_instance_statement(shared_data: &SharedInstanceLockData) -> PreparedStatement {
        let mut stmt = PreparedStatement::new(CharStatements::INS_INSTANCE.sql());
        stmt.set_u32(0, shared_data.instance_id);
        stmt.set_string(1, &shared_data.data.data);
        stmt.set_u32(2, shared_data.data.completed_encounters_mask);
        stmt.set_u32(3, shared_data.data.entrance_world_safe_loc_id);
        stmt
    }

    pub fn delete_all_respawns_statement(map_id: u32, instance_id: u32) -> PreparedStatement {
        let mut stmt = PreparedStatement::new(CharStatements::DEL_ALL_RESPAWNS.sql());
        stmt.set_u32(0, map_id);
        stmt.set_u32(1, instance_id);
        stmt
    }

    pub fn delete_account_instance_lock_times_statement(account_id: u32) -> PreparedStatement {
        let mut stmt =
            PreparedStatement::new(CharStatements::DEL_ACCOUNT_INSTANCE_LOCK_TIMES.sql());
        stmt.set_u32(0, account_id);
        stmt
    }

    pub fn insert_account_instance_lock_time_statement(
        account_id: u32,
        instance_id: u32,
        release_time: InstanceResetTime,
    ) -> PreparedStatement {
        let mut stmt =
            PreparedStatement::new(CharStatements::INS_ACCOUNT_INSTANCE_LOCK_TIMES.sql());
        stmt.set_u32(0, account_id);
        stmt.set_u32(1, instance_id);
        stmt.set_u64(2, release_time);
        stmt
    }

    fn find_active_instance_lock_inner(
        &self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        now: InstanceResetTime,
        ignore_temporary: bool,
        ignore_expired: bool,
    ) -> Option<&InstanceLock> {
        if !entries.has_reset_schedule() {
            return None;
        }

        let lock = self
            .instance_locks_by_player
            .get(&player_guid)
            .and_then(|locks| locks.get(&entries.key()));
        if let Some(lock) = lock {
            if !ignore_expired || !lock.is_expired_at(now) || lock.extended {
                return Some(lock);
            }
        }

        if ignore_temporary {
            return None;
        }

        self.temporary_instance_locks_by_player
            .get(&player_guid)
            .and_then(|locks| locks.get(&entries.key()))
    }
}

/// C++ `InstanceLockMgr::GetNextResetTime`, evaluated against an explicit
/// `now` so tests and callers do not rely on wall-clock state.
pub fn next_reset_time_at(
    entries: &MapDb2Entries,
    schedule: ResetSchedule,
    now: InstanceResetTime,
) -> InstanceResetTime {
    if !entries.has_reset_schedule() {
        return now;
    }

    let mut days = (now / 86_400) as i64;
    let mut hour = ((now % 86_400) / 3_600) as i32;
    let reset_hour = i32::from(schedule.hour);

    match entries.reset_interval {
        MapDifficultyResetInterval::Daily => {
            if hour >= reset_hour {
                days += 1;
            }
            hour = reset_hour;
        }
        MapDifficultyResetInterval::Weekly => {
            let reset_day = i64::from(schedule.week_day);
            let week_day = (days + 4).rem_euclid(7);
            let mut days_adjust = reset_day - week_day;
            if week_day > reset_day || (week_day == reset_day && hour >= reset_hour) {
                days_adjust += 7;
            }
            days += days_adjust;
            hour = reset_hour;
        }
        MapDifficultyResetInterval::Anytime => {}
    }

    (days as u64 * 86_400) + (hour as u64 * 3_600)
}

impl Default for EncounterState {
    fn default() -> Self {
        Self::ToBeDecided
    }
}

impl EncounterState {
    fn from_i64_like_cpp(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::NotStarted),
            1 => Some(Self::InProgress),
            2 => Some(Self::Fail),
            3 => Some(Self::Done),
            4 => Some(Self::Special),
            5 => Some(Self::ToBeDecided),
            _ => None,
        }
    }

    const fn save_load_normalized_like_cpp(self) -> Self {
        match self {
            Self::InProgress | Self::Fail | Self::Special => Self::NotStarted,
            other => other,
        }
    }
}

/// Numeric values persisted by C++ `PersistentInstanceScriptValue<T>`.
#[derive(Debug, Clone, PartialEq)]
pub enum PersistentInstanceScriptValue {
    I64(i64),
    F64(f64),
}

impl PersistentInstanceScriptValue {
    fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::I64(value) => serde_json::Value::from(*value),
            Self::F64(value) => serde_json::Value::from(*value),
        }
    }

    fn from_json_number_like_cpp(value: &serde_json::Value) -> Option<Self> {
        value
            .as_i64()
            .map(Self::I64)
            .or_else(|| value.as_f64().map(Self::F64))
    }
}

/// C++ `InstanceScriptDataReader::Result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceScriptDataLoadError {
    MalformedJson,
    RootIsNotAnObject,
    MissingHeader,
    UnexpectedHeader,
    MissingBossStates,
    BossStatesIsNotAnArray,
    UnknownBoss,
    BossStateIsNotANumber,
    AdditionalDataIsNotAnObject,
    AdditionalDataUnexpectedValueType,
}

/// Side effects C++ `InstanceScript::SetBossState` performs after a valid state
/// transition. Runtime callers still own the actual `InstanceMap` operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossStateTransitionPlan {
    pub boss_id: u32,
    pub previous_state: EncounterState,
    pub new_state: EncounterState,
    pub initialize_combat_resurrections: bool,
    pub reset_combat_resurrections: bool,
    pub send_encounter_start: bool,
    pub send_encounter_end: bool,
    pub notify_players_start: bool,
    pub notify_players_end: bool,
    pub dungeon_encounter_id: Option<u32>,
    pub update_lock: bool,
    pub update_criteria: bool,
    pub send_boss_kill_credit: bool,
    pub update_lfg: bool,
    pub update_doors_minions_and_spawn_groups: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatResurrectionEvent {
    GainCharge {
        in_combat_res_count: u8,
        combat_res_charge_recovery: u32,
    },
    InCombatResurrection,
}

/// C++ `InstanceScript` combat-resurrection counters/timer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatResurrectionTracker {
    charges: u8,
    timer_ms: u32,
    timer_started: bool,
}

impl CombatResurrectionTracker {
    pub fn initialize_like_cpp(&mut self, charges: u8, interval_ms: u32) {
        self.charges = charges;
        if interval_ms == 0 {
            return;
        }

        self.timer_ms = interval_ms;
        self.timer_started = true;
    }

    pub fn reset_like_cpp(&mut self) {
        self.charges = 0;
        self.timer_ms = 0;
        self.timer_started = false;
    }

    pub fn add_charge_like_cpp(&mut self, player_count: u32) -> CombatResurrectionEvent {
        self.charges = self.charges.wrapping_add(1);
        self.timer_ms = combat_resurrection_charge_interval_like_cpp(player_count);
        CombatResurrectionEvent::GainCharge {
            in_combat_res_count: self.charges,
            combat_res_charge_recovery: self.timer_ms,
        }
    }

    pub fn use_charge_like_cpp(&mut self) -> CombatResurrectionEvent {
        self.charges = self.charges.wrapping_sub(1);
        CombatResurrectionEvent::InCombatResurrection
    }

    pub fn update_like_cpp(
        &mut self,
        diff_ms: u32,
        player_count: u32,
    ) -> Option<CombatResurrectionEvent> {
        if !self.timer_started {
            return None;
        }

        if self.timer_ms <= diff_ms {
            Some(self.add_charge_like_cpp(player_count))
        } else {
            self.timer_ms -= diff_ms;
            None
        }
    }

    pub const fn charges(&self) -> u8 {
        self.charges
    }

    pub const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }

    pub const fn timer_started(&self) -> bool {
        self.timer_started
    }
}

pub const fn combat_resurrection_charge_interval_like_cpp(player_count: u32) -> u32 {
    if player_count == 0 {
        0
    } else {
        (90 * 60 * 1000) / player_count
    }
}

/// C++ `DungeonEncounterData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonEncounterData {
    pub boss_id: u32,
    pub dungeon_encounter_ids: [u32; MAX_DUNGEON_ENCOUNTERS_PER_BOSS],
}

/// Minimal C++ `BossAI::GetBossId()` contract.
pub trait BossAiLikeCpp {
    fn boss_id(&self) -> u32;
}

/// Small value object for tests and future script/AI adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossAiRef {
    boss_id: u32,
}

impl BossAiRef {
    pub fn new(boss_id: u32) -> Self {
        Self { boss_id }
    }
}

impl BossAiLikeCpp for BossAiRef {
    fn boss_id(&self) -> u32 {
        self.boss_id
    }
}

/// Minimal C++ `BossInfo` data needed for `GetBossDungeonEncounter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossInfo {
    pub state: EncounterState,
    dungeon_encounters: [Option<u32>; MAX_DUNGEON_ENCOUNTERS_PER_BOSS],
}

impl Default for BossInfo {
    fn default() -> Self {
        Self {
            state: EncounterState::ToBeDecided,
            dungeon_encounters: [None; MAX_DUNGEON_ENCOUNTERS_PER_BOSS],
        }
    }
}

impl BossInfo {
    /// C++ `BossInfo::GetDungeonEncounterForDifficulty`.
    pub fn dungeon_encounter_for_difficulty<'a>(
        &self,
        store: &'a DungeonEncounterStore,
        difficulty_id: u32,
    ) -> Option<&'a DungeonEncounterEntry> {
        self.dungeon_encounters
            .iter()
            .flatten()
            .filter_map(|encounter_id| store.get(*encounter_id))
            .find(|encounter| {
                encounter.difficulty_id == 0
                    || u32::try_from(encounter.difficulty_id).ok() == Some(difficulty_id)
            })
    }
}

/// Minimal C++ `InstanceScript` base data for encounter metadata lookup.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceScriptBase {
    difficulty_id: u32,
    header: String,
    bosses: Vec<BossInfo>,
    persistent_values: Vec<(String, PersistentInstanceScriptValue)>,
    combat_resurrections: CombatResurrectionTracker,
    entrance_id: u32,
    temporary_entrance_id: u32,
    activated_area_triggers: HashSet<u32>,
}

impl InstanceScriptBase {
    pub fn new(difficulty_id: u32, boss_count: usize) -> Self {
        Self {
            difficulty_id,
            header: String::new(),
            bosses: vec![BossInfo::default(); boss_count],
            persistent_values: Vec::new(),
            combat_resurrections: CombatResurrectionTracker::default(),
            entrance_id: 0,
            temporary_entrance_id: 0,
            activated_area_triggers: HashSet::new(),
        }
    }

    pub fn difficulty_id(&self) -> u32 {
        self.difficulty_id
    }

    pub fn set_header(&mut self, header: impl Into<String>) {
        self.header = header.into();
    }

    pub fn header(&self) -> &str {
        &self.header
    }

    pub fn boss_count(&self) -> usize {
        self.bosses.len()
    }

    pub fn boss(&self, boss_id: u32) -> Option<&BossInfo> {
        self.bosses.get(boss_id as usize)
    }

    pub fn is_encounter_in_progress_like_cpp(&self) -> bool {
        self.bosses
            .iter()
            .any(|boss| boss.state == EncounterState::InProgress)
    }

    pub fn boss_state(&self, boss_id: u32) -> EncounterState {
        self.boss(boss_id)
            .map(|boss| boss.state)
            .unwrap_or(EncounterState::ToBeDecided)
    }

    pub fn set_boss_state_like_cpp(&mut self, boss_id: u32, state: EncounterState) -> bool {
        let Some(boss) = self.bosses.get_mut(boss_id as usize) else {
            return false;
        };
        boss.state = state;
        true
    }

    pub fn set_boss_state_planned_like_cpp(
        &mut self,
        store: &DungeonEncounterStore,
        boss_id: u32,
        state: EncounterState,
        has_alive_world_boss_minion: bool,
    ) -> Option<BossStateTransitionPlan> {
        let boss = self.bosses.get_mut(boss_id as usize)?;
        let previous_state = boss.state;

        if previous_state == EncounterState::ToBeDecided {
            boss.state = state;
            return None;
        }
        if previous_state == state || previous_state == EncounterState::Done {
            return None;
        }
        if state == EncounterState::Done && has_alive_world_boss_minion {
            return None;
        }

        let dungeon_encounter_id = boss
            .dungeon_encounter_for_difficulty(store, self.difficulty_id)
            .map(|encounter| encounter.id)
            .filter(|_| state == EncounterState::Done);
        boss.state = state;

        Some(BossStateTransitionPlan {
            boss_id,
            previous_state,
            new_state: state,
            initialize_combat_resurrections: state == EncounterState::InProgress,
            reset_combat_resurrections: matches!(
                state,
                EncounterState::Fail | EncounterState::Done
            ),
            send_encounter_start: state == EncounterState::InProgress,
            send_encounter_end: matches!(state, EncounterState::Fail | EncounterState::Done),
            notify_players_start: state == EncounterState::InProgress,
            notify_players_end: matches!(state, EncounterState::Fail | EncounterState::Done),
            dungeon_encounter_id,
            update_lock: dungeon_encounter_id.is_some(),
            update_criteria: dungeon_encounter_id.is_some(),
            send_boss_kill_credit: dungeon_encounter_id.is_some(),
            update_lfg: dungeon_encounter_id.is_some(),
            update_doors_minions_and_spawn_groups: true,
        })
    }

    pub fn create_like_cpp(&mut self) {
        for boss_id in 0..self.bosses.len() {
            self.set_boss_state_like_cpp(boss_id as u32, EncounterState::NotStarted);
        }
    }

    pub fn register_persistent_value_like_cpp(
        &mut self,
        name: impl Into<String>,
        value: PersistentInstanceScriptValue,
    ) {
        self.persistent_values.push((name.into(), value));
    }

    pub fn persistent_value(&self, name: &str) -> Option<&PersistentInstanceScriptValue> {
        self.persistent_values
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value))
    }

    pub fn combat_resurrections(&self) -> CombatResurrectionTracker {
        self.combat_resurrections
    }

    pub fn initialize_combat_resurrections_like_cpp(&mut self, charges: u8, interval_ms: u32) {
        self.combat_resurrections
            .initialize_like_cpp(charges, interval_ms);
    }

    pub fn reset_combat_resurrections_like_cpp(&mut self) {
        self.combat_resurrections.reset_like_cpp();
    }

    pub fn update_combat_resurrection_like_cpp(
        &mut self,
        diff_ms: u32,
        player_count: u32,
    ) -> Option<CombatResurrectionEvent> {
        self.combat_resurrections
            .update_like_cpp(diff_ms, player_count)
    }

    pub fn use_combat_resurrection_like_cpp(&mut self) -> CombatResurrectionEvent {
        self.combat_resurrections.use_charge_like_cpp()
    }

    pub fn set_entrance_location_like_cpp(&mut self, world_safe_location_id: u32) {
        self.entrance_id = world_safe_location_id;
        self.temporary_entrance_id = 0;
    }

    pub fn set_temporary_entrance_location_like_cpp(&mut self, world_safe_location_id: u32) {
        self.temporary_entrance_id = world_safe_location_id;
    }

    pub const fn entrance_location_like_cpp(&self) -> u32 {
        if self.temporary_entrance_id != 0 {
            self.temporary_entrance_id
        } else {
            self.entrance_id
        }
    }

    pub const fn compute_entrance_location_for_completed_encounters_like_cpp(
        &self,
        _completed_encounters_mask: u32,
    ) -> Option<u32> {
        None
    }

    pub fn entrance_location_for_completed_encounters_like_cpp(
        &self,
        is_using_encounter_locks: bool,
        completed_encounters_mask: u32,
    ) -> Option<u32> {
        if !is_using_encounter_locks {
            return Some(self.entrance_id);
        }

        self.compute_entrance_location_for_completed_encounters_like_cpp(completed_encounters_mask)
    }

    pub fn mark_area_trigger_done_like_cpp(&mut self, id: u32) {
        self.activated_area_triggers.insert(id);
    }

    pub fn reset_area_trigger_done_like_cpp(&mut self, id: u32) {
        self.activated_area_triggers.remove(&id);
    }

    pub fn is_area_trigger_done_like_cpp(&self, id: u32) -> bool {
        self.activated_area_triggers.contains(&id)
    }

    pub fn get_save_data_like_cpp(&self) -> String {
        let header = serde_json::to_string(&self.header).unwrap();
        let boss_states = self
            .bosses
            .iter()
            .map(|boss| (boss.state as u8).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut data = format!(
            "{{\"{}\":{},\"{}\":[{}]",
            INSTANCE_SCRIPT_HEADER_KEY, header, INSTANCE_SCRIPT_BOSS_STATES_KEY, boss_states
        );
        if !self.persistent_values.is_empty() {
            let additional = self
                .persistent_values
                .iter()
                .map(|(name, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(name).unwrap(),
                        value.to_json_value()
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            data.push_str(&format!(
                ",\"{}\":{{{}}}",
                INSTANCE_SCRIPT_ADDITIONAL_DATA_KEY, additional
            ));
        }
        data.push('}');
        data
    }

    pub fn load_save_data_like_cpp(
        &mut self,
        data: &str,
    ) -> Result<(), InstanceScriptDataLoadError> {
        let doc: serde_json::Value =
            serde_json::from_str(data).map_err(|_| InstanceScriptDataLoadError::MalformedJson)?;
        let root = doc
            .as_object()
            .ok_or(InstanceScriptDataLoadError::RootIsNotAnObject)?;

        let header = root
            .get(INSTANCE_SCRIPT_HEADER_KEY)
            .ok_or(InstanceScriptDataLoadError::MissingHeader)?;
        if header.as_str() != Some(self.header()) {
            return Err(InstanceScriptDataLoadError::UnexpectedHeader);
        }

        let boss_states = root
            .get(INSTANCE_SCRIPT_BOSS_STATES_KEY)
            .ok_or(InstanceScriptDataLoadError::MissingBossStates)?
            .as_array()
            .ok_or(InstanceScriptDataLoadError::BossStatesIsNotAnArray)?;

        for (boss_id, boss_state) in boss_states.iter().enumerate() {
            if boss_id >= self.bosses.len() {
                return Err(InstanceScriptDataLoadError::UnknownBoss);
            }

            let state_value = boss_state
                .as_i64()
                .ok_or(InstanceScriptDataLoadError::BossStateIsNotANumber)?;
            let Some(state) = EncounterState::from_i64_like_cpp(state_value) else {
                continue;
            };
            let state = state.save_load_normalized_like_cpp();
            if state != EncounterState::ToBeDecided {
                self.set_boss_state_like_cpp(boss_id as u32, state);
            }
        }

        let Some(additional_data) = root.get(INSTANCE_SCRIPT_ADDITIONAL_DATA_KEY) else {
            return Ok(());
        };
        let additional_data = additional_data
            .as_object()
            .ok_or(InstanceScriptDataLoadError::AdditionalDataIsNotAnObject)?;
        for (name, value) in &mut self.persistent_values {
            let Some(saved_value) = additional_data.get(name) else {
                continue;
            };
            if saved_value.is_null() {
                continue;
            }
            *value = PersistentInstanceScriptValue::from_json_number_like_cpp(saved_value)
                .ok_or(InstanceScriptDataLoadError::AdditionalDataUnexpectedValueType)?;
        }

        Ok(())
    }

    /// C++ `InstanceScript::LoadDungeonEncounterData(uint32, array<uint32, 4>)`.
    pub fn load_dungeon_encounter_data(
        &mut self,
        store: &DungeonEncounterStore,
        boss_id: u32,
        dungeon_encounter_ids: [u32; MAX_DUNGEON_ENCOUNTERS_PER_BOSS],
    ) {
        let Some(boss) = self.bosses.get_mut(boss_id as usize) else {
            return;
        };

        for (slot, encounter_id) in dungeon_encounter_ids.into_iter().enumerate() {
            boss.dungeon_encounters[slot] = store.get(encounter_id).map(|entry| entry.id);
        }
    }

    /// C++ `InstanceScript::LoadDungeonEncounterData(T const&)`.
    pub fn load_dungeon_encounter_data_rows(
        &mut self,
        store: &DungeonEncounterStore,
        rows: impl IntoIterator<Item = DungeonEncounterData>,
    ) {
        for row in rows {
            self.load_dungeon_encounter_data(store, row.boss_id, row.dungeon_encounter_ids);
        }
    }

    /// C++ `InstanceScript::GetBossDungeonEncounter(uint32)`.
    pub fn boss_dungeon_encounter<'a>(
        &self,
        store: &'a DungeonEncounterStore,
        boss_id: u32,
    ) -> Option<&'a DungeonEncounterEntry> {
        self.boss(boss_id)?
            .dungeon_encounter_for_difficulty(store, self.difficulty_id)
    }

    pub fn is_encounter_completed_like_cpp(
        &self,
        store: &DungeonEncounterStore,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.bosses.iter().any(|boss| {
            boss.dungeon_encounters
                .iter()
                .flatten()
                .filter_map(|encounter_id| store.get(*encounter_id))
                .any(|encounter| encounter.id == dungeon_encounter_id)
                && boss.state == EncounterState::Done
        })
    }

    pub fn is_encounter_completed_in_mask_by_boss_id_like_cpp(
        &self,
        store: &DungeonEncounterStore,
        completed_encounters_mask: u32,
        boss_id: u32,
    ) -> bool {
        let Some(encounter) = self.boss_dungeon_encounter(store, boss_id) else {
            return false;
        };
        let Ok(bit) = u32::try_from(encounter.bit) else {
            return false;
        };

        (completed_encounters_mask & (1u32 << bit)) != 0
            && self.boss_state(boss_id) == EncounterState::Done
    }

    /// C++ `InstanceScript::GetBossDungeonEncounter(Creature const*)` after
    /// the `dynamic_cast<BossAI const*>` succeeds.
    pub fn boss_dungeon_encounter_for_boss_ai<'a, T: BossAiLikeCpp>(
        &self,
        store: &'a DungeonEncounterStore,
        boss_ai: Option<&T>,
    ) -> Option<&'a DungeonEncounterEntry> {
        self.boss_dungeon_encounter(store, boss_ai?.boss_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_database::SqlParam;

    fn player(counter: i64) -> ObjectGuid {
        ObjectGuid::new(0x10, counter)
    }

    fn encounter(id: u32, difficulty_id: i32) -> DungeonEncounterEntry {
        encounter_with_bit(id, difficulty_id, 0)
    }

    fn encounter_with_bit(id: u32, difficulty_id: i32, bit: i8) -> DungeonEncounterEntry {
        DungeonEncounterEntry {
            id,
            map_id: 631,
            difficulty_id,
            order_index: 0,
            bit,
            flags: 0,
            faction: -1,
        }
    }

    fn raid_entries() -> MapDb2Entries {
        MapDb2Entries {
            map_id: 631,
            difficulty_id: 4,
            lock_id: 7,
            reset_interval: MapDifficultyResetInterval::Weekly,
            max_players: 25,
            is_flex_locking: false,
            is_using_encounter_locks: false,
        }
    }

    fn flex_entries() -> MapDb2Entries {
        MapDb2Entries {
            is_flex_locking: true,
            is_using_encounter_locks: true,
            ..raid_entries()
        }
    }

    fn update_event(instance_id: u32, bit: Option<u8>) -> InstanceLockUpdateEvent {
        InstanceLockUpdateEvent {
            instance_id,
            new_data: "bosses:1".to_string(),
            instance_completed_encounters_mask: 0b100,
            completed_encounter_bit: bit,
            entrance_world_safe_loc_id: Some(42),
        }
    }

    #[test]
    fn map_db2_entries_key_and_binding_match_cpp() {
        let entries = raid_entries();

        assert_eq!(entries.key(), (631, 7));
        assert!(entries.is_instance_id_bound());
        assert!(!flex_entries().is_instance_id_bound());
        assert!(
            !MapDb2Entries {
                reset_interval: MapDifficultyResetInterval::Anytime,
                ..entries
            }
            .has_reset_schedule()
        );
    }

    #[test]
    fn map_db2_entries_from_stores_match_cpp_fields() {
        let maps = wow_data::MapStore::from_entries([wow_data::MapEntry {
            id: 631,
            instance_type: wow_data::map::MAP_RAID,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
        }]);
        let difficulties =
            wow_data::MapDifficultyStore::from_entries([wow_data::MapDifficultyEntry {
                id: 900,
                map_id: 631,
                difficulty_id: 15,
                lock_id: 7,
                reset_interval: 2,
                max_players: 25,
                flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
            }]);

        let entries = MapDb2Entries::from_stores_like_cpp(&maps, &difficulties, 631, 15).unwrap();

        assert_eq!(
            entries,
            MapDb2Entries {
                map_id: 631,
                difficulty_id: 15,
                lock_id: 7,
                reset_interval: MapDifficultyResetInterval::Weekly,
                max_players: 25,
                is_flex_locking: true,
                is_using_encounter_locks: true,
            }
        );
        assert!(MapDb2Entries::from_stores_like_cpp(&maps, &difficulties, 631, 3).is_none());
    }

    #[test]
    fn map_db2_entries_from_downscaled_stores_match_cpp_fields() {
        let maps = wow_data::MapStore::from_entries([wow_data::MapEntry {
            id: 33,
            instance_type: wow_data::map::MAP_INSTANCE,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
        }]);
        let difficulties = wow_data::DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: 5,
                instance_type: 1,
                flags: 0,
                fallback_difficulty_id: 2,
                toggle_difficulty_id: 0,
            },
            wow_data::DifficultyEntry {
                id: 2,
                instance_type: 1,
                flags: 0,
                fallback_difficulty_id: 1,
                toggle_difficulty_id: 0,
            },
        ]);
        let map_difficulties =
            wow_data::MapDifficultyStore::from_entries([wow_data::MapDifficultyEntry {
                id: 900,
                map_id: 33,
                difficulty_id: 2,
                lock_id: 9,
                reset_interval: 1,
                max_players: 5,
                flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
            }]);

        let entries = MapDb2Entries::from_downscaled_stores_like_cpp(
            &maps,
            &map_difficulties,
            &difficulties,
            33,
            5,
        )
        .unwrap();

        assert_eq!(
            entries,
            MapDb2Entries {
                map_id: 33,
                difficulty_id: 2,
                lock_id: 9,
                reset_interval: MapDifficultyResetInterval::Daily,
                max_players: 5,
                is_flex_locking: false,
                is_using_encounter_locks: true,
            }
        );
    }

    #[test]
    fn reset_schedule_default_matches_cpp_world_config_defaults() {
        assert_eq!(
            ResetSchedule::default(),
            ResetSchedule {
                hour: 8,
                week_day: 2,
            }
        );
    }

    #[test]
    fn next_reset_time_daily_and_weekly_match_cpp_hour_rules() {
        let daily = MapDb2Entries {
            reset_interval: MapDifficultyResetInterval::Daily,
            ..raid_entries()
        };
        let schedule = ResetSchedule {
            hour: 9,
            week_day: 2,
        };
        let day10_08 = 10 * 86_400 + 8 * 3_600;
        let day10_10 = 10 * 86_400 + 10 * 3_600;

        assert_eq!(
            next_reset_time_at(&daily, schedule, day10_08),
            10 * 86_400 + 9 * 3_600
        );
        assert_eq!(
            next_reset_time_at(&daily, schedule, day10_10),
            11 * 86_400 + 9 * 3_600
        );

        let weekly = raid_entries();
        let tuesday_08 = 5 * 86_400 + 8 * 3_600;
        let tuesday_10 = 5 * 86_400 + 10 * 3_600;

        assert_eq!(
            next_reset_time_at(&weekly, schedule, tuesday_08),
            5 * 86_400 + 9 * 3_600
        );
        assert_eq!(
            next_reset_time_at(&weekly, schedule, tuesday_10),
            12 * 86_400 + 9 * 3_600
        );
    }

    #[test]
    fn create_instance_lock_for_new_instance_stores_temporary_new_lock_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        let lock = mgr
            .create_instance_lock_for_new_instance_at(
                player(1),
                &entries,
                9001,
                ResetSchedule::default(),
                100,
            )
            .unwrap();

        assert_eq!(lock.instance_id, 9001);
        assert!(lock.is_new);
        assert!(mgr.statistics().instance_count == 1);
        assert!(
            mgr.find_active_instance_lock_at(player(1), &entries, 100)
                .unwrap()
                .is_new
        );
        assert_eq!(mgr.statistics().player_count, 0);
    }

    #[test]
    fn find_active_instance_lock_honors_extended_expired_and_temporary_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(100, None),
            ResetSchedule::default(),
            100,
        );
        mgr.instance_locks_by_player
            .get_mut(&player(1))
            .unwrap()
            .get_mut(&entries.key())
            .unwrap()
            .expiry_time = 10;
        assert!(
            mgr.find_active_instance_lock_at(player(1), &entries, 100)
                .is_none()
        );

        mgr.instance_locks_by_player
            .get_mut(&player(1))
            .unwrap()
            .get_mut(&entries.key())
            .unwrap()
            .extended = true;
        assert!(
            mgr.find_active_instance_lock_at(player(1), &entries, 100)
                .is_some()
        );

        mgr.create_instance_lock_for_new_instance_at(
            player(2),
            &entries,
            200,
            ResetSchedule::default(),
            100,
        );
        assert!(
            mgr.find_active_instance_lock_at(player(2), &entries, 100)
                .is_some()
        );
    }

    #[test]
    fn set_active_instance_lock_instance_id_updates_active_lock_like_cpp() {
        let entries = flex_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.create_instance_lock_for_new_instance_at(
            player(1),
            &entries,
            9001,
            ResetSchedule::default(),
            100,
        );
        assert!(mgr.set_active_instance_lock_instance_id_at(player(1), &entries, 100, 9002));

        assert_eq!(
            mgr.find_active_instance_lock_at(player(1), &entries, 100)
                .unwrap()
                .instance_id,
            9002
        );
    }

    #[test]
    fn set_active_instance_lock_instance_id_skips_expired_permanent_like_cpp() {
        let entries = flex_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(9001, None),
            ResetSchedule::default(),
            100,
        );
        mgr.instance_locks_by_player
            .get_mut(&player(1))
            .unwrap()
            .get_mut(&entries.key())
            .unwrap()
            .expiry_time = 10;

        assert!(!mgr.set_active_instance_lock_instance_id_at(player(1), &entries, 100, 9002));
        assert!(
            mgr.find_active_instance_lock_at(player(1), &entries, 100)
                .is_none()
        );
    }

    #[test]
    fn update_instance_lock_promotes_temporary_and_merges_masks_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.create_instance_lock_for_new_instance_at(
            player(1),
            &entries,
            9001,
            ResetSchedule::default(),
            100,
        );
        let lock = mgr
            .update_instance_lock_for_player_at(
                player(1),
                &entries,
                update_event(9001, Some(1)),
                ResetSchedule::default(),
                100,
            )
            .unwrap();

        assert_eq!(lock.instance_id, 9001);
        assert!(!lock.is_new);
        assert_eq!(lock.data.data, "bosses:1");
        assert_eq!(lock.data.completed_encounters_mask, 0b110);
        assert_eq!(lock.data.entrance_world_safe_loc_id, 42);
        assert!(
            !mgr.temporary_instance_locks_by_player
                .contains_key(&player(1))
        );
        assert_eq!(mgr.statistics().player_count, 1);
    }

    #[test]
    fn update_instance_lock_tx_appends_delete_insert_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();
        let mut tx = SqlTransaction::new();

        let lock = mgr
            .update_instance_lock_for_player_tx_at(
                &mut tx,
                player(1),
                &entries,
                update_event(9001, Some(1)),
                ResetSchedule::default(),
                100,
            )
            .unwrap();

        assert_eq!(tx.len(), 2);
        assert_eq!(lock.instance_id, 9001);
        assert_eq!(lock.data.completed_encounters_mask, 0b110);
    }

    #[test]
    fn update_instance_lock_replaces_expired_non_extended_lock_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            InstanceLockUpdateEvent {
                instance_completed_encounters_mask: 0b1000,
                completed_encounter_bit: Some(0),
                ..update_event(100, None)
            },
            ResetSchedule::default(),
            100,
        );
        let old_lock = mgr
            .instance_locks_by_player
            .get_mut(&player(1))
            .unwrap()
            .get_mut(&entries.key())
            .unwrap();
        old_lock.expiry_time = 10;
        old_lock.data.completed_encounters_mask = 0b1001;

        let new_lock = mgr
            .update_instance_lock_for_player_at(
                player(1),
                &entries,
                InstanceLockUpdateEvent {
                    instance_completed_encounters_mask: 0,
                    completed_encounter_bit: Some(2),
                    ..update_event(200, None)
                },
                ResetSchedule::default(),
                100,
            )
            .unwrap();

        assert_eq!(new_lock.instance_id, 200);
        assert_eq!(new_lock.data.completed_encounters_mask, 0b100);
    }

    #[test]
    fn load_from_rows_reconstructs_shared_and_character_locks_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        let issues = mgr.load_from_rows_like_cpp(
            [SharedInstanceLockRow {
                instance_id: 9001,
                data: "shared".to_string(),
                completed_encounters_mask: 0b1010,
                entrance_world_safe_loc_id: 77,
            }],
            [CharacterInstanceLockRow {
                player_guid_counter: 55,
                map_id: entries.map_id,
                lock_id: entries.lock_id,
                instance_id: 9001,
                difficulty_id: entries.difficulty_id,
                data: "player".to_string(),
                completed_encounters_mask: 0b0010,
                entrance_world_safe_loc_id: 11,
                expiry_time: 500,
                extended: true,
            }],
            |_, _| Some(entries),
        );

        assert!(issues.is_empty());
        let locks =
            mgr.get_instance_locks_for_player(ObjectGuid::create_global(HighGuid::Player, 0, 55));
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].data.data, "player");
        assert_eq!(locks[0].instance_initialization_data().data, "shared");
        assert!(locks[0].extended);
        assert_eq!(mgr.statistics().instance_count, 1);
        assert_eq!(mgr.statistics().player_count, 1);
        assert_eq!(mgr.registered_instance_ids_like_cpp_order(), vec![9001]);
    }

    #[test]
    fn raid_info_locks_for_player_match_cpp_send_raid_info_fields() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.load_from_rows_like_cpp(
            [SharedInstanceLockRow {
                instance_id: 9001,
                data: String::new(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
            }],
            [CharacterInstanceLockRow {
                player_guid_counter: 55,
                map_id: entries.map_id,
                lock_id: entries.lock_id,
                instance_id: 9001,
                difficulty_id: entries.difficulty_id,
                data: "player".to_string(),
                completed_encounters_mask: 0b101,
                entrance_world_safe_loc_id: 0,
                expiry_time: 500,
                extended: false,
            }],
            |_, _| Some(entries),
        );

        let views = mgr.get_raid_info_locks_for_player_at(
            ObjectGuid::create_global(HighGuid::Player, 0, 55),
            100,
            ResetSchedule::default(),
            |_, _| Some(entries),
        );

        assert_eq!(
            views,
            vec![InstanceRaidInfoLock {
                instance_id: 9001,
                map_id: entries.map_id,
                difficulty_id: u32::from(entries.difficulty_id),
                time_remaining: 400,
                completed_mask: 0b101,
                locked: true,
                extended: false,
            }]
        );
    }

    #[test]
    fn raid_info_locks_extend_effective_expiry_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.load_from_rows_like_cpp(
            [SharedInstanceLockRow {
                instance_id: 9001,
                data: String::new(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
            }],
            [CharacterInstanceLockRow {
                player_guid_counter: 55,
                map_id: entries.map_id,
                lock_id: entries.lock_id,
                instance_id: 9001,
                difficulty_id: entries.difficulty_id,
                data: String::new(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
                expiry_time: 500,
                extended: true,
            }],
            |_, _| Some(entries),
        );

        let views = mgr.get_raid_info_locks_for_player_at(
            ObjectGuid::create_global(HighGuid::Player, 0, 55),
            100,
            ResetSchedule::default(),
            |_, _| Some(entries),
        );

        assert_eq!(
            views[0].time_remaining as u64,
            400 + entries.reset_interval.raid_duration_secs()
        );
        assert!(views[0].locked);
        assert!(views[0].extended);
    }

    #[test]
    fn load_from_rows_skips_missing_shared_instance_data_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        let issues = mgr.load_from_rows_like_cpp(
            [],
            [CharacterInstanceLockRow {
                player_guid_counter: 55,
                map_id: entries.map_id,
                lock_id: entries.lock_id,
                instance_id: 9001,
                difficulty_id: entries.difficulty_id,
                data: "player".to_string(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
                expiry_time: 500,
                extended: false,
            }],
            |_, _| Some(entries),
        );

        assert_eq!(
            issues,
            vec![InstanceLockLoadIssue::MissingSharedInstanceData {
                player_guid_counter: 55,
                instance_id: 9001
            }]
        );
        assert!(
            mgr.get_instance_locks_for_player(ObjectGuid::create_global(HighGuid::Player, 0, 55))
                .is_empty()
        );
        assert_eq!(mgr.registered_instance_ids_like_cpp_order(), vec![9001]);
    }

    #[test]
    fn cleanup_unreferenced_shared_instance_data_matches_cpp_delete_path() {
        let mut mgr = InstanceLockMgr::default();

        let issues = mgr.load_from_rows_like_cpp(
            [SharedInstanceLockRow {
                instance_id: 9001,
                data: "orphan".to_string(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
            }],
            [],
            |_, _| None,
        );

        assert!(issues.is_empty());
        assert_eq!(mgr.statistics().instance_count, 1);

        let stmt = mgr
            .cleanup_unreferenced_shared_instance_lock_data_like_cpp(9001)
            .unwrap();

        assert_eq!(stmt.sql(), CharStatements::DEL_INSTANCE.sql());
        assert!(matches!(stmt.params(), [SqlParam::U32(9001)]));
        assert_eq!(mgr.statistics().instance_count, 0);
    }

    #[test]
    fn cleanup_keeps_referenced_shared_instance_data_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.load_from_rows_like_cpp(
            [SharedInstanceLockRow {
                instance_id: 9001,
                data: "shared".to_string(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
            }],
            [CharacterInstanceLockRow {
                player_guid_counter: 55,
                map_id: entries.map_id,
                lock_id: entries.lock_id,
                instance_id: 9001,
                difficulty_id: entries.difficulty_id,
                data: "player".to_string(),
                completed_encounters_mask: 0,
                entrance_world_safe_loc_id: 0,
                expiry_time: 500,
                extended: false,
            }],
            |_, _| Some(entries),
        );

        assert!(
            mgr.cleanup_unreferenced_shared_instance_lock_data_like_cpp(9001)
                .is_none()
        );
        assert_eq!(mgr.statistics().instance_count, 1);
    }

    #[test]
    fn update_shared_instance_lock_tx_appends_delete_insert_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();
        let mut tx = SqlTransaction::new();

        mgr.create_instance_lock_for_new_instance_at(
            player(1),
            &entries,
            9001,
            ResetSchedule::default(),
            100,
        );

        let shared = mgr
            .update_shared_instance_lock_tx(&mut tx, update_event(9001, Some(2)))
            .unwrap();

        assert_eq!(tx.len(), 2);
        assert_eq!(shared.instance_id, 9001);
        assert_eq!(shared.data.completed_encounters_mask, 0b100);
    }

    #[test]
    fn prepared_statement_builders_bind_cxx_parameters_in_order() {
        let entries = raid_entries();
        let guid = player(77);
        let mut lock = InstanceLock::new(entries.map_id, entries.difficulty_id, 500, 9001);
        lock.data.data = "player".to_string();
        lock.data.completed_encounters_mask = 0b11;
        lock.data.entrance_world_safe_loc_id = 42;
        lock.extended = true;

        let del = InstanceLockMgr::delete_character_instance_lock_statement(guid, &entries);
        assert_eq!(del.sql(), CharStatements::DEL_CHARACTER_INSTANCE_LOCK.sql());
        assert!(matches!(
            del.params(),
            [SqlParam::U64(77), SqlParam::U32(631), SqlParam::U32(7)]
        ));

        let ins = InstanceLockMgr::insert_character_instance_lock_statement(guid, &entries, &lock);
        assert_eq!(ins.sql(), CharStatements::INS_CHARACTER_INSTANCE_LOCK.sql());
        assert!(matches!(ins.params()[0], SqlParam::U64(77)));
        assert!(matches!(ins.params()[3], SqlParam::U32(9001)));
        assert!(matches!(ins.params()[4], SqlParam::U8(4)));
        assert!(matches!(&ins.params()[5], SqlParam::String(s) if s == "player"));
        assert!(matches!(ins.params()[6], SqlParam::U32(0b11)));
        assert!(matches!(ins.params()[7], SqlParam::U32(42)));
        assert!(matches!(ins.params()[8], SqlParam::U64(500)));
        assert!(matches!(ins.params()[9], SqlParam::U8(1)));

        let shared = SharedInstanceLockData {
            instance_id: 9001,
            data: InstanceLockData {
                data: "shared".to_string(),
                completed_encounters_mask: 0b101,
                entrance_world_safe_loc_id: 99,
            },
        };
        let ins_instance = InstanceLockMgr::insert_instance_statement(&shared);
        assert_eq!(ins_instance.sql(), CharStatements::INS_INSTANCE.sql());
        assert!(matches!(ins_instance.params()[0], SqlParam::U32(9001)));
        assert!(matches!(&ins_instance.params()[1], SqlParam::String(s) if s == "shared"));

        let del_respawns = InstanceLockMgr::delete_all_respawns_statement(631, 9001);
        assert_eq!(del_respawns.sql(), CharStatements::DEL_ALL_RESPAWNS.sql());
        assert!(matches!(
            del_respawns.params(),
            [SqlParam::U32(631), SqlParam::U32(9001)]
        ));

        let extension = InstanceLockMgr::update_character_instance_lock_extension_statement(
            guid, &entries, true,
        );
        assert_eq!(
            extension.sql(),
            CharStatements::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION.sql()
        );
        assert!(matches!(
            extension.params(),
            [
                SqlParam::U8(1),
                SqlParam::U64(77),
                SqlParam::U32(631),
                SqlParam::U32(7)
            ]
        ));

        let force_expire =
            InstanceLockMgr::force_expire_character_instance_lock_statement(guid, &entries, 1234);
        assert_eq!(
            force_expire.sql(),
            CharStatements::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE.sql()
        );
        assert!(matches!(
            force_expire.params(),
            [
                SqlParam::U64(1234),
                SqlParam::U64(77),
                SqlParam::U32(631),
                SqlParam::U32(7)
            ]
        ));

        let del_times = InstanceLockMgr::delete_account_instance_lock_times_statement(22);
        assert_eq!(
            del_times.sql(),
            CharStatements::DEL_ACCOUNT_INSTANCE_LOCK_TIMES.sql()
        );
        assert!(matches!(del_times.params(), [SqlParam::U32(22)]));

        let ins_time = InstanceLockMgr::insert_account_instance_lock_time_statement(22, 9001, 5555);
        assert_eq!(
            ins_time.sql(),
            CharStatements::INS_ACCOUNT_INSTANCE_LOCK_TIMES.sql()
        );
        assert!(matches!(
            ins_time.params(),
            [SqlParam::U32(22), SqlParam::U32(9001), SqlParam::U64(5555)]
        ));
    }

    #[test]
    fn can_join_instance_lock_blocks_different_non_encounter_instance_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(100, None),
            ResetSchedule::default(),
            100,
        );
        let target_lock = InstanceLock::new(entries.map_id, entries.difficulty_id, 10_000, 200);

        assert_eq!(
            mgr.can_join_instance_lock_at(player(1), &entries, &target_lock, 100),
            TransferAbortReason::LockedToDifferentInstance
        );
    }

    #[test]
    fn can_join_instance_lock_checks_flex_completed_masks_like_cpp() {
        let entries = flex_entries();
        let mut mgr = InstanceLockMgr::default();

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            InstanceLockUpdateEvent {
                instance_completed_encounters_mask: 0,
                completed_encounter_bit: Some(2),
                ..update_event(100, None)
            },
            ResetSchedule::default(),
            100,
        );
        let target_lock = InstanceLock {
            data: InstanceLockData {
                completed_encounters_mask: 0,
                ..InstanceLockData::default()
            },
            ..InstanceLock::new(entries.map_id, entries.difficulty_id, 10_000, 100)
        };

        assert_eq!(
            mgr.can_join_instance_lock_at(player(1), &entries, &target_lock, 100),
            TransferAbortReason::AlreadyCompletedEncounter
        );
    }

    #[test]
    fn update_instance_lock_extension_tx_appends_update_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();
        let mut tx = SqlTransaction::new();
        let schedule = ResetSchedule::default();
        let now = 10 * 86_400;

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(100, None),
            schedule,
            now,
        );

        let (old_expiry, new_expiry) = mgr
            .update_instance_lock_extension_for_player_tx_at(
                &mut tx,
                player(1),
                &entries,
                true,
                schedule,
                now,
            )
            .unwrap();

        assert_eq!(tx.len(), 1);
        assert!(new_expiry > old_expiry);
    }

    #[test]
    fn reset_instance_locks_skips_in_use_and_expires_reset_locks_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();
        let schedule = ResetSchedule::default();
        let now = 10 * 86_400;

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(100, None),
            schedule,
            now,
        );
        mgr.update_instance_lock_for_player_at(
            player(2),
            &entries,
            update_event(200, None),
            schedule,
            now,
        );
        mgr.instance_locks_by_player
            .get_mut(&player(2))
            .unwrap()
            .get_mut(&entries.key())
            .unwrap()
            .is_in_use = true;
        let entries_by_key = HashMap::from([(entries.key(), entries)]);

        let reset_one = mgr.reset_instance_locks_for_player_at(
            player(1),
            None,
            None,
            &entries_by_key,
            schedule,
            now,
        );
        assert_eq!(reset_one.reset.len(), 1);
        assert!(reset_one.failed_to_reset.is_empty());
        assert!(
            mgr.find_active_instance_lock_at(player(1), &entries, now)
                .is_none()
        );

        let reset_two = mgr.reset_instance_locks_for_player_at(
            player(2),
            None,
            None,
            &entries_by_key,
            schedule,
            now,
        );
        assert!(reset_two.reset.is_empty());
        assert_eq!(reset_two.failed_to_reset.len(), 1);
    }

    #[test]
    fn reset_instance_locks_tx_appends_force_expire_like_cpp() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();
        let mut tx = SqlTransaction::new();
        let schedule = ResetSchedule::default();
        let now = 10 * 86_400;

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(100, None),
            schedule,
            now,
        );
        let entries_by_key = HashMap::from([(entries.key(), entries)]);

        let result = mgr.reset_instance_locks_for_player_tx_at(
            &mut tx,
            player(1),
            None,
            None,
            &entries_by_key,
            schedule,
            now,
        );

        assert_eq!(result.reset.len(), 1);
        assert_eq!(tx.len(), 1);
        assert!(
            mgr.find_active_instance_lock_at(player(1), &entries, now)
                .is_none()
        );
    }

    #[test]
    fn player_lock_map_difficulties_are_unique_and_sorted() {
        let entries = raid_entries();
        let mut mgr = InstanceLockMgr::default();
        let schedule = ResetSchedule::default();
        let now = 10 * 86_400;

        mgr.update_instance_lock_for_player_at(
            player(1),
            &entries,
            update_event(100, None),
            schedule,
            now,
        );

        assert_eq!(mgr.player_lock_map_difficulties(player(1)), vec![(631, 4)]);
        assert!(mgr.player_lock_map_difficulties(player(2)).is_empty());
    }

    #[test]
    fn instance_script_create_sets_all_bosses_not_started_like_cpp() {
        let mut script = InstanceScriptBase::new(4, 3);

        script.create_like_cpp();

        assert_eq!(script.boss_state(0), EncounterState::NotStarted);
        assert_eq!(script.boss_state(1), EncounterState::NotStarted);
        assert_eq!(script.boss_state(2), EncounterState::NotStarted);
    }

    #[test]
    fn instance_script_save_data_matches_cpp_json_shape() {
        let mut script = InstanceScriptBase::new(4, 2);
        script.set_header("TEST");
        script.set_boss_state_like_cpp(0, EncounterState::Done);
        script.set_boss_state_like_cpp(1, EncounterState::InProgress);
        script.register_persistent_value_like_cpp("Kills", PersistentInstanceScriptValue::I64(7));
        script.register_persistent_value_like_cpp("Ratio", PersistentInstanceScriptValue::F64(2.5));

        assert_eq!(
            script.get_save_data_like_cpp(),
            "{\"Header\":\"TEST\",\"BossStates\":[3,1],\"AdditionalData\":{\"Kills\":7,\"Ratio\":2.5}}"
        );
    }

    #[test]
    fn instance_script_load_normalizes_transient_boss_states_like_cpp() {
        let mut script = InstanceScriptBase::new(4, 5);
        script.set_header("TEST");
        script.create_like_cpp();

        script
            .load_save_data_like_cpp(
                "{\"Header\":\"TEST\",\"BossStates\":[1,2,3,4,5],\"AdditionalData\":{}}",
            )
            .unwrap();

        assert_eq!(script.boss_state(0), EncounterState::NotStarted);
        assert_eq!(script.boss_state(1), EncounterState::NotStarted);
        assert_eq!(script.boss_state(2), EncounterState::Done);
        assert_eq!(script.boss_state(3), EncounterState::NotStarted);
        assert_eq!(script.boss_state(4), EncounterState::NotStarted);
    }

    #[test]
    fn instance_script_load_persistent_values_like_cpp() {
        let mut script = InstanceScriptBase::new(4, 1);
        script.set_header("TEST");
        script.register_persistent_value_like_cpp("Kills", PersistentInstanceScriptValue::I64(0));
        script.register_persistent_value_like_cpp("Ratio", PersistentInstanceScriptValue::F64(0.0));

        script
            .load_save_data_like_cpp(
                "{\"Header\":\"TEST\",\"BossStates\":[0],\"AdditionalData\":{\"Kills\":9,\"Ratio\":1.25}}",
            )
            .unwrap();

        assert_eq!(
            script.persistent_value("Kills"),
            Some(&PersistentInstanceScriptValue::I64(9))
        );
        assert_eq!(
            script.persistent_value("Ratio"),
            Some(&PersistentInstanceScriptValue::F64(1.25))
        );
    }

    #[test]
    fn instance_script_load_rejects_cpp_error_cases() {
        let mut script = InstanceScriptBase::new(4, 1);
        script.set_header("TEST");
        script.register_persistent_value_like_cpp("Kills", PersistentInstanceScriptValue::I64(0));

        assert_eq!(
            script.load_save_data_like_cpp("{").unwrap_err(),
            InstanceScriptDataLoadError::MalformedJson
        );
        assert_eq!(
            script.load_save_data_like_cpp("[]").unwrap_err(),
            InstanceScriptDataLoadError::RootIsNotAnObject
        );
        assert_eq!(
            script
                .load_save_data_like_cpp("{\"BossStates\":[0]}")
                .unwrap_err(),
            InstanceScriptDataLoadError::MissingHeader
        );
        assert_eq!(
            script
                .load_save_data_like_cpp("{\"Header\":\"BAD\",\"BossStates\":[0]}")
                .unwrap_err(),
            InstanceScriptDataLoadError::UnexpectedHeader
        );
        assert_eq!(
            script
                .load_save_data_like_cpp("{\"Header\":\"TEST\"}")
                .unwrap_err(),
            InstanceScriptDataLoadError::MissingBossStates
        );
        assert_eq!(
            script
                .load_save_data_like_cpp("{\"Header\":\"TEST\",\"BossStates\":{}}")
                .unwrap_err(),
            InstanceScriptDataLoadError::BossStatesIsNotAnArray
        );
        assert_eq!(
            script
                .load_save_data_like_cpp("{\"Header\":\"TEST\",\"BossStates\":[0,0]}")
                .unwrap_err(),
            InstanceScriptDataLoadError::UnknownBoss
        );
        assert_eq!(
            script
                .load_save_data_like_cpp("{\"Header\":\"TEST\",\"BossStates\":[\"x\"]}")
                .unwrap_err(),
            InstanceScriptDataLoadError::BossStateIsNotANumber
        );
        assert_eq!(
            script
                .load_save_data_like_cpp(
                    "{\"Header\":\"TEST\",\"BossStates\":[0],\"AdditionalData\":[]}"
                )
                .unwrap_err(),
            InstanceScriptDataLoadError::AdditionalDataIsNotAnObject
        );
        assert_eq!(
            script
                .load_save_data_like_cpp(
                    "{\"Header\":\"TEST\",\"BossStates\":[0],\"AdditionalData\":{\"Kills\":\"x\"}}"
                )
                .unwrap_err(),
            InstanceScriptDataLoadError::AdditionalDataUnexpectedValueType
        );
    }

    #[test]
    fn instance_script_encounter_progress_helpers_match_cpp() {
        let mut script = InstanceScriptBase::new(4, 2);

        assert!(!script.is_encounter_in_progress_like_cpp());
        script.set_boss_state_like_cpp(1, EncounterState::InProgress);
        assert!(script.is_encounter_in_progress_like_cpp());
        script.set_boss_state_like_cpp(1, EncounterState::Done);
        assert!(!script.is_encounter_in_progress_like_cpp());
    }

    #[test]
    fn instance_script_encounter_completed_by_dungeon_encounter_id_matches_cpp() {
        let store = DungeonEncounterStore::from_entries([
            encounter_with_bit(10, 4, 1),
            encounter_with_bit(20, 4, 2),
        ]);
        let mut script = InstanceScriptBase::new(4, 2);
        script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);
        script.load_dungeon_encounter_data(&store, 1, [20, 0, 0, 0]);

        assert!(!script.is_encounter_completed_like_cpp(&store, 10));
        script.set_boss_state_like_cpp(0, EncounterState::Done);
        assert!(script.is_encounter_completed_like_cpp(&store, 10));
        assert!(!script.is_encounter_completed_like_cpp(&store, 20));
        assert!(!script.is_encounter_completed_like_cpp(&store, 99));
    }

    #[test]
    fn instance_script_encounter_completed_mask_by_boss_id_matches_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
        let mut script = InstanceScriptBase::new(4, 1);
        script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

        script.set_boss_state_like_cpp(0, EncounterState::InProgress);
        assert!(!script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 3, 0));

        script.set_boss_state_like_cpp(0, EncounterState::Done);
        assert!(script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 3, 0));
        assert!(!script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 2, 0));
        assert!(!script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 3, 99));
    }

    #[test]
    fn set_boss_state_loading_initializes_without_effects_like_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
        let mut script = InstanceScriptBase::new(4, 1);
        script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

        let plan =
            script.set_boss_state_planned_like_cpp(&store, 0, EncounterState::NotStarted, false);

        assert!(plan.is_none());
        assert_eq!(script.boss_state(0), EncounterState::NotStarted);
    }

    #[test]
    fn set_boss_state_in_progress_plans_cpp_start_effects() {
        let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
        let mut script = InstanceScriptBase::new(4, 1);
        script.create_like_cpp();
        script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

        let plan = script
            .set_boss_state_planned_like_cpp(&store, 0, EncounterState::InProgress, false)
            .unwrap();

        assert_eq!(plan.previous_state, EncounterState::NotStarted);
        assert_eq!(plan.new_state, EncounterState::InProgress);
        assert!(plan.initialize_combat_resurrections);
        assert!(plan.send_encounter_start);
        assert!(plan.notify_players_start);
        assert!(!plan.reset_combat_resurrections);
        assert!(!plan.send_encounter_end);
        assert_eq!(plan.dungeon_encounter_id, None);
        assert!(plan.update_doors_minions_and_spawn_groups);
    }

    #[test]
    fn set_boss_state_done_plans_cpp_completion_effects() {
        let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
        let mut script = InstanceScriptBase::new(4, 1);
        script.create_like_cpp();
        script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

        let plan = script
            .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Done, false)
            .unwrap();

        assert_eq!(script.boss_state(0), EncounterState::Done);
        assert!(plan.reset_combat_resurrections);
        assert!(plan.send_encounter_end);
        assert!(plan.notify_players_end);
        assert_eq!(plan.dungeon_encounter_id, Some(10));
        assert!(plan.update_lock);
        assert!(plan.update_criteria);
        assert!(plan.send_boss_kill_credit);
        assert!(plan.update_lfg);
        assert!(plan.update_doors_minions_and_spawn_groups);
    }

    #[test]
    fn set_boss_state_blocks_cpp_invalid_transitions() {
        let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
        let mut script = InstanceScriptBase::new(4, 1);
        script.create_like_cpp();
        script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

        assert!(
            script
                .set_boss_state_planned_like_cpp(&store, 99, EncounterState::Done, false)
                .is_none()
        );
        assert!(
            script
                .set_boss_state_planned_like_cpp(&store, 0, EncounterState::NotStarted, false)
                .is_none()
        );
        assert!(
            script
                .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Done, true)
                .is_none()
        );
        assert_eq!(script.boss_state(0), EncounterState::NotStarted);

        script
            .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Done, false)
            .unwrap();
        assert!(
            script
                .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Fail, false)
                .is_none()
        );
        assert_eq!(script.boss_state(0), EncounterState::Done);
    }

    #[test]
    fn combat_resurrection_interval_matches_cpp_player_count_rule() {
        assert_eq!(combat_resurrection_charge_interval_like_cpp(0), 0);
        assert_eq!(combat_resurrection_charge_interval_like_cpp(1), 5_400_000);
        assert_eq!(combat_resurrection_charge_interval_like_cpp(9), 600_000);
    }

    #[test]
    fn combat_resurrection_initialize_update_and_gain_charge_match_cpp() {
        let mut tracker = CombatResurrectionTracker::default();

        tracker.initialize_like_cpp(1, 600_000);
        assert_eq!(tracker.charges(), 1);
        assert_eq!(tracker.timer_ms(), 600_000);
        assert!(tracker.timer_started());

        assert_eq!(tracker.update_like_cpp(100_000, 9), None);
        assert_eq!(tracker.timer_ms(), 500_000);

        assert_eq!(
            tracker.update_like_cpp(500_000, 9),
            Some(CombatResurrectionEvent::GainCharge {
                in_combat_res_count: 2,
                combat_res_charge_recovery: 600_000,
            })
        );
        assert_eq!(tracker.charges(), 2);
        assert_eq!(tracker.timer_ms(), 600_000);
    }

    #[test]
    fn combat_resurrection_use_and_reset_match_cpp() {
        let mut tracker = CombatResurrectionTracker::default();
        tracker.initialize_like_cpp(1, 600_000);

        assert_eq!(
            tracker.use_charge_like_cpp(),
            CombatResurrectionEvent::InCombatResurrection
        );
        assert_eq!(tracker.charges(), 0);

        tracker.reset_like_cpp();
        assert_eq!(tracker.charges(), 0);
        assert_eq!(tracker.timer_ms(), 0);
        assert!(!tracker.timer_started());
        assert_eq!(tracker.update_like_cpp(600_000, 9), None);
    }

    #[test]
    fn instance_script_combat_resurrection_wrappers_use_tracker_like_cpp() {
        let mut script = InstanceScriptBase::new(4, 1);

        script.initialize_combat_resurrections_like_cpp(1, 600_000);
        assert_eq!(script.combat_resurrections().charges(), 1);
        assert_eq!(
            script.update_combat_resurrection_like_cpp(600_000, 9),
            Some(CombatResurrectionEvent::GainCharge {
                in_combat_res_count: 2,
                combat_res_charge_recovery: 600_000,
            })
        );
        assert_eq!(
            script.use_combat_resurrection_like_cpp(),
            CombatResurrectionEvent::InCombatResurrection
        );
        assert_eq!(script.combat_resurrections().charges(), 1);

        script.reset_combat_resurrections_like_cpp();
        assert_eq!(
            script.combat_resurrections(),
            CombatResurrectionTracker::default()
        );
    }

    #[test]
    fn entrance_location_prefers_temporary_and_set_clears_temporary_like_cpp() {
        let mut script = InstanceScriptBase::new(4, 1);

        assert_eq!(script.entrance_location_like_cpp(), 0);
        script.set_entrance_location_like_cpp(100);
        assert_eq!(script.entrance_location_like_cpp(), 100);

        script.set_temporary_entrance_location_like_cpp(200);
        assert_eq!(script.entrance_location_like_cpp(), 200);

        script.set_entrance_location_like_cpp(300);
        assert_eq!(script.entrance_location_like_cpp(), 300);
    }

    #[test]
    fn entrance_location_for_completed_encounters_matches_cpp_base_behavior() {
        let mut script = InstanceScriptBase::new(4, 1);
        script.set_entrance_location_like_cpp(100);

        assert_eq!(
            script.entrance_location_for_completed_encounters_like_cpp(false, 0xFF),
            Some(100)
        );
        assert_eq!(
            script.entrance_location_for_completed_encounters_like_cpp(true, 0xFF),
            None
        );
    }

    #[test]
    fn area_trigger_done_set_matches_cpp_mark_reset_query() {
        let mut script = InstanceScriptBase::new(4, 1);

        assert!(!script.is_area_trigger_done_like_cpp(7));
        script.mark_area_trigger_done_like_cpp(7);
        script.mark_area_trigger_done_like_cpp(7);
        assert!(script.is_area_trigger_done_like_cpp(7));

        script.reset_area_trigger_done_like_cpp(7);
        assert!(!script.is_area_trigger_done_like_cpp(7));
    }

    #[test]
    fn boss_info_selects_first_any_or_matching_difficulty_like_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter(1, 0), encounter(2, 4)]);
        let mut script = InstanceScriptBase::new(4, 1);

        script.load_dungeon_encounter_data(&store, 0, [1, 2, 0, 0]);

        assert_eq!(script.boss_dungeon_encounter(&store, 0).unwrap().id, 1);
    }

    #[test]
    fn boss_info_skips_non_matching_difficulty_like_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter(1, 3), encounter(2, 4)]);
        let mut script = InstanceScriptBase::new(4, 1);

        script.load_dungeon_encounter_data(&store, 0, [1, 2, 0, 0]);

        assert_eq!(script.boss_dungeon_encounter(&store, 0).unwrap().id, 2);
    }

    #[test]
    fn load_dungeon_encounter_data_ignores_invalid_boss_or_missing_rows_like_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter(2, 4)]);
        let mut script = InstanceScriptBase::new(4, 1);

        script.load_dungeon_encounter_data(&store, 99, [2, 0, 0, 0]);
        assert!(script.boss_dungeon_encounter(&store, 0).is_none());

        script.load_dungeon_encounter_data(&store, 0, [1, 0, 0, 0]);
        assert!(script.boss_dungeon_encounter(&store, 0).is_none());
    }

    #[test]
    fn creature_overload_uses_boss_ai_boss_id_like_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter(2, 4)]);
        let mut script = InstanceScriptBase::new(4, 2);
        let boss_ai = BossAiRef::new(1);

        script.load_dungeon_encounter_data(&store, 1, [2, 0, 0, 0]);

        assert_eq!(
            script
                .boss_dungeon_encounter_for_boss_ai(&store, Some(&boss_ai))
                .unwrap()
                .id,
            2
        );
    }

    #[test]
    fn creature_overload_returns_none_when_dynamic_cast_fails_like_cpp() {
        let store = DungeonEncounterStore::from_entries([encounter(2, 4)]);
        let mut script = InstanceScriptBase::new(4, 2);

        script.load_dungeon_encounter_data(&store, 1, [2, 0, 0, 0]);

        assert!(
            script
                .boss_dungeon_encounter_for_boss_ai::<BossAiRef>(&store, None)
                .is_none()
        );
    }
}
