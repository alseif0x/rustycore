//! A keys-only reservation while an owned respawn query is outside the Map guard.
//! Checked writers return their inputs on Busy. Legacy fixed-shape facades retain
//! their reservation-empty behavior: queue returns its input, removals fail closed,
//! and scalar add/save or unchecked transport fail-stop instead of fabricating an
//! ordinary success/rejection. New staged callers use the explicit try APIs.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnReserved { pub key: RespawnKey }

impl RespawnStoreLikeCpp {
    pub fn is_reserved(&self, key: RespawnKey) -> bool { self.reserved.contains(&key) }
    pub fn has_reservations(&self) -> bool { !self.reserved.is_empty() }

    pub fn try_queue_actor(&mut self, incoming: PendingRespawn)
        -> Result<Result<(), PendingRespawn>, (RespawnReserved, PendingRespawn)>
    {
        let key = RespawnKey::for_actor(&incoming);
        if self.is_reserved(key) { return Err((RespawnReserved { key }, incoming)); }
        Ok(self.queue_actor(incoming))
    }

    pub fn try_add_info(&mut self, incoming: RespawnInfoLikeCpp)
        -> Result<AddRespawnInfoOutcomeLikeCpp, (RespawnReserved, RespawnInfoLikeCpp)>
    {
        let key = RespawnKey::Persistent(incoming.object_type, incoming.spawn_id);
        if self.is_reserved(key) { return Err((RespawnReserved { key }, incoming)); }
        Ok(self.add_respawn_info_like_cpp(incoming))
    }

    pub fn try_save_row(&mut self, incoming: PersistedRespawnRowLikeCpp)
        -> Result<crate::map_manager::LegacyRespawnTimeAddOutcomeLikeCpp,
            (RespawnReserved, PersistedRespawnRowLikeCpp)>
    {
        let key = RespawnKey::Persistent(incoming.object_type, incoming.spawn_id);
        if self.is_reserved(key) { return Err((RespawnReserved { key }, incoming)); }
        Ok(self.save_row(incoming))
    }

    pub fn try_cancel_owned(&mut self, key: RespawnKey)
        -> Result<Option<OwnedRespawn>, RespawnReserved>
    {
        if self.is_reserved(key) { return Err(RespawnReserved { key }); }
        Ok(self.cancel_owned(key))
    }

    pub fn try_save_actor_row(&mut self, incoming: PendingRespawn, map_id: u16,
        instance_id: u32, now: Instant, now_secs: i64)
        -> Result<(PendingRespawn, Option<wow_persistence::RespawnPersistenceMutationLikeCpp>),
            (RespawnReserved, PendingRespawn)>
    {
        let key = RespawnKey::for_actor(&incoming);
        if self.is_reserved(key) { return Err((RespawnReserved { key }, incoming)); }
        let mutation = self.save_actor_row(&incoming, map_id, instance_id, now, now_secs);
        Ok((incoming, mutation))
    }

    pub fn try_remove_info(&mut self, object_type: SpawnObjectType, spawn_id: SpawnId)
        -> Result<Option<RespawnInfoLikeCpp>, RespawnReserved>
    {
        let key = RespawnKey::Persistent(object_type, spawn_id);
        if self.is_reserved(key) { return Err(RespawnReserved { key }); }
        Ok(self.remove_respawn_time_like_cpp(object_type, spawn_id))
    }

    pub fn try_remove_saved_row(&mut self, object_type: SpawnObjectType, spawn_id: SpawnId)
        -> Result<Option<PersistedRespawnRowLikeCpp>, RespawnReserved>
    {
        let key = RespawnKey::Persistent(object_type, spawn_id);
        if self.is_reserved(key) { return Err(RespawnReserved { key }); }
        Ok(self.remove_saved_row(object_type, spawn_id))
    }

    pub(crate) fn reserve_ready_actors(&mut self, now: Instant) -> Vec<PendingRespawn> {
        assert!(!self.has_reservations(), "one operation per respawn owner");
        let ready = self.drain_ready_actors(now);
        self.reserved.extend(ready.iter().map(RespawnKey::for_actor));
        ready
    }

    pub(crate) fn release_respawn_key(&mut self, key: RespawnKey) {
        assert!(self.reserved.remove(&key), "only the owning operation releases a key");
    }
}
