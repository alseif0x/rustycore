//! Scheduled `InstanceMap::Update` expiry and its map-owned recipient snapshot.
//!
//! The one-shot event belongs to the map: it is captured from the raw attached
//! lock expiry and is consumed only by the map's post-session update phase.

use super::{
    CreateMapInstanceLockContext, InstanceResetResultLikeCpp, ManagedMap, MapKey, MapManager,
    MapSessionPassParticipantLikeCpp,
};
use wow_core::{GameTime, ObjectGuid};

/// Current attached-lock data used by `InstanceMap::Update` to schedule its
/// next expiry after the original raw-expiry event fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceMapExpiryContextLikeCpp {
    pub current_expiry_time: u64,
    pub entries: wow_instances::MapDb2Entries,
    pub reset_schedule: wow_instances::ResetSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMapInstanceExpiryUpdateLikeCpp {
    /// Raw expiry captured when this map first attached its lock.
    pub event_time: u64,
    pub instance_lock_context: CreateMapInstanceLockContext,
    pub has_instance_script: bool,
    /// `InstanceMap::Reset(Expire)` result, determined before fanout.
    pub reset_result: InstanceResetResultLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTickInstanceExpiryUpdateLikeCpp {
    pub key: MapKey,
    pub incarnation: u64,
    pub update: ManagedMapInstanceExpiryUpdateLikeCpp,
    /// Current owner incarnations frozen with the reset result under the
    /// MapManager lock. Publication rechecks each against the session registry.
    pub occupants: Vec<MapSessionPassParticipantLikeCpp>,
}

/// Exact map identity needed to decide the C++ last-player unload rule after
/// reading the current lock manager outside the MapManager guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceMapLastLeaveSnapshotLikeCpp {
    pub key: MapKey,
    pub incarnation: u64,
    pub instance_lock_context: CreateMapInstanceLockContext,
    pub leave_time: u64,
}

impl ManagedMap {
    pub fn set_instance_expiry_context_like_cpp(
        &mut self,
        context: Option<InstanceMapExpiryContextLikeCpp>,
    ) {
        if let Some(context) = context {
            if self.instance_expire_event_time.is_none() && context.entries.has_reset_schedule() {
                // TrinityCore stores GetExpiryTime(), even when the lock is
                // extended. Repeated admissions refresh policy only.
                self.instance_expire_event_time = Some(context.current_expiry_time);
            }
        } else {
            self.instance_expire_event_time = None;
        }
        self.instance_expiry_context = context;
    }

    pub(super) fn mark_instance_player_entry_pending_like_cpp(&mut self, guid: ObjectGuid) {
        if self.instance_expire_event_time.is_some() {
            self.pending_instance_map_entries.insert(guid);
        }
    }

    pub(super) fn complete_instance_player_entry_like_cpp(&mut self, guid: ObjectGuid) {
        self.pending_instance_map_entries.remove(&guid);
    }

    pub(super) fn take_last_instance_expiry_update_like_cpp(
        &mut self,
    ) -> Option<ManagedMapInstanceExpiryUpdateLikeCpp> {
        self.last_instance_expiry_update.take()
    }

    pub(super) fn expire_instance_after_map_update_at_like_cpp(
        &mut self,
        now: u64,
    ) -> Option<ManagedMapInstanceExpiryUpdateLikeCpp> {
        let context = self.instance_lock_context?;
        let expiry_context = self.instance_expiry_context?;
        let event_time = self.instance_expire_event_time?;
        // C++ treats zero as an unset event and uses strict `< GameTime`.
        if event_time == 0 || event_time >= now {
            return None;
        }

        let admitted_occupants = self
            .runtime
            .map
            .map_reference_order_like_cpp()
            .iter()
            .copied()
            .filter(|guid| !self.pending_instance_map_entries.contains(guid))
            .collect::<Vec<_>>();
        let reset_result = if !admitted_occupants.is_empty() {
            InstanceResetResultLikeCpp::NotEmpty {
                occupant_guids: admitted_occupants,
            }
        } else if !self.pending_instance_map_entries.is_empty() {
            // A preattached entrant is not in C++'s map-reference set until
            // AddPlayerToMap reaches its entry-prompt boundary.
            return None;
        } else {
            // Expiry bypasses Reset's non-expiry lock-is-new gate. A player
            // admission cancels this one-update unload by clearing can_unload.
            self.can_unload = true;
            InstanceResetResultLikeCpp::Success
        };

        self.instance_expire_event_time = Some(wow_instances::next_reset_time_at(
            &expiry_context.entries,
            expiry_context.reset_schedule,
            now,
        ));

        Some(ManagedMapInstanceExpiryUpdateLikeCpp {
            event_time,
            instance_lock_context: context,
            has_instance_script: self.instance_script.is_some(),
            reset_result,
        })
    }
}

impl MapManager {
    pub fn set_instance_map_expiry_context_like_cpp(
        &mut self,
        key: MapKey,
        context: Option<InstanceMapExpiryContextLikeCpp>,
    ) {
        if let Some(map) = self.find_map_mut(key.map_id, key.instance_id) {
            map.set_instance_expiry_context_like_cpp(context);
        }
    }

    pub fn mark_instance_player_entry_pending_like_cpp(
        &mut self,
        key: MapKey,
        player_guid: ObjectGuid,
    ) {
        if let Some(map) = self.find_map_mut(key.map_id, key.instance_id) {
            map.mark_instance_player_entry_pending_like_cpp(player_guid);
        }
    }

    pub fn complete_instance_player_entry_like_cpp(
        &mut self,
        key: MapKey,
        player_guid: ObjectGuid,
    ) {
        if let Some(map) = self.find_map_mut(key.map_id, key.instance_id) {
            map.complete_instance_player_entry_like_cpp(player_guid);
        }
    }

    pub fn instance_map_last_leave_snapshot_like_cpp(
        &self,
        key: MapKey,
        player_guid: ObjectGuid,
    ) -> Option<InstanceMapLastLeaveSnapshotLikeCpp> {
        let map = self.find_map(key.map_id, key.instance_id)?;
        let occupants = map.map().map_reference_order_like_cpp();
        if !map.kind().is_dungeon()
            || map.player_count() != 1
            || occupants.len() != 1
            || occupants.first() != Some(&player_guid)
        {
            return None;
        }
        Some(InstanceMapLastLeaveSnapshotLikeCpp {
            key,
            incarnation: self.map_incarnation_like_cpp(key)?,
            instance_lock_context: map.instance_lock_context()?,
            leave_time: GameTime::now().as_secs(),
        })
    }

    /// Recheck the departed map after the caller read the canonical lock
    /// manager without holding MapManager. The incarnation, attached lock
    /// token, and empty map must all still match that read.
    pub fn arm_expired_instance_unload_like_cpp(
        &mut self,
        snapshot: InstanceMapLastLeaveSnapshotLikeCpp,
    ) -> bool {
        if self.map_incarnation_like_cpp(snapshot.key) != Some(snapshot.incarnation) {
            return false;
        }
        let Some(map) = self.find_map_mut(snapshot.key.map_id, snapshot.key.instance_id) else {
            return false;
        };
        if map.instance_lock_context() != Some(snapshot.instance_lock_context)
            || map.player_count() != 0
            || !map.map().map_reference_order_like_cpp().is_empty()
        {
            return false;
        }
        map.set_can_unload(true);
        true
    }

    pub fn take_last_instance_expiry_updates_like_cpp(
        &mut self,
    ) -> Vec<MapTickInstanceExpiryUpdateLikeCpp> {
        std::mem::take(&mut self.last_instance_expiry_updates_like_cpp)
    }

    pub fn take_pending_instance_map_last_leave_checks_like_cpp(
        &mut self,
    ) -> Vec<InstanceMapLastLeaveSnapshotLikeCpp> {
        std::mem::take(&mut self.pending_instance_map_last_leave_checks_like_cpp)
    }

    pub(super) fn append_instance_expiry_update_like_cpp(
        &mut self,
        key: MapKey,
        incarnation: u64,
        update: ManagedMapInstanceExpiryUpdateLikeCpp,
    ) {
        let occupants = match &update.reset_result {
            InstanceResetResultLikeCpp::NotEmpty { occupant_guids } => occupant_guids
                .iter()
                .filter_map(|guid| {
                    let (handle, residence_revision) =
                        self.current_player_admission_like_cpp(*guid)?;
                    (self.player_active_residence_revision_like_cpp(handle)
                        == Some((key, residence_revision)))
                    .then_some(MapSessionPassParticipantLikeCpp {
                        guid: *guid,
                        handle,
                        residence_revision,
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        self.last_instance_expiry_updates_like_cpp
            .push(MapTickInstanceExpiryUpdateLikeCpp {
                key,
                incarnation,
                update,
                occupants,
            });
    }

    pub(super) fn clear_instance_expiry_updates_like_cpp(&mut self) {
        self.last_instance_expiry_updates_like_cpp.clear();
    }
}

#[cfg(test)]
#[path = "instance_expiry_tests.rs"]
mod tests;
