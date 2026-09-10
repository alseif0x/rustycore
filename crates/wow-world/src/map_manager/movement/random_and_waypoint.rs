//! Random and waypoint operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl WorldCreature {
    pub fn can_wander(&self) -> bool {
        self.creature.can_ai_wander()
    }

    pub fn initialize_default_waypoint_movement_like_cpp(
        &mut self,
        loaded_path: Option<WaypointPath>,
    ) -> WaypointMovementAction {
        self.creature
            .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Waypoint);
        let mut generator = WaypointMovementGenerator::from_db_path_id(0, true);
        let action = generator.initialize_like_cpp(
            true,
            self.creature.waypoint_path_id_like_cpp(),
            loaded_path,
        );
        if action == WaypointMovementAction::StopMoving {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .stop_moving();
            self.creature
                .set_ai_state(wow_entities::CreatureAiState::WalkingWaypoint);
        }
        self.active_waypoint_generator = Some(generator);
        action
    }

    pub fn initialize_default_waypoint_movement_with_path_resolver_like_cpp(
        &mut self,
        mut resolve_path: impl FnMut(u32) -> Option<WaypointPath>,
    ) -> WaypointMovementAction {
        let owner_path_id = self.creature.waypoint_path_id_like_cpp();
        let loaded_path = (owner_path_id != 0)
            .then(|| resolve_path(owner_path_id))
            .flatten();
        self.initialize_default_waypoint_movement_like_cpp(loaded_path)
    }

    pub fn initialize_default_random_movement_like_cpp(&mut self) -> bool {
        if self.creature.default_movement_type() != wow_entities::MovementGeneratorType::Random
            || !self.is_alive()
            || self.creature.ai_ownership().wander_radius <= 0.0
        {
            return false;
        }

        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .stop_moving();
        self.active_move_spline = None;
        let next_wander_steps_roll = self.runtime_rng_like_cpp.gen_range(2..=10);
        let snapshot = self.random_unit_snapshot_like_cpp(
            true,
            RandomPathResult::Success,
            0.0,
            0.0,
            next_wander_steps_roll,
            4,
            0,
        );
        let mut generator = RandomMovementGenerator::new(0.0, None);
        let _ = generator.initialize_like_cpp(true, snapshot);
        self.active_random_generator = Some(generator);
        // C++ `RandomMovementGenerator<Creature>::DoInitialize` drops the
        // generator's `PathGenerator` (`RandomMovementGenerator.cpp:95`), so the
        // next query starts from an empty corridor.
        self.active_random_path_poly_refs.clear();
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = None;
        ai.move_start_ms = now_ms;
        ai.move_duration_ms = 0;
        ai.wander_delay_ms = 0;
        ai.wander_steps_remaining = next_wander_steps_roll;
        ai.state = CreatureAiState::Idle;
        true
    }

    pub fn update_default_random_movement_with_path_resolver_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        self.update_default_random_movement_with_path_resolver_and_terrain_like_cpp(
            diff_ms,
            should_try_pathfinding,
            None,
            resolve_path,
        )
    }

    pub fn update_default_random_movement_with_path_resolver_and_terrain_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        self.update_default_random_movement_after_optional_spline_like_cpp(
            diff_ms,
            should_try_pathfinding,
            terrain,
            true,
            resolve_path,
        )
    }

    pub(crate) fn update_default_random_movement_after_spline_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        self.update_default_random_movement_after_optional_spline_like_cpp(
            diff_ms,
            should_try_pathfinding,
            terrain,
            false,
            resolve_path,
        )
    }

    fn update_default_random_movement_after_optional_spline_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        update_spline: bool,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        if self.active_random_generator.is_none() {
            if !self.initialize_default_random_movement_like_cpp() {
                if self.state() == CreatureAiState::WalkingRandom && self.movement_finished() {
                    self.finish_move();
                    self.creature.set_ai_state(CreatureAiState::Idle);
                }
                return None;
            }
        }
        if update_spline {
            self.update_move_spline_like_cpp();
        }

        let move_spline_finalized = self
            .active_move_spline
            .as_ref()
            .is_none_or(MoveSpline::finalized);
        let should_set_location = self
            .active_random_generator
            .as_ref()
            .is_some_and(|generator| generator.timer_ms().saturating_sub(diff_ms as i32) <= 0)
            && move_spline_finalized;

        let point_path_limit =
            point_path_limit_for_distance_like_cpp(RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP);
        let mut detour_path = None;
        let mut path_result = RandomPathResult::Success;
        let mut distance_roll = 0.0;
        let mut angle_roll = 0.0;
        let mut next_wander_steps_roll = 2;
        let mut pause_seconds_roll = 4;

        if should_set_location {
            distance_roll = self.runtime_rng_like_cpp.gen_range(0.0..=1.0);
            angle_roll = self.runtime_rng_like_cpp.gen_range(0.0..=1.0);
            next_wander_steps_roll = self.runtime_rng_like_cpp.gen_range(2..=10);
            pause_seconds_roll = self.runtime_rng_like_cpp.gen_range(4..=10);
            let reference = self
                .active_random_generator
                .as_ref()
                .map(RandomMovementGenerator::reference)
                .unwrap_or_else(|| self.position());
            let destination = compute_random_destination_like_cpp(
                reference,
                self.creature.ai_ownership().wander_radius,
                distance_roll,
                angle_roll,
            )
            .destination;
            if should_try_pathfinding {
                // Built here so the retained corridor is read after a possible
                // generator (re)initialization dropped it, and the filter after
                // any state change this tick — C++ constructs its `PathGenerator`
                // at exactly this point.
                detour_path = resolve_path(CreaturePathQueryLikeCpp {
                    start: self.position(),
                    destination,
                    point_path_limit,
                    force_destination: false,
                    filter_context: self.path_query_filter_context_like_cpp(),
                    owner: self.detour_owner_capabilities_like_cpp(),
                    previous_poly_refs: self.active_random_path_poly_refs.clone(),
                });
                if let Some(path) = detour_path.as_ref() {
                    let path_type = path_type_from_detour_like_cpp(path.point_path.path_type);
                    path_result = random_path_result_from_path_type_like_cpp(path_type);
                    // The generator keeps its `PathGenerator` alive, so the
                    // corridor this query produced is the one the next one may
                    // reuse (`PathGenerator.cpp:291-413`).
                    self.active_random_path_poly_refs
                        .clone_from(&path.poly_refs);
                } else {
                    path_result = RandomPathResult::Failed;
                }
            }
        }

        let snapshot = self.random_unit_snapshot_like_cpp(
            true,
            path_result,
            distance_roll,
            angle_roll,
            next_wander_steps_roll,
            pause_seconds_roll,
            0,
        );
        let action = match self.active_random_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, diff_ms, snapshot),
            None => return None,
        };
        self.apply_random_movement_action_with_terrain_like_cpp(
            action,
            detour_path.as_ref(),
            0,
            terrain,
        )
    }

    pub fn update_default_waypoint_movement_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> WaypointMovementAction {
        self.update_default_waypoint_movement_with_launch_like_cpp(diff_ms)
            .0
    }

    pub fn update_default_waypoint_movement_with_path_resolver_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_path_resolver_and_terrain_like_cpp(
            diff_ms,
            should_try_pathfinding,
            None,
            resolve_path,
        )
    }

    pub fn update_default_waypoint_movement_with_path_resolver_and_terrain_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            should_try_pathfinding,
            terrain,
            true,
            resolve_path,
        )
    }

    pub(crate) fn update_default_waypoint_movement_after_spline_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            should_try_pathfinding,
            terrain,
            false,
            resolve_path,
        )
    }

    fn apply_random_movement_action_with_terrain_like_cpp(
        &mut self,
        action: RandomMovementAction,
        detour_path: Option<&DetourPolyPath>,
        planned_travel_time_ms: i32,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline)> {
        match action {
            RandomMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                self.active_move_spline = None;
                None
            }
            RandomMovementAction::Launch(launch) => {
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::ROAMING_MOVE.bits());
                let movement = self
                    .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
                        launch.destination,
                        detour_path,
                        false,
                        terrain,
                    )
                    .map(|(from, spline, _path)| (from, spline))?;
                self.creature
                    .set_ai_state(wow_entities::CreatureAiState::WalkingRandom);
                self.creature.ai_ownership_mut().wander_steps_remaining = self
                    .active_random_generator
                    .as_ref()
                    .map(RandomMovementGenerator::wander_steps)
                    .unwrap_or_default();
                if let Some(generator) = self.active_random_generator.as_mut() {
                    generator.adjust_launch_timer_for_actual_travel_time_like_cpp(
                        planned_travel_time_ms,
                        movement.1.duration_ms(),
                    );
                }
                Some(movement)
            }
            RandomMovementAction::RetryAfterLosFailure { .. }
            | RandomMovementAction::RetryAfterPathFailure { .. }
            | RandomMovementAction::Continue
            | RandomMovementAction::Finished
            | RandomMovementAction::DurationFinished => None,
        }
    }

    pub fn update_default_waypoint_movement_with_launch_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            false,
            None,
            true,
            |_| None,
        )
    }

    pub fn update_default_waypoint_movement_with_wait_roll_like_cpp(
        &mut self,
        diff_ms: u32,
        wait_time_roll_ms: Option<i32>,
    ) -> WaypointMovementAction {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            wait_time_roll_ms,
            false,
            None,
            true,
            |_| None,
        )
        .0
    }

    fn update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
        &mut self,
        diff_ms: u32,
        wait_time_roll_ms: Option<i32>,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        update_spline: bool,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        if let Some(mut random) = self.active_waypoint_random_at_path_end {
            if update_spline {
                let _ = self.update_move_spline_like_cpp();
            }
            random.duration_ms = random.duration_ms.saturating_sub(diff_ms as i32);
            if random.duration_ms > 0 {
                self.active_waypoint_random_at_path_end = Some(random);
            } else {
                self.active_waypoint_random_at_path_end = None;
            }
            return (WaypointMovementAction::Continue, None);
        }

        // C++ `Unit::Update` advances `UpdateSplineMovement` before
        // `MotionMaster::Update`, so waypoint generators observe an arrived
        // `movespline` in the same tick and can launch the next segment.
        if update_spline {
            let _ = self.update_move_spline_like_cpp();
        }

        let snapshot = self.waypoint_unit_snapshot_like_cpp();
        let Some(generator) = self.active_waypoint_generator.as_mut() else {
            return (WaypointMovementAction::Continue, None);
        };
        let action = generator.update_like_cpp(true, diff_ms, snapshot, wait_time_roll_ms);
        let launch_result = self.apply_waypoint_movement_action_with_path_resolver_like_cpp(
            action,
            should_try_pathfinding,
            terrain,
            &mut resolve_path,
        );
        if matches!(
            action,
            WaypointMovementAction::Arrived(arrived)
                if arrived.timer_ms.is_none() && arrived.move_random_at_path_end.is_none()
        ) {
            let snapshot = self.waypoint_unit_snapshot_like_cpp();
            if let Some(generator) = self.active_waypoint_generator.as_mut() {
                let chained = generator.update_like_cpp(true, 0, snapshot, None);
                if chained != WaypointMovementAction::Continue {
                    let chained_launch = self
                        .apply_waypoint_movement_action_with_path_resolver_like_cpp(
                            chained,
                            should_try_pathfinding,
                            terrain,
                            &mut resolve_path,
                        );
                    return (chained, chained_launch);
                }
            }
        }
        (action, launch_result)
    }

    fn apply_waypoint_movement_action_with_path_resolver_like_cpp(
        &mut self,
        action: WaypointMovementAction,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: &mut impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        match action {
            WaypointMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                None
            }
            WaypointMovementAction::Arrived(arrived) => {
                if arrived.clear_roaming_move {
                    self.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                }
                self.creature.record_ai_movement_inform(
                    arrived.inform.movement_type.trinity_id(),
                    arrived.inform.node_id,
                );
                if let Some(random) = arrived.move_random_at_path_end {
                    let launch_result = self.begin_waypoint_random_at_path_end_like_cpp(random);
                    self.active_waypoint_random_at_path_end = Some(random);
                    launch_result
                } else {
                    None
                }
            }
            WaypointMovementAction::PathEnded(ended) => {
                let home = self
                    .creature
                    .ai_ownership()
                    .move_target
                    .unwrap_or_else(|| self.position());
                self.creature.set_ai_home_position(home);
                self.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                self.creature
                    .set_ai_state(wow_entities::CreatureAiState::Idle);
                let _ = ended;
                None
            }
            WaypointMovementAction::Launch(launch) => {
                let detour_path = (launch.generate_path && should_try_pathfinding)
                    .then(|| {
                        resolve_path(CreaturePathQueryLikeCpp {
                            start: self.position(),
                            destination: launch.destination,
                            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                            force_destination: false,
                            filter_context: self.path_query_filter_context_like_cpp(),
                            owner: self.detour_owner_capabilities_like_cpp(),
                            // C++ `MoveSplineInit::MoveTo` builds a fresh
                            // `PathGenerator` per waypoint leg.
                            previous_poly_refs: Vec::new(),
                        })
                    })
                    .flatten();
                self.begin_waypoint_launch_with_detour_path_like_cpp(
                    launch,
                    detour_path.as_ref(),
                    terrain,
                )
            }
            _ => None,
        }
    }

    fn waypoint_unit_snapshot_like_cpp(&self) -> WaypointUnitSnapshot {
        let unit = self.creature.unit();
        WaypointUnitSnapshot {
            owner_alive: self.creature.is_alive(),
            owner_unit_state: unit.unit_state(),
            movement_prevented_by_casting: unit.has_unit_state(UnitState::CASTING.bits()),
            move_spline_finalized: unit.subsystems().motion.spline.finalized,
            owner_is_on_transport: false,
            owner_is_formation_leader: false,
            formation_leader_move_allowed: true,
            owner_orientation: self.position().orientation,
            owner_position: self.position(),
            ai_enabled: true,
        }
    }

    fn begin_waypoint_launch_with_detour_path_like_cpp(
        &mut self,
        launch: WaypointLaunchPlan,
        detour_path: Option<&DetourPolyPath>,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        if launch.disable_transport_transform {
            init.disable_transport_path_transformations();
        }
        let path = detour_path
            .map(|detour_path| {
                self.path_generator_from_detour_for_creature_like_cpp(
                    launch.destination,
                    detour_path,
                    false,
                    terrain,
                )
            })
            .filter(|path| !path.path_type().contains(PathType::NOPATH));
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
        self.creature
            .unit_mut()
            .add_unit_state(launch.add_unit_state);
        self.launch_move_spline_init_like_cpp(&mut init, launch.destination)
    }

    fn begin_waypoint_random_at_path_end_like_cpp(
        &mut self,
        random: WaypointRandomAtPathEnd,
    ) -> Option<(Position, MoveSpline)> {
        let dst =
            self.pick_random_destination_from_current_position_like_cpp(random.wander_distance)?;
        self.begin_move_spline_like_cpp(dst)
    }

    pub fn should_wander(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::Idle
            && self.creature.default_movement_type() == wow_entities::MovementGeneratorType::Random
            && self.can_wander()
            && self.creature.ai_ownership().wander_radius > 0.0
            && self
                .runtime_elapsed_ms_like_cpp()
                .saturating_sub(self.creature.ai_ownership().move_start_ms)
                >= self.creature.ai_ownership().wander_delay_ms
    }

    pub fn pick_wander_destination(&mut self) -> Option<Position> {
        let angle = self
            .runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = self.creature.ai_ownership().wander_radius.max(0.0);
        let dist = self.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let home = self.home_position();
        let x = home.x + angle.cos() * dist;
        let y = home.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Some(Position::new(x, y, home.z, o))
    }

    pub fn pick_random_destination_from_current_position_like_cpp(
        &mut self,
        wander_distance: f32,
    ) -> Option<Position> {
        let angle = self
            .runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = wander_distance.max(0.0);
        let dist = self.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let reference = self.position();
        let x = reference.x + angle.cos() * dist;
        let y = reference.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Some(Position::new(x, y, reference.z, o))
    }

    pub fn reset_wander_timer(&mut self) -> bool {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let wander_delay_ms = self.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        true
    }

    pub fn initialize_random_wander_steps_like_cpp(&mut self) -> bool {
        let wander_steps_remaining = self.runtime_rng_like_cpp.gen_range(2..=10);
        self.creature.ai_ownership_mut().wander_steps_remaining = wander_steps_remaining;
        true
    }

    pub fn record_random_movement_launch_like_cpp(&mut self) -> bool {
        if self.creature.ai_ownership().wander_steps_remaining == 0 {
            if !self.initialize_random_wander_steps_like_cpp() {
                return false;
            }
        }
        let ai = self.creature.ai_ownership_mut();
        ai.wander_steps_remaining = ai.wander_steps_remaining.saturating_sub(1);
        ai.state = CreatureAiState::WalkingRandom;
        true
    }

    pub fn schedule_after_random_movement_like_cpp(&mut self) -> bool {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        if self.creature.ai_ownership().wander_steps_remaining > 0 {
            let ai = self.creature.ai_ownership_mut();
            ai.move_start_ms = now_ms;
            ai.wander_delay_ms = 0;
            return true;
        }
        let wander_delay_ms = self.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let wander_steps_remaining = self.runtime_rng_like_cpp.gen_range(2..=10);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        ai.wander_steps_remaining = wander_steps_remaining;
        true
    }
}
