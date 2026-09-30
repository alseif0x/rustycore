//! Whole-map ownership admission, before producers or after permanent closure.
//!
//! This is a new, deliberately stricter ownership contract, not a claim that
//! legacy and canonical gameplay are equivalent. Distinct loot authorities
//! reject even when pristine; the existing snapshot reconciler is not used.
//! C++ a5f8da2e Map.cpp:530-577 / Creature.cpp:333-351 own one live object.
//! This Rust transition moves the existing motor without replaying that Add.

use super::{AccessorObjectKind, GridLifecycle, Map, MapObjectRecord, MapObjectStoreError, TerrainGridLoader};
use crate::map_manager::{
    GridCoord as LegacyGridCoord, MapInstance, LegacyCreatureTransportError,
    LegacyCreatureTransportSlot,
};
use wow_core::ObjectGuid;
use std::collections::HashSet;

mod commit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureActorTransportError {
    MapBusy,
    MissingMap,
    StaleIncarnation,
    WrongLegacyMap,
    MissingLegacyMap,
    SourceSlot { guid: ObjectGuid, grid: LegacyGridCoord },
    MissingSourceGrid { grid: LegacyGridCoord },
    SourceGridCoordinateMismatch { grid: LegacyGridCoord },
    MissingSourceActor { guid: ObjectGuid },
    SourceGuidMismatch { guid: ObjectGuid },
    SourceMapMismatch { guid: ObjectGuid },
    DuplicateSourceGuid { guid: ObjectGuid },
    MissingCounterpart { guid: ObjectGuid },
    CanonicalOnly { guid: ObjectGuid },
    ExistingActor { guid: ObjectGuid },
    NotExactCreature { guid: ObjectGuid },
    GuidMismatch { guid: ObjectGuid },
    Store { guid: ObjectGuid, error: MapObjectStoreError },
    SpawnMismatch { guid: ObjectGuid, source: u64, canonical: u64 },
    SpawnIndexMismatch { guid: ObjectGuid, spawn_id: u64 },
    CardinalityMismatch { source: usize, canonical: usize },
    HealthTimelineMismatch { guid: ObjectGuid },
    LootAuthorityMismatch { guid: ObjectGuid },
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreatureActorTransportSummary {
    pub transported: usize,
    pub source_inner_winners: usize,
    pub canonical_inner_winners: usize,
}

struct PreparedCreatureTransport {
    source_slot: LegacyCreatureTransportSlot,
    guid: ObjectGuid,
    source_wins: bool,
    threat_delta: Option<PreparedCreatureThreatDelta>,
}

struct PreparedCreatureThreatDelta {
    guid: ObjectGuid,
    mirrored: Vec<ObjectGuid>,
    removed: Vec<ObjectGuid>,
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// The manager checks Idle/incarnation before obtaining this exclusive
    /// map borrow. APP additionally owes startup/permanent-close quiescence.
    /// No callback or guard release intervenes between full preflight/commit.
    pub(crate) fn transport_legacy_creature_ownership(
        &mut self,
        source: &mut MapInstance,
    ) -> Result<CreatureActorTransportSummary, CreatureActorTransportError> {
        let prepared = self.preflight_creature_transport(source)?;
        let mut summary = CreatureActorTransportSummary {
            transported: 0, source_inner_winners: 0, canonical_inner_winners: 0,
        };
        let mut threat_deltas = Vec::new();
        for planned in prepared {
            // Full-map preflight covered every source and target before any
            // extraction. Promote every winning inner before touching reciprocal
            // references, so a later SOURCE winner cannot overwrite a delta.
            if let Some(delta) = self.commit_creature_transport(source, planned, &mut summary) {
                threat_deltas.push(delta);
            }
        }
        // Preserve the captured source HashMap order and each ADD -> PURGE.
        // Both phases remain inside the same exclusive Map/source borrows.
        for delta in threat_deltas {
            self.apply_transported_threat_delta(delta);
        }
        Ok(summary)
    }

    fn preflight_creature_transport(
        &self,
        source: &MapInstance,
    ) -> Result<Vec<PreparedCreatureTransport>, CreatureActorTransportError> {
        if u32::from(source.map_id) != self.map_id() || source.instance_id != self.instance_id() {
            return Err(CreatureActorTransportError::WrongLegacyMap);
        }
        // Metadata only: no Creature, motor, loot owner or second actor table.
        // Preserve actual source HashMap traversal; do not sort or filter it.
        let mut prepared = Vec::new();
        for (coord, grid) in &source.grids {
            for (guid, actor) in &grid.creatures {
                let source_slot = source.preflight_creature_transport_slot(*coord, *guid)
                    .map_err(|error| match error {
                        LegacyCreatureTransportError::WrongInstance => CreatureActorTransportError::WrongLegacyMap,
                        LegacyCreatureTransportError::MissingGrid => CreatureActorTransportError::MissingSourceGrid { grid: *coord },
                        LegacyCreatureTransportError::GridCoordinateMismatch => CreatureActorTransportError::SourceGridCoordinateMismatch { grid: *coord },
                        LegacyCreatureTransportError::MissingActor => CreatureActorTransportError::MissingSourceActor { guid: *guid },
                        LegacyCreatureTransportError::GuidMismatch => CreatureActorTransportError::SourceGuidMismatch { guid: *guid },
                        LegacyCreatureTransportError::ActorMapMismatch => CreatureActorTransportError::SourceMapMismatch { guid: *guid },
                        LegacyCreatureTransportError::DuplicateGuid => {
                            CreatureActorTransportError::DuplicateSourceGuid { guid: *guid }
                        }
                        LegacyCreatureTransportError::Occupied => CreatureActorTransportError::SourceSlot { guid: *guid, grid: *coord },
                    })?;
                self.validate_creature_actor(actor)
                    .map_err(|error| CreatureActorTransportError::Store { guid: *guid, error })?;
                let current = self.entity_world.creature_transport_counterpart(*guid)?;
                MapObjectRecord::validate_world_object(
                    AccessorObjectKind::Creature, current.unit().world(),
                ).map_err(MapObjectStoreError::from)
                    .and_then(|()| self.validate_map_object(current.unit().world()))
                    .map_err(|error| CreatureActorTransportError::Store { guid: *guid, error })?;
                if current.spawn_id() != actor.creature.spawn_id() {
                    return Err(CreatureActorTransportError::SpawnMismatch {
                        guid: *guid, source: actor.creature.spawn_id(), canonical: current.spawn_id(),
                    });
                }
                if current.spawn_id() != 0 && !self.creatures_by_spawn_id
                    .get(&current.spawn_id()).is_some_and(|guids| guids.contains(guid))
                {
                    return Err(CreatureActorTransportError::SpawnIndexMismatch {
                        guid: *guid, spawn_id: current.spawn_id(),
                    });
                }
                let incoming_unit = actor.creature.unit();
                let current_unit = current.unit();
                if !incoming_unit.shares_health_state_revision_authority_like_cpp(
                    &current_unit.health_state_revision_authority_like_cpp(),
                ) {
                    return Err(CreatureActorTransportError::HealthTimelineMismatch { guid: *guid });
                }
                if !actor.creature.loot_authority_like_cpp()
                    .shares_storage_like_cpp(current.loot_authority_like_cpp())
                {
                    return Err(CreatureActorTransportError::LootAuthorityMismatch { guid: *guid });
                }
                let incoming_revision = incoming_unit.health_state_revision_like_cpp();
                let current_revision = current_unit.health_state_revision_like_cpp();
                let health_tuple_matches = incoming_unit.data().health == current_unit.data().health
                    && incoming_unit.data().max_health == current_unit.data().max_health
                    && incoming_unit.death_state() == current_unit.death_state();
                let source_wins = incoming_revision > current_revision
                    || (incoming_revision == current_revision && health_tuple_matches);
                // Freeze only GUID facts before any promotion. Preserve the
                // adapter's incoming HashSet iteration and old sorted removals.
                let threat_delta = source_wins.then(|| {
                    let old_threat_guids = current_unit.subsystems().combat.sorted_threat_guids();
                    let incoming_threat_guids: HashSet<_> = incoming_unit.subsystems().combat
                        .sorted_threat_guids().into_iter().collect();
                    let removed = old_threat_guids.into_iter()
                        .filter(|target| !incoming_threat_guids.contains(target)).collect();
                    let mirrored = incoming_threat_guids.iter().copied().collect();
                    PreparedCreatureThreatDelta { guid: *guid, mirrored, removed }
                });
                prepared.push(PreparedCreatureTransport {
                    source_slot, guid: *guid, source_wins, threat_delta,
                });
            }
        }
        let mut canonical_count = 0;
        for (guid, view) in self.entity_world.iter() {
            if view.kind() != AccessorObjectKind::Creature {
                continue;
            }
            // Also reject invalid Creature-kind bodies and Actor collisions
            // without a legacy counterpart. No selected-subset filter is used.
            self.entity_world.creature_transport_counterpart(*guid)?;
            canonical_count += 1;
            if !prepared.iter().any(|planned| planned.guid == *guid) {
                return Err(CreatureActorTransportError::CanonicalOnly { guid: *guid });
            }
        }
        if prepared.len() != canonical_count {
            return Err(CreatureActorTransportError::CardinalityMismatch {
                source: prepared.len(), canonical: canonical_count,
            });
        }
        for (spawn_id, guids) in &self.creatures_by_spawn_id {
            for guid in guids {
                let creature = self.entity_world.creature_transport_counterpart(*guid)?;
                if *spawn_id == 0 || creature.spawn_id() != *spawn_id {
                    return Err(CreatureActorTransportError::SpawnIndexMismatch {
                        guid: *guid, spawn_id: *spawn_id,
                    });
                }
            }
        }
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests;
