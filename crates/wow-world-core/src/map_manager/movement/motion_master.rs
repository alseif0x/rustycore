//! Motion master operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl WorldCreature {
    /// Build the persistent `Unit::i_motionMaster` for one canonical creature.
    ///
    /// The construction moved to the canonical owner under #1263 F6-8A; this
    /// bridge entry point keeps the legacy call site's name and signature.
    pub(in crate::map_manager) fn new_runtime_motion_master_like_cpp(
        creature: &Creature,
    ) -> MotionMaster {
        wow_entities::new_runtime_motion_master_like_cpp(creature)
    }

    pub fn active_waypoint_generator_like_cpp(&self) -> Option<&WaypointMovementGenerator> {
        self.creature.active_waypoint_generator_like_cpp()
    }

    pub fn active_waypoint_random_at_path_end_like_cpp(&self) -> Option<WaypointRandomAtPathEnd> {
        self.creature.active_waypoint_random_at_path_end_like_cpp()
    }

    pub fn position(&self) -> Position {
        self.creature.position()
    }

    pub fn home_position(&self) -> Position {
        self.creature.home_position()
    }

    pub fn pause_interaction_movement_like_cpp(&mut self) -> bool {
        self.creature.pause_interaction_movement_like_cpp()
    }

    pub fn move_target(&self) -> Option<Position> {
        self.creature.move_target()
    }

    pub fn active_move_spline_like_cpp(&self) -> Option<&MoveSpline> {
        self.creature.active_move_spline_like_cpp()
    }

    pub fn spline_id(&self) -> u32 {
        self.creature.spline_id()
    }

    pub fn sync_runtime_motion_master_like_cpp(&mut self) {
        self.creature.sync_runtime_motion_master_like_cpp();
    }

    /// Advances the represented active lifecycle and the runtime selector once
    /// for this creature's frame, then returns the selected generator.
    pub fn tick_runtime_motion_master_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.creature.tick_runtime_motion_master_like_cpp(diff_ms)
    }

    pub fn runtime_motion_master_current_kind_like_cpp(
        &self,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.creature.runtime_motion_master_current_kind_like_cpp()
    }

    pub const fn runtime_motion_master_ticks_like_cpp(&self) -> u64 {
        self.creature.runtime_motion_master_ticks_like_cpp()
    }

    /// The chase generator currently selected for this creature, kept alongside
    /// the random/waypoint ones so its C++ state (`_lastTargetPosition`,
    /// `_rangeCheckTimer`, `_movingTowards`, `_path`) survives between ticks.
    pub fn active_chase_generator_like_cpp(&self) -> Option<&ChaseMovementGenerator> {
        self.creature.active_chase_generator_like_cpp()
    }

    /// The corridor the random generator's `PathGenerator` still holds, for
    /// callers that build its next path request.
    pub fn active_random_path_poly_refs_like_cpp(&self) -> &[u64] {
        self.creature.active_random_path_poly_refs_like_cpp()
    }

    /// Same, for the chase generator.
    pub fn active_chase_path_poly_refs_like_cpp(&self) -> &[u64] {
        self.creature.active_chase_path_poly_refs_like_cpp()
    }
}
