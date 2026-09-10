//! Home and chase operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

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
        let snapshot = self.home_unit_snapshot_like_cpp();
        let from_update = self.active_home_generator.is_some();
        let action = match self.active_home_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, snapshot),
            None => {
                let mut generator = HomeMovementGenerator::new();
                let action = generator.initialize_like_cpp(true, snapshot);
                self.active_home_generator = Some(generator);
                // C++ `CreatureAI::EnterEvadeMode` adds `UNIT_STATE_EVADE`
                // immediately before `MoveTargetedHome()` (`CreatureAI.cpp:237`),
                // and `HomeMovementGenerator::DoFinalize` is what clears it
                // (`HomeMovementGenerator.cpp:143`). The state is what makes the
                // creature immune to attacks and un-aggroable while it walks
                // back; without it, this now multi-tick return would let a
                // player damage and re-aggro a fully reset creature.
                //
                // Boundary: C++ sets it one step earlier, in the AI evade entry,
                // after `_EnterEvadeMode()` bookkeeping and only on the
                // no-charmer branch. This runtime has no live AI evade entry,
                // so the state is added where the return actually begins. A
                // full `CreatureAI::EnterEvadeMode` port stays with M2.5.
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::EVADE.bits());
                action
            }
        };

        match action {
            wow_movement::HomeMovementAction::Continue => ChaseTickOutcomeLikeCpp::Idle,
            // C++ `SetTargetLocation` sets `MOVEMENTGENERATOR_FLAG_INTERRUPTED`
            // and returns without launching while ROOT/STUNNED/DISTRACTED; the
            // generator stays installed and only the *next* `DoUpdate` sets
            // `INFORM_ENABLED` and finalizes (`HomeMovementGenerator.cpp:53-58,
            // 117-122`). Finalizing here in the initialize frame would skip that
            // `INFORM_ENABLED`, suppressing `JustReachedHome` and clearing evade
            // one frame early. So `Interrupted` from initialization keeps the
            // generator; only a `Finished` from an update finalizes.
            wow_movement::HomeMovementAction::Interrupted if !from_update => {
                ChaseTickOutcomeLikeCpp::Idle
            }
            wow_movement::HomeMovementAction::Interrupted
            | wow_movement::HomeMovementAction::Finished => {
                self.finish_home_movement_like_cpp();
                ChaseTickOutcomeLikeCpp::Idle
            }
            wow_movement::HomeMovementAction::Launch(plan) => {
                self.creature
                    .unit_mut()
                    .clear_unit_state(plan.clear_unit_state_mask);
                self.creature.unit_mut().add_unit_state(plan.add_unit_state);

                let destination =
                    self.normalize_path_position_z_like_cpp(plan.destination, terrain);
                let detour_path = should_try_pathfinding
                    .then(|| {
                        // Built after `UNIT_STATE_EVADE` was added above, which
                        // is what makes `UpdateFilter` include
                        // `NAV_GROUND_STEEP` — C++ sets evade before
                        // `MoveTargetedHome` constructs the path, so sampling the
                        // filter earlier would path the return without steep
                        // ground.
                        resolve_path(CreaturePathQueryLikeCpp {
                            start: self.position(),
                            destination,
                            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                            force_destination: false,
                            filter_context: self.path_query_filter_context_like_cpp(),
                            owner: self.detour_owner_capabilities_like_cpp(),
                            // C++ `MoveSplineInit::MoveTo` builds a fresh
                            // `PathGenerator`, so the home leg has no corridor to
                            // reuse.
                            previous_poly_refs: Vec::new(),
                        })
                    })
                    .flatten();

                // C++ goes through `MoveSplineInit::MoveTo(..., generatePath)`,
                // which falls back to a direct two-point spline whenever the path
                // is unusable (`MoveSplineInit.cpp:261-277`).
                let path = detour_path
                    .as_ref()
                    .map(|detour_path| {
                        self.path_generator_from_detour_for_creature_like_cpp(
                            destination,
                            detour_path,
                            false,
                            terrain,
                        )
                    })
                    .filter(|path| !path.path_type().contains(PathType::NOPATH));

                let spline_id = self.spline_id().saturating_add(1);
                let mut init = MoveSplineInit::new(spline_id);
                init.set_walk(plan.walk);
                match path {
                    Some(path) => init.move_by_path(path.path_points().to_vec(), 0),
                    None => init.move_to(destination),
                }
                init.set_facing_angle(plan.facing);

                match self.launch_move_spline_init_like_cpp(&mut init, destination) {
                    Some((from, spline)) => ChaseTickOutcomeLikeCpp::Launched(from, spline),
                    None => {
                        self.finish_home_movement_like_cpp();
                        ChaseTickOutcomeLikeCpp::Idle
                    }
                }
            }
        }
    }

    fn home_unit_snapshot_like_cpp(&self) -> wow_movement::HomeUnitSnapshot {
        wow_movement::HomeUnitSnapshot {
            owner_alive: self.creature.is_alive(),
            owner_unit_state: self.creature.unit().unit_state(),
            home_position: self.creature.ai_ownership().home_position,
            move_spline_finalized: self
                .active_move_spline
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
    fn finish_home_movement_like_cpp(&mut self) {
        let snapshot = self.home_unit_snapshot_like_cpp();
        let finalize = self
            .active_home_generator
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
                self.home_health_restored_pending_like_cpp = true;
                self.creature.record_ai_just_reached_home();
            }
        }
        self.active_home_generator = None;
        self.creature.ai_ownership_mut().move_target = None;
        self.creature.set_ai_state(CreatureAiState::Idle);
    }

    pub fn take_home_health_restored_pending_like_cpp(&mut self) -> bool {
        std::mem::take(&mut self.home_health_restored_pending_like_cpp)
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
                .active_move_spline
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
        // C++ installs a *new* `ChaseMovementGenerator` per `MoveChase` call and
        // its `AbstractFollower` is bound to that victim for the generator's
        // whole life (`ChaseMovementGenerator.cpp:68-76`,
        // `AbstractFollower.cpp:21-31`). Reusing one across a victim switch
        // would carry the previous follower, `_lastTargetPosition`,
        // `_rangeCheckTimer`, `_movingTowards` and the arrival
        // `MovementInform` counter onto the new target, so the generator is
        // rebuilt whenever the victim differs.
        let victim_changed = self
            .active_chase_generator
            .as_ref()
            .is_none_or(|generator| generator.target() != Some(target.guid));
        if victim_changed {
            let mut generator = ChaseMovementGenerator::new(target.guid, None, None);
            generator.initialize_like_cpp();
            self.active_chase_generator = Some(generator);
            self.active_chase_path_poly_refs.clear();
        }

        let snapshot = self.chase_unit_snapshot_like_cpp(target);
        let action = match self.active_chase_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, target.in_world, diff_ms, snapshot),
            None => return ChaseTickOutcomeLikeCpp::Idle,
        };

        match action {
            wow_movement::ChaseMovementAction::Continue => ChaseTickOutcomeLikeCpp::Idle,
            // C++ chase `Update` returns false when the victim is gone or has
            // left the world (`ChaseMovementGenerator.cpp:97,101-103`), which
            // pops the generator via `MotionMaster::Update` and runs `Finalize`.
            // Clearing only the corridor would leave the generator,
            // `UNIT_STATE_CHASE_MOVE` and the in-flight spline intact, so the
            // creature keeps sliding toward the corpse and is re-selected as
            // chasing every tick.
            wow_movement::ChaseMovementAction::Finished => {
                match self.finalize_runtime_chase_movement_like_cpp() {
                    Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
            // C++ `StopMoving()` clears `UNIT_STATE_MOVING` (which contains
            // `UNIT_STATE_CHASE_MOVE`) and stops the spline.
            wow_movement::ChaseMovementAction::StopMoving
            | wow_movement::ChaseMovementAction::CannotReachTarget => {
                self.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::CHASE_MOVE.bits());
                self.active_chase_path_poly_refs.clear();
                match self.stop_move_spline_like_cpp() {
                    Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
            wow_movement::ChaseMovementAction::StopMovingAndFaceInform(inform)
            | wow_movement::ChaseMovementAction::ClearChaseMoveAndFaceInform(inform) => {
                // C++ `SetInFront(target)` only turns the owner server-side, and
                // then reports arrival to the AI.
                self.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::CHASE_MOVE.bits());
                let mut position = self.position();
                position.orientation = absolute_angle_like_cpp(position, target.position);
                self.creature.set_ai_position(position);
                self.creature.record_ai_movement_inform(
                    inform.movement_type.trinity_id(),
                    inform.target_counter,
                );
                self.active_chase_path_poly_refs.clear();
                match self.stop_move_spline_like_cpp() {
                    Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
            wow_movement::ChaseMovementAction::Launch(plan) => {
                if plan.direction_changed {
                    // C++ replaces the owned `PathGenerator` before
                    // `CalculatePath` when chase direction flips
                    // (`ChaseMovementGenerator.cpp:171-175`). The retained
                    // Detour corridor belongs to that old path object.
                    self.active_chase_path_poly_refs.clear();
                }

                // C++ picks the target centre when closing in without an angle
                // constraint, otherwise a point on the tolerance ring
                // (`ChaseMovementGenerator.cpp:177-191`).
                let destination = if plan.move_toward && plan.desired_relative_angle.is_none() {
                    target.position
                } else {
                    let hitbox_sum =
                        self.creature.unit().data().combat_reach.max(0.0) + target.combat_reach;
                    let absolute_angle = match plan.desired_relative_angle {
                        Some(relative) => wow_movement::normalize_orientation_like_cpp(
                            target.position.orientation + relative,
                        ),
                        None => absolute_angle_like_cpp(target.position, self.position()),
                    };
                    self.near_point_like_cpp(
                        target,
                        plan.desired_distance - hitbox_sum,
                        absolute_angle,
                        terrain,
                    )
                };

                // C++ `ChaseMovementGenerator::Update` calls
                // `CalculatePath(x, y, z, owner->CanFly())`
                // (`ChaseMovementGenerator.cpp:196`), and `_forceDestination` is
                // consumed *inside* `BuildPointPath` (`PathGenerator.cpp:603-619`)
                // — setting it afterwards on the Rust `PathGenerator` would
                // record the flag without rebuilding the clamped point path, so
                // it has to travel with the query itself.
                let mut query_failed = false;
                let detour_path = if should_try_pathfinding {
                    // Built here, not by the caller: the victim-change reset
                    // above may already have dropped the retained corridor, and
                    // reusing the previous victim's `_pathPolyRefs` would let the
                    // ~80% prefix branch steer the first spline back toward the
                    // old target.
                    let resolved = resolve_path(CreaturePathQueryLikeCpp {
                        start: self.position(),
                        destination,
                        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                        force_destination: plan.allow_flying_path,
                        filter_context: self.path_query_filter_context_like_cpp(),
                        owner: self.detour_owner_capabilities_like_cpp(),
                        previous_poly_refs: self.active_chase_path_poly_refs.clone(),
                    });
                    // The resolver already answers a missing navmesh/tile with
                    // the C++ `BuildShortcut()` path, so `None` here means the
                    // query was attempted and genuinely failed. C++ has no such
                    // case — its own failures went through `BuildShortcut()` +
                    // `PATHFIND_NOPATH` — so it must not be confused with
                    // "there is no navmesh", which is launchable.
                    query_failed = resolved.is_none();
                    resolved
                } else {
                    None
                };

                let mut path = match detour_path.as_ref() {
                    Some(detour_path) => self.path_generator_from_detour_for_creature_like_cpp(
                        destination,
                        detour_path,
                        plan.allow_flying_path,
                        terrain,
                    ),
                    None if query_failed => {
                        // Reproduce the C++ `PATHFIND_NOPATH` branch so the
                        // bail-out below stops and retries.
                        let mut path = PathGenerator::new();
                        path.apply_detour_path_like_cpp(
                            self.position(),
                            destination,
                            destination,
                            [],
                            &[],
                            PathType::NOPATH,
                            plan.allow_flying_path,
                        );
                        path
                    }
                    None => {
                        // Pathfinding is off for this map/owner: C++
                        // `CalculatePath` answers with `BuildShortcut()` and
                        // `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`, which
                        // chase launches (`PathGenerator.cpp:79-86`).
                        let mut path = PathGenerator::new();
                        path.calculate_without_navmesh_like_cpp(
                            self.position(),
                            destination,
                            plan.allow_flying_path,
                        );
                        path
                    }
                };

                // C++ bails out only on `PATHFIND_NOPATH`; SHORTCUT, INCOMPLETE,
                // SHORT and FARFROMPOLY all proceed
                // (`ChaseMovementGenerator.cpp:197-203`).
                if path.path_type().contains(PathType::NOPATH) {
                    if let Some(generator) = self.active_chase_generator.as_mut() {
                        generator.cannot_reach_target = true;
                    }
                    self.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::CHASE_MOVE.bits());
                    self.active_chase_path_poly_refs.clear();
                    return match self.stop_move_spline_like_cpp() {
                        Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                        None => ChaseTickOutcomeLikeCpp::Idle,
                    };
                }

                if plan.shorten_path {
                    // C++ shortens against the target's exact position, using
                    // line of sight from each candidate; VMap LOS is a stub, so
                    // every candidate is treated as visible.
                    path.shorten_path_until_dist_like_cpp(
                        target.position,
                        plan.desired_distance,
                        |_| true,
                    );
                }

                if let Some(generator) = self.active_chase_generator.as_mut() {
                    // C++ clears `CannotReachTarget` after a successful
                    // `CalculatePath` and enables the next arrival inform
                    // immediately before launching the spline. A failed query
                    // must preserve the previous inform lifecycle.
                    generator.confirm_path_ready_like_cpp();
                }
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::CHASE_MOVE.bits());

                let points = path.path_points().to_vec();
                let Some(dst) = points.last().copied() else {
                    return ChaseTickOutcomeLikeCpp::Idle;
                };
                let spline_id = self.spline_id().saturating_add(1);
                let mut init = MoveSplineInit::new(spline_id);
                init.set_walk(plan.walk);
                init.move_by_path(points, 0);
                // C++ `init.SetFacing(target)` is client-side target tracking.
                init.set_facing_target_with_angle(
                    target.guid,
                    absolute_angle_like_cpp(self.position(), target.position),
                );

                match self.launch_move_spline_init_like_cpp(&mut init, dst) {
                    Some((from, spline)) => {
                        if let Some(detour_path) = detour_path.as_ref() {
                            self.active_chase_path_poly_refs
                                .clone_from(&detour_path.poly_refs);
                        } else {
                            self.active_chase_path_poly_refs.clear();
                        }
                        if let Some(generator) = self.active_chase_generator.as_mut() {
                            generator.confirm_launch_like_cpp(plan);
                        }
                        ChaseTickOutcomeLikeCpp::Launched(from, spline)
                    }
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
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
        self.active_chase_generator = None;
        self.active_chase_path_poly_refs.clear();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::CHASE_MOVE.bits());
        self.stop_move_spline_like_cpp()
    }
}
