//! Saved rows are not queue requests. JUST_DIED persistence is invisible to
//! GetRespawnInfo/GetRespawnTime until CorpseRemoval or ordinary addInfo queues it.
use super::*;
use crate::map_manager::LegacyRespawnTimeAddOutcomeLikeCpp as Outcome;

impl RespawnStoreLikeCpp {
    pub fn save_actor_row(
        &mut self, respawn: &PendingRespawn, map_id: u16, instance_id: u32,
        now: Instant, now_secs: i64,
    ) -> Option<wow_persistence::RespawnPersistenceMutationLikeCpp> {
        assert!(!self.is_reserved(RespawnKey::for_actor(respawn)),
            "use try_save_actor_row for a reserved respawn key");
        let row = PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: respawn.spawn_id,
            respawn_time: crate::map_manager::respawn_time_from_instant_like_cpp(
                respawn.respawn_at, now, now_secs,
            ),
            map_id, instance_id,
        };
        match self.save_row(row) {
            Outcome::Inserted | Outcome::ReplacedExisting => Some(
                wow_persistence::RespawnPersistenceMutationLikeCpp::Save {
                    key: wow_persistence::RespawnPersistenceKeyLikeCpp {
                        object_type_raw: u16::from(row.object_type as u8),
                        spawn_id: row.spawn_id, map_id: row.map_id, instance_id: row.instance_id,
                    },
                    respawn_time: row.respawn_time,
                }
            ),
            _ => None,
        }
    }

    pub fn save_row(&mut self, row: PersistedRespawnRowLikeCpp) -> Outcome {
        if row.spawn_id == 0 {
            return Outcome::RejectedZeroSpawnId;
        }
        if !Self::has_respawn_map_like_cpp(row.object_type) {
            return Outcome::RejectedUnsupportedType;
        }
        let key = RespawnKey::Persistent(row.object_type, row.spawn_id);
        assert!(!self.is_reserved(key), "use try_save_row for a reserved respawn key");
        let existing = self.slots.get(&key).and_then(RespawnSlot::row);
        if existing.is_some_and(|old| row.respawn_time > old.respawn_time) {
            return Outcome::RejectedExistingSoonerOrEqual;
        }
        let replaced = existing.is_some();
        match self.slots.get_mut(&key) {
            Some(RespawnSlot::SavedOnly(old)) => *old = row,
            Some(RespawnSlot::QueuedCatalog { row: old, .. })
            | Some(RespawnSlot::QueuedActor { row: old, .. }) => *old = Some(row),
            None => { self.slots.insert(key, RespawnSlot::SavedOnly(row)); }
        }
        if replaced { Outcome::ReplacedExisting } else { Outcome::Inserted }
    }

    pub fn saved_row(&self, object_type: SpawnObjectType, spawn_id: SpawnId)
        -> Option<&PersistedRespawnRowLikeCpp>
    {
        self.slots.get(&RespawnKey::Persistent(object_type, spawn_id)).and_then(RespawnSlot::row)
    }

    pub fn saved_rows(&self) -> Vec<PersistedRespawnRowLikeCpp> {
        self.slots.values().filter_map(RespawnSlot::row).copied().collect()
    }

    pub fn remove_saved_row(&mut self, object_type: SpawnObjectType, spawn_id: SpawnId)
        -> Option<PersistedRespawnRowLikeCpp>
    {
        let key = RespawnKey::Persistent(object_type, spawn_id);
        if self.is_reserved(key) { return None; }
        match self.slots.get_mut(&key)? {
            RespawnSlot::SavedOnly(_) => match self.slots.remove(&key)? {
                RespawnSlot::SavedOnly(row) => Some(row),
                _ => unreachable!(),
            },
            RespawnSlot::QueuedCatalog { row, .. } | RespawnSlot::QueuedActor { row, .. } => row.take(),
        }
    }
}
