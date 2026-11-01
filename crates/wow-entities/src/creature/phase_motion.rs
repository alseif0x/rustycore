//! Canonical creature phase operations, part 2 of 2: the motion master.
//!
//! #1263 F6-8D1 ports `Unit::i_motionMaster` operations that C++ keeps on the
//! live `Creature` object onto canonical `Creature`/`CreatureRuntimeLikeCpp`
//! ownership: the default/chase/represented-active selector synchronization,
//! the one `MotionMaster::Update` advance per owning frame, and the represented
//! generator finalization that follows it.
//!
//! Every body is the legacy `WorldCreature` bridge body with one owner
//! substituted for another. The advance counter moved into
//! [`CreatureRuntimeLikeCpp::runtime_motion_master_ticks_like_cpp`], so the
//! bridge no longer holds creature runtime state.

use wow_movement::{
    ChaseMovementGenerator, MoveSpline, MoveSplineInit, MoveSplineStopInput, MoveSplineStopResult,
    MovementGeneratorType as RuntimeMovementGeneratorType, MovementSlot as RuntimeMovementSlot,
    WaypointMovementGenerator, WaypointRandomAtPathEnd,
};

use super::*;
use crate::{
    MotionMasterUpdateContext, MotionMasterUpdateOutcome, MovementGeneratorRef, MovementSlot,
};

impl Creature {
    pub fn active_waypoint_generator_like_cpp(&self) -> Option<&WaypointMovementGenerator> {
        self.runtime_like_cpp().active_waypoint_generator.as_ref()
    }

    pub fn active_waypoint_random_at_path_end_like_cpp(&self) -> Option<WaypointRandomAtPathEnd> {
        self.runtime_like_cpp().active_waypoint_random_at_path_end
    }

    /// C++ `WorldObject::GetHomePosition()`.
    pub const fn home_position(&self) -> Position {
        self.ai_home_position()
    }

    /// C++ interaction handlers call `PauseMovement(timer)` and then
    /// `SetHomePosition(GetPosition())` for gossip/vendor/quest interactions.
    pub fn pause_interaction_movement_like_cpp(&mut self) -> bool {
        let pause_timer = self.interaction_pause_timer_ms_like_cpp();
        if pause_timer == 0 {
            return false;
        }

        let current_position = self.position();
        let motion = &mut self.unit_mut().subsystems_mut().motion;
        motion.pause_current_movement_like_cpp(pause_timer, MovementSlot::Default, true);
        self.set_ai_home_position(current_position);
        true
    }

    pub fn move_target(&self) -> Option<Position> {
        self.ai_ownership().move_target
    }

    pub fn active_move_spline_like_cpp(&self) -> Option<&MoveSpline> {
        self.runtime_like_cpp().active_move_spline.as_ref()
    }

    pub fn spline_id(&self) -> u32 {
        self.ai_ownership().spline_id
    }

    /// C++ `MotionMaster::Initialize`/`MoveChase`/`MoveIdle` selection.
    ///
    /// The represented subsystem already owns concrete Point/Distract/Charge/
    /// etc. lifecycle. Mirror its selected active entry into the runtime
    /// selector so adding normal-priority chase cannot incorrectly interrupt a
    /// higher-priority generator. C++ keeps both entries in the MotionMaster
    /// multiset and selects by mode/priority.
    pub fn sync_runtime_motion_master_like_cpp(&mut self) {
        let expected_default = match self.default_movement_type() {
            MovementGeneratorType::Idle => RuntimeMovementGeneratorType::Idle,
            MovementGeneratorType::Random => RuntimeMovementGeneratorType::Random,
            MovementGeneratorType::Waypoint => RuntimeMovementGeneratorType::Waypoint,
        };
        if self
            .runtime_like_cpp_mut()
            .runtime_motion_master
            .current_kind_for_slot(RuntimeMovementSlot::Default)
            != Some(expected_default)
        {
            let default_generator = runtime_default_generator_like_cpp(self);
            self.runtime_like_cpp_mut()
                .runtime_motion_master
                .add(default_generator, RuntimeMovementSlot::Default);
        }

        let expected_chase_target = self
            .ai_ownership()
            .combat_target
            .filter(|_| self.ai_state() == CreatureAiState::InCombat && self.is_alive());
        if self.runtime_like_cpp_mut().runtime_chase_target != expected_chase_target {
            self.runtime_like_cpp_mut()
                .runtime_motion_master
                .remove_kind(
                    RuntimeMovementGeneratorType::Chase,
                    RuntimeMovementSlot::Active,
                );
            self.unit_mut()
                .subsystems_mut()
                .motion
                .remove_generator_kind(MovementGeneratorKind::Chase, MovementSlot::Active);
            if let Some(target) = expected_chase_target {
                self.runtime_like_cpp_mut().runtime_motion_master.add(
                    Box::new(ChaseMovementGenerator::new(target, None, None)),
                    RuntimeMovementSlot::Active,
                );
                self.unit_mut()
                    .subsystems_mut()
                    .motion
                    .move_chase_like_cpp(target);
            }
            self.runtime_like_cpp_mut().runtime_chase_target = expected_chase_target;
        }

        let expected_represented_active = {
            let motion = &self.unit().subsystems().motion;
            (motion.current_slot() == MovementSlot::Active)
                .then(|| motion.current_movement_generator())
                .filter(|generator| generator.kind != MovementGeneratorKind::Chase)
                .and_then(RuntimeRepresentedActiveGeneratorLikeCpp::from_represented)
        };
        let expected_key = expected_represented_active
            .as_ref()
            .map(RuntimeRepresentedActiveGeneratorLikeCpp::key);
        let runtime_proxy_missing = expected_key.is_some_and(|key| {
            !self
                .runtime_like_cpp_mut()
                .runtime_motion_master
                .has_generator_kind(key.kind, RuntimeMovementSlot::Active)
        });
        if self.runtime_like_cpp_mut().runtime_represented_active != expected_key
            || runtime_proxy_missing
        {
            if let Some(previous) = self.runtime_like_cpp_mut().runtime_represented_active {
                self.runtime_like_cpp_mut()
                    .runtime_motion_master
                    .remove_kind(previous.kind, RuntimeMovementSlot::Active);
            }
            if let Some(generator) = expected_represented_active {
                self.runtime_like_cpp_mut()
                    .runtime_motion_master
                    .add(Box::new(generator), RuntimeMovementSlot::Active);
            }
            self.runtime_like_cpp_mut().runtime_represented_active = expected_key;
        }
    }

    fn tick_runtime_represented_motion_like_cpp(&mut self, diff_ms: u32) {
        let unit = self.unit();
        let active_spline = self.runtime_like_cpp().active_move_spline.as_ref();
        let context = MotionMasterUpdateContext {
            diff_ms,
            can_move: !unit.has_unit_state(UnitState::NOT_MOVE.bits()),
            owner_exists: true,
            owner_is_standing: unit.is_stand_state_like_cpp(),
            spline_finalized: active_spline.is_none_or(MoveSpline::finalized),
            spline_cyclic: active_spline.is_some_and(MoveSpline::is_cyclic),
            current_orientation: self.position().orientation,
        };
        let outcome = self
            .unit_mut()
            .subsystems_mut()
            .motion
            .update_motion_master_like_cpp(context);
        if let MotionMasterUpdateOutcome::Updated {
            popped: Some(generator),
            ..
        } = outcome
        {
            self.finalize_runtime_represented_generator_like_cpp(generator);
        }
    }

    /// Advances the represented active lifecycle and the runtime selector once
    /// for this creature's frame, then returns the selected generator.
    pub fn tick_runtime_motion_master_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.sync_runtime_motion_master_like_cpp();
        self.tick_runtime_represented_motion_like_cpp(diff_ms);
        self.sync_runtime_motion_master_like_cpp();
        self.runtime_like_cpp_mut()
            .runtime_motion_master
            .update(diff_ms);
        self.runtime_like_cpp_mut()
            .record_runtime_motion_master_tick_like_cpp();
        self.runtime_like_cpp().runtime_motion_master.current_kind()
    }

    pub fn runtime_motion_master_current_kind_like_cpp(
        &self,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.runtime_like_cpp().runtime_motion_master.current_kind()
    }

    pub const fn runtime_motion_master_ticks_like_cpp(&self) -> u64 {
        self.runtime_like_cpp()
            .runtime_motion_master_ticks_like_cpp()
    }

    /// The chase generator currently selected for this creature, kept alongside
    /// the random/waypoint ones so its C++ state (`_lastTargetPosition`,
    /// `_rangeCheckTimer`, `_movingTowards`, `_path`) survives between ticks.
    pub fn active_chase_generator_like_cpp(&self) -> Option<&ChaseMovementGenerator> {
        self.runtime_like_cpp().active_chase_generator.as_ref()
    }

    /// The corridor the random generator's `PathGenerator` still holds, for
    /// callers that build its next path request.
    pub fn active_random_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.runtime_like_cpp().active_random_path_poly_refs
    }

    /// Same, for the chase generator.
    pub fn active_chase_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.runtime_like_cpp().active_chase_path_poly_refs
    }

    /// C++ `Unit::DisableSpline` and `MoveSplineInit::Stop` remove FORWARD.
    fn disable_spline_movement_like_cpp(&mut self) {
        let mut movement_flags = self.movement_flags_like_cpp();
        movement_flags.remove(MovementFlag::FORWARD);
        self.set_movement_flags_runtime_like_cpp(movement_flags);
    }

    /// C++ `MoveSplineInit::Stop` through `Unit::DisableSpline`.
    ///
    /// #1263 F6-8D1: the body is the legacy bridge body with the canonical
    /// owner substituted for the bridge. Boundary: the legacy bridge also
    /// mirrored the resulting flags into its cached `CreatureCreateData`
    /// packet projection; that projection is bridge state, so the bridge
    /// entry point performs the mirror and this canonical operation mutates
    /// only canonical unit state.
    pub fn stop_move_spline_like_cpp(&mut self) -> Option<MoveSplineStopResult> {
        let mut spline = self.runtime_like_cpp_mut().active_move_spline.take()?;
        if spline.finalized() {
            return None;
        }

        let elapsed_ms = self
            .runtime_like_cpp()
            .runtime_elapsed_ms_like_cpp()
            .saturating_sub(self.ai_ownership().move_start_ms)
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

        self.set_ai_position(stop.position);
        let ai = self.ai_ownership_mut();
        ai.move_target = None;
        ai.move_duration_ms = 0;
        ai.spline_id = stop.spline_id;
        let motion = &mut self.unit_mut().subsystems_mut().motion;
        motion.finalize_spline();
        motion.spline.spline_id = stop.spline_id;
        self.disable_spline_movement_like_cpp();
        self.unit_mut()
            .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        Some(stop)
    }

    /// Finalize one represented generator that the motion master popped.
    pub fn finalize_runtime_represented_generator_like_cpp(
        &mut self,
        mut generator: MovementGeneratorRef,
    ) {
        match generator.kind {
            MovementGeneratorKind::Point => {
                let finalize = generator.finalize_point_like_cpp(true, true);
                if finalize.clear_roaming_move {
                    self.unit_mut()
                        .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                }
                if let Some(inform) = finalize.inform {
                    self.record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            MovementGeneratorKind::Rotate => {
                if let Some(inform) = generator.finalize_rotate_like_cpp(true, true).inform {
                    self.record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            MovementGeneratorKind::Distract => {
                let finalize = generator.finalize_distract_like_cpp(true, true);
                if finalize.set_home_orientation {
                    let current = self.position();
                    let home = self.home_position();
                    self.set_ai_position(Position::new(
                        current.x,
                        current.y,
                        current.z,
                        home.orientation,
                    ));
                }
            }
            MovementGeneratorKind::Effect => {
                if let Some(inform) = generator.finalize_generic_like_cpp(true) {
                    self.record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            _ => {}
        }
    }
}
