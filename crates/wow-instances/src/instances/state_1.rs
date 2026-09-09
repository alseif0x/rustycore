//! Instance lifecycle state state definitions, part 1 of 2.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

pub(crate) const INSTANCE_SCRIPT_HEADER_KEY: &str = "Header";

pub(crate) const INSTANCE_SCRIPT_BOSS_STATES_KEY: &str = "BossStates";

pub(crate) const INSTANCE_SCRIPT_ADDITIONAL_DATA_KEY: &str = "AdditionalData";

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
    pub(crate) temporary_instance_locks_by_player:
        HashMap<ObjectGuid, HashMap<InstanceLockKey, InstanceLock>>,
    pub(crate) instance_locks_by_player:
        HashMap<ObjectGuid, HashMap<InstanceLockKey, InstanceLock>>,
    pub(crate) instance_lock_data_by_id: HashMap<u32, Weak<RwLock<SharedInstanceLockData>>>,
    pub(crate) loaded_character_instance_ids_like_cpp: Vec<u32>,
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
    pub(crate) fn from_i64_like_cpp(value: i64) -> Option<Self> {
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

    pub(crate) const fn save_load_normalized_like_cpp(self) -> Self {
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
    pub(crate) fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::I64(value) => serde_json::Value::from(*value),
            Self::F64(value) => serde_json::Value::from(*value),
        }
    }

    pub(crate) fn from_json_number_like_cpp(value: &serde_json::Value) -> Option<Self> {
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
    pub(crate) charges: u8,
    pub(crate) timer_ms: u32,
    pub(crate) timer_started: bool,
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
    pub(crate) boss_id: u32,
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
    pub(crate) dungeon_encounters: [Option<u32>; MAX_DUNGEON_ENCOUNTERS_PER_BOSS],
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
    pub(crate) difficulty_id: u32,
    pub(crate) header: String,
    pub(crate) bosses: Vec<BossInfo>,
    pub(crate) persistent_values: Vec<(String, PersistentInstanceScriptValue)>,
    pub(crate) combat_resurrections: CombatResurrectionTracker,
    pub(crate) entrance_id: u32,
    pub(crate) temporary_entrance_id: u32,
    pub(crate) activated_area_triggers: HashSet<u32>,
}
