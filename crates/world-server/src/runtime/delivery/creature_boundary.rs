//! Typed input and outcome for the map-owned Creature runtime boundary.
//!
//! This submodule keeps the delivery facade under its physical source budget;
//! it does not add an owner or a second runtime clock.

use super::*;

/// Phases emitted by the one map-owned Creature runtime boundary.
///
/// The order is the order of the current production bridge: the world-session
/// player pass completes before the map-owned lifecycle/object work, and the
/// Creature sub-phases then run once on the global owner. Keeping this as a
/// typed value makes the order testable without treating a helper call or a
/// timer mutation as a complete C++ `Creature::Update`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreatureRuntimePhaseLikeCpp {
    PlayerMelee,
    Lifecycle,
    Movement,
    Aggro,
    Spell,
    Melee,
}

/// The map incarnation captured when one Creature runtime tick starts.
///
/// A map key alone can be reused after unload/recreate. The incarnation is
/// therefore part of the outcome envelope and is rechecked before the caller
/// considers the tick's deferred work current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CreatureRuntimeMapStampLikeCpp {
    pub map_id: u32,
    pub instance_id: u32,
    pub incarnation: u64,
}

/// A canonical Creature admitted by the map's loaded-grid ObjectUpdater pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CreatureRuntimeObjectStampLikeCpp {
    pub creature_guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    pub incarnation: u64,
}

/// Immutable input captured for one global Creature runtime tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureRuntimeTickInputLikeCpp {
    pub tick_epoch: u64,
    pub diff_ms: u32,
    pub game_time_secs: i64,
    pub map_stamps: Vec<CreatureRuntimeMapStampLikeCpp>,
    pub admitted_creatures: Vec<CreatureRuntimeObjectStampLikeCpp>,
}

impl CreatureRuntimeTickInputLikeCpp {
    pub(super) fn capture_like_cpp(
        tick_epoch: u64,
        diff_ms: u32,
        canonical_map_manager: Option<&SharedCanonicalMapManager>,
    ) -> Self {
        let mut map_stamps = Vec::new();
        let mut admitted_creatures = Vec::new();
        if let Some(canonical_map_manager) = canonical_map_manager {
            let manager = canonical_map_manager
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            manager.do_for_all_maps(|managed| {
                let key = wow_map::MapKey::new(managed.map_id(), managed.instance_id());
                if let Some(incarnation) = manager.map_incarnation_like_cpp(key) {
                    map_stamps.push(CreatureRuntimeMapStampLikeCpp {
                        map_id: key.map_id,
                        instance_id: key.instance_id,
                        incarnation,
                    });
                    admitted_creatures.extend(
                        managed
                            .map()
                            .admitted_creature_guids_like_cpp()
                            .into_iter()
                            .map(|creature_guid| CreatureRuntimeObjectStampLikeCpp {
                                creature_guid,
                                map_id: key.map_id,
                                instance_id: key.instance_id,
                                incarnation,
                            }),
                    );
                }
            });
            map_stamps.sort_unstable();
            admitted_creatures.sort_unstable();
        }
        Self {
            tick_epoch,
            diff_ms,
            game_time_secs: i64::try_from(current_unix_time_secs_like_cpp()).unwrap_or(i64::MAX),
            map_stamps,
            admitted_creatures,
        }
    }

    pub(crate) fn current_map_incarnation_mismatches_like_cpp(
        &self,
        canonical_map_manager: Option<&SharedCanonicalMapManager>,
    ) -> usize {
        let Some(canonical_map_manager) = canonical_map_manager else {
            return 0;
        };
        let manager = canonical_map_manager
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.map_stamps
            .iter()
            .filter(|stamp| {
                manager
                    .map_incarnation_like_cpp(wow_map::MapKey::new(stamp.map_id, stamp.instance_id))
                    != Some(stamp.incarnation)
            })
            .count()
    }
}

/// Typed state/effect/publication envelope for one Creature runtime tick.
///
/// The detailed phase results remain on
/// [`super::LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp`] for compatibility
/// with existing consumers. This envelope is the single boundary metadata
/// shared by those results: input identity, completed phase order,
/// map-incarnation validation and aggregate deferred work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureRuntimeBoundaryOutcomeLikeCpp {
    pub input: CreatureRuntimeTickInputLikeCpp,
    pub completed_phases: Vec<CreatureRuntimePhaseLikeCpp>,
    pub map_incarnation_mismatches: usize,
    pub publication_events: usize,
    pub session_commands: usize,
    pub db_mutations_produced: usize,
    pub db_mutations_submitted: usize,
}
