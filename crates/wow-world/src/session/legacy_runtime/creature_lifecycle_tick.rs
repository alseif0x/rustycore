//! Legacy creature lifecycle tick, respawn and despawn.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// The exact residence one lifecycle step addresses.
///
/// F6-8A residence contract: active residence is **canonical**, so a lifecycle
/// step addresses the exact `(map_id, instance_id)` key it observed in the
/// legacy store. A detached resolution must never invent instance zero, so this
/// type has no `Default` and no general zero fallback: it is constructed only
/// from an observed key, and every canonical lookup on the lifecycle path is
/// taken through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LifecycleResidenceLikeCpp {
    map_id: u16,
    instance_id: u32,
}

impl LifecycleResidenceLikeCpp {
    /// The residence observed for one legacy map key.
    const fn observed_like_cpp(map_id: u16, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
        }
    }
}

/// One ready respawn that passed the duplicate check and is built but not yet
/// published in the legacy store.
///
/// R1b: the legacy store publication is deferred until the canonical admission
/// has decided the incarnation, so a refused candidate can never be left
/// holding its own allocatable authority.
struct ReadyRespawnCandidateLikeCpp {
    /// The exact residence this candidate belongs to. It is never re-derived and
    /// never defaulted, so a ready respawn cannot be published — or admitted —
    /// under a different key than the one its queue entry was drained from.
    residence: LifecycleResidenceLikeCpp,
    guid: ObjectGuid,
    world_creature: crate::map_manager::WorldCreature,
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
    /// The canonical map admitted the candidate; the legacy store may publish it
    /// with the authority, the canonical health timeline and the aura
    /// provenance that admission returned.
    Admitted(wow_world_entities::CanonicalCreatureInsertOutcomeLikeCpp),
    /// No canonical map manager is configured, so the legacy store is the only
    /// store: the previous legacy-only publication path is kept.
    LegacyOnly,
    /// A canonical map manager is configured but no canonical incarnation could
    /// admit the candidate (no canonical map instance for this key, or the
    /// candidate could not be installed), so the candidate has no admitted
    /// owner. R7b-1: its publication is deferred — it goes back to the map's
    /// spawn queue — instead of publishing a representation the mutation root
    /// must refuse for its whole life.
    Deferred,
    /// The canonical owner already exists and this candidate would be a second
    /// claimable pool, so the legacy store must not publish it at all.
    Refused,
}

/// Whether a canonical creature incarnation already exists for this map key.
///
/// R1a's admission returns `None` both for a refusal and for "not admitted".
/// A candidate that is not admitted is never installed, so a canonical
/// incarnation that exists after that call is the refusal; this read therefore
/// classifies R1a's decision without re-deriving it.
///
/// The lookup is taken at the **exact** residence the lifecycle observed
/// (`find_map` is an exact `(map_id, instance_id)` key lookup); a detached
/// resolution must not fall back to instance zero.
fn canonical_creature_incarnation_exists_like_cpp(
    manager: &SharedCanonicalMapManager,
    residence: LifecycleResidenceLikeCpp,
    guid: ObjectGuid,
) -> bool {
    let Ok(manager) = manager.lock() else {
        return false;
    };
    manager
        .find_map(u32::from(residence.map_id), residence.instance_id)
        .is_some_and(|map| map.map().with_creature_like_cpp(guid, |_| ()).is_some())
}

/// Runs one global legacy creature lifecycle tick without spawning a loop.
///
/// This is Slice 4A.3c.3 dormant infrastructure. It covers only the parts of
/// the legacy session creature tick that change creature existence:
///
/// - corpse removal after `corpse_despawn_at`;
/// - pushing the map-owned respawn queue;
/// - draining ready respawns and re-adding map/canonical creature state.
///
/// Packet delivery remains outside this function. Callers must fan out
/// `RefreshVisibleWorldCreaturesLikeCpp` for each `refresh_map_keys` entry so
/// every session runs its own C++-style `Player::UpdateVisibilityOf` seam.
pub fn run_legacy_creature_lifecycle_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    map_store: &wow_data::MapStore,
    now: Instant,
) -> LegacyCreatureLifecycleTickOutcomeLikeCpp {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp,
        respawn_time_from_instant_like_cpp, world_creature_from_pending_respawn_like_cpp,
        world_to_grid_coords,
    };
    use std::collections::BTreeSet;

    let mut outcome = LegacyCreatureLifecycleTickOutcomeLikeCpp::default();
    let mut affected_maps = BTreeSet::new();
    let mut canonical_respawn_despawns: Vec<(u32, u32, ObjectGuid, wow_map::RespawnInfoLikeCpp)> =
        Vec::new();
    let mut canonical_plain_despawns: Vec<(u32, u32, ObjectGuid)> = Vec::new();
    let mut canonical_respawn_removes: Vec<(u32, u32, wow_map::SpawnObjectType, wow_map::SpawnId)> =
        Vec::new();
    let mut respawn_candidates: Vec<(ReadyRespawnCandidateLikeCpp, wow_entities::Creature)> =
        Vec::new();
    // `now` is the scheduler's tick deadline and may predate a blocking-worker
    // stall. Keep it for due checks, but pair conversions to Unix game time
    // with a fresh monotonic snapshot from the same execution point.
    let conversion_now = Instant::now();
    let conversion_now_secs = unix_now();
    let legacy_map_keys = legacy_map_manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .active_map_keys();
    let persistent_world_map_keys: BTreeSet<(u16, u32)> = legacy_map_keys
        .iter()
        .copied()
        .filter(|(map_id, _)| {
            map_store
                .get(u32::from(*map_id))
                .is_some_and(|entry| !entry.is_instanceable_like_cpp())
        })
        .collect();

    {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }

        let map_keys = legacy_map_keys;
        outcome.maps_seen = map_keys.len();
        for (map_id, instance_id) in map_keys {
            let guids = manager.creature_guids(map_id, instance_id);
            outcome.creatures_seen += guids.len();

            // C++ saves the respawn time from JUST_DIED, not when the corpse
            // is eventually removed. Only persistent world-map spawns belong
            // in the global characters.respawn table.
            if persistent_world_map_keys.contains(&(map_id, instance_id)) {
                for guid in &guids {
                    let pending =
                        manager
                            .find_creature(map_id, instance_id, *guid)
                            .and_then(|creature| {
                                (!creature.is_alive()
                                    && creature.creature.spawn_id() != 0
                                    && creature.creature.runtime_state().save_respawn_requested)
                                    .then(|| {
                                        pending_respawn_from_world_creature_like_cpp(
                                            creature,
                                            creature.respawn_at_from_death_at_game_time_like_cpp(
                                                conversion_now,
                                                conversion_now_secs,
                                            ),
                                            map_id,
                                        )
                                    })
                            });
                    if let Some(pending) = pending {
                        if let Some(stmt) = manager.save_pending_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            &pending,
                            conversion_now,
                            conversion_now_secs,
                        ) {
                            outcome.respawn_db_mutations.push(stmt);
                        }
                        if let Some(creature) =
                            manager.find_creature_mut(map_id, instance_id, *guid)
                        {
                            creature.creature.runtime_state_mut().save_respawn_requested = false;
                        }
                    }
                }
            }

            let despawn_guids: Vec<ObjectGuid> = guids
                .iter()
                .filter(|guid| {
                    manager
                        .find_creature(map_id, instance_id, **guid)
                        .is_some_and(|creature| {
                            !creature.is_alive()
                                && creature.creature.unit().death_state()
                                    == wow_constants::DeathState::Corpse
                                && creature.corpse_despawn_due_like_cpp()
                        })
                })
                .copied()
                .collect();

            for guid in despawn_guids {
                if let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) {
                    creature.creature.clear_loot_like_cpp();
                }
                let Some(creature) = manager.remove_creature_any(map_id, instance_id, guid) else {
                    continue;
                };
                let respawn_at = creature.respawn_at_from_death_at_game_time_like_cpp(
                    conversion_now,
                    conversion_now_secs,
                );
                let pending =
                    pending_respawn_from_world_creature_like_cpp(&creature, respawn_at, map_id);
                let grid = wow_map::compute_grid_coord(pending.home_pos.x, pending.home_pos.y);
                let pending_respawn_secs = respawn_time_from_instant_like_cpp(
                    respawn_at,
                    conversion_now,
                    conversion_now_secs,
                );
                let canonical_respawn_info = wow_map::RespawnInfoLikeCpp {
                    object_type: wow_map::SpawnObjectType::Creature,
                    spawn_id: pending.spawn_id,
                    entry: pending.create_data.entry,
                    respawn_time: pending_respawn_secs,
                    grid_id: grid.get_id(),
                };
                if persistent_world_map_keys.contains(&(map_id, instance_id))
                    && creature.creature.spawn_id() != 0
                    && manager
                        .persisted_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
                            pending.spawn_id,
                        )
                        .is_none_or(|stored| stored < pending_respawn_secs)
                {
                    // REP_RESPAWN is an upsert, so replace the in-memory row
                    // directly; no intermediate DEL is needed for the DB row.
                    let _ = manager.remove_persisted_respawn_time_like_cpp(
                        map_id,
                        instance_id,
                        wow_map::SpawnObjectType::Creature,
                        pending.spawn_id,
                    );
                    if let Some(stmt) = manager.save_pending_respawn_time_like_cpp(
                        map_id,
                        instance_id,
                        &pending,
                        conversion_now,
                        conversion_now_secs,
                    ) {
                        outcome.respawn_db_mutations.push(stmt);
                    }
                }
                manager.push_respawn(map_id, instance_id, pending);
                if creature.creature.spawn_id() != 0 {
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

            let ready_respawns = manager.drain_ready_respawns(map_id, instance_id, now);
            for respawn in ready_respawns {
                let guid = respawn.create_data.guid;
                if manager.find_creature(map_id, instance_id, guid).is_some()
                    || (respawn.persistent_spawn
                        && manager
                            .find_creature_guid_by_spawn_id_like_cpp(
                                map_id,
                                instance_id,
                                respawn.spawn_id,
                            )
                            .is_some())
                {
                    if respawn.persistent_spawn {
                        if let Some(stmt) = manager.remove_persisted_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
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
                let mut world_creature =
                    world_creature_from_pending_respawn_like_cpp(&respawn, instance_id);
                // C++ Creature::Respawn ground-snaps via UpdateAllowedPositionZ
                // (Creature.cpp:461). No-op when terrain is not wired.
                if let Some(terrain) = manager.terrain() {
                    crate::map_manager::snap_respawn_creature_to_ground_like_cpp(
                        &mut world_creature,
                        map_id,
                        &terrain,
                    );
                }
                let canonical_creature = world_creature.creature.clone();
                // R1b: build the candidate, but let the canonical admission
                // decide the incarnation before the legacy store publishes it.
                // The duplicate check above is unchanged; only the store
                // insertion moved after admission.
                respawn_candidates.push((
                    ReadyRespawnCandidateLikeCpp {
                        residence: LifecycleResidenceLikeCpp::observed_like_cpp(
                            map_id,
                            instance_id,
                        ),
                        guid,
                        world_creature,
                        pending: respawn,
                    },
                    canonical_creature,
                ));
            }
        }
    }

    if let Some(canonical_map_manager) = canonical_map_manager {
        for (map_id, instance_id, guid) in canonical_plain_despawns {
            remove_canonical_creature_map_object_on_map_like_cpp(
                canonical_map_manager,
                map_id,
                instance_id,
                guid,
            );
            outcome.canonical_removes += 1;
        }
        for (map_id, instance_id, guid, info) in canonical_respawn_despawns {
            let (respawn_added, object_removed) =
                add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
                    canonical_map_manager,
                    map_id,
                    instance_id,
                    guid,
                    info,
                );
            if respawn_added {
                outcome.canonical_respawn_adds += 1;
            }
            if object_removed {
                outcome.canonical_removes += 1;
            }
        }
        for (map_id, instance_id, object_type, spawn_id) in canonical_respawn_removes {
            if remove_canonical_respawn_time_on_map_like_cpp(
                canonical_map_manager,
                map_id,
                instance_id,
                object_type,
                spawn_id,
            ) {
                outcome.canonical_respawn_removes += 1;
            }
        }
    }

    // F6-8A publication gate. Admission decides each ready respawn's incarnation
    // first, and the legacy store publication below is reachable **only** through
    // the decision this loop records: the authority, the canonical health
    // timeline and the aura provenance of that one decision are consumed
    // together. A refused candidate and a candidate the canonical owner could
    // not admit yet are never published, so neither can leave a competing
    // claimable allocation or an un-admitted representation behind. Each
    // candidate is admitted at its own observed residence — the exact
    // `(map_id, instance_id)` key it was drained from — never at a default.
    let mut respawn_admissions: Vec<(ReadyRespawnCandidateLikeCpp, ReadyRespawnAdmissionLikeCpp)> =
        Vec::with_capacity(respawn_candidates.len());
    for (candidate, canonical_creature) in respawn_candidates {
        let admission = match canonical_map_manager {
            Some(canonical_map_manager) => {
                let admitted = insert_canonical_creature_map_object_on_map_like_cpp(
                    canonical_map_manager,
                    u32::from(candidate.residence.map_id),
                    candidate.residence.instance_id,
                    canonical_creature,
                );
                match admitted {
                    Some(admitted) => ReadyRespawnAdmissionLikeCpp::Admitted(admitted),
                    None if canonical_creature_incarnation_exists_like_cpp(
                        canonical_map_manager,
                        candidate.residence,
                        candidate.guid,
                    ) =>
                    {
                        ReadyRespawnAdmissionLikeCpp::Refused
                    }
                    // R7b-1: a configured canonical manager that admitted
                    // nothing leaves this candidate without an admitted owner
                    // (no canonical map instance for this key, or an
                    // installation failure), so its publication is deferred.
                    None => ReadyRespawnAdmissionLikeCpp::Deferred,
                }
            }
            None => ReadyRespawnAdmissionLikeCpp::LegacyOnly,
        };
        respawn_admissions.push((candidate, admission));
    }

    {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for (candidate, admission) in respawn_admissions {
            let admitted = match admission {
                ReadyRespawnAdmissionLikeCpp::Admitted(admitted) => Some(admitted),
                ReadyRespawnAdmissionLikeCpp::Refused => {
                    // The canonical incarnation already owns this GUID, exactly
                    // like the "already present" branch above: drop the stale
                    // persisted respawn row and publish nothing in the legacy
                    // store.
                    outcome.respawn_publications_refused_like_cpp += 1;
                    if candidate.pending.persistent_spawn {
                        if let Some(stmt) = manager.remove_persisted_respawn_time_like_cpp(
                            candidate.residence.map_id,
                            candidate.residence.instance_id,
                            wow_map::SpawnObjectType::Creature,
                            candidate.pending.spawn_id,
                        ) {
                            outcome.respawn_db_mutations.push(stmt);
                        }
                        if let Some(canonical_map_manager) = canonical_map_manager
                            && remove_canonical_respawn_time_on_map_like_cpp(
                                canonical_map_manager,
                                u32::from(candidate.residence.map_id),
                                candidate.residence.instance_id,
                                wow_map::SpawnObjectType::Creature,
                                candidate.pending.spawn_id,
                            )
                        {
                            outcome.canonical_respawn_removes += 1;
                        }
                    }
                    affected_maps
                        .insert((candidate.residence.map_id, candidate.residence.instance_id));
                    continue;
                }
                ReadyRespawnAdmissionLikeCpp::Deferred => {
                    // R7b-1: publish nothing. The spawn goes back to the map's
                    // own queue unchanged, so its identity, due time and
                    // persisted-respawn row are preserved and the next tick
                    // retries the same admission. The creature was never
                    // published, so no visibility refresh is requested here; the
                    // corpse despawn that queued this respawn already signalled
                    // the map key in this tick.
                    outcome.respawn_publications_deferred_like_cpp += 1;
                    manager.push_respawn(
                        candidate.residence.map_id,
                        candidate.residence.instance_id,
                        candidate.pending,
                    );
                    continue;
                }
                ReadyRespawnAdmissionLikeCpp::LegacyOnly => None,
            };
            let expected_legacy_authority = candidate
                .world_creature
                .creature
                .loot_authority_like_cpp()
                .clone();
            let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
            // A fresh admission installs exactly this candidate, so its own
            // allocation is the incarnation's authority; a pristine duplicate
            // instead returns the pre-existing canonical allocation.
            let fresh_canonical_insert = admitted.as_ref().is_some_and(|admitted| {
                admitted
                    .loot_authority
                    .shares_storage_like_cpp(&expected_legacy_authority)
            });
            let (grid_x, grid_y) =
                world_to_grid_coords(candidate.pending.home_pos.x, candidate.pending.home_pos.y);
            if !manager.add_creature(
                candidate.residence.map_id,
                candidate.residence.instance_id,
                grid_x,
                grid_y,
                candidate.world_creature,
            ) {
                // A concurrent publication won the GUID, so this candidate's
                // publication failed after its admission decision was taken and
                // nothing of it is published. The counter records the failed
                // legacy publication itself, before the fresh-insertion
                // condition below is examined, so it also counts failures that
                // installed no canonical object and have nothing to roll back.
                outcome.respawn_publications_failed_like_cpp += 1;
                if fresh_canonical_insert && let Some(canonical_map_manager) = canonical_map_manager
                {
                    remove_canonical_creature_map_object_on_map_like_cpp(
                        canonical_map_manager,
                        u32::from(candidate.residence.map_id),
                        candidate.residence.instance_id,
                        candidate.guid,
                    );
                }
                continue;
            }
            if let Some(admitted) = &admitted
                && let Some(world_creature) = manager.find_creature_mut(
                    candidate.residence.map_id,
                    candidate.residence.instance_id,
                    candidate.guid,
                )
            {
                // R1b: the published representation consumes the authority, the
                // canonical health timeline and the aura provenance of the one
                // admission decision together. The expected-stamp CAS still
                // decides whether this is that incarnation's representation, so
                // a delayed publication cannot alter a replacement.
                world_creature
                    .creature
                    .take_pending_addon_aura_provenance_like_cpp();
                for (slot, spell_id, provenance) in admitted.aura_provenance.iter() {
                    let auras = &mut world_creature.creature.unit_mut().subsystems_mut().auras;
                    if auras
                        .visible_auras
                        .get(slot)
                        .is_some_and(|aura| aura.spell_id == *spell_id)
                    {
                        auras.set_aura_cast_provenance_like_cpp(*slot, provenance.clone());
                    }
                }
                let rebound = world_creature
                    .creature
                    .rebind_loot_authority_if_current_like_cpp(
                        &expected_legacy_authority,
                        expected_legacy_stamp,
                        admitted.loot_authority.clone(),
                    );
                if rebound.is_some() {
                    world_creature
                        .creature
                        .unit_mut()
                        .preserve_authoritative_health_state_for_snapshot_like_cpp(
                            &admitted.health_owner,
                        );
                }
            }
            if candidate.pending.persistent_spawn {
                if let Some(stmt) = manager.remove_persisted_respawn_time_like_cpp(
                    candidate.residence.map_id,
                    candidate.residence.instance_id,
                    wow_map::SpawnObjectType::Creature,
                    candidate.pending.spawn_id,
                ) {
                    outcome.respawn_db_mutations.push(stmt);
                }
                // Applied here rather than in the deferred canonical phase
                // because admission already ran; the mutation, its condition
                // (a successful legacy publication) and the counter are
                // unchanged.
                if let Some(canonical_map_manager) = canonical_map_manager
                    && remove_canonical_respawn_time_on_map_like_cpp(
                        canonical_map_manager,
                        u32::from(candidate.residence.map_id),
                        candidate.residence.instance_id,
                        wow_map::SpawnObjectType::Creature,
                        candidate.pending.spawn_id,
                    )
                {
                    outcome.canonical_respawn_removes += 1;
                }
            }
            affected_maps.insert((candidate.residence.map_id, candidate.residence.instance_id));
            outcome.respawns_processed += 1;
            outcome.canonical_inserts += 1;
        }
    }

    outcome.refresh_map_keys = affected_maps.into_iter().collect();
    outcome
}
