//! Motion master operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl WorldCreature {
    pub(in crate::map_manager) fn new_runtime_motion_master_like_cpp(
        creature: &Creature,
    ) -> MotionMaster {
        let mut motion_master =
            MotionMaster::new(Self::runtime_default_generator_like_cpp(creature));
        if creature.ai_state() == CreatureAiState::InCombat
            && let Some(target) = creature.ai_ownership().combat_target
        {
            motion_master.add(
                Box::new(ChaseMovementGenerator::new(target, None, None)),
                RuntimeMovementSlot::Active,
            );
        }
        motion_master
    }

    pub fn active_waypoint_generator_like_cpp(&self) -> Option<&WaypointMovementGenerator> {
        self.active_waypoint_generator.as_ref()
    }

    pub fn active_waypoint_random_at_path_end_like_cpp(&self) -> Option<WaypointRandomAtPathEnd> {
        self.active_waypoint_random_at_path_end
    }

    pub fn position(&self) -> Position {
        self.creature.ai_position()
    }

    pub fn home_position(&self) -> Position {
        self.creature.ai_home_position()
    }

    /// C++ interaction handlers call `PauseMovement(timer)` and then
    /// `SetHomePosition(GetPosition())` for gossip/vendor/quest interactions.
    pub fn pause_interaction_movement_like_cpp(&mut self) -> bool {
        let pause_timer = self.creature.interaction_pause_timer_ms_like_cpp();
        if pause_timer == 0 {
            return false;
        }

        let current_position = self.position();
        let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
        motion.pause_current_movement_like_cpp(pause_timer, MovementSlot::Default, true);
        self.creature.set_ai_home_position(current_position);
        true
    }

    pub fn move_target(&self) -> Option<Position> {
        self.creature.ai_ownership().move_target
    }

    pub fn active_move_spline_like_cpp(&self) -> Option<&MoveSpline> {
        self.active_move_spline.as_ref()
    }

    pub fn spline_id(&self) -> u32 {
        self.creature.ai_ownership().spline_id
    }

    pub(crate) fn sync_runtime_motion_master_like_cpp(&mut self) {
        let expected_default = match self.creature.default_movement_type() {
            MovementGeneratorType::Idle => RuntimeMovementGeneratorType::Idle,
            MovementGeneratorType::Random => RuntimeMovementGeneratorType::Random,
            MovementGeneratorType::Waypoint => RuntimeMovementGeneratorType::Waypoint,
        };
        if self
            .runtime_motion_master
            .current_kind_for_slot(RuntimeMovementSlot::Default)
            != Some(expected_default)
        {
            self.runtime_motion_master.add(
                Self::runtime_default_generator_like_cpp(&self.creature),
                RuntimeMovementSlot::Default,
            );
        }

        let expected_chase_target = self.creature.ai_ownership().combat_target.filter(|_| {
            self.creature.ai_state() == CreatureAiState::InCombat && self.creature.is_alive()
        });
        if self.runtime_chase_target != expected_chase_target {
            self.runtime_motion_master.remove_kind(
                RuntimeMovementGeneratorType::Chase,
                RuntimeMovementSlot::Active,
            );
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .remove_generator_kind(MovementGeneratorKind::Chase, MovementSlot::Active);
            if let Some(target) = expected_chase_target {
                self.runtime_motion_master.add(
                    Box::new(ChaseMovementGenerator::new(target, None, None)),
                    RuntimeMovementSlot::Active,
                );
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .move_chase_like_cpp(target);
            }
            self.runtime_chase_target = expected_chase_target;
        }

        // The represented subsystem already owns concrete Point/Distract/
        // Charge/etc. lifecycle. Mirror its selected active entry into the
        // runtime selector so adding normal-priority chase cannot incorrectly
        // interrupt a higher-priority generator. C++ keeps both entries in the
        // MotionMaster multiset and selects by mode/priority.
        let expected_represented_active = {
            let motion = &self.creature.unit().subsystems().motion;
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
                .runtime_motion_master
                .has_generator_kind(key.kind, RuntimeMovementSlot::Active)
        });
        if self.runtime_represented_active != expected_key || runtime_proxy_missing {
            if let Some(previous) = self.runtime_represented_active {
                self.runtime_motion_master
                    .remove_kind(previous.kind, RuntimeMovementSlot::Active);
            }
            if let Some(generator) = expected_represented_active {
                self.runtime_motion_master
                    .add(Box::new(generator), RuntimeMovementSlot::Active);
            }
            self.runtime_represented_active = expected_key;
        }
    }

    fn tick_runtime_represented_motion_like_cpp(&mut self, diff_ms: u32) {
        let unit = self.creature.unit();
        let active_spline = self.active_move_spline.as_ref();
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
            .creature
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
        self.runtime_motion_master.update(diff_ms);
        self.runtime_motion_master_ticks = self.runtime_motion_master_ticks.saturating_add(1);
        self.runtime_motion_master.current_kind()
    }

    pub fn runtime_motion_master_current_kind_like_cpp(
        &self,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.runtime_motion_master.current_kind()
    }

    pub const fn runtime_motion_master_ticks_like_cpp(&self) -> u64 {
        self.runtime_motion_master_ticks
    }

    /// The chase generator currently selected for this creature, kept alongside
    /// the random/waypoint ones so its C++ state (`_lastTargetPosition`,
    /// `_rangeCheckTimer`, `_movingTowards`, `_path`) survives between ticks.
    pub fn active_chase_generator_like_cpp(&self) -> Option<&ChaseMovementGenerator> {
        self.active_chase_generator.as_ref()
    }

    /// The corridor the random generator's `PathGenerator` still holds, for
    /// callers that build its next path request.
    pub fn active_random_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.active_random_path_poly_refs
    }

    /// Same, for the chase generator.
    pub fn active_chase_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.active_chase_path_poly_refs
    }
}
