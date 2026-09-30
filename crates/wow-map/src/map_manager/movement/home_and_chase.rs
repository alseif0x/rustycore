//! Home and chase operations of movement.
//!
//! Legacy synchronous signatures drive the private consumable movement engine.

use super::*;

impl WorldCreature {
    /// Drives the home (evade-return) generator for one frame, mirroring C++
    /// `HomeMovementGenerator<Creature>` (`HomeMovementGenerator.cpp:48-157`).
    ///
    /// C++ `SetTargetLocation` launches `init.MoveTo(home)` with the defaults
    /// `generatePath = true, forceDestination = false`, so the return trip is a
    /// real navmesh path — not a teleport.
    pub fn update_runtime_home_movement_like_cpp(
        &mut self,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> ChaseTickOutcomeLikeCpp {
        let progress = pending::prepare_home(self, should_try_pathfinding, terrain.is_some());
        match pending::run(self, progress, terrain, &mut resolve_path) {
            pending::MovementCompletion::Home(outcome) => outcome,
            _ => unreachable!("home preparation completes a home operation"),
        }
    }

    pub(super) fn home_unit_snapshot_like_cpp(&self) -> wow_movement::HomeUnitSnapshot {
        wow_movement::HomeUnitSnapshot {
            owner_alive: self.creature.is_alive(),
            owner_unit_state: self.creature.unit().unit_state(),
            home_position: self.creature.ai_ownership().home_position,
            move_spline_finalized: self
                .runtime.active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            can_swim_out_of_combat: !self.creature.is_missing_can_swim_flag_out_of_combat(),
            is_vehicle: false,
        }
    }

    /// C++ `HomeMovementGenerator<Creature>::DoFinalize` reached-home payload:
    /// clears `UNIT_STATE_ROAMING_MOVE | UNIT_STATE_EVADE` and reports
    /// `JustReachedHome` (`HomeMovementGenerator.cpp:141-157`).
    ///
    /// Boundary: the spawn-health, creature-addon and sparring-health reloads
    /// C++ performs there are respawn-owned work in this runtime and stay with
    /// the lifecycle tick.
    pub(super) fn finish_home_movement_like_cpp(&mut self) {
        let snapshot = self.home_unit_snapshot_like_cpp();
        let finalize = self
            .runtime.active_home_generator
            .as_mut()
            .map(|generator| generator.finalize_like_cpp(true, true, snapshot));
        if let Some(finalize) = finalize {
            // C++ clears `UNIT_STATE_ROAMING_MOVE | UNIT_STATE_EVADE` here when
            // the generator was active (`HomeMovementGenerator.cpp:141-143`).
            self.creature
                .unit_mut()
                .clear_unit_state(finalize.clear_unit_state_mask);
            if finalize.remove_can_swim_flag {
                self.creature.restore_can_swim_flag_after_home_like_cpp();
            }
            if finalize.just_reached_home {
                // C++ `SetSpawnHealth()` precedes `AI()->JustReachedHome()`
                // (`HomeMovementGenerator.cpp:148-156`). Addon/sparring health
                // overlays remain respawn-owned in this runtime.
                self.creature.set_spawn_health_like_cpp();
                self.runtime.home_health_restored_pending_like_cpp = true;
                self.creature.record_ai_just_reached_home();
            }
        }
        self.runtime.active_home_generator = None;
        self.creature.ai_ownership_mut().move_target = None;
        self.creature.set_ai_state(CreatureAiState::Idle);
    }

    pub fn take_home_health_restored_pending_like_cpp(&mut self) -> bool {
        std::mem::take(&mut self.runtime.home_health_restored_pending_like_cpp)
    }

    pub(in crate::map_manager) fn chase_unit_snapshot_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
    ) -> wow_movement::ChaseUnitSnapshot {
        let unit = self.creature.unit();
        let owner_combat_reach = unit.data().combat_reach.max(0.0);
        // C++ `Unit::GetMeleeRange`: reaches plus 4/3, floored at
        // `NOMINAL_MELEE_RANGE` (`Unit.cpp:664-668`).
        let owner_melee_range = (owner_combat_reach + target.combat_reach + 4.0 / 3.0)
            .max(NOMINAL_MELEE_RANGE_LIKE_CPP);
        let can_enter_water = self.creature.can_enter_water_like_cpp();
        let can_walk = self.creature.can_walk_like_cpp();
        let can_fly = self.creature.can_fly_like_cpp();
        wow_movement::ChaseUnitSnapshot {
            owner_position: self.position(),
            target_position: target.position,
            owner_combat_reach,
            target_combat_reach: target.combat_reach,
            owner_melee_range,
            owner_alive: self.creature.is_alive(),
            target_in_world: target.in_world,
            can_move: !unit.has_unit_state(UnitState::NOT_MOVE.bits()),
            movement_prevented_by_casting: unit.has_unit_state(UnitState::CASTING.bits()),
            owner_victim_is_target: self.creature.ai_ownership().combat_target == Some(target.guid),
            owner_has_chase_move: unit.has_unit_state(UnitState::CHASE_MOVE.bits()),
            owner_movespline_finalized: self
                .runtime.active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            // C++ `IsMutualChase` needs the target's own MotionMaster; only
            // creatures chase, and the runtime has no cross-object accessor in
            // this step, so a mutual chase is never detected yet. That only ever
            // *keeps* the chase angle applied, never drops a real constraint.
            mutual_chase: false,
            // VMap line of sight is a stub; C++ `PositionOkay` requires it.
            owner_has_los: true,
            // C++ `Unit::isInAccessiblePlaceFor` picks exactly one branch from
            // the victim's real `IsInWater()`. With no liquid data for creature
            // victims, taking either branch would be a guess, and guessing
            // "not in water" is the harmful one: it makes an aquatic,
            // non-walking chaser report `CannotReachTarget` and freeze on a
            // victim C++ would let it reach. The unknown case therefore accepts
            // the union of both branches — the Detour query and its
            // `PATHFIND_NOPATH` bail-out remain the real gate.
            target_accessible: match target.in_water {
                Some(true) => can_enter_water,
                Some(false) => can_walk || can_fly,
                None => can_enter_water || can_walk || can_fly,
            },
            owner_can_fly: can_fly,
            owner_is_creature: true,
            creature_is_pet: self.creature.unit().world().object().guid().is_pet(),
            creature_chase_walk: match self.creature.chase_movement_type_like_cpp() {
                value if value == wow_constants::CreatureChaseMovementType::CanWalk as u8 => {
                    wow_movement::ChaseWalkMode::CanWalk
                }
                value if value == wow_constants::CreatureChaseMovementType::AlwaysWalk as u8 => {
                    wow_movement::ChaseWalkMode::AlwaysWalk
                }
                _ => wow_movement::ChaseWalkMode::Default,
            },
            owner_is_walking: self
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
        }
    }

    /// Advances the selected chase generator for one frame and executes its
    /// decision, mirroring C++ `ChaseMovementGenerator::Update`
    /// (`ChaseMovementGenerator.cpp:94-240`).
    ///
    /// The path query itself is delegated so the caller keeps ownership of the
    /// off-thread pathfinder, exactly as the random and waypoint arms do.
    pub fn update_runtime_chase_movement_like_cpp(
        &mut self,
        diff_ms: u32,
        target: ChaseTargetSnapshotLikeCpp,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> ChaseTickOutcomeLikeCpp {
        let progress = pending::prepare_chase(self, diff_ms, target, should_try_pathfinding, terrain.is_some());
        match pending::run(self, progress, terrain, &mut resolve_path) {
            pending::MovementCompletion::Chase(outcome) => outcome,
            _ => unreachable!("chase preparation completes a chase operation"),
        }
    }

    /// Retires the chase generator the way C++ `MotionMaster` does when chase
    /// `Update` returns false: `Finalize` clears `UNIT_STATE_CHASE_MOVE` and
    /// `SetCannotReachTarget(false)`, and the generator is removed so a lower
    /// slot resumes (`ChaseMovementGenerator.cpp:251-260`). The superseded
    /// spline is stopped so the creature does not keep coasting toward a victim
    /// that is gone.
    ///
    /// Boundary: this is the movement half only. C++ also clears the victim
    /// through the kill/threat path (`UpdateVictim`, evade); the combat target
    /// and engagement are not reset here — that is M2.5 — so a creature still
    /// flagged in combat may have chase re-selected next tick, but it no longer
    /// drives toward the gone target with a stale `UNIT_STATE_CHASE_MOVE`.
    pub fn finalize_runtime_chase_movement_like_cpp(&mut self) -> Option<MoveSplineStopResult> {
        self.runtime.active_chase_generator = None;
        self.runtime.active_chase_path_poly_refs.clear();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::CHASE_MOVE.bits());
        self.stop_move_spline_like_cpp()
    }
}
