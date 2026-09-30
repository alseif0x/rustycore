//! Random and waypoint operations of movement.
//!
//! Legacy synchronous signatures drive the private consumable movement engine.

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
        self.runtime.active_waypoint_generator = Some(generator);
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
        self.runtime.active_move_spline = None;
        let next_wander_steps_roll = self.runtime.runtime_rng_like_cpp.gen_range(2..=10);
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
        self.runtime.active_random_generator = Some(generator);
        // C++ `RandomMovementGenerator<Creature>::DoInitialize` drops the
        // generator's `PathGenerator` (`RandomMovementGenerator.cpp:95`), so the
        // next query starts from an empty corridor.
        self.runtime.active_random_path_poly_refs.clear();
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

    pub fn update_default_random_movement_after_spline_like_cpp(
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
        let progress = pending::prepare_random(self, diff_ms, should_try_pathfinding, terrain.is_some(), update_spline);
        match pending::run(self, progress, terrain, &mut resolve_path) {
            pending::MovementCompletion::Random(outcome) => outcome,
            _ => unreachable!("random preparation completes a random operation"),
        }
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

    pub fn update_default_waypoint_movement_after_spline_like_cpp(
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
        let progress = pending::prepare_waypoint(self, diff_ms, wait_time_roll_ms, should_try_pathfinding, terrain.is_some(), update_spline);
        match pending::run(self, progress, terrain, &mut resolve_path) {
            pending::MovementCompletion::Waypoint(action, movement) => (action, movement),
            _ => unreachable!("waypoint preparation completes a waypoint operation"),
        }
    }

    pub(super) fn waypoint_unit_snapshot_like_cpp(&self) -> WaypointUnitSnapshot {
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

    pub(super) fn begin_waypoint_random_at_path_end_like_cpp(
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
            .runtime.runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = self.creature.ai_ownership().wander_radius.max(0.0);
        let dist = self.runtime.runtime_rng_like_cpp.gen_range(0.0..=radius);
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
            .runtime.runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = wander_distance.max(0.0);
        let dist = self.runtime.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let reference = self.position();
        let x = reference.x + angle.cos() * dist;
        let y = reference.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Some(Position::new(x, y, reference.z, o))
    }

    pub fn reset_wander_timer(&mut self) -> bool {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let wander_delay_ms = self.runtime.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        true
    }

    pub fn initialize_random_wander_steps_like_cpp(&mut self) -> bool {
        let wander_steps_remaining = self.runtime.runtime_rng_like_cpp.gen_range(2..=10);
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
        let wander_delay_ms = self.runtime.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let wander_steps_remaining = self.runtime.runtime_rng_like_cpp.gen_range(2..=10);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        ai.wander_steps_remaining = wander_steps_remaining;
        true
    }
}
