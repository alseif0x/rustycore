//! Prepared canonical Actor lifecycle. Production producers remain legacy.
//! C++ a5f8da2e Creature.cpp:419,2194,2651 and Map.cpp:2191,3516.
use super::*;
use crate::map_manager::PendingRespawn;
use crate::spawn::{ActorRespawnAttempt, ActorRespawnPhaseOutcome, ActorRespawnStatus, RespawnKey};
use std::time::Instant;
use std::collections::VecDeque;
use wow_persistence::{RespawnPersistenceKeyLikeCpp, RespawnPersistenceMutationLikeCpp};

impl Map<NoopTerrainGridLoader, NoopGridLifecycle>
{
    /// Memory-only prefix for the concrete canonical runtime. The manager owns
    /// admission identity, queries and completion; no factory/terrain callback
    /// runs while this Map is borrowed. Captured clocks retain their old heights.
    pub(crate) fn prepare_actor_respawns(
        &mut self, now: Instant, conversion_now: Instant, conversion_now_secs: i64,
        persistent_world_map: bool,
    ) -> (ActorRespawnPhaseOutcome, VecDeque<PendingRespawn>) {
        super::prefix::prepare_canonical_creature_respawns(
            self, now, conversion_now, conversion_now_secs, persistent_world_map,
        )
    }

    pub(crate) fn initial_actor_respawn_guard(&self, pending: &PendingRespawn)
        -> Option<ActorRespawnStatus>
    {
        let occupied = self.get_creature(pending.create_data.guid).is_some();
        let spawn_occupied = !occupied && pending.persistent_spawn
            && self.creature_spawn_id_store_guids_like_cpp(pending.spawn_id).iter()
                .any(|guid| self.get_typed_creature(*guid).is_some_and(|creature|
                    creature.spawn_id() == pending.spawn_id && creature.is_alive()));
        if occupied { Some(ActorRespawnStatus::GuidOccupied) }
        else if spawn_occupied { Some(ActorRespawnStatus::SpawnOccupied) }
        else { None }
    }

    pub(crate) fn settle_actor_respawn(
        &mut self, pending: &PendingRespawn, status: ActorRespawnStatus,
        outcome: &mut ActorRespawnPhaseOutcome,
    ) {
        let key = RespawnKey::for_actor(pending);
        self.respawn_store.release_respawn_key(key);
        if pending.persistent_spawn && status != ActorRespawnStatus::AdmissionRejected {
            self.delete_actor_saved_row(pending.map_id, pending.spawn_id, outcome);
        }
        outcome.attempts.push(ActorRespawnAttempt {
            key, guid: pending.create_data.guid, status,
        });
    }

    fn delete_actor_saved_row(&mut self, map_id: u16, spawn_id: u64, outcome: &mut ActorRespawnPhaseOutcome) {
        if self.respawn_store.remove_saved_row(SpawnObjectType::Creature, spawn_id).is_some() {
            outcome.respawn_db_mutations.push(RespawnPersistenceMutationLikeCpp::Delete {
                key: RespawnPersistenceKeyLikeCpp {
                    object_type_raw: u16::from(SpawnObjectType::Creature as u8),
                    spawn_id, map_id, instance_id: self.instance_id,
                },
            });
        }
    }
}

#[cfg(test)]
mod tests;
