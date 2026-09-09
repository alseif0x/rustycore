//! Spawn object and group model state definitions, part 2 of 2.
//!
//! Separated from the spawn.rs root under #644. Behaviour is preserved.

use super::*;

impl Ord for RespawnQueueKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.respawn_time
            .cmp(&other.respawn_time)
            // Trinity's heap comparator makes larger spawn ids win equal-time ties.
            .then_with(|| other.spawn_id.cmp(&self.spawn_id))
            // Same spawn id can exist for different types; C++ then orders larger type first.
            .then_with(|| other.object_type.cmp(&self.object_type))
    }
}

impl PartialOrd for RespawnQueueKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RespawnStoreLikeCpp {
    pub(super) creature_respawn_times_by_spawn_id: BTreeMap<SpawnId, RespawnInfoLikeCpp>,
    pub(super) gameobject_respawn_times_by_spawn_id: BTreeMap<SpawnId, RespawnInfoLikeCpp>,
    pub(super) respawn_times: BTreeSet<RespawnQueueKey>,
}

impl RespawnStoreLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_respawn_info_like_cpp(
        &mut self,
        info: RespawnInfoLikeCpp,
    ) -> AddRespawnInfoOutcomeLikeCpp {
        if info.spawn_id == 0 {
            return AddRespawnInfoOutcomeLikeCpp::RejectedZeroSpawnId;
        }
        if !Self::has_respawn_map_like_cpp(info.object_type) {
            return AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType;
        }

        let existing = self
            .get_respawn_info_like_cpp(info.object_type, info.spawn_id)
            .cloned();
        let replaced_existing = if let Some(existing) = existing {
            if info.respawn_time <= existing.respawn_time {
                self.remove_respawn_time_like_cpp(info.object_type, info.spawn_id);
                true
            } else {
                return AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual;
            }
        } else {
            false
        };

        self.respawn_times.insert(RespawnQueueKey::from_info(&info));
        let Some(by_spawn_id) = self.map_mut_for_type_like_cpp(info.object_type) else {
            self.respawn_times
                .remove(&RespawnQueueKey::from_info(&info));
            return AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType;
        };
        by_spawn_id.insert(info.spawn_id, info);

        if replaced_existing {
            AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
        } else {
            AddRespawnInfoOutcomeLikeCpp::Inserted
        }
    }

    pub fn get_respawn_time_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> i64 {
        self.get_respawn_info_like_cpp(object_type, spawn_id)
            .map_or(0, |info| info.respawn_time)
    }

    pub fn get_respawn_info_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&RespawnInfoLikeCpp> {
        self.map_for_type_like_cpp(object_type)
            .and_then(|map| map.get(&spawn_id))
    }

    pub fn remove_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<RespawnInfoLikeCpp> {
        let info = self
            .map_mut_for_type_like_cpp(object_type)
            .and_then(|map| map.remove(&spawn_id))?;
        self.respawn_times
            .remove(&RespawnQueueKey::from_info(&info));
        Some(info)
    }

    pub fn unload_all_respawn_infos_like_cpp(&mut self) {
        self.respawn_times.clear();
        self.creature_respawn_times_by_spawn_id.clear();
        self.gameobject_respawn_times_by_spawn_id.clear();
    }

    pub fn respawn_timer_keys_like_cpp(
        &self,
    ) -> impl Iterator<Item = (SpawnObjectType, SpawnId)> + '_ {
        self.respawn_times
            .iter()
            .map(|key| (key.object_type, key.spawn_id))
    }

    pub fn process_due_respawns_like_cpp(
        &mut self,
        now: i64,
        mut is_part_of_pool: impl FnMut(SpawnObjectType, SpawnId) -> Option<u32>,
        mut check_respawn: impl FnMut(&mut RespawnInfoLikeCpp) -> CheckRespawnOutcomeLikeCpp,
    ) -> Vec<ProcessRespawnActionLikeCpp> {
        let mut actions = Vec::new();

        while let Some(next_key) = self.respawn_times.iter().next().copied() {
            if now < next_key.respawn_time {
                break;
            }

            let Some(mut info) =
                self.remove_respawn_time_like_cpp(next_key.object_type, next_key.spawn_id)
            else {
                self.respawn_times.remove(&next_key);
                continue;
            };

            if let Some(pool_id) = is_part_of_pool(info.object_type, info.spawn_id) {
                actions.push(ProcessRespawnActionLikeCpp::UpdatePool {
                    pool_id,
                    object_type: info.object_type,
                    spawn_id: info.spawn_id,
                });
                continue;
            }

            match check_respawn(&mut info) {
                CheckRespawnOutcomeLikeCpp::Allowed => {
                    actions.push(ProcessRespawnActionLikeCpp::DoRespawn {
                        object_type: info.object_type,
                        spawn_id: info.spawn_id,
                        grid_id: info.grid_id,
                    });
                }
                CheckRespawnOutcomeLikeCpp::Blocked if info.respawn_time == 0 => {
                    actions.push(ProcessRespawnActionLikeCpp::DeleteRespawn {
                        object_type: info.object_type,
                        spawn_id: info.spawn_id,
                    });
                }
                CheckRespawnOutcomeLikeCpp::Blocked if now < info.respawn_time => {
                    let stored_info = info.clone();
                    let outcome = self.add_respawn_info_like_cpp(info);
                    debug_assert!(matches!(outcome, AddRespawnInfoOutcomeLikeCpp::Inserted));
                    actions
                        .push(ProcessRespawnActionLikeCpp::RescheduleAndSave { info: stored_info });
                }
                CheckRespawnOutcomeLikeCpp::Blocked => {
                    let stored_info = info.clone();
                    let outcome = self.add_respawn_info_like_cpp(info);
                    debug_assert!(matches!(outcome, AddRespawnInfoOutcomeLikeCpp::Inserted));
                    actions.push(ProcessRespawnActionLikeCpp::InvalidRescheduleNotFuture {
                        info: stored_info,
                    });
                    break;
                }
            }
        }

        actions
    }

    pub(super) const fn has_respawn_map_like_cpp(object_type: SpawnObjectType) -> bool {
        matches!(
            object_type,
            SpawnObjectType::Creature | SpawnObjectType::GameObject
        )
    }

    pub(super) fn map_for_type_like_cpp(
        &self,
        object_type: SpawnObjectType,
    ) -> Option<&BTreeMap<SpawnId, RespawnInfoLikeCpp>> {
        match object_type {
            SpawnObjectType::Creature => Some(&self.creature_respawn_times_by_spawn_id),
            SpawnObjectType::GameObject => Some(&self.gameobject_respawn_times_by_spawn_id),
            SpawnObjectType::AreaTrigger => None,
        }
    }

    pub(super) fn map_mut_for_type_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
    ) -> Option<&mut BTreeMap<SpawnId, RespawnInfoLikeCpp>> {
        match object_type {
            SpawnObjectType::Creature => Some(&mut self.creature_respawn_times_by_spawn_id),
            SpawnObjectType::GameObject => Some(&mut self.gameobject_respawn_times_by_spawn_id),
            SpawnObjectType::AreaTrigger => None,
        }
    }
}
