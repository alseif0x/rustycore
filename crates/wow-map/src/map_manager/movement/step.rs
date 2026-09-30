//! One creature movement step; application policy and path resolution are lazy inputs.

use crate::map_manager::{
    ChaseTargetSnapshotLikeCpp, CreaturePathQueryLikeCpp, LiveTerrainHeights, WorldCreature,
};
use wow_constants::UnitState;
use wow_core::Position;
use wow_entities::PhaseShift;
use wow_movement::{MoveSpline, MoveSplineStopResult};
use wow_recastdetour::DetourPolyPath;

use super::pending as family;
use super::pending::step_pending::{StepMetadata, wrap};
pub(crate) use super::pending::step_pending::{StepPending, StepProgress};

/// The generator that launched a movement spline, used by application tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureMovementSource {
    Home,
    Random,
    Waypoint,
    Chase,
}

/// An owned movement result awaiting application serialization and publication.
#[derive(Debug)]
pub enum CreatureMovementStep {
    Launch {
        source: CreatureMovementSource,
        from: Position,
        spline: MoveSpline,
    },
    Stop(MoveSplineStopResult),
}

impl WorldCreature {
    /// Advances the existing clock, spline and selected generator in their original order.
    /// Path resolution runs only where the selected generator requests a query.
    pub fn step_movement(
        &mut self,
        diff_ms: u32,
        chase_target: Option<ChaseTargetSnapshotLikeCpp>,
        terrain: Option<&LiveTerrainHeights>,
        mut policy: impl FnMut(u32, bool) -> bool,
        mut path: impl FnMut(CreaturePathQueryLikeCpp, u32, u32, &PhaseShift) -> Option<DetourPolyPath>,
    ) -> Option<CreatureMovementStep> {
        let mut progress =
            self.prepare_movement_step(diff_ms, chase_target, terrain.is_some(), &mut policy);
        loop {
            progress = match progress {
                StepProgress::Complete(result) => return result,
                StepProgress::Pending(StepPending::Path(request)) => {
                    let (query, continuation) = request.into_parts();
                    let response = path(
                        query,
                        continuation.map_id(),
                        continuation.instance_id(),
                        continuation.phase_shift(),
                    );
                    continuation.resume(self, response)
                }
                StepProgress::Pending(StepPending::StaticHeight(request)) => {
                    let (query, continuation) = request.into_parts();
                    let height = terrain
                        .expect("height requests require terrain")
                        .static_height_like_cpp(
                            query.map_id,
                            query.point.x,
                            query.point.y,
                            query.probe_z,
                        );
                    continuation.resume(self, height)
                }
                StepProgress::Pending(StepPending::GridHeight(request)) => {
                    let (query, continuation) = request.into_parts();
                    let height = terrain
                        .expect("height requests require terrain")
                        .grid_height_like_cpp(query.map_id, query.point.x, query.point.y);
                    continuation.resume(self, height)
                }
            };
        }
    }

    /// Prepares exactly one step on an exclusively owned actor. A pending query
    /// retains the dispatch metadata and consumes no further clock or policy.
    /// Future map consumers must gate the actor lifetime and every resumed write.
    pub(crate) fn prepare_movement_step(
        &mut self,
        diff_ms: u32,
        chase_target: Option<ChaseTargetSnapshotLikeCpp>,
        terrain_enabled: bool,
        mut policy: impl FnMut(u32, bool) -> bool,
    ) -> StepProgress {
        let creature = self;
        // C++ `Unit::Update(p_time)` gives `UpdateSplineMovement` and
        // `MotionMaster::Update` the same map-owned diff. Advance the creature's
        // logical deadline clock exactly once at that shared boundary; no wall
        // clock may independently move either side of the movement state machine.
        creature.advance_runtime_clock_like_cpp(diff_ms);

        if creature.is_alive() {
            // C++ `Unit::Update` advances `movespline` before calling
            // `i_motionMaster->Update(diff)`. The selected concrete generator below
            // therefore observes the spline's state from this same frame.
            let _ = creature.update_move_spline_like_cpp();
        }
        let current_generator = creature.tick_runtime_motion_master_like_cpp(diff_ms);

        if !creature.is_alive() {
            // Respawn ownership belongs to the global lifecycle tick. Reviving here
            // skips corpse removal, persisted timers, and destroy/create visibility,
            // leaving the client with a dead model that continues to move.
            return StepProgress::Complete(None);
        }

        if creature.state() == wow_entities::CreatureAiState::Returning {
            // C++ `HomeMovementGenerator<Creature>::SetTargetLocation` launches
            // `init.MoveTo(GetHomePosition())` with `generatePath = true`, so an
            // evading creature walks a navmesh route home instead of snapping there
            // (`HomeMovementGenerator.cpp:53-82`).
            let owner_ignores_pathfinding = creature
                .creature
                .unit()
                .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
            let source_map_id = creature.map_id();
            let source_instance_id = creature.instance_id();
            let phase_shift = creature.phase_shift().clone();
            let filter_context = creature.path_query_filter_context_like_cpp();
            let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
            let should_try_pathfinding = policy(source_map_id, owner_ignores_pathfinding);

            let progress = family::prepare_home(creature, should_try_pathfinding, terrain_enabled);
            return wrap(
                progress,
                StepMetadata {
                    source: CreatureMovementSource::Home,
                    map_id: source_map_id,
                    instance_id: source_instance_id,
                    phase_shift,
                },
            );
        }

        if creature.state() == wow_entities::CreatureAiState::WalkingRandom
            && current_generator != Some(wow_movement::MovementGeneratorType::Random)
            && creature.movement_finished()
        {
            // Keep the legacy AI-state compatibility cleanup bounded even when a
            // stale WalkingRandom state no longer corresponds to the selected
            // MotionMaster default.
            creature.finish_move();
            creature
                .creature
                .set_ai_state(wow_entities::CreatureAiState::Idle);
            return StepProgress::Complete(None);
        }

        match current_generator {
            Some(wow_movement::MovementGeneratorType::Random)
                if matches!(
                    creature.state(),
                    wow_entities::CreatureAiState::Idle
                        | wow_entities::CreatureAiState::WalkingRandom
                ) =>
            {
                let owner_ignores_pathfinding = creature
                    .creature
                    .unit()
                    .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
                let source_map_id = creature.map_id();
                let source_instance_id = creature.instance_id();
                let phase_shift = creature.phase_shift().clone();
                // C++ builds the Detour filter from the owner in
                // `PathGenerator::CreateFilter`, so it must be sampled from this
                // creature rather than assumed.
                let filter_context = creature.path_query_filter_context_like_cpp();
                let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
                // C++ `RandomMovementGenerator` keeps one `PathGenerator` for the
                // generator's lifetime, so its corridor is available to the next
                // query (`RandomMovementGenerator.cpp:140-143`).
                let previous_poly_refs = creature.active_random_path_poly_refs_like_cpp().to_vec();
                let should_try_pathfinding = policy(source_map_id, owner_ignores_pathfinding);
                let progress = family::prepare_random(
                    creature,
                    diff_ms,
                    should_try_pathfinding,
                    terrain_enabled,
                    false,
                );
                return wrap(
                    progress,
                    StepMetadata {
                        source: CreatureMovementSource::Random,
                        map_id: source_map_id,
                        instance_id: source_instance_id,
                        phase_shift,
                    },
                );
            }
            Some(wow_movement::MovementGeneratorType::Waypoint)
                if creature.state() == wow_entities::CreatureAiState::WalkingWaypoint =>
            {
                // C++ `WaypointMovementGenerator<Creature>::DoUpdate` advances the
                // generator from the map-owned creature update and `StartMove`
                // launches `MoveSplineInit::MoveTo(..., _generatePath)`, which
                // resolves `PathGenerator` before falling back to a direct segment.
                let owner_ignores_pathfinding = creature
                    .creature
                    .unit()
                    .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
                let source_map_id = creature.map_id();
                let source_instance_id = creature.instance_id();
                let phase_shift = creature.phase_shift().clone();
                // Same owner-derived filter as the random generator: C++ constructs
                // one `PathGenerator` per query and always runs `CreateFilter`.
                let filter_context = creature.path_query_filter_context_like_cpp();
                let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
                let should_try_pathfinding = policy(source_map_id, owner_ignores_pathfinding);
                let progress = family::prepare_waypoint(
                    creature,
                    diff_ms,
                    None,
                    should_try_pathfinding,
                    terrain_enabled,
                    false,
                );
                return wrap(
                    progress,
                    StepMetadata {
                        source: CreatureMovementSource::Waypoint,
                        map_id: source_map_id,
                        instance_id: source_instance_id,
                        phase_shift,
                    },
                );
            }
            Some(wow_movement::MovementGeneratorType::Chase) => {
                // `MoveChase` replaces the default random/waypoint generator at the
                // top of C++ MotionMaster. With a live victim snapshot the generator
                // paths to it exactly as `ChaseMovementGenerator::Update` does;
                // without one (no accessor for this target) the superseded wander
                // spline is still stopped so neither server nor clients keep running
                // the lower-priority movement.
                let Some(target) = chase_target else {
                    // The combat target vanished this tick (e.g. a player victim
                    // died and dropped out of the world snapshot, or its GUID no
                    // longer resolves). C++ chase `Update` returns false on
                    // `!target || !target->IsInWorld()` and `MotionMaster` finalizes
                    // the generator, so the runtime chase must be retired here too —
                    // not just its spline stopped — or `UNIT_STATE_CHASE_MOVE` and
                    // the generator would persist and re-drive toward the gone
                    // target every tick.
                    if let Some(stop) = creature.finalize_runtime_chase_movement_like_cpp() {
                        return StepProgress::Complete(Some(CreatureMovementStep::Stop(stop)));
                    }
                    return StepProgress::Complete(None);
                };

                let owner_ignores_pathfinding = creature
                    .creature
                    .unit()
                    .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
                let source_map_id = creature.map_id();
                let source_instance_id = creature.instance_id();
                let phase_shift = creature.phase_shift().clone();
                let filter_context = creature.path_query_filter_context_like_cpp();
                let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
                // C++ chase keeps its `PathGenerator` between updates, so its
                // corridor is reusable (`ChaseMovementGenerator.cpp:174-175`).
                let previous_poly_refs = creature.active_chase_path_poly_refs_like_cpp().to_vec();
                let should_try_pathfinding = policy(source_map_id, owner_ignores_pathfinding);

                let progress = family::prepare_chase(
                    creature,
                    diff_ms,
                    target,
                    should_try_pathfinding,
                    terrain_enabled,
                );
                return wrap(
                    progress,
                    StepMetadata {
                        source: CreatureMovementSource::Chase,
                        map_id: source_map_id,
                        instance_id: source_instance_id,
                        phase_shift,
                    },
                );
            }
            _ => {}
        }
        StepProgress::Complete(None)
    }
}

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod pending_tests;
