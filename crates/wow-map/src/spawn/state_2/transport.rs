//! Quiescent, move-only handoff. No clone, timer conversion or second executor.
use super::*;
use crate::MapKey;
use std::sync::MutexGuard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnTransferError {
    WrongMap,
    MissingMap,
    MapBusy,
    StaleIncarnation,
    OccupiedActor { key: RespawnKey },
    SourceNotEmpty,
}

/// The writer guard cannot be released while this ownership envelope is live.
/// APP acquires its existing mutation-order fence before either map guard and
/// must also quiesce producers and any previously drained legacy borrowers.
#[derive(Debug)]
#[must_use = "retain or explicitly dispose the owned respawn transfer"]
pub struct RespawnTransfer<'fence> {
    key: MapKey,
    incarnation: u64,
    store: RespawnStoreLikeCpp,
    _writer_fence: std::marker::PhantomData<&'fence MutexGuard<'fence, ()>>,
}

impl RespawnTransfer<'_> {
    pub fn key(&self) -> MapKey {
        self.key
    }
    pub fn incarnation(&self) -> u64 {
        self.incarnation
    }
}

impl RespawnStoreLikeCpp {
    pub fn try_take_transfer<'fence>(
        &mut self,
        key: MapKey,
        incarnation: u64,
        writer_fence: &'fence MutexGuard<'_, ()>,
    ) -> Result<RespawnTransfer<'fence>, RespawnTransferError> {
        if self.has_reservations() {
            return Err(RespawnTransferError::MapBusy);
        }
        Ok(self.take_transfer(key, incarnation, writer_fence))
    }

    pub fn take_transfer<'fence>(
        &mut self,
        key: MapKey,
        incarnation: u64,
        _writer_fence: &'fence MutexGuard<'_, ()>,
    ) -> RespawnTransfer<'fence> {
        assert!(
            !self.has_reservations(),
            "owned respawn operation must settle before transport"
        );
        RespawnTransfer {
            key,
            incarnation,
            store: std::mem::take(self),
            _writer_fence: std::marker::PhantomData,
        }
    }

    /// Every failure returns the complete envelope and leaves the destination
    /// unchanged. Preflight precedes ALL index/slot mutation.
    pub fn accept_transfer<'fence>(
        &mut self,
        incoming: RespawnTransfer<'fence>,
        key: MapKey,
        incarnation: u64,
    ) -> Result<(), (RespawnTransferError, RespawnTransfer<'fence>)> {
        if self.has_reservations() || incoming.store.has_reservations() {
            return Err((RespawnTransferError::MapBusy, incoming));
        }
        let error = if incoming.key != key {
            Some(RespawnTransferError::WrongMap)
        } else if incoming.incarnation != incarnation {
            Some(RespawnTransferError::StaleIncarnation)
        } else {
            incoming.store.slots.iter().find_map(|(slot_key, slot)| {
                if slot.row().is_some_and(|row| {
                    u32::from(row.map_id) != key.map_id || row.instance_id != key.instance_id
                }) || matches!(slot, RespawnSlot::QueuedActor { payload, .. }
                        if u32::from(payload.map_id) != key.map_id)
                {
                    Some(RespawnTransferError::WrongMap)
                } else if matches!(
                    self.slots.get(slot_key),
                    Some(RespawnSlot::QueuedActor { .. })
                ) {
                    Some(RespawnTransferError::OccupiedActor { key: *slot_key })
                } else {
                    None
                }
            })
        };
        if let Some(error) = error {
            return Err((error, incoming));
        }

        let RespawnTransfer { store, .. } = incoming;
        let RespawnStoreLikeCpp {
            slots, actor_order, ..
        } = store;
        // Catalog/SavedOnly collisions merge their scalar projections with the
        // existing earlier/equal filters. Actor payload wins exclusively: it is
        // moved once and never exposed in a Catalog outcome/summary Record.
        for (key, slot) in slots {
            let previous = self.slots.remove(&key);
            let old_info = previous.as_ref().and_then(RespawnSlot::info).cloned();
            let old_row = previous.as_ref().and_then(RespawnSlot::row).copied();
            let row = match (old_row, slot.row().copied()) {
                (Some(old), Some(new)) if new.respawn_time > old.respawn_time => Some(old),
                (_, Some(new)) => Some(new),
                (old, None) => old,
            };
            let info = match (old_info, slot.info().cloned()) {
                (Some(old), Some(new)) if new.respawn_time > old.respawn_time => Some(old),
                (_, Some(new)) => Some(new),
                (old, None) => old,
            };
            let merged = match slot {
                RespawnSlot::QueuedActor { payload, .. } => {
                    RespawnSlot::QueuedActor { payload, info, row }
                }
                _ => match info {
                    Some(info) => RespawnSlot::QueuedCatalog { info, row },
                    None => RespawnSlot::SavedOnly(row.expect("saved slot retains a row")),
                },
            };
            self.slots.insert(key, merged);
        }
        self.actor_order.extend(actor_order);
        self.respawn_times = self
            .slots
            .values()
            .filter_map(RespawnSlot::info)
            .map(RespawnQueueKey::from_info)
            .collect();
        Ok(())
    }

    pub fn restore_transfer<'fence>(
        &mut self,
        transfer: RespawnTransfer<'fence>,
    ) -> Result<(), (RespawnTransferError, RespawnTransfer<'fence>)> {
        if self.has_reservations() || transfer.store.has_reservations() {
            return Err((RespawnTransferError::MapBusy, transfer));
        }
        if !self.slots.is_empty() {
            return Err((RespawnTransferError::SourceNotEmpty, transfer));
        }
        *self = transfer.store;
        Ok(())
    }
}
