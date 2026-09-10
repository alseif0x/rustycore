//! Spline operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl WorldCreature {
    fn set_movement_flags_like_cpp(&mut self, movement_flags: MovementFlag) {
        self.creature
            .set_movement_flags_runtime_like_cpp(movement_flags);
        self.create_data.movement_flags = movement_flags.bits();
    }

    fn apply_launch_movement_flags_like_cpp(&mut self, movement_flags: MovementFlag) {
        // C++ `MoveSplineInit::Launch` writes Unit::m_movementInfo before
        // initializing/sending the spline. Keep the create bridge in lockstep.
        self.set_movement_flags_like_cpp(movement_flags);
    }

    fn disable_spline_movement_like_cpp(&mut self) {
        // C++ `Unit::DisableSpline` and `MoveSplineInit::Stop` remove FORWARD.
        let mut movement_flags = self.creature.movement_flags_like_cpp();
        movement_flags.remove(MovementFlag::FORWARD);
        self.set_movement_flags_like_cpp(movement_flags);
    }

    pub fn movement_finished(&self) -> bool {
        if let Some(spline) = &self.active_move_spline {
            return spline.finalized();
        }
        self.creature
            .ai_ownership()
            .move_target
            .map(|_| {
                self.runtime_elapsed_ms_like_cpp()
                    .saturating_sub(self.creature.ai_ownership().move_start_ms)
                    >= u64::from(self.creature.ai_ownership().move_duration_ms)
            })
            .unwrap_or(true)
    }

    pub fn interpolated_position(&self) -> Position {
        let Some(dst) = self.creature.ai_ownership().move_target else {
            return self.position();
        };
        let elapsed =
            self.runtime_elapsed_ms_like_cpp()
                .saturating_sub(self.creature.ai_ownership().move_start_ms) as f32;
        let total = self.creature.ai_ownership().move_duration_ms as f32;
        if total <= 0.0 {
            return dst;
        }
        let src = self.position();
        let t = (elapsed / total).min(1.0);
        Position::new(
            src.x + (dst.x - src.x) * t,
            src.y + (dst.y - src.y) * t,
            src.z + (dst.z - src.z) * t,
            dst.orientation,
        )
    }

    pub fn begin_move(&mut self, dst: Position) {
        let dist = self.position().distance(&dst);
        let walk_speed = 2.5f32;
        let duration_ms = ((dist / walk_speed) * 1000.0) as u32;
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = Some(dst);
        ai.move_start_ms = now_ms;
        ai.move_duration_ms = duration_ms.max(500);
        ai.spline_id = ai.spline_id.saturating_add(1);
    }

    pub(super) fn launch_move_spline_init_like_cpp(
        &mut self,
        init: &mut MoveSplineInit,
        dst: Position,
    ) -> Option<(Position, MoveSpline)> {
        // C++ `MoveSplineInit::MoveSplineInit(Unit*)` snapshots `CanSwim()` into
        // every new spline before the generator customizes it
        // (`MoveSplineInit.cpp:198-207`).
        if self.creature.can_swim_like_cpp() {
            init.args.flags.insert(MoveSplineFlag::CAN_SWIM);
        } else {
            init.args.flags.remove(MoveSplineFlag::CAN_SWIM);
        }
        let spline_id = init.args.spline_id;
        let active_spline_position = self
            .active_move_spline
            .as_ref()
            .filter(|spline| !spline.finalized() && !spline.on_transport)
            .and_then(MoveSpline::compute_position);

        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let mut spline = self
            .active_move_spline
            .take()
            .unwrap_or_else(MoveSpline::new);
        let launch = init
            .launch(
                &mut spline,
                MoveSplineLaunchInput {
                    current_position: self.position(),
                    active_spline_position,
                    movement_flags: self.creature.movement_flags_like_cpp(),
                    selected_speed: if init.args.walk {
                        self.walk_speed_like_cpp()
                    } else {
                        self.run_speed_like_cpp()
                    },
                    run_speed: self.run_speed_like_cpp(),
                    assistance_speed_factor: 1.0,
                    on_transport: false,
                },
            )
            .ok()?;
        let duration_ms = launch.duration_ms.max(1) as u32;
        {
            let ai = self.creature.ai_ownership_mut();
            ai.move_target = Some(dst);
            ai.move_start_ms = now_ms;
            ai.move_duration_ms = duration_ms;
            ai.spline_id = spline_id;
        }
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .launch_spline(
                spline_id,
                duration_ms,
                position_to_i32_tuple(dst),
                false,
                false,
                None,
            );
        self.creature
            .unit_mut()
            .add_unit_state(UnitState::ROAMING_MOVE.bits());
        self.apply_launch_movement_flags_like_cpp(launch.movement_flags);
        self.active_move_spline = Some(spline.clone());
        Some((launch.real_position, spline))
    }

    pub fn begin_move_spline_like_cpp(&mut self, dst: Position) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_to(dst);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn begin_random_move_spline_like_cpp(
        &mut self,
        dst: Position,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_walk(self.random_movement_walk_like_cpp());
        init.move_to(dst);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn begin_random_move_spline_by_path_like_cpp<I>(
        &mut self,
        path: I,
    ) -> Option<(Position, MoveSpline)>
    where
        I: IntoIterator<Item = Position>,
    {
        let points = path.into_iter().collect::<Vec<_>>();
        let dst = points.last().copied()?;
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_walk(self.random_movement_walk_like_cpp());
        init.move_by_path(points, 0);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn random_movement_walk_like_cpp(&self) -> bool {
        match self.creature.random_movement_type_like_cpp() {
            value if value == ConstantsCreatureRandomMovementType::CanRun as u8 => self
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
            value if value == ConstantsCreatureRandomMovementType::AlwaysRun as u8 => false,
            _ => true,
        }
    }

    pub fn begin_move_spline_by_path_like_cpp<I>(
        &mut self,
        path: I,
    ) -> Option<(Position, MoveSpline)>
    where
        I: IntoIterator<Item = Position>,
    {
        let points = path.into_iter().collect::<Vec<_>>();
        let dst = points.last().copied()?;
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_by_path(points, 0);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn begin_move_spline_with_detour_path_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        self.begin_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            detour_path,
            force_destination,
            None,
        )
    }

    pub fn begin_move_spline_with_detour_path_and_terrain_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        let Some(detour_path) = detour_path else {
            return self
                .begin_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, None));
        };

        let path = self.path_generator_from_detour_for_creature_like_cpp(
            dst,
            detour_path,
            force_destination,
            terrain,
        );
        if path.path_type().contains(PathType::NOPATH) {
            return self
                .begin_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, Some(path)));
        }

        let points = path.path_points().to_vec();
        self.begin_move_spline_by_path_like_cpp(points)
            .map(|(from, spline)| (from, spline, Some(path)))
    }

    pub fn begin_random_move_spline_with_detour_path_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        self.begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            detour_path,
            force_destination,
            None,
        )
    }

    pub fn begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        let Some(detour_path) = detour_path else {
            return self
                .begin_random_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, None));
        };

        let path = self.path_generator_from_detour_for_creature_like_cpp(
            dst,
            detour_path,
            force_destination,
            terrain,
        );
        if path
            .path_type()
            .intersects(PathType::NOPATH | PathType::SHORTCUT)
        {
            return None;
        }

        let points = path.path_points().to_vec();
        self.begin_random_move_spline_by_path_like_cpp(points)
            .map(|(from, spline)| (from, spline, Some(path)))
    }

    pub fn begin_facing_spline_like_cpp(
        &mut self,
        facing_angle: f32,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let current = self.position();
        let active_spline_position = self
            .active_move_spline
            .as_ref()
            .filter(|spline| !spline.finalized() && !spline.on_transport)
            .and_then(MoveSpline::compute_position);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_to(current);
        init.set_facing_angle(facing_angle);

        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let mut spline = self
            .active_move_spline
            .take()
            .unwrap_or_else(MoveSpline::new);
        let launch = init
            .launch(
                &mut spline,
                MoveSplineLaunchInput {
                    current_position: current,
                    active_spline_position,
                    movement_flags: MovementFlag::NONE,
                    selected_speed: 2.5,
                    run_speed: 2.5,
                    assistance_speed_factor: 1.0,
                    on_transport: false,
                },
            )
            .ok()?;
        let duration_ms = launch.duration_ms.max(1) as u32;
        {
            let ai = self.creature.ai_ownership_mut();
            ai.move_target = Some(current);
            ai.move_start_ms = now_ms;
            ai.move_duration_ms = duration_ms;
            ai.spline_id = spline_id;
        }
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .launch_spline(
                spline_id,
                duration_ms,
                position_to_i32_tuple(current),
                false,
                false,
                None,
            );
        self.apply_launch_movement_flags_like_cpp(launch.movement_flags);
        self.active_move_spline = Some(spline.clone());
        Some((launch.real_position, spline))
    }

    pub fn update_move_spline_like_cpp(&mut self) -> bool {
        let Some(mut spline) = self.active_move_spline.take() else {
            return self.movement_finished();
        };

        if !spline.finalized() {
            let elapsed_ms = self
                .runtime_elapsed_ms_like_cpp()
                .saturating_sub(self.creature.ai_ownership().move_start_ms)
                .min(i32::MAX as u64) as i32;
            let diff_ms = elapsed_ms.saturating_sub(spline.time_passed_ms());
            if diff_ms > 0 {
                spline.update_state(diff_ms);
            }
            let progress_ms = spline.time_passed_ms().max(0) as u32;
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .set_spline_progress(progress_ms);
        }

        if let Some(pos) = spline.compute_position() {
            self.creature.set_ai_position(pos);
        }

        let finalized = spline.finalized();
        if finalized {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .finalize_spline();
            self.disable_spline_movement_like_cpp();
            self.creature
                .unit_mut()
                .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        } else {
            self.active_move_spline = Some(spline);
        }
        finalized
    }

    pub fn stop_move_spline_like_cpp(&mut self) -> Option<MoveSplineStopResult> {
        let mut spline = self.active_move_spline.take()?;
        if spline.finalized() {
            return None;
        }

        let elapsed_ms = self
            .runtime_elapsed_ms_like_cpp()
            .saturating_sub(self.creature.ai_ownership().move_start_ms)
            .min(i32::MAX as u64) as i32;
        let diff_ms = elapsed_ms.saturating_sub(spline.time_passed_ms());
        if diff_ms > 0 {
            spline.update_state(diff_ms);
        }
        if spline.finalized() {
            return None;
        }

        let stop_position = spline.compute_position().unwrap_or_else(|| self.position());
        let mut init = MoveSplineInit::new(self.spline_id().saturating_add(1));
        let stop = init.stop(
            &mut spline,
            MoveSplineStopInput {
                current_position: self.position(),
                active_spline_position: Some(stop_position),
                on_transport: false,
            },
        )?;

        self.creature.set_ai_position(stop.position);
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = None;
        ai.move_duration_ms = 0;
        ai.spline_id = stop.spline_id;
        let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
        motion.finalize_spline();
        motion.spline.spline_id = stop.spline_id;
        self.disable_spline_movement_like_cpp();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        Some(stop)
    }

    pub fn finish_move(&mut self) {
        if let Some(dst) = self.creature.ai_ownership_mut().move_target.take() {
            self.creature.set_ai_position(dst);
        }
        self.creature.ai_ownership_mut().move_duration_ms = 0;
        self.active_move_spline = None;
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .finalize_spline();
        self.disable_spline_movement_like_cpp();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::ROAMING_MOVE.bits());
    }
}
