//! Canonical creature store fixtures and single-phase drivers.
//!
//! #1263 F6-8E deleted the legacy creature store and its runtime. The canonical
//! map is the only creature store and the admitted executor
//! (`run_admitted_creature_combat_phases_on_locked_manager_like_cpp`) its only
//! runner. Scenario tests that pin one phase's behaviour drive that phase on
//! its own through the drivers below: each locks the canonical manager, selects
//! every creature of every canonical map (sorted by GUID, so the visit order is
//! stable) and runs the same production phase body the executor runs.
//!
//! #1363 slice 3b moved these drivers here from the `wow-world` session tests,
//! next to the phase bodies they drive: the runtime's own scenario tests and the
//! `wow-world` session tests both reach them, and `wow-world-test-support` cannot
//! name runtime types without a dev-dependency cycle. They are gated like the
//! other test-only runtime entries and are never built into production.

use std::collections::HashMap;
use std::time::Instant;

use wow_core::ObjectGuid;
use wow_world_core::session::{MMapRuntimeConfigLikeCpp, SharedCanonicalMapManager};
use wow_world_entities::creature_aggro_contracts::CreatureAggroConfigLikeCpp;

use crate::{
    AdmittedCanonicalCreatureStoreLikeCpp, AdmittedCreatureExecutionMapLikeCpp,
    AdmittedCreatureExecutionObjectLikeCpp, CreatureAggroCandidateLikeCpp,
    CreatureAggroTickOutcomeLikeCpp, CreatureLifecycleTickOutcomeLikeCpp,
    CreatureMeleeTickOutcomeLikeCpp, CreatureMovementTickOutcomeLikeCpp,
    CreatureSpellTickOutcomeLikeCpp, PlayerMeleeAttackerSnapshotLikeCpp,
    PlayerMeleePhaseStateLikeCpp, PlayerMeleeTickOutcomeLikeCpp,
};

/// Every map of the canonical manager and every creature on it, sorted by
/// GUID: the selection a single-phase driver runs on.
pub fn all_canonical_creatures_selection_like_cpp(
    manager: &wow_map::MapManager,
) -> (
    Vec<AdmittedCreatureExecutionMapLikeCpp>,
    Vec<AdmittedCreatureExecutionObjectLikeCpp>,
) {
    let mut maps = Vec::new();
    let mut objects = Vec::new();
    manager.do_for_all_maps(|managed| {
        maps.push(AdmittedCreatureExecutionMapLikeCpp {
            map_id: managed.map_id(),
            instance_id: managed.instance_id(),
            incarnation: 0,
        });
        let map = managed.map();
        let mut guids: Vec<_> = map
            .typed_combat_unit_guids_like_cpp()
            .into_iter()
            .filter(|guid| map.get_typed_creature(*guid).is_some())
            .collect();
        guids.sort();
        objects.extend(guids.into_iter().map(|creature_guid| {
            AdmittedCreatureExecutionObjectLikeCpp {
                map_id: managed.map_id(),
                instance_id: managed.instance_id(),
                incarnation: 0,
                creature_guid,
                record_identity: 0,
            }
        }));
    });
    (maps, objects)
}

fn lock_canonical_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
) -> std::sync::MutexGuard<'_, wow_map::MapManager> {
    canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The aggro/threat/assist/evade phase body alone, without the executor's
/// attack start/stop commit.
pub fn run_canonical_creature_aggro_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    candidates: &[CreatureAggroCandidateLikeCpp],
    config: CreatureAggroConfigLikeCpp,
) -> CreatureAggroTickOutcomeLikeCpp {
    run_canonical_creature_aggro_phase_with_terrain_for_test_like_cpp(
        canonical, None, candidates, config,
    )
}

/// [`run_canonical_creature_aggro_phase_for_test_like_cpp`] with a map
/// environment for the family-assistance LOS check.
pub fn run_canonical_creature_aggro_phase_with_terrain_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    terrain: Option<&wow_world_core::terrain::LiveTerrainHeights>,
    candidates: &[CreatureAggroCandidateLikeCpp],
    config: CreatureAggroConfigLikeCpp,
) -> CreatureAggroTickOutcomeLikeCpp {
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    let mut store =
        AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(&mut manager, &maps, &objects);
    crate::run_creature_aggro_phase_on_store_like_cpp(
        &mut store, terrain, candidates, &config,
    )
}

/// The creature melee phase (`DoMeleeAttackIfReady`).
pub fn run_canonical_creature_melee_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    config: &CreatureAggroConfigLikeCpp,
) -> CreatureMeleeTickOutcomeLikeCpp {
    let mut outcome = CreatureMeleeTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::run_creature_melee_phase_on_manager_like_cpp(
        &mut manager,
        &maps,
        &objects,
        config,
        &mut outcome,
    );
    outcome
}

/// The creature spell phase (`CombatAI`/`TurretAI::UpdateAI`).
pub fn run_canonical_creature_spell_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    config: &CreatureAggroConfigLikeCpp,
) -> CreatureSpellTickOutcomeLikeCpp {
    let mut outcome = CreatureSpellTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::run_creature_spell_phase_on_manager_like_cpp(
        &mut manager,
        &maps,
        &objects,
        config,
        &mut outcome,
    );
    outcome
}

/// The creature lifecycle phase (JUST_DIED save, corpse despawn, respawn).
pub fn run_canonical_creature_lifecycle_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_store: &wow_data::MapStore,
    now: Instant,
) -> CreatureLifecycleTickOutcomeLikeCpp {
    let mut outcome = CreatureLifecycleTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, mut objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::run_creature_lifecycle_phase_on_manager_like_cpp(
        &mut manager,
        &maps,
        &mut objects,
        None,
        map_store,
        now,
        &mut outcome,
    );
    outcome
}

/// The creature movement phase (`Unit::Update` clock, spline, motion master).
pub fn run_canonical_creature_movement_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&wow_world_core::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    chase_targets: &HashMap<
        (u16, u32, ObjectGuid),
        wow_world_core::creature_movement::ChaseTargetSnapshotLikeCpp,
    >,
    diff_ms: u32,
) -> CreatureMovementTickOutcomeLikeCpp {
    let mut outcome = CreatureMovementTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::run_creature_movement_phase_on_manager_like_cpp(
        &mut manager,
        &maps,
        &objects,
        None,
        mmap_config,
        mmap_pathfinder,
        chase_targets,
        diff_ms,
        &mut outcome,
    );
    outcome
}

/// The player melee phase (`Player::Update` → `DoMeleeAttackIfReady`).
pub fn run_canonical_player_melee_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    attackers: &[PlayerMeleeAttackerSnapshotLikeCpp],
    diff_ms: u32,
    phase_state: &mut PlayerMeleePhaseStateLikeCpp,
    config: &CreatureAggroConfigLikeCpp,
) -> PlayerMeleeTickOutcomeLikeCpp {
    let mut outcome = PlayerMeleeTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    crate::run_player_melee_phase_on_manager_like_cpp(
        &mut manager,
        attackers,
        diff_ms,
        phase_state,
        config,
        &mut outcome,
    );
    outcome
}

/// One movement step of one creature, outside any store (C++
/// `Unit::Update(p_time)`'s spline/motion master step on that unit).
#[allow(clippy::too_many_arguments)]
pub fn step_creature_movement_like_cpp(
    creature: &mut wow_entities::Creature,
    guid: ObjectGuid,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&wow_world_core::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    terrain: Option<&wow_world_core::terrain::LiveTerrainHeights>,
    chase_target: Option<wow_world_core::creature_movement::ChaseTargetSnapshotLikeCpp>,
    diff_ms: u32,
) -> Option<Vec<u8>> {
    crate::step_creature_movement_on_view_like_cpp(
        &mut wow_world_core::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(creature),
        guid,
        mmap_config,
        mmap_pathfinder,
        terrain,
        chase_target,
        diff_ms,
    )
}
