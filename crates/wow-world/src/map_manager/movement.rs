// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature movement: splines, pathfinding and relocation.

use super::*;

impl WorldCreature {
    pub(super) fn new_runtime_motion_master_like_cpp(creature: &Creature) -> MotionMaster {
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

    pub fn all_loot_removed_from_corpse_like_cpp(
        &mut self,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> bool {
        self.all_loot_removed_from_corpse_at_game_time_like_cpp(
            Instant::now(),
            game_time_secs_like_cpp(),
            decay_rate,
            is_fully_skinned,
        )
    }

    pub fn all_loot_removed_from_corpse_at_game_time_like_cpp(
        &mut self,
        now: Instant,
        game_time_secs: i64,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> bool {
        let plan = self.creature.all_loot_removed_from_corpse(
            game_time_secs,
            decay_rate,
            is_fully_skinned,
        );
        if plan.is_empty() {
            return false;
        }

        // C++ stores `m_corpseRemoveTime` in the absolute GameTime domain;
        // the legacy AI mirror is elapsed milliseconds from `clock_started_at`.
        let corpse_remove_at = instant_from_respawn_time_like_cpp(
            self.creature.corpse_remove_time(),
            now,
            game_time_secs,
        );
        let corpse_remove_time_ms = corpse_remove_at
            .checked_duration_since(self.clock_started_at)
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        self.creature
            .set_ai_corpse_despawn_at(Some(corpse_remove_time_ms));
        true
    }

    pub fn remove_lootable_dynamic_flag_like_cpp(&mut self) {
        let object = self.creature.unit_mut().world_mut().object_mut();
        object.remove_dynamic_flag(UnitDynFlags::Lootable as u32);
        object.force_dynamic_flags_update_like_cpp();
    }

    pub fn can_wander(&self) -> bool {
        self.creature.can_ai_wander()
    }

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
                self.now_ms()
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
            self.now_ms()
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
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = Some(dst);
        ai.move_start_ms = now_ms;
        ai.move_duration_ms = duration_ms.max(500);
        ai.spline_id = ai.spline_id.saturating_add(1);
    }

    fn launch_move_spline_init_like_cpp(
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

        let now_ms = self.now_ms();
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
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = None;
        ai.move_start_ms = now_ms;
        ai.move_duration_ms = 0;
        ai.wander_delay_ms = 0;
        ai.wander_steps_remaining = next_wander_steps_roll;
        ai.state = CreatureAiState::Idle;
        true
    }

    /// C++ `PathGenerator::CreateFilter` + `PathGenerator::UpdateFilter`
    /// (`PathGenerator.cpp:648-698`) derive the Detour query filter from the
    /// *owner*, never from a constant: `Creature::CanWalk()` adds `NAV_GROUND`,
    /// `Creature::CanEnterWater()` adds `NAV_WATER | NAV_MAGMA_SLIME`, and
    /// `Unit::IsInCombat() || Creature::IsInEvadeMode()` adds
    /// `NAV_GROUND_STEEP`.
    ///
    /// Boundary: `UpdateFilter` also ORs in
    /// `Map::GetForceEnabled/DisabledNavMeshFilterFlags()` and, while the owner
    /// `IsInWater()/IsUnderWater()`, `GetNavTerrain()` from
    /// `Map::GetLiquidStatus`. Neither map-level source exists in the Rust
    /// runtime yet, so those stay at their neutral values here.
    pub fn path_query_filter_context_like_cpp(&self) -> PathQueryFilterContext {
        // C++ `Unit::IsInCombat()` is `HasUnitFlag(UNIT_FLAG_IN_COMBAT)`, and C++
        // really does set that flag on entering combat. RustyCore's
        // `Creature::enter_ai_combat` sets the AI state and the attacking GUID
        // but not the client-visible flag, so reading the flag alone would leave
        // every chasing creature without `NAV_GROUND_STEEP`. Both signals are
        // consulted, so the filter is correct today and still correct once the
        // flag itself is maintained.
        //
        // Boundary: that missing `UNIT_FLAG_IN_COMBAT` is a separate parity
        // defect with client-visible UpdateField consequences; it is not fixed
        // here.
        let in_combat = self
            .creature
            .unit()
            .unit_flags_like_cpp()
            .contains(wow_constants::unit::UnitFlags::IN_COMBAT)
            || self.creature.is_in_combat();
        PathQueryFilterContext::creature(
            self.creature.can_walk_like_cpp(),
            self.creature.can_enter_water_like_cpp(),
            in_combat,
            self.creature.is_in_evade_mode_like_cpp(),
        )
    }

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

    /// The chase generator currently selected for this creature, kept alongside
    /// the random/waypoint ones so its C++ state (`_lastTargetPosition`,
    /// `_rangeCheckTimer`, `_movingTowards`, `_path`) survives between ticks.
    pub fn active_chase_generator_like_cpp(&self) -> Option<&ChaseMovementGenerator> {
        self.active_chase_generator.as_ref()
    }

    pub(super) fn chase_unit_snapshot_like_cpp(
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

    /// The corridor the random generator's `PathGenerator` still holds, for
    /// callers that build its next path request.
    pub fn active_random_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.active_random_path_poly_refs
    }

    /// Same, for the chase generator.
    pub fn active_chase_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.active_chase_path_poly_refs
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

    fn allowed_position_z_caps_like_cpp(&self) -> AllowedPositionZCaps {
        let hover_offset = if self
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::HOVER)
        {
            self.creature.unit().data().hover_height
        } else {
            0.0
        };
        AllowedPositionZCaps {
            on_transport: false,
            can_fly: self.creature.can_fly_like_cpp(),
            can_swim: self.creature.can_swim_like_cpp(),
            hover_offset,
        }
    }

    pub(super) fn normalize_path_position_z_like_cpp(
        &self,
        point: Position,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position {
        let Some(terrain) = terrain else {
            return point;
        };
        let probe_z = point.z + Z_OFFSET_FIND_HEIGHT;
        let static_ground =
            terrain.static_height_like_cpp(self.map_id(), point.x, point.y, probe_z);
        // C++ GetMapHeight combines terrain and VMap before
        // UpdateAllowedPositionZ clamps the point. Rust does not yet have the
        // VMap half, so lowering a valid elevated Detour point to terrain
        // destroys bridge/platform paths. Preserve elevations; the branch
        // below still raises points that are under known terrain.
        let mut ground = if static_ground >= point.z {
            static_ground
        } else {
            INVALID_HEIGHT
        };
        if ground <= INVALID_HEIGHT {
            let grid_ground = terrain.grid_height_like_cpp(self.map_id(), point.x, point.y);
            if grid_ground > INVALID_HEIGHT
                && point.z < grid_ground
                && grid_ground - point.z <= DEFAULT_HEIGHT_SEARCH
            {
                ground = grid_ground;
            }
        }
        let z = allowed_position_z_from_ground_like_cpp(
            true,
            ground,
            point.z,
            self.allowed_position_z_caps_like_cpp(),
        );
        Position::new(point.x, point.y, z, point.orientation)
    }

    fn path_generator_from_detour_for_creature_like_cpp(
        &self,
        destination: Position,
        detour_path: &DetourPolyPath,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> PathGenerator {
        path_generator_from_detour_with_normalizer_like_cpp(
            self.position(),
            destination,
            detour_path,
            force_destination,
            |point| self.normalize_path_position_z_like_cpp(point, terrain),
        )
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

    pub fn begin_point_movement_like_cpp(
        &mut self,
        movement_id: u32,
        dst: Position,
        can_move: bool,
    ) -> Option<(Position, MoveSpline)> {
        if movement_id == EVENT_CHARGE_PREPATH {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_charge(movement_id);
        } else {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_point(movement_id);
        }

        let action = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion.active_generators.iter_mut().find(|generator| {
                generator.kind == MovementGeneratorKind::Point
                    && generator.movement_id == movement_id
            })?;
            generator.initialize_point_like_cpp(can_move)
        };

        match action {
            PointMovementAction::LaunchSpline => self.begin_move_spline_like_cpp(dst),
            PointMovementAction::MarkRoamingMove => {
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::ROAMING_MOVE.bits());
                None
            }
            PointMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                None
            }
            _ => None,
        }
    }

    pub fn finalize_point_movement_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
    ) -> Option<PointMovementInform> {
        let finalize = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Point)?;
            generator.finalize_point_like_cpp(active, movement_inform)
        };
        if finalize.clear_roaming_move {
            self.creature
                .unit_mut()
                .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        }
        if let Some(inform) = finalize.inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        finalize.inform
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

        let now_ms = self.now_ms();
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

    pub fn begin_distract_movement_like_cpp(
        &mut self,
        timer_ms: u32,
        orientation: f32,
    ) -> Option<(DistractMovementAction, Position, MoveSpline)> {
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .move_distract_like_cpp(timer_ms);

        let owner_is_standing = self.creature.unit().is_stand_state_like_cpp();
        let action = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)?;
            generator.initialize_distract_like_cpp(owner_is_standing)
        };
        if action.stand_up {
            self.creature
                .unit_mut()
                .set_stand_state_like_cpp(UnitStandStateType::Stand);
        }
        let (from, spline) = self.begin_facing_spline_like_cpp(orientation)?;
        Some((action, from, spline))
    }

    pub fn tick_rotate_movement_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Option<(RotateMovementUpdate, MoveSpline)> {
        let update = {
            let current_orientation = self.position().orientation;
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Rotate)?;
            generator.update_rotate_like_cpp(true, diff_ms, current_orientation)
        };
        let (_, spline) = self.begin_facing_spline_like_cpp(update.facing_angle?)?;
        Some((update, spline))
    }

    pub fn finalize_distract_movement_like_cpp(&mut self, movement_inform: bool) -> bool {
        let finalize = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let Some(generator) = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)
            else {
                return false;
            };
            generator.finalize_distract_like_cpp(movement_inform, true)
        };

        if finalize.set_home_orientation {
            let current = self.position();
            let home = self.home_position();
            self.creature.set_ai_position(Position::new(
                current.x,
                current.y,
                current.z,
                home.orientation,
            ));
        }
        finalize.set_home_orientation
    }

    pub fn finalize_rotate_movement_like_cpp(
        &mut self,
        movement_inform: bool,
    ) -> Option<PointMovementInform> {
        let inform = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Rotate)?;
            generator
                .finalize_rotate_like_cpp(movement_inform, true)
                .inform
        };
        if let Some(inform) = inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        inform
    }

    pub fn finalize_generic_movement_like_cpp(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        movement_inform: bool,
    ) -> Option<GenericMovementInform> {
        let inform = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == kind && generator.movement_id == movement_id)?;
            generator.finalize_generic_like_cpp(movement_inform)
        };
        if let Some(inform) = inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        inform
    }

    pub fn update_move_spline_like_cpp(&mut self) -> bool {
        let Some(mut spline) = self.active_move_spline.take() else {
            return self.movement_finished();
        };

        if !spline.finalized() {
            let elapsed_ms = self
                .now_ms()
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
            .now_ms()
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

    pub fn should_wander(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::Idle
            && self.creature.default_movement_type() == wow_entities::MovementGeneratorType::Random
            && self.can_wander()
            && self.creature.ai_ownership().wander_radius > 0.0
            && self
                .now_ms()
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
        let now_ms = self.now_ms();
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
        let now_ms = self.now_ms();
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

impl MapInstance {
    pub fn remove_grid(&mut self, x: i16, y: i16) -> bool {
        let coord = GridCoord::new(x, y);
        let removed = self.grids.remove(&coord).is_some();
        if removed {
            self.personal_phases
                .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
        }
        removed
    }

    pub fn remove_creature(&mut self, x: i16, y: i16, guid: ObjectGuid) -> bool {
        if let Some(grid) = self.get_grid_mut(x, y) {
            grid.remove_creature(guid)
        } else {
            false
        }
    }

    pub fn remove_personal_phase_objects_like_cpp(&mut self) -> usize {
        let objects_to_remove = std::mem::take(&mut self.personal_phase_objects_to_remove);
        let removed = objects_to_remove.len();
        for object in objects_to_remove {
            for grid in self.grids.values_mut() {
                grid.remove_creature(object);
            }
        }
        removed
    }

    pub fn queued_personal_phase_remove_count_like_cpp(&self) -> usize {
        self.personal_phase_objects_to_remove.len()
    }

    // ── Respawn queue (Slice 4A.2a) ───────────────────────────────────────────
    //
    // Mirrors `Map::_respawnTimes` (Map.h:748-750) ownership model.
    // The queue is a plain `Vec`; heap/SpawnId convergence is deferred.

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<PersistedRespawnRowLikeCpp> {
        self.persisted_respawn_times
            .remove(&(object_type, spawn_id))
    }
}

impl MapManager {
    pub fn remove_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> bool {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            map.remove_creature(x, y, guid)
        } else {
            false
        }
    }

    pub fn remove_creature_any(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<WorldCreature> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.grids
            .values_mut()
            .find_map(|grid| grid.creatures.remove(&guid))
    }

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<PreparedStatement> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.remove_persisted_respawn_time_like_cpp(object_type, spawn_id)?;
        Some(respawn_delete_statement_like_cpp(
            object_type,
            spawn_id,
            map_id,
            instance_id,
        ))
    }

    pub fn player_move(
        &mut self,
        map_id: u16,
        instance_id: u32,
        from: (i16, i16),
        to: (i16, i16),
        player_guid: ObjectGuid,
        pos: Position,
    ) {
        let (from_x, from_y) = from;
        let (to_x, to_y) = to;

        // Leave old grid
        self.player_leave_grid(map_id, instance_id, from_x, from_y, player_guid);

        // Enter new grid
        self.player_enter_grid(map_id, instance_id, to_x, to_y, player_guid, pos);
    }
}
