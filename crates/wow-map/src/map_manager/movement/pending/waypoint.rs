//! Waypoint frame, zero-diff arrival handoff, path query and launch continuation.
use super::*;
pub(super) struct Query { action: WaypointMovementAction, launch: WaypointLaunchPlan }
pub(super) struct Launch { action: WaypointMovementAction, launch: WaypointLaunchPlan, init: MoveSplineInit }
fn complete(action: WaypointMovementAction, movement: Option<(Position, MoveSpline)>) -> MovementProgress { MovementProgress::Complete(MovementCompletion::Waypoint(action, movement)) }
pub(in crate::map_manager::movement) fn prepare(actor: &mut WorldCreature, diff_ms: u32, wait_time_roll_ms: Option<i32>, should_try_pathfinding: bool, terrain_enabled: bool, update_spline: bool) -> MovementProgress {
        if let Some(mut random) = actor.runtime.active_waypoint_random_at_path_end {
            if update_spline {
                let _ = actor.update_move_spline_like_cpp();
            }
            random.duration_ms = random.duration_ms.saturating_sub(diff_ms as i32);
            if random.duration_ms > 0 {
                actor.runtime.active_waypoint_random_at_path_end = Some(random);
            } else {
                actor.runtime.active_waypoint_random_at_path_end = None;
            }
            return complete(WaypointMovementAction::Continue, None);
        }

        // C++ `Unit::Update` advances `UpdateSplineMovement` before
        // `MotionMaster::Update`, so waypoint generators observe an arrived
        // `movespline` in the same tick and can launch the next segment.
        if update_spline {
            let _ = actor.update_move_spline_like_cpp();
        }

        let snapshot = actor.waypoint_unit_snapshot_like_cpp();
        let Some(generator) = actor.runtime.active_waypoint_generator.as_mut() else {
            return complete(WaypointMovementAction::Continue, None);
        };
        let action = generator.update_like_cpp(true, diff_ms, snapshot, wait_time_roll_ms);

        apply_action(actor, action, should_try_pathfinding, terrain_enabled, true)
}
fn apply_action(actor: &mut WorldCreature, action: WaypointMovementAction, should_try_pathfinding: bool, terrain_enabled: bool, allow_chain: bool) -> MovementProgress {
    let movement =
        match action {
            WaypointMovementAction::StopMoving => {
                actor.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                None
            }
            WaypointMovementAction::Arrived(arrived) => {
                if arrived.clear_roaming_move {
                    actor.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                }
                actor.creature.record_ai_movement_inform(
                    arrived.inform.movement_type.trinity_id(),
                    arrived.inform.node_id,
                );
                if let Some(random) = arrived.move_random_at_path_end {
                    let launch_result = actor.begin_waypoint_random_at_path_end_like_cpp(random);
                    actor.runtime.active_waypoint_random_at_path_end = Some(random);
                    launch_result
                } else {
                    None
                }
            }
            WaypointMovementAction::PathEnded(ended) => {
                let home = actor
                    .creature
                    .ai_ownership()
                    .move_target
                    .unwrap_or_else(|| actor.position());
                actor.creature.set_ai_home_position(home);
                actor.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                actor.creature
                    .set_ai_state(wow_entities::CreatureAiState::Idle);
                let _ = ended;
                None
            }

            WaypointMovementAction::Launch(launch) => {
                if launch.generate_path && should_try_pathfinding {
                let query = CreaturePathQueryLikeCpp {
                            start: actor.position(),
                            destination: launch.destination,
                            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                            force_destination: false,
                            filter_context: actor.path_query_filter_context_like_cpp(),
                            owner: actor.detour_owner_capabilities_like_cpp(),
                            // C++ `MoveSplineInit::MoveTo` builds a fresh
                            // `PathGenerator` per waypoint leg.
                            previous_poly_refs: Vec::new(),
                };

                    return request_path(query, PathPurpose::Waypoint(Query { action, launch }), terrain_enabled);
                }
                return resolved(actor, Query { action, launch }, None, terrain_enabled);
            }
            _ => None,
        };
    if allow_chain && matches!(action, WaypointMovementAction::Arrived(arrived) if arrived.timer_ms.is_none() && arrived.move_random_at_path_end.is_none()) {
        let snapshot = actor.waypoint_unit_snapshot_like_cpp();
        if let Some(generator) = actor.runtime.active_waypoint_generator.as_mut() {
            let chained = generator.update_like_cpp(true, 0, snapshot, None);
            if chained != WaypointMovementAction::Continue {
                return apply_action(actor, chained, should_try_pathfinding, terrain_enabled, false);
            }
        }
    }
    complete(action, movement)
}
pub(super) fn resolved(actor: &mut WorldCreature, query: Query, detour: Option<DetourPolyPath>, terrain_enabled: bool) -> MovementProgress {
    let Query { action, launch } = query;
        let spline_id = actor.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        if launch.disable_transport_transform {
            init.disable_transport_path_transformations();
        }

    let pending = Launch { action, launch, init };
    match detour {
        Some(detour) => terrain::normalize_path(actor, launch.destination, detour, false, terrain_enabled, terrain::PathFinish::Waypoint(pending)),
        None => self::launch(actor, pending, None),
    }
}
pub(super) fn launch(actor: &mut WorldCreature, pending: Launch, path: Option<PathGenerator>) -> MovementProgress {
    let Launch { action, launch, mut init } = pending;
    let path = path.filter(|path| !path.path_type().contains(PathType::NOPATH));
        if let Some(path) = path {
            init.move_by_path(path.path_points().to_vec(), 0);
        } else {
            init.move_to(launch.destination);
        }
        if let Some(facing) = launch.facing {
            init.set_facing_angle(facing);
        }
        if let Some(walk) = launch.walk {
            init.set_walk(walk);
        }
        if let Some(velocity) = launch.velocity {
            init.set_velocity(velocity);
        }
        if let Some(animation) = launch.animation {
            match animation {
                WaypointAnimation::Ground => init.set_animation(0, 0, 0),
                WaypointAnimation::Hover => init.set_animation(2, 0, 0),
            }
        }
        actor.creature
            .unit_mut()
            .add_unit_state(launch.add_unit_state);
        let movement = actor.launch_move_spline_init_like_cpp(&mut init, launch.destination);
        complete(action, movement)
}
