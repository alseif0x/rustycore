//! Chase destination heights, attempted-query distinction and launch continuation.
use super::*;
pub(super) struct Destination {
    plan: wow_movement::ChaseLaunchPlan,
    target: ChaseTargetSnapshotLikeCpp,
    should_try_pathfinding: bool,
    terrain_enabled: bool,
}
pub(super) struct Launch {
    plan: wow_movement::ChaseLaunchPlan,
    target: ChaseTargetSnapshotLikeCpp,
    destination: Position,
}
fn complete(outcome: ChaseTickOutcomeLikeCpp) -> MovementProgress {
    MovementProgress::Complete(MovementCompletion::Chase(outcome))
}
pub(in crate::map_manager::movement) fn prepare(
    actor: &mut WorldCreature,
    diff_ms: u32,
    target: ChaseTargetSnapshotLikeCpp,
    should_try_pathfinding: bool,
    terrain_enabled: bool,
) -> MovementProgress {
    // C++ installs a *new* `ChaseMovementGenerator` per `MoveChase` call and
    // its `AbstractFollower` is bound to that victim for the generator's
    // whole life (`ChaseMovementGenerator.cpp:68-76`,
    // `AbstractFollower.cpp:21-31`). Reusing one across a victim switch
    // would carry the previous follower, `_lastTargetPosition`,
    // `_rangeCheckTimer`, `_movingTowards` and the arrival
    // `MovementInform` counter onto the new target, so the generator is
    // rebuilt whenever the victim differs.
    let victim_changed = actor
        .runtime
        .active_chase_generator
        .as_ref()
        .is_none_or(|generator| generator.target() != Some(target.guid));
    if victim_changed {
        let mut generator = ChaseMovementGenerator::new(target.guid, None, None);
        generator.initialize_like_cpp();
        actor.runtime.active_chase_generator = Some(generator);
        actor.runtime.active_chase_path_poly_refs.clear();
    }

    let snapshot = actor.chase_unit_snapshot_like_cpp(target);
    let action = match actor.runtime.active_chase_generator.as_mut() {
        Some(generator) => generator.update_like_cpp(true, target.in_world, diff_ms, snapshot),
        None => return complete(ChaseTickOutcomeLikeCpp::Idle),
    };

    match action {
        wow_movement::ChaseMovementAction::Continue => complete(ChaseTickOutcomeLikeCpp::Idle),
        // C++ chase `Update` returns false when the victim is gone or has
        // left the world (`ChaseMovementGenerator.cpp:97,101-103`), which
        // pops the generator via `MotionMaster::Update` and runs `Finalize`.
        // Clearing only the corridor would leave the generator,
        // `UNIT_STATE_CHASE_MOVE` and the in-flight spline intact, so the
        // creature keeps sliding toward the corpse and is re-selected as
        // chasing every tick.
        wow_movement::ChaseMovementAction::Finished => {
            match actor.finalize_runtime_chase_movement_like_cpp() {
                Some(stop) => complete(ChaseTickOutcomeLikeCpp::Stopped(stop)),
                None => complete(ChaseTickOutcomeLikeCpp::Idle),
            }
        }
        // C++ `StopMoving()` clears `UNIT_STATE_MOVING` (which contains
        // `UNIT_STATE_CHASE_MOVE`) and stops the spline.
        wow_movement::ChaseMovementAction::StopMoving
        | wow_movement::ChaseMovementAction::CannotReachTarget => {
            actor
                .creature
                .unit_mut()
                .clear_unit_state(UnitState::CHASE_MOVE.bits());
            actor.runtime.active_chase_path_poly_refs.clear();
            match actor.stop_move_spline_like_cpp() {
                Some(stop) => complete(ChaseTickOutcomeLikeCpp::Stopped(stop)),
                None => complete(ChaseTickOutcomeLikeCpp::Idle),
            }
        }
        wow_movement::ChaseMovementAction::StopMovingAndFaceInform(inform)
        | wow_movement::ChaseMovementAction::ClearChaseMoveAndFaceInform(inform) => {
            // C++ `SetInFront(target)` only turns the owner server-side, and
            // then reports arrival to the AI.
            actor
                .creature
                .unit_mut()
                .clear_unit_state(UnitState::CHASE_MOVE.bits());
            let mut position = actor.position();
            position.orientation = absolute_angle_like_cpp(position, target.position);
            actor.creature.set_ai_position(position);
            actor.creature.record_ai_movement_inform(
                inform.movement_type.trinity_id(),
                inform.target_counter,
            );
            actor.runtime.active_chase_path_poly_refs.clear();
            match actor.stop_move_spline_like_cpp() {
                Some(stop) => complete(ChaseTickOutcomeLikeCpp::Stopped(stop)),
                None => complete(ChaseTickOutcomeLikeCpp::Idle),
            }
        }
        wow_movement::ChaseMovementAction::Launch(plan) => {
            if plan.direction_changed {
                // C++ replaces the owned `PathGenerator` before
                // `CalculatePath` when chase direction flips
                // (`ChaseMovementGenerator.cpp:171-175`). The retained
                // Detour corridor belongs to that old path object.
                actor.runtime.active_chase_path_poly_refs.clear();
            }

            // C++ picks the target centre when closing in without an angle
            // constraint, otherwise a point on the tolerance ring
            // (`ChaseMovementGenerator.cpp:177-191`).
            let destination = if plan.move_toward && plan.desired_relative_angle.is_none() {
                target.position
            } else {
                let hitbox_sum =
                    actor.creature.unit().data().combat_reach.max(0.0) + target.combat_reach;
                let absolute_angle = match plan.desired_relative_angle {
                    Some(relative) => wow_movement::normalize_orientation_like_cpp(
                        target.position.orientation + relative,
                    ),
                    None => absolute_angle_like_cpp(target.position, actor.position()),
                };
                actor.near_point_like_cpp(
                    target,
                    plan.desired_distance - hitbox_sum,
                    absolute_angle,
                    None,
                )
            };

            let pending = Destination {
                plan,
                target,
                should_try_pathfinding,
                terrain_enabled,
            };
            if plan.move_toward && plan.desired_relative_angle.is_none() {
                destination_ready(actor, pending, destination)
            } else {
                terrain::normalize(
                    actor,
                    destination,
                    terrain_enabled,
                    TerrainPurpose::Chase(pending),
                )
            }
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
        target,
        should_try_pathfinding,
        terrain_enabled,
    } = pending;
    let launch = Launch {
        plan,
        target,
        destination,
    };
    // C++ `ChaseMovementGenerator::Update` calls
    // `CalculatePath(x, y, z, owner->CanFly())`
    // (`ChaseMovementGenerator.cpp:196`), and `_forceDestination` is
    // consumed *inside* `BuildPointPath` (`PathGenerator.cpp:603-619`)
    // — setting it afterwards on the Rust `PathGenerator` would
    // record the flag without rebuilding the clamped point path, so
    // it has to travel with the query itself.

    if should_try_pathfinding {
        // Built here, not by the caller: the victim-change reset
        // above may already have dropped the retained corridor, and
        // reusing the previous victim's `_pathPolyRefs` would let the
        // ~80% prefix branch steer the first spline back toward the
        // old target.
        let query = CreaturePathQueryLikeCpp {
            start: actor.position(),
            destination,
            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
            force_destination: plan.allow_flying_path,
            filter_context: actor.path_query_filter_context_like_cpp(),
            owner: actor.detour_owner_capabilities_like_cpp(),
            previous_poly_refs: actor.runtime.active_chase_path_poly_refs.clone(),
        };

        request_path(query, PathPurpose::Chase(launch), terrain_enabled)
    } else {
        resolved(actor, launch, None, false, terrain_enabled)
    }
}
pub(super) fn resolved(
    actor: &mut WorldCreature,
    launch: Launch,
    detour_path: Option<DetourPolyPath>,
    query_attempted: bool,
    enabled: bool,
) -> MovementProgress {
    let plan = launch.plan;
    let destination = launch.destination;
    // The resolver already answers a missing navmesh/tile with
    // the C++ `BuildShortcut()` path, so `None` here means the
    // query was attempted and genuinely failed. C++ has no such
    // case — its own failures went through `BuildShortcut()` +
    // `PATHFIND_NOPATH` — so it must not be confused with
    // "there is no navmesh", which is launchable.
    let query_failed = query_attempted && detour_path.is_none();
    if let Some(detour) = detour_path {
        return terrain::normalize_path(
            actor,
            destination,
            detour,
            plan.allow_flying_path,
            enabled,
            terrain::PathFinish::Chase(launch),
        );
    }
    let path = match query_failed {
        true => {
            // Reproduce the C++ `PATHFIND_NOPATH` branch so the
            // bail-out below stops and retries.
            let mut path = PathGenerator::new();
            path.apply_detour_path_like_cpp(
                actor.position(),
                destination,
                destination,
                [],
                &[],
                PathType::NOPATH,
                plan.allow_flying_path,
            );
            path
        }
        false => {
            // Pathfinding is off for this map/owner: C++
            // `CalculatePath` answers with `BuildShortcut()` and
            // `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`, which
            // chase launches (`PathGenerator.cpp:79-86`).
            let mut path = PathGenerator::new();
            path.calculate_without_navmesh_like_cpp(
                actor.position(),
                destination,
                plan.allow_flying_path,
            );
            path
        }
    };

    self::launch(actor, launch, path, None)
}
pub(super) fn launch(
    actor: &mut WorldCreature,
    launch: Launch,
    mut path: PathGenerator,
    detour_path: Option<DetourPolyPath>,
) -> MovementProgress {
    let Launch {
        plan,
        target,
        destination: _,
    } = launch;
    // C++ bails out only on `PATHFIND_NOPATH`; SHORTCUT, INCOMPLETE,
    // SHORT and FARFROMPOLY all proceed
    // (`ChaseMovementGenerator.cpp:197-203`).
    if path.path_type().contains(PathType::NOPATH) {
        if let Some(generator) = actor.runtime.active_chase_generator.as_mut() {
            generator.cannot_reach_target = true;
        }
        actor
            .creature
            .unit_mut()
            .clear_unit_state(UnitState::CHASE_MOVE.bits());
        actor.runtime.active_chase_path_poly_refs.clear();
        return match actor.stop_move_spline_like_cpp() {
            Some(stop) => complete(ChaseTickOutcomeLikeCpp::Stopped(stop)),
            None => complete(ChaseTickOutcomeLikeCpp::Idle),
        };
    }

    if plan.shorten_path {
        // C++ shortens against the target's exact position, using
        // line of sight from each candidate; VMap LOS is a stub, so
        // every candidate is treated as visible.
        path.shorten_path_until_dist_like_cpp(target.position, plan.desired_distance, |_| true);
    }

    if let Some(generator) = actor.runtime.active_chase_generator.as_mut() {
        // C++ clears `CannotReachTarget` after a successful
        // `CalculatePath` and enables the next arrival inform
        // immediately before launching the spline. A failed query
        // must preserve the previous inform lifecycle.
        generator.confirm_path_ready_like_cpp();
    }
    actor
        .creature
        .unit_mut()
        .add_unit_state(UnitState::CHASE_MOVE.bits());

    let points = path.path_points().to_vec();
    let Some(dst) = points.last().copied() else {
        return complete(ChaseTickOutcomeLikeCpp::Idle);
    };
    let spline_id = actor.spline_id().saturating_add(1);
    let mut init = MoveSplineInit::new(spline_id);
    init.set_walk(plan.walk);
    init.move_by_path(points, 0);
    // C++ `init.SetFacing(target)` is client-side target tracking.
    init.set_facing_target_with_angle(
        target.guid,
        absolute_angle_like_cpp(actor.position(), target.position),
    );

    match actor.launch_move_spline_init_like_cpp(&mut init, dst) {
        Some((from, spline)) => {
            if let Some(detour_path) = detour_path.as_ref() {
                actor
                    .runtime
                    .active_chase_path_poly_refs
                    .clone_from(&detour_path.poly_refs);
            } else {
                actor.runtime.active_chase_path_poly_refs.clear();
            }
            if let Some(generator) = actor.runtime.active_chase_generator.as_mut() {
                generator.confirm_launch_like_cpp(plan);
            }
            complete(ChaseTickOutcomeLikeCpp::Launched(from, spline))
        }
        None => complete(ChaseTickOutcomeLikeCpp::Idle),
    }
}
