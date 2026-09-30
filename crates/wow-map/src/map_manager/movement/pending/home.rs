//! Home destination, path query and launch continuation.
use super::*;
pub(super) struct Destination {
    plan: wow_movement::HomeLaunchPlan,
    should_try_pathfinding: bool,
    terrain_enabled: bool,
}
pub(super) struct Launch {
    plan: wow_movement::HomeLaunchPlan,
    destination: Position,
}
fn complete(outcome: ChaseTickOutcomeLikeCpp) -> MovementProgress {
    MovementProgress::Complete(MovementCompletion::Home(outcome))
}
pub(in crate::map_manager::movement) fn prepare(
    actor: &mut WorldCreature,
    should_try_pathfinding: bool,
    terrain_enabled: bool,
) -> MovementProgress {
    let snapshot = actor.home_unit_snapshot_like_cpp();
    let from_update = actor.runtime.active_home_generator.is_some();
    let action = match actor.runtime.active_home_generator.as_mut() {
        Some(generator) => generator.update_like_cpp(true, snapshot),
        None => {
            let mut generator = HomeMovementGenerator::new();
            let action = generator.initialize_like_cpp(true, snapshot);
            actor.runtime.active_home_generator = Some(generator);
            // C++ `CreatureAI::EnterEvadeMode` adds `UNIT_STATE_EVADE`
            // immediately before `MoveTargetedHome()` (`CreatureAI.cpp:237`),
            // and `HomeMovementGenerator::DoFinalize` is what clears it
            // (`HomeMovementGenerator.cpp:143`). The state is what makes the
            // creature immune to attacks and un-aggroable while it walks
            // back; without it, this now multi-tick return would let a
            // player damage and re-aggro a fully reset creature.
            //
            // Boundary: C++ sets it one step earlier, in the AI evade entry,
            // after `_EnterEvadeMode()` bookkeeping and only on the
            // no-charmer branch. This runtime has no live AI evade entry,
            // so the state is added where the return actually begins. A
            // full `CreatureAI::EnterEvadeMode` port stays with M2.5.
            actor
                .creature
                .unit_mut()
                .add_unit_state(UnitState::EVADE.bits());
            action
        }
    };

    match action {
        wow_movement::HomeMovementAction::Continue => complete(ChaseTickOutcomeLikeCpp::Idle),
        // C++ `SetTargetLocation` sets `MOVEMENTGENERATOR_FLAG_INTERRUPTED`
        // and returns without launching while ROOT/STUNNED/DISTRACTED; the
        // generator stays installed and only the *next* `DoUpdate` sets
        // `INFORM_ENABLED` and finalizes (`HomeMovementGenerator.cpp:53-58,
        // 117-122`). Finalizing here in the initialize frame would skip that
        // `INFORM_ENABLED`, suppressing `JustReachedHome` and clearing evade
        // one frame early. So `Interrupted` from initialization keeps the
        // generator; only a `Finished` from an update finalizes.
        wow_movement::HomeMovementAction::Interrupted if !from_update => {
            complete(ChaseTickOutcomeLikeCpp::Idle)
        }
        wow_movement::HomeMovementAction::Interrupted
        | wow_movement::HomeMovementAction::Finished => {
            actor.finish_home_movement_like_cpp();
            complete(ChaseTickOutcomeLikeCpp::Idle)
        }
        wow_movement::HomeMovementAction::Launch(plan) => {
            actor
                .creature
                .unit_mut()
                .clear_unit_state(plan.clear_unit_state_mask);
            actor
                .creature
                .unit_mut()
                .add_unit_state(plan.add_unit_state);

            terrain::normalize(
                actor,
                plan.destination,
                terrain_enabled,
                TerrainPurpose::Home(Destination {
                    plan,
                    should_try_pathfinding,
                    terrain_enabled,
                }),
            )
        }
    }
}
pub(super) fn destination_ready(
    actor: &mut WorldCreature,
    pending: Destination,
    destination: Position,
) -> MovementProgress {
    let Destination {
        plan,
        should_try_pathfinding,
        terrain_enabled,
    } = pending;
    let launch = Launch { plan, destination };
    if !should_try_pathfinding {
        return resolved(actor, launch, None, terrain_enabled);
    }
    // Built after `UNIT_STATE_EVADE` was added above, which
    // is what makes `UpdateFilter` include
    // `NAV_GROUND_STEEP` — C++ sets evade before
    // `MoveTargetedHome` constructs the path, so sampling the
    // filter earlier would path the return without steep
    // ground.
    let query = CreaturePathQueryLikeCpp {
        start: actor.position(),
        destination,
        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
        force_destination: false,
        filter_context: actor.path_query_filter_context_like_cpp(),
        owner: actor.detour_owner_capabilities_like_cpp(),
        // C++ `MoveSplineInit::MoveTo` builds a fresh
        // `PathGenerator`, so the home leg has no corridor to
        // reuse.
        previous_poly_refs: Vec::new(),
    };
    request_path(query, PathPurpose::Home(launch), terrain_enabled)
}
pub(super) fn resolved(
    actor: &mut WorldCreature,
    launch: Launch,
    detour: Option<DetourPolyPath>,
    enabled: bool,
) -> MovementProgress {
    match detour {
        Some(detour) => terrain::normalize_path(
            actor,
            launch.destination,
            detour,
            false,
            enabled,
            terrain::PathFinish::Home(launch),
        ),
        None => self::launch(actor, launch, None),
    }
}
pub(super) fn launch(
    actor: &mut WorldCreature,
    launch: Launch,
    path: Option<PathGenerator>,
) -> MovementProgress {
    let Launch { plan, destination } = launch;
    // C++ goes through `MoveSplineInit::MoveTo(..., generatePath)`,
    // which falls back to a direct two-point spline whenever the path
    // is unusable (`MoveSplineInit.cpp:261-277`).
    let path = path.filter(|path| !path.path_type().contains(PathType::NOPATH));
    let spline_id = actor.spline_id().saturating_add(1);
    let mut init = MoveSplineInit::new(spline_id);
    init.set_walk(plan.walk);
    match path {
        Some(path) => init.move_by_path(path.path_points().to_vec(), 0),
        None => init.move_to(destination),
    }
    init.set_facing_angle(plan.facing);

    match actor.launch_move_spline_init_like_cpp(&mut init, destination) {
        Some((from, spline)) => complete(ChaseTickOutcomeLikeCpp::Launched(from, spline)),
        None => {
            actor.finish_home_movement_like_cpp();
            complete(ChaseTickOutcomeLikeCpp::Idle)
        }
    }
}
