//! Creature lifecycle phase: respawn and despawn.
//!
//! Moved out of the Session root under #619. #1263 F6-8D3a-3: the lifecycle
//! phase is written once, [`run_creature_lifecycle_phase_on_store_like_cpp`],
//! over the admitted canonical map the executor holds locked
//! ([`AdmittedCanonicalCreatureLifecycleStoreLikeCpp`]), with the canonical
//! map's own respawn queue (#1263 F6-8E: the only creature store).
//!
//! C++ anchors: `Creature::setDeathState(JUST_DIED)` → `SaveRespawnTime`
//! (`Creature.cpp:2193-2247, 2651-2667`); `Creature::Update` `CORPSE` →
//! `RemoveCorpse` (`Creature.cpp:696+, 419-470`); `Map::ProcessRespawns` →
//! `DoRespawn` (`Map.cpp:2191-2238`) and `Creature::Respawn`
//! (`Creature.cpp:2296-2350`).

use super::*;
use wow_world_core::session::{
    add_canonical_creature_respawn_info_and_remove_map_object_on_locked_map_like_cpp,
    remove_canonical_creature_map_object_on_locked_map_like_cpp,
    remove_canonical_respawn_time_on_locked_map_like_cpp,
};

/// The exact residence one lifecycle step addresses.
///
/// F6-8A residence contract: active residence is **canonical**, so a lifecycle
/// step addresses the exact `(map_id, instance_id)` key it observed in the
/// store. A detached resolution must never invent instance zero, so this type
/// has no `Default` and no general zero fallback: it is constructed only from
/// an observed key, and every canonical lookup on the lifecycle path is taken
/// through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct LifecycleResidenceLikeCpp {
    map_id: u16,
    instance_id: u32,
}

impl LifecycleResidenceLikeCpp {
    /// The residence observed for one map key.
    const fn observed_like_cpp(map_id: u16, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
        }
    }
}

/// One ready respawn that passed the duplicate check and is built but not yet
/// published.
///
/// R1b: the store publication is deferred until the canonical admission has
/// decided the incarnation, so a refused candidate can never be left holding
/// its own allocatable authority.
pub(in crate::session) struct ReadyRespawnCandidateLikeCpp {
    /// The exact residence this candidate belongs to. It is never re-derived and
    /// never defaulted, so a ready respawn cannot be published — or admitted —
    /// under a different key than the one its queue entry was drained from.
    residence: LifecycleResidenceLikeCpp,
    guid: ObjectGuid,
    /// The queue entry this candidate was drained from.
    ///
    /// R7b-1: when no canonical incarnation can admit the candidate yet, this is
    /// pushed back into the map's spawn queue unchanged, so the deferred spawn
    /// keeps its identity, its due time and its persisted-respawn bookkeeping
    /// and is retried by the next tick.
    pending: crate::map_manager::PendingRespawn,
}

/// The canonical admission decision for one ready respawn.
enum ReadyRespawnAdmissionLikeCpp {
    /// The canonical map admitted and installed the candidate.
    Admitted,
    /// A canonical map manager is configured but no canonical incarnation could
    /// admit the candidate (no canonical map instance for this key, or the
    /// candidate could not be installed), so the candidate has no admitted
    /// owner. R7b-1: its publication is deferred — it goes back to the map's
    /// spawn queue — instead of publishing a representation the mutation root
    /// must refuse for its whole life.
    Deferred,
    /// The canonical owner already exists and this candidate would be a second
    /// claimable pool, so the store must not publish it at all.
    Refused,
}

/// Whether a canonical creature incarnation already exists for this map key.
///
/// R1a's admission returns `None` both for a refusal and for "not admitted".
/// A candidate that is not admitted is never installed, so a canonical
/// incarnation that exists after that call is the refusal; this read therefore
/// classifies R1a's decision without re-deriving it. The lookup is taken at the
/// **exact** residence the lifecycle observed.
fn canonical_creature_incarnation_exists_like_cpp(
    manager: &wow_map::MapManager,
    residence: LifecycleResidenceLikeCpp,
    guid: ObjectGuid,
) -> bool {
    manager
        .find_map(u32::from(residence.map_id), residence.instance_id)
        .is_some_and(|map| map.map().with_creature_like_cpp(guid, |_| ()).is_some())
}

/// The creature lifecycle phase (#1263 F6-8D3a-3).
///
/// Stage order and DB statement order: JUST_DIED respawn-time saves and corpse despawns per map with the
/// ready-respawn drain; the canonical despawns and respawn-time removals; the
/// canonical admission of each ready respawn; its publication and persisted
/// row removal. `refresh_map_keys` names the maps whose sessions must
/// recompute creature visibility. `conversion_clock` is the `(Instant, Unix
/// seconds)` pair every respawn-time conversion of the tick uses.
pub(in crate::session) fn run_creature_lifecycle_phase_on_store_like_cpp(
    store: &mut AdmittedCanonicalCreatureLifecycleStoreLikeCpp<'_, '_>,
    map_keys: Vec<(u16, u32)>,
    map_store: &wow_data::MapStore,
    now: Instant,
    conversion_clock: (Instant, i64),
    outcome: &mut LegacyCreatureLifecycleTickOutcomeLikeCpp,
) {
    use crate::map_manager::{
        creature_corpse_despawn_due_like_cpp, creature_from_pending_respawn_like_cpp,
        creature_respawn_at_from_death_at_game_time_like_cpp,
        pending_respawn_from_creature_like_cpp, respawn_time_from_instant_like_cpp,
        save_pending_respawn_time_on_queue_like_cpp,
    };
    use std::collections::BTreeSet;

    let (conversion_now, conversion_now_secs) = conversion_clock;
    let mut affected_maps = BTreeSet::new();
    let mut canonical_respawn_despawns: Vec<(u32, u32, ObjectGuid, wow_map::RespawnInfoLikeCpp)> =
        Vec::new();
    let mut canonical_plain_despawns: Vec<(u32, u32, ObjectGuid)> = Vec::new();
    let mut canonical_respawn_removes: Vec<(u32, u32, wow_map::SpawnObjectType, wow_map::SpawnId)> =
        Vec::new();
    let mut respawn_candidates: Vec<(ReadyRespawnCandidateLikeCpp, wow_entities::Creature)> =
        Vec::new();
    let persistent_world_map_keys: BTreeSet<(u16, u32)> = map_keys
        .iter()
        .copied()
        .filter(|(map_id, _)| {
            map_store
                .get(u32::from(*map_id))
                .is_some_and(|entry| !entry.is_instanceable_like_cpp())
        })
        .collect();

    outcome.maps_seen = map_keys.len();
    for (map_id, instance_id) in map_keys {
        let guids = store.lifecycle_creature_guids_like_cpp(map_id, instance_id);
        outcome.creatures_seen += guids.len();

        // C++ saves the respawn time from JUST_DIED, not when the corpse is
        // eventually removed. Only persistent world-map spawns belong in the
        // global characters.respawn table.
        if persistent_world_map_keys.contains(&(map_id, instance_id)) {
            for guid in &guids {
                let pending = store
                    .lifecycle_creature_like_cpp(map_id, instance_id, *guid)
                    .and_then(|creature| {
                        (!creature.is_alive()
                            && creature.spawn_id() != 0
                            && creature.runtime_state().save_respawn_requested)
                            .then(|| {
                                pending_respawn_from_creature_like_cpp(
                                    creature,
                                    creature_respawn_at_from_death_at_game_time_like_cpp(
                                        creature,
                                        conversion_now,
                                        conversion_now_secs,
                                    ),
                                    map_id,
                                )
                            })
                    });
                if let Some(pending) = pending {
                    if let Some(stmt) = store
                        .lifecycle_respawns_mut_like_cpp(map_id, instance_id)
                        .and_then(|queue| {
                            save_pending_respawn_time_on_queue_like_cpp(
                                queue,
                                map_id,
                                instance_id,
                                &pending,
                                conversion_now,
                                conversion_now_secs,
                            )
                        })
                    {
                        outcome.respawn_db_mutations.push(stmt);
                    }
                    if let Some(creature) =
                        store.lifecycle_creature_mut_like_cpp(map_id, instance_id, *guid)
                    {
                        creature.runtime_state_mut().save_respawn_requested = false;
                    }
                }
            }
        }

        let despawn_guids: Vec<ObjectGuid> = guids
            .iter()
            .filter(|guid| {
                store
                    .lifecycle_creature_like_cpp(map_id, instance_id, **guid)
                    .is_some_and(|creature| {
                        !creature.is_alive()
                            && creature.unit().death_state() == wow_constants::DeathState::Corpse
                            && creature_corpse_despawn_due_like_cpp(creature)
                    })
            })
            .copied()
            .collect();

        for guid in despawn_guids {
            if let Some(creature) = store.lifecycle_creature_mut_like_cpp(map_id, instance_id, guid)
            {
                creature.clear_loot_like_cpp();
            }
            let Some(creature) = store.lifecycle_take_creature_like_cpp(map_id, instance_id, guid)
            else {
                continue;
            };
            let respawn_at = creature_respawn_at_from_death_at_game_time_like_cpp(
                &creature,
                conversion_now,
                conversion_now_secs,
            );
            let pending = pending_respawn_from_creature_like_cpp(&creature, respawn_at, map_id);
            let grid = wow_map::compute_grid_coord(pending.home_pos.x, pending.home_pos.y);
            let pending_respawn_secs =
                respawn_time_from_instant_like_cpp(respawn_at, conversion_now, conversion_now_secs);
            let canonical_respawn_info = wow_map::RespawnInfoLikeCpp {
                object_type: wow_map::SpawnObjectType::Creature,
                spawn_id: pending.spawn_id,
                entry: pending.create_data.entry,
                respawn_time: pending_respawn_secs,
                grid_id: grid.get_id(),
            };
            if let Some(queue) = store.lifecycle_respawns_mut_like_cpp(map_id, instance_id) {
                if persistent_world_map_keys.contains(&(map_id, instance_id))
                    && creature.spawn_id() != 0
                    && queue
                        .persisted_respawn_time_like_cpp(
                            wow_map::SpawnObjectType::Creature,
                            pending.spawn_id,
                        )
                        .is_none_or(|stored| stored < pending_respawn_secs)
                {
                    // REP_RESPAWN is an upsert, so replace the in-memory row
                    // directly; no intermediate DEL is needed for the DB row.
                    let _ = queue.remove_persisted_respawn_time_like_cpp(
                        wow_map::SpawnObjectType::Creature,
                        pending.spawn_id,
                    );
                    if let Some(stmt) = save_pending_respawn_time_on_queue_like_cpp(
                        queue,
                        map_id,
                        instance_id,
                        &pending,
                        conversion_now,
                        conversion_now_secs,
                    ) {
                        outcome.respawn_db_mutations.push(stmt);
                    }
                }
                queue.push_respawn(pending);
            }
            if creature.spawn_id() != 0 {
                canonical_respawn_despawns.push((
                    u32::from(map_id),
                    instance_id,
                    guid,
                    canonical_respawn_info,
                ));
            } else {
                canonical_plain_despawns.push((u32::from(map_id), instance_id, guid));
            }
            affected_maps.insert((map_id, instance_id));
            outcome.corpses_despawned += 1;
        }

        let ready_respawns = store
            .lifecycle_respawns_mut_like_cpp(map_id, instance_id)
            .map(|queue| queue.drain_ready_respawns(now))
            .unwrap_or_default();
        for respawn in ready_respawns {
            let guid = respawn.create_data.guid;
            if store.lifecycle_holds_respawn_like_cpp(
                map_id,
                instance_id,
                guid,
                respawn.persistent_spawn.then_some(respawn.spawn_id),
            ) {
                if respawn.persistent_spawn {
                    if let Some(stmt) = remove_persisted_respawn_time_on_store_like_cpp(
                        store,
                        map_id,
                        instance_id,
                        respawn.spawn_id,
                    ) {
                        outcome.respawn_db_mutations.push(stmt);
                    }
                    canonical_respawn_removes.push((
                        u32::from(map_id),
                        instance_id,
                        wow_map::SpawnObjectType::Creature,
                        respawn.spawn_id,
                    ));
                }
                affected_maps.insert((map_id, instance_id));
                continue;
            }
            let mut creature = creature_from_pending_respawn_like_cpp(&respawn, instance_id);
            // C++ Creature::Respawn ground-snaps via UpdateAllowedPositionZ
            // (Creature.cpp:461). No-op when terrain is not wired.
            store.lifecycle_snap_to_ground_like_cpp(&mut creature, map_id);
            // R1b: build the candidate, but let the canonical admission decide
            // the incarnation.
            respawn_candidates.push((
                ReadyRespawnCandidateLikeCpp {
                    residence: LifecycleResidenceLikeCpp::observed_like_cpp(map_id, instance_id),
                    guid,
                    pending: respawn,
                },
                creature,
            ));
        }
    }
    {
        for (map_id, instance_id, guid) in canonical_plain_despawns {
            store.lifecycle_with_canonical_like_cpp(|manager| {
                remove_canonical_creature_map_object_on_locked_map_like_cpp(
                    manager,
                    map_id,
                    instance_id,
                    guid,
                );
            });
            outcome.canonical_removes += 1;
        }
        for (map_id, instance_id, guid, info) in canonical_respawn_despawns {
            let (respawn_added, object_removed) =
                store.lifecycle_with_canonical_like_cpp(|manager| {
                    add_canonical_creature_respawn_info_and_remove_map_object_on_locked_map_like_cpp(
                        manager,
                        map_id,
                        instance_id,
                        guid,
                        info,
                    )
                });
            if respawn_added {
                outcome.canonical_respawn_adds += 1;
            }
            if object_removed {
                outcome.canonical_removes += 1;
            }
        }
        for (map_id, instance_id, object_type, spawn_id) in canonical_respawn_removes {
            if store.lifecycle_with_canonical_like_cpp(|manager| {
                remove_canonical_respawn_time_on_locked_map_like_cpp(
                    manager,
                    map_id,
                    instance_id,
                    object_type,
                    spawn_id,
                )
            }) {
                outcome.canonical_respawn_removes += 1;
            }
        }
    }

    // F6-8A publication gate. Admission decides each ready respawn's incarnation
    // first, and the store publication below is reachable **only** through the
    // decision this loop records: the authority, the canonical health timeline
    // and the aura provenance of that one decision are consumed together. A
    // refused candidate and a candidate the canonical owner could not admit yet
    // are never published, so neither can leave a competing claimable
    // allocation or an un-admitted representation behind. Each candidate is
    // admitted at its own observed residence — the exact `(map_id, instance_id)`
    // key it was drained from — never at a default.
    let mut respawn_admissions: Vec<(ReadyRespawnCandidateLikeCpp, ReadyRespawnAdmissionLikeCpp)> =
        Vec::with_capacity(respawn_candidates.len());
    for (candidate, canonical_creature) in respawn_candidates {
        let admission = {
            let residence = candidate.residence;
            let guid = candidate.guid;
            let admitted = store.lifecycle_with_canonical_like_cpp(|manager| {
                wow_world_entities::insert_canonical_creature_map_object_on_locked_map_like_cpp(
                    manager,
                    u32::from(residence.map_id),
                    residence.instance_id,
                    canonical_creature,
                )
            });
            match admitted {
                Some(_) => ReadyRespawnAdmissionLikeCpp::Admitted,
                None if store.lifecycle_with_canonical_like_cpp(|manager| {
                    canonical_creature_incarnation_exists_like_cpp(manager, residence, guid)
                }) =>
                {
                    ReadyRespawnAdmissionLikeCpp::Refused
                }
                // R7b-1: a configured canonical manager that admitted nothing
                // leaves this candidate without an admitted owner (no canonical
                // map instance for this key, or an installation failure), so its
                // publication is deferred.
                None => ReadyRespawnAdmissionLikeCpp::Deferred,
            }
        };
        respawn_admissions.push((candidate, admission));
    }

    for (candidate, admission) in respawn_admissions {
        let residence = candidate.residence;
        let persistent_spawn = candidate.pending.persistent_spawn;
        let spawn_id = candidate.pending.spawn_id;
        match admission {
            ReadyRespawnAdmissionLikeCpp::Admitted => {}
            ReadyRespawnAdmissionLikeCpp::Refused => {
                // The canonical incarnation already owns this GUID, exactly
                // like the "already present" branch above: drop the stale
                // persisted respawn row and publish nothing in the store.
                outcome.respawn_publications_refused_like_cpp += 1;
                if persistent_spawn {
                    remove_respawn_row_after_publication_like_cpp(
                        store, residence, spawn_id, outcome,
                    );
                }
                affected_maps.insert((residence.map_id, residence.instance_id));
                continue;
            }
            ReadyRespawnAdmissionLikeCpp::Deferred => {
                // R7b-1: publish nothing. The spawn goes back to the map's own
                // queue unchanged, so its identity, due time and persisted
                // respawn row are preserved and the next tick retries the same
                // admission. The creature was never published, so no
                // visibility refresh is requested here; the corpse despawn that
                // queued this respawn already signalled the map key in this
                // tick.
                outcome.respawn_publications_deferred_like_cpp += 1;
                if let Some(queue) =
                    store.lifecycle_respawns_mut_like_cpp(residence.map_id, residence.instance_id)
                {
                    queue.push_respawn(candidate.pending);
                }
                continue;
            }
        }
        // The admission installed the canonical incarnation itself; there is
        // no second representation to publish.
        store
            .respawned
            .push(((residence.map_id, residence.instance_id), candidate.guid));
        if persistent_spawn {
            // Applied here rather than in the deferred canonical phase because
            // admission already ran; the mutation, its condition (a successful
            // publication) and the counter are unchanged.
            remove_respawn_row_after_publication_like_cpp(store, residence, spawn_id, outcome);
        }
        affected_maps.insert((residence.map_id, residence.instance_id));
        outcome.respawns_processed += 1;
        outcome.canonical_inserts += 1;
    }

    outcome.refresh_map_keys = affected_maps.into_iter().collect();
}

/// Remove one persisted creature respawn row from the store's queue and return
/// its DB delete statement.
fn remove_persisted_respawn_time_on_store_like_cpp(
    store: &mut AdmittedCanonicalCreatureLifecycleStoreLikeCpp<'_, '_>,
    map_id: u16,
    instance_id: u32,
    spawn_id: u64,
) -> Option<wow_persistence::RespawnPersistenceMutationLikeCpp> {
    store
        .lifecycle_respawns_mut_like_cpp(map_id, instance_id)?
        .remove_persisted_respawn_time_like_cpp(wow_map::SpawnObjectType::Creature, spawn_id)?;
    Some(crate::map_manager::respawn_delete_mutation_like_cpp(
        wow_map::SpawnObjectType::Creature,
        spawn_id,
        map_id,
        instance_id,
    ))
}

/// The persisted-row delete and canonical respawn-time removal that follow a
/// refused or published respawn.
fn remove_respawn_row_after_publication_like_cpp(
    store: &mut AdmittedCanonicalCreatureLifecycleStoreLikeCpp<'_, '_>,
    residence: LifecycleResidenceLikeCpp,
    spawn_id: u64,
    outcome: &mut LegacyCreatureLifecycleTickOutcomeLikeCpp,
) {
    if let Some(stmt) = remove_persisted_respawn_time_on_store_like_cpp(
        store,
        residence.map_id,
        residence.instance_id,
        spawn_id,
    ) {
        outcome.respawn_db_mutations.push(stmt);
    }
    if store.lifecycle_with_canonical_like_cpp(|manager| {
        remove_canonical_respawn_time_on_locked_map_like_cpp(
            manager,
            u32::from(residence.map_id),
            residence.instance_id,
            wow_map::SpawnObjectType::Creature,
            spawn_id,
        )
    }) {
        outcome.canonical_respawn_removes += 1;
    }
}

/// The admitted canonical store for the lifecycle phase: the admitted
/// selection on the canonical map the executor holds locked, the canonical
/// map's own respawn queue, and the executor's map environment.
pub(in crate::session) struct AdmittedCanonicalCreatureLifecycleStoreLikeCpp<'s, 'a> {
    pub(in crate::session) store: AdmittedCanonicalCreatureStoreLikeCpp<'a>,
    pub(in crate::session) terrain: Option<&'s crate::map_manager::LiveTerrainHeights>,
    /// Corpses taken out this tick; their canonical objects are removed by the
    /// canonical stage, after which nothing on the map holds them.
    pub(in crate::session) taken: Vec<((u16, u32), ObjectGuid)>,
    /// Respawns installed on the canonical map this tick, in publication order.
    pub(in crate::session) respawned: Vec<((u16, u32), ObjectGuid)>,
}

impl AdmittedCanonicalCreatureLifecycleStoreLikeCpp<'_, '_> {
    fn taken_like_cpp(&self, map_id: u16, instance_id: u32, guid: ObjectGuid) -> bool {
        self.taken.contains(&((map_id, instance_id), guid))
    }

    fn lifecycle_creature_guids_like_cpp(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid> {
        self.store
            .phase_creature_guids_like_cpp(map_id, instance_id)
    }

    fn lifecycle_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature> {
        if self.taken_like_cpp(map_id, instance_id, guid) {
            return None;
        }
        self.store
            .phase_creature_like_cpp(map_id, instance_id, guid)
    }

    fn lifecycle_creature_mut_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut wow_entities::Creature> {
        if self.taken_like_cpp(map_id, instance_id, guid) {
            return None;
        }
        self.store
            .phase_creature_mut_like_cpp(map_id, instance_id, guid)
    }

    fn lifecycle_take_creature_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<wow_entities::Creature> {
        let creature = self
            .lifecycle_creature_like_cpp(map_id, instance_id, guid)?
            .clone();
        self.taken.push(((map_id, instance_id), guid));
        Some(creature)
    }

    fn lifecycle_holds_respawn_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
        persistent_spawn_id: Option<u64>,
    ) -> bool {
        let Some(map) = self
            .store
            .manager_like_cpp()
            .find_map(u32::from(map_id), instance_id)
        else {
            return false;
        };
        let map = map.map();
        let held = |guid: ObjectGuid| !self.taken_like_cpp(map_id, instance_id, guid);
        (map.get_typed_creature(guid).is_some() && held(guid))
            || persistent_spawn_id.is_some_and(|spawn_id| {
                spawn_id != 0
                    && map
                        .typed_combat_unit_guids_like_cpp()
                        .into_iter()
                        .any(|other| {
                            held(other)
                                && map.get_typed_creature(other).is_some_and(|creature| {
                                    creature.is_alive() && creature.spawn_id() == spawn_id
                                })
                        })
            })
    }

    fn lifecycle_respawns_mut_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
    ) -> Option<&mut crate::map_manager::CreatureRespawnQueueLikeCpp> {
        Some(
            self.store
                .manager_mut_like_cpp()
                .find_map_mut(u32::from(map_id), instance_id)?
                .map_mut()
                .creature_respawn_queue_like_cpp_mut(),
        )
    }

    fn lifecycle_snap_to_ground_like_cpp(
        &self,
        creature: &mut wow_entities::Creature,
        map_id: u16,
    ) {
        if let Some(terrain) = self.terrain {
            crate::map_manager::snap_respawn_creature_to_ground_like_cpp(creature, map_id, terrain);
        }
    }

    fn lifecycle_with_canonical_like_cpp<R>(
        &mut self,
        operation: impl FnOnce(&mut wow_map::MapManager) -> R,
    ) -> R {
        operation(self.store.manager_mut_like_cpp())
    }
}
