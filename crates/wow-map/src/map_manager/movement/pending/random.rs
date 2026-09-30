//! Random draws and classification occur once, before normalization and launch.
use super::*;
pub(super) struct Query { diff_ms: u32, distance_roll: f32, angle_roll: f32, next_wander_steps_roll: u8, pause_seconds_roll: i32 }
pub(super) struct Launch { destination: Position }
fn complete(outcome: Option<(Position, MoveSpline)>) -> MovementProgress { MovementProgress::Complete(MovementCompletion::Random(outcome)) }
pub(in crate::map_manager::movement) fn prepare(actor: &mut WorldCreature, diff_ms: u32, should_try_pathfinding: bool, terrain_enabled: bool, update_spline: bool) -> MovementProgress {
        if actor.runtime.active_random_generator.is_none() {
            if !actor.initialize_default_random_movement_like_cpp() {
                if actor.state() == CreatureAiState::WalkingRandom && actor.movement_finished() {
                    actor.finish_move();
                    actor.creature.set_ai_state(CreatureAiState::Idle);
                }
                return complete(None);
            }
        }
        if update_spline {
            actor.update_move_spline_like_cpp();
        }

        let move_spline_finalized = actor
            .runtime.active_move_spline
            .as_ref()
            .is_none_or(MoveSpline::finalized);
        let should_set_location = actor
            .runtime.active_random_generator
            .as_ref()
            .is_some_and(|generator| generator.timer_ms().saturating_sub(diff_ms as i32) <= 0)
            && move_spline_finalized;

        let point_path_limit =
            point_path_limit_for_distance_like_cpp(RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP);
        let mut distance_roll = 0.0;
        let mut angle_roll = 0.0;
        let mut next_wander_steps_roll = 2;
        let mut pause_seconds_roll = 4;

        if should_set_location {
            distance_roll = actor.runtime.runtime_rng_like_cpp.gen_range(0.0..=1.0);
            angle_roll = actor.runtime.runtime_rng_like_cpp.gen_range(0.0..=1.0);
            next_wander_steps_roll = actor.runtime.runtime_rng_like_cpp.gen_range(2..=10);
            pause_seconds_roll = actor.runtime.runtime_rng_like_cpp.gen_range(4..=10);
            let reference = actor
                .runtime.active_random_generator
                .as_ref()
                .map(RandomMovementGenerator::reference)
                .unwrap_or_else(|| actor.position());
            let destination = compute_random_destination_like_cpp(
                reference,
                actor.creature.ai_ownership().wander_radius,
                distance_roll,
                angle_roll,
            )
            .destination;

            if should_try_pathfinding {
                // Built here so the retained corridor is read after a possible
                // generator (re)initialization dropped it, and the filter after
                // any state change this tick — C++ constructs its `PathGenerator`
                // at exactly this point.
                let query = CreaturePathQueryLikeCpp {
                    start: actor.position(),
                    destination,
                    point_path_limit,
                    force_destination: false,
                    filter_context: actor.path_query_filter_context_like_cpp(),
                    owner: actor.detour_owner_capabilities_like_cpp(),
                    previous_poly_refs: actor.runtime.active_random_path_poly_refs.clone(),
                };

                let pending = Query { diff_ms, distance_roll, angle_roll, next_wander_steps_roll, pause_seconds_roll };
                return request_path(query, PathPurpose::Random(pending), terrain_enabled);
            }
        }
        let pending = Query { diff_ms, distance_roll, angle_roll, next_wander_steps_roll, pause_seconds_roll };
        advance_generator(actor, pending, None, RandomPathResult::Success, terrain_enabled)
}
pub(super) fn resolved(actor: &mut WorldCreature, pending: Query, detour_path: Option<DetourPolyPath>, enabled: bool) -> MovementProgress {
    let path_result = if let Some(path) = detour_path.as_ref() {
                    let path_type = path_type_from_detour_like_cpp(path.point_path.path_type);
                    let path_result = random_path_result_from_path_type_like_cpp(path_type);
                    // The generator keeps its `PathGenerator` alive, so the
                    // corridor this query produced is the one the next one may
                    // reuse (`PathGenerator.cpp:291-413`).
                    actor.runtime.active_random_path_poly_refs
                        .clone_from(&path.poly_refs);
                    path_result
                } else {
                    RandomPathResult::Failed
                };
    advance_generator(actor, pending, detour_path, path_result, enabled)
}
fn advance_generator(actor: &mut WorldCreature, pending: Query, detour_path: Option<DetourPolyPath>, path_result: RandomPathResult, enabled: bool) -> MovementProgress {
    let Query { diff_ms, distance_roll, angle_roll, next_wander_steps_roll, pause_seconds_roll } = pending;
        let snapshot = actor.random_unit_snapshot_like_cpp(
            true,
            path_result,
            distance_roll,
            angle_roll,
            next_wander_steps_roll,
            pause_seconds_roll,
            0,
        );
        let action = match actor.runtime.active_random_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, diff_ms, snapshot),
            None => return complete(None),
        };
        match action {
            RandomMovementAction::StopMoving => {
                actor.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                actor.runtime.active_move_spline = None;
                complete(None)
            }

            RandomMovementAction::Launch(launch) => {
                actor.creature.unit_mut().add_unit_state(UnitState::ROAMING_MOVE.bits());
                let launch = Launch { destination: launch.destination };
                match detour_path {
                    Some(detour) => terrain::normalize_path(actor, launch.destination, detour, false, enabled, terrain::PathFinish::Random(launch)),
                    None => self::launch(actor, launch, None),
                }
            }
            RandomMovementAction::RetryAfterLosFailure { .. }
            | RandomMovementAction::RetryAfterPathFailure { .. }
            | RandomMovementAction::Continue
            | RandomMovementAction::Finished
            | RandomMovementAction::DurationFinished => complete(None),
        }
}
pub(super) fn launch(actor: &mut WorldCreature, launch: Launch, path: Option<PathGenerator>) -> MovementProgress {
    let movement = match path {
        Some(path) => actor.begin_random_move_spline_with_path(path).map(|(from, spline, _path)| (from, spline)),
        None => actor.begin_random_move_spline_like_cpp(launch.destination),
    };
    let Some(movement) = movement else { return complete(None); };
    actor.creature
        .set_ai_state(wow_entities::CreatureAiState::WalkingRandom);
    actor.creature.ai_ownership_mut().wander_steps_remaining = actor
        .runtime.active_random_generator
        .as_ref()
        .map(RandomMovementGenerator::wander_steps)
        .unwrap_or_default();
    if let Some(generator) = actor.runtime.active_random_generator.as_mut() {
        generator.adjust_launch_timer_for_actual_travel_time_like_cpp(
            0,
            movement.1.duration_ms(),
        );
    }
    complete(Some(movement))
}
