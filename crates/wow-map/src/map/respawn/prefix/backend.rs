//! Closed storage/removal differences, borrowed only for each existing phase.
use super::models::{CapturedCorpse, Clocks, RemovedCreatureCorpse};
use crate::map::{Map, NoopGridLifecycle, NoopTerrainGridLoader};
use crate::map_manager::{
    MapManager as LegacyMapManager, PendingRespawn, WorldCreature,
    pending_respawn_from_world_creature_like_cpp, respawn_time_from_instant_like_cpp,
};
use crate::spawn::{RespawnInfoLikeCpp, SpawnObjectType};
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_persistence::RespawnPersistenceMutationLikeCpp;

pub(super) enum Memory<'a> {
    Canonical(&'a mut Map<NoopTerrainGridLoader, NoopGridLifecycle>),
    Legacy {
        manager: &'a mut LegacyMapManager,
        key: (u16, u32),
    },
}

pub(super) fn capture_pending(
    actor: &WorldCreature,
    map_id: u16,
    clocks: Clocks,
) -> PendingRespawn {
    pending_respawn_from_world_creature_like_cpp(
        actor,
        actor.respawn_at_from_death_at_game_time_like_cpp(
            clocks.conversion_now,
            clocks.conversion_now_secs,
        ),
        map_id,
    )
}

fn capture_corpse(
    actor: &WorldCreature,
    map_id: u16,
    clocks: Clocks,
    legacy: bool,
) -> CapturedCorpse {
    let respawn_at = actor.respawn_at_from_death_at_game_time_like_cpp(
        clocks.conversion_now,
        clocks.conversion_now_secs,
    );
    let pending = pending_respawn_from_world_creature_like_cpp(actor, respawn_at, map_id);
    // Legacy computed the grid before the seconds conversion, even for a
    // synthetic spawn. Canonical computes it lazily only for persistent info.
    let legacy_grid =
        legacy.then(|| crate::compute_grid_coord(pending.home_pos.x, pending.home_pos.y));
    let seconds = respawn_time_from_instant_like_cpp(
        respawn_at,
        clocks.conversion_now,
        clocks.conversion_now_secs,
    );
    let info = (legacy || pending.persistent_spawn).then(|| RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: pending.spawn_id,
        entry: pending.create_data.entry,
        respawn_time: seconds,
        grid_id: legacy_grid
            .unwrap_or_else(|| crate::compute_grid_coord(pending.home_pos.x, pending.home_pos.y))
            .get_id(),
    });
    CapturedCorpse {
        pending,
        seconds,
        info,
        _detached_actor: None,
    }
}

impl Memory<'_> {
    pub(super) fn key(&self) -> Option<(u16, u32)> {
        match self {
            Self::Canonical(map) => Some((u16::try_from(map.map_id).ok()?, map.instance_id)),
            Self::Legacy { key, .. } => Some(*key),
        }
    }

    pub(super) fn actor_guids(&self) -> Vec<ObjectGuid> {
        match self {
            Self::Canonical(map) => map
                .entity_world
                .iter()
                .filter_map(|(guid, _)| map.creature_actor(*guid).map(|_| *guid))
                .collect(),
            Self::Legacy { manager, key } => manager.creature_guids(key.0, key.1),
        }
    }

    pub(super) fn actor(&self, guid: ObjectGuid) -> Option<&WorldCreature> {
        match self {
            Self::Canonical(map) => map.creature_actor(guid),
            Self::Legacy { manager, key } => manager.find_creature(key.0, key.1, guid),
        }
    }

    pub(super) fn clear_death_save_request(&mut self, guid: ObjectGuid) {
        match self {
            Self::Canonical(map) => {
                map.creature_actor_mut(guid)
                    .expect("same exclusive Actor stage")
                    .creature
                    .runtime_state_mut()
                    .save_respawn_requested = false;
            }
            Self::Legacy { manager, key } => {
                if let Some(actor) = manager.find_creature_mut(key.0, key.1, guid) {
                    actor.creature.runtime_state_mut().save_respawn_requested = false;
                }
            }
        }
    }

    pub(super) fn save_row(
        &mut self,
        pending: &PendingRespawn,
        key: (u16, u32),
        clocks: Clocks,
    ) -> Option<RespawnPersistenceMutationLikeCpp> {
        match self {
            Self::Canonical(map) => map.respawn_store.save_actor_row(
                pending,
                key.0,
                key.1,
                clocks.conversion_now,
                clocks.conversion_now_secs,
            ),
            Self::Legacy { manager, .. } => manager.save_pending_respawn_time_like_cpp(
                key.0,
                key.1,
                pending,
                clocks.conversion_now,
                clocks.conversion_now_secs,
            ),
        }
    }

    pub(super) fn cleanup_and_capture(
        &mut self,
        guid: ObjectGuid,
        map_id: u16,
        clocks: Clocks,
    ) -> Option<CapturedCorpse> {
        match self {
            Self::Canonical(map) => {
                let actor = map.creature_actor_mut(guid)?;
                actor.creature.clear_loot_like_cpp();
                // The SAME canonical Actor remains attached through row/queue/info.
                Some(capture_corpse(actor, map_id, clocks, false))
            }
            Self::Legacy { manager, key } => {
                if let Some(actor) = manager.find_creature_mut(key.0, key.1, guid) {
                    actor.creature.clear_loot_like_cpp();
                }
                let actor = manager.remove_creature_any(key.0, key.1, guid)?;
                // Capture the SAME moved value after physical legacy removal.
                let mut captured = capture_corpse(&actor, map_id, clocks, true);
                captured._detached_actor = Some(actor);
                Some(captured)
            }
        }
    }

    pub(super) fn saved_time(&self, spawn_id: u64) -> Option<i64> {
        match self {
            Self::Canonical(map) => map
                .respawn_store
                .saved_row(SpawnObjectType::Creature, spawn_id)
                .map(|row| row.respawn_time),
            Self::Legacy { manager, key } => manager.persisted_respawn_time_like_cpp(
                key.0,
                key.1,
                SpawnObjectType::Creature,
                spawn_id,
            ),
        }
    }

    pub(super) fn remove_saved_row(&mut self, spawn_id: u64) {
        match self {
            Self::Canonical(map) => {
                map.respawn_store
                    .remove_saved_row(SpawnObjectType::Creature, spawn_id);
            }
            Self::Legacy { manager, key } => {
                let _ = manager.remove_persisted_respawn_time_like_cpp(
                    key.0,
                    key.1,
                    SpawnObjectType::Creature,
                    spawn_id,
                );
            }
        }
    }

    pub(super) fn queue(&mut self, pending: PendingRespawn) {
        match self {
            Self::Canonical(map) => {
                let _ = map.respawn_store.queue_actor(pending);
            }
            Self::Legacy { manager, key } => manager.push_respawn(key.0, key.1, pending),
        }
    }

    pub(super) fn finish_corpse(
        &mut self,
        guid: ObjectGuid,
        info: Option<RespawnInfoLikeCpp>,
        persistent_spawn: bool,
        removed: &mut Vec<RemovedCreatureCorpse>,
    ) -> bool {
        match self {
            Self::Canonical(map) => {
                if let Some(info) = info {
                    map.add_respawn_info_like_cpp(info);
                }
                map.remove_from_map_like_cpp(guid, true).is_ok()
            }
            Self::Legacy { .. } => {
                removed.push(RemovedCreatureCorpse {
                    guid,
                    respawn_info: info.filter(|_| persistent_spawn),
                });
                true
            }
        }
    }

    pub(super) fn ready(&mut self, now: Instant) -> Vec<PendingRespawn> {
        match self {
            Self::Canonical(map) => map.respawn_store.reserve_ready_actors(now),
            Self::Legacy { manager, key } => manager.drain_ready_respawns(key.0, key.1, now),
        }
    }
}
