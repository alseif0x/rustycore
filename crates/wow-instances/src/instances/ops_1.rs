//! Instance lifecycle state operations, part 1 of 1.
//!
//! The inherent `InstanceLockMgr` impl is divided by responsibility under
//! #658; every method keeps its original body.

use super::*;

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
    /// Mutates the in-memory lock first, then appends the semantic equivalent
    /// of `CHAR_DEL_CHARACTER_INSTANCE_LOCK` +
    /// `CHAR_INS_CHARACTER_INSTANCE_LOCK` to the caller-owned plan.
    pub fn update_instance_lock_for_player_with_persistence_at(
        &mut self,
        plan: &mut InstanceLockPersistencePlanLikeCpp,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        update_event: InstanceLockUpdateEvent,
        schedule: ResetSchedule,
        now: InstanceResetTime,
    ) -> Option<InstanceLock> {
        let lock = self
            .update_instance_lock_for_player_at(player_guid, entries, update_event, schedule, now)?
            .clone();

        plan.push(Self::delete_character_instance_lock_mutation(
            player_guid,
            entries,
        ));
        plan.push(Self::insert_character_instance_lock_mutation(
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
    pub fn update_shared_instance_lock_with_persistence(
        &mut self,
        plan: &mut InstanceLockPersistencePlanLikeCpp,
        update_event: InstanceLockUpdateEvent,
    ) -> Option<SharedInstanceLockData> {
        let shared_data = self.update_shared_instance_lock(update_event)?;

        plan.push(Self::delete_instance_mutation(shared_data.instance_id));
        plan.push(Self::insert_instance_mutation(&shared_data));

        Some(shared_data)
    }
    pub fn cleanup_unreferenced_shared_instance_lock_data_like_cpp(
        &mut self,
        instance_id: u32,
    ) -> Option<InstanceLockPersistenceMutationLikeCpp> {
        let weak_data = self.instance_lock_data_by_id.get(&instance_id)?;
        if weak_data.upgrade().is_some() {
            return None;
        }

        self.instance_lock_data_by_id.remove(&instance_id);
        Some(Self::delete_instance_mutation(instance_id))
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
    pub fn update_instance_lock_extension_for_player_with_persistence_at(
        &mut self,
        plan: &mut InstanceLockPersistencePlanLikeCpp,
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

        plan.push(Self::update_character_instance_lock_extension_mutation(
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
    pub fn reset_instance_locks_for_player_with_persistence_at(
        &mut self,
        plan: &mut InstanceLockPersistencePlanLikeCpp,
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
                plan.push(Self::force_expire_character_instance_lock_mutation(
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
    pub(crate) fn delete_character_instance_lock_mutation(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
    ) -> InstanceLockPersistenceMutationLikeCpp {
        InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock {
            player_guid_counter: player_guid.counter() as u64,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
        }
    }
    pub(crate) fn insert_character_instance_lock_mutation(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        lock: &InstanceLock,
    ) -> InstanceLockPersistenceMutationLikeCpp {
        InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock {
            player_guid_counter: player_guid.counter() as u64,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
            instance_id: lock.instance_id,
            difficulty_id: entries.difficulty_id,
            data: lock.data.data.clone(),
            completed_encounters_mask: lock.data.completed_encounters_mask,
            entrance_world_safe_loc_id: lock.data.entrance_world_safe_loc_id,
            expiry_time: lock.expiry_time,
            extended: lock.extended,
        }
    }
    pub(crate) fn update_character_instance_lock_extension_mutation(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        extended: bool,
    ) -> InstanceLockPersistenceMutationLikeCpp {
        InstanceLockPersistenceMutationLikeCpp::UpdateCharacterLockExtension {
            extended,
            player_guid_counter: player_guid.counter() as u64,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
        }
    }
    pub(crate) fn force_expire_character_instance_lock_mutation(
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        expiry_time: InstanceResetTime,
    ) -> InstanceLockPersistenceMutationLikeCpp {
        InstanceLockPersistenceMutationLikeCpp::ForceExpireCharacterLock {
            expiry_time,
            player_guid_counter: player_guid.counter() as u64,
            map_id: entries.map_id,
            lock_id: entries.lock_id,
        }
    }
    pub(crate) fn delete_instance_mutation(
        instance_id: u32,
    ) -> InstanceLockPersistenceMutationLikeCpp {
        InstanceLockPersistenceMutationLikeCpp::DeleteSharedInstance { instance_id }
    }
    pub(crate) fn insert_instance_mutation(
        shared_data: &SharedInstanceLockData,
    ) -> InstanceLockPersistenceMutationLikeCpp {
        InstanceLockPersistenceMutationLikeCpp::InsertSharedInstance {
            instance_id: shared_data.instance_id,
            data: shared_data.data.data.clone(),
            completed_encounters_mask: shared_data.data.completed_encounters_mask,
            entrance_world_safe_loc_id: shared_data.data.entrance_world_safe_loc_id,
        }
    }
    pub(crate) fn find_active_instance_lock_inner(
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
