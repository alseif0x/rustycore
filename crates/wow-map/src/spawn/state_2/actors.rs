//! Exact-Instant Actor lane. C++ Map.cpp:2191/Creature.cpp:2651 anchors;
//! the captured metadata/fallback and transient compatibility rail are retained.
use super::*;

impl RespawnKey {
    pub fn for_actor(payload: &PendingRespawn) -> Self {
        if payload.persistent_spawn {
            Self::Persistent(SpawnObjectType::Creature, payload.spawn_id)
        } else {
            Self::TransientCreature(payload.spawn_id)
        }
    }
}

/// Explicit cancellation returns ownership instead of dropping captured metadata.
#[derive(Debug)]
#[must_use = "retain or explicitly dispose the cancelled owned respawn"]
pub struct OwnedRespawn {
    slot: RespawnSlot,
}

impl OwnedRespawn {
    pub fn into_actor(self) -> Option<PendingRespawn> {
        match self.slot {
            RespawnSlot::QueuedActor { payload, .. } => Some(*payload),
            _ => None,
        }
    }
}

impl RespawnStoreLikeCpp {
    /// A later duplicate is returned intact. Earlier/equal duplicates are removed
    /// and appended, retaining the old Vec's observable insertion order.
    pub fn queue_actor(&mut self, incoming: PendingRespawn) -> Result<(), PendingRespawn> {
        let key = RespawnKey::for_actor(&incoming);
        if self.is_reserved(key) { return Err(incoming); }
        if let Some(RespawnSlot::QueuedActor { payload, .. }) = self.slots.get(&key) {
            if incoming.respawn_at > payload.respawn_at {
                return Err(incoming);
            }
        }
        let previous = self.slots.remove(&key);
        let info = previous.as_ref().and_then(RespawnSlot::info).cloned();
        let row = previous.as_ref().and_then(RespawnSlot::row).copied();
        self.actor_order.retain(|old| *old != key);
        self.actor_order.push(key);
        self.slots.insert(key, RespawnSlot::QueuedActor {
            payload: Box::new(incoming), info, row,
        });
        Ok(())
    }

    pub fn actor_queue_len(&self) -> usize {
        self.actor_order.len()
    }

    /// Ready actors leave in original ordinal order, never Unix-time order.
    /// The saved row survives draining until the existing Save/Delete pipeline
    /// explicitly removes it; failure still consumes the payload as before.
    pub fn drain_ready_actors(&mut self, now: Instant) -> Vec<PendingRespawn> {
        let ready: Vec<_> = self.actor_order.iter().copied().filter(|key| {
            !self.is_reserved(*key) && matches!(self.slots.get(key), Some(RespawnSlot::QueuedActor { payload, .. })
                if now >= payload.respawn_at)
        }).collect();
        let mut result = Vec::with_capacity(ready.len());
        for key in ready {
            if let Some(RespawnSlot::QueuedActor { payload, info, row }) = self.slots.remove(&key) {
                if let Some(info) = info {
                    self.respawn_times.remove(&RespawnQueueKey::from_info(&info));
                }
                if let Some(row) = row {
                    self.slots.insert(key, RespawnSlot::SavedOnly(row));
                }
                result.push(*payload);
            }
            self.actor_order.retain(|old| *old != key);
        }
        result
    }

    pub fn cancel_owned(&mut self, key: RespawnKey) -> Option<OwnedRespawn> {
        if self.is_reserved(key) { return None; }
        let slot = self.slots.remove(&key)?;
        if let Some(info) = slot.info() {
            self.respawn_times.remove(&RespawnQueueKey::from_info(info));
        }
        self.actor_order.retain(|old| *old != key);
        Some(OwnedRespawn { slot })
    }
}
