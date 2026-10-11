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
//! The store access sugar these scenarios share with the handler tests lives
//! in `crate::canonical_creature_fixtures`.

use super::*;
use crate::session::{
    AdmittedCanonicalCreatureStoreLikeCpp, AdmittedCreatureExecutionMapLikeCpp,
    AdmittedCreatureExecutionObjectLikeCpp, LegacyCreatureAggroCandidateLikeCpp,
    LegacyCreatureAggroTickOutcomeLikeCpp, LegacyCreatureLifecycleTickOutcomeLikeCpp,
    LegacyCreatureMeleeTickOutcomeLikeCpp, LegacyCreatureMovementTickOutcomeLikeCpp,
    LegacyCreatureSpellTickOutcomeLikeCpp, LegacyPlayerMeleeTickOutcomeLikeCpp,
    PlayerMeleeAttackerSnapshotLikeCpp, PlayerMeleePhaseStateLikeCpp,
};

/// Every map of the canonical manager and every creature on it, sorted by
/// GUID: the selection a single-phase driver runs on.
pub(in crate::session::tests) fn all_canonical_creatures_selection_like_cpp(
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
pub(in crate::session::tests) fn run_canonical_creature_aggro_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    candidates: &[LegacyCreatureAggroCandidateLikeCpp],
    config: LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureAggroTickOutcomeLikeCpp {
    run_canonical_creature_aggro_phase_with_terrain_for_test_like_cpp(
        canonical, None, candidates, config,
    )
}

/// [`run_canonical_creature_aggro_phase_for_test_like_cpp`] with a map
/// environment for the family-assistance LOS check.
pub(in crate::session::tests) fn run_canonical_creature_aggro_phase_with_terrain_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    terrain: Option<&crate::terrain::LiveTerrainHeights>,
    candidates: &[LegacyCreatureAggroCandidateLikeCpp],
    config: LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureAggroTickOutcomeLikeCpp {
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    let mut store =
        AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(&mut manager, &maps, &objects);
    crate::session::run_creature_aggro_phase_on_store_like_cpp(
        &mut store, terrain, candidates, &config,
    )
}

/// The creature melee phase (`DoMeleeAttackIfReady`).
pub(in crate::session::tests) fn run_canonical_creature_melee_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureMeleeTickOutcomeLikeCpp {
    let mut outcome = LegacyCreatureMeleeTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::session::run_creature_melee_phase_on_manager_like_cpp(
        &mut manager,
        &maps,
        &objects,
        config,
        &mut outcome,
    );
    outcome
}

/// The creature spell phase (`CombatAI`/`TurretAI::UpdateAI`).
pub(in crate::session::tests) fn run_canonical_creature_spell_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureSpellTickOutcomeLikeCpp {
    let mut outcome = LegacyCreatureSpellTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::session::run_creature_spell_phase_on_manager_like_cpp(
        &mut manager,
        &maps,
        &objects,
        config,
        &mut outcome,
    );
    outcome
}

/// The creature lifecycle phase (JUST_DIED save, corpse despawn, respawn).
pub(in crate::session::tests) fn run_canonical_creature_lifecycle_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_store: &wow_data::MapStore,
    now: Instant,
) -> LegacyCreatureLifecycleTickOutcomeLikeCpp {
    let mut outcome = LegacyCreatureLifecycleTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, mut objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::session::run_creature_lifecycle_phase_on_manager_like_cpp(
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
pub(in crate::session::tests) fn run_canonical_creature_movement_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&crate::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    chase_targets: &HashMap<
        (u16, u32, ObjectGuid),
        crate::creature_movement::ChaseTargetSnapshotLikeCpp,
    >,
    diff_ms: u32,
) -> LegacyCreatureMovementTickOutcomeLikeCpp {
    let mut outcome = LegacyCreatureMovementTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    let (maps, objects) = all_canonical_creatures_selection_like_cpp(&manager);
    crate::session::run_creature_movement_phase_on_manager_like_cpp(
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
pub(in crate::session::tests) fn run_canonical_player_melee_phase_for_test_like_cpp(
    canonical: &SharedCanonicalMapManager,
    attackers: &[PlayerMeleeAttackerSnapshotLikeCpp],
    diff_ms: u32,
    phase_state: &mut PlayerMeleePhaseStateLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyPlayerMeleeTickOutcomeLikeCpp {
    let mut outcome = LegacyPlayerMeleeTickOutcomeLikeCpp::default();
    let mut manager = lock_canonical_for_test_like_cpp(canonical);
    crate::session::run_player_melee_phase_on_manager_like_cpp(
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
pub(in crate::session::tests) fn step_creature_movement_like_cpp(
    creature: &mut wow_entities::Creature,
    guid: ObjectGuid,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&crate::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    terrain: Option<&crate::terrain::LiveTerrainHeights>,
    chase_target: Option<crate::creature_movement::ChaseTargetSnapshotLikeCpp>,
    diff_ms: u32,
) -> Option<Vec<u8>> {
    crate::session::step_creature_movement_on_view_like_cpp(
        &mut crate::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(
            &mut creature.creature,
        ),
        guid,
        mmap_config,
        mmap_pathfinder,
        terrain,
        chase_target,
        diff_ms,
    )
}

/// Run the map-owned player melee phase for this session's player the way the
/// canonical map loop does (C++ `Player::Update` → `DoMeleeAttackIfReady`),
/// then deliver its results to the session as the registry would.
///
/// #1263 F6-8E replaced the per-session combat tick with this phase.
pub(in crate::session::tests) fn run_session_player_melee_phase_for_test_like_cpp(
    session: &mut WorldSession,
    diff_ms: u32,
) -> LegacyPlayerMeleeTickOutcomeLikeCpp {
    let player_guid = session
        .player_guid()
        .expect("the player melee phase needs a player");
    let canonical = session
        .core
        .canonical_map_manager
        .clone()
        .expect("the player melee phase runs on the canonical map");
    let (map_id, instance_id) = session.core.current_runtime_map_key_like_cpp();
    // The phase reaches the session through its own outbound and command
    // channels, as through its registry registration in production.
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(
            player_guid,
            session.send_tx().clone(),
            session.core.session_command_tx.clone(),
        ),
        Default::default(),
    );
    let attackers = vec![PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id,
        instance_id,
        in_combat_mirror: crate::session::hub_ref(session)
            .resolved_in_combat_like_cpp()
            .unwrap_or(false),
        tap_group_guids: Vec::new(),
    }];
    let config = session.config.legacy_creature_aggro_config_like_cpp.clone();
    let mut phase_state = PlayerMeleePhaseStateLikeCpp::default();
    let outcome = run_canonical_player_melee_phase_for_test_like_cpp(
        &canonical,
        &attackers,
        diff_ms,
        &mut phase_state,
        &config,
    );
    // The registry delivers to an in-world session; its handler re-gates on
    // `LoggedIn`, which these fixture sessions model by construction.
    if session.state() != SessionState::LoggedIn {
        session.set_state(SessionState::LoggedIn);
    }
    for command in outcome.commands.clone() {
        session.handle_apply_player_melee_result_like_cpp_command_like_cpp(command);
    }
    outcome
}
