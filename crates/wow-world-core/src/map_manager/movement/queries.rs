//! Read-only movement queries of a canonical creature.
//!
//! #1263 F6-8D3a-2: these bodies read only the canonical `Creature`, so they
//! are written once against it and serve both the legacy `WorldCreature`
//! bridge (through its `creature` field) and the admitted canonical executor.
//! Each body is the former `WorldCreature` body with the owner substituted.

use super::*;

/// The movement queries C++ answers from the live `Creature` object.
pub trait CreatureMovementQueriesLikeCpp {
    fn home_unit_snapshot_like_cpp(&self) -> wow_movement::HomeUnitSnapshot;
    fn chase_unit_snapshot_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
    ) -> wow_movement::ChaseUnitSnapshot;
    fn can_wander(&self) -> bool;
    fn waypoint_unit_snapshot_like_cpp(&self) -> WaypointUnitSnapshot;
    fn should_wander(&self) -> bool;
    fn movement_finished(&self) -> bool;
    fn interpolated_position(&self) -> Position;
    fn random_movement_walk_like_cpp(&self) -> bool;
    fn path_query_filter_context_like_cpp(&self) -> PathQueryFilterContext;
    fn allowed_position_z_caps_like_cpp(&self) -> AllowedPositionZCaps;
    fn normalize_path_position_z_like_cpp(
        &self,
        point: Position,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position;
    fn path_generator_from_detour_for_creature_like_cpp(
        &self,
        destination: Position,
        detour_path: &DetourPolyPath,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> PathGenerator;
    fn detour_owner_capabilities_like_cpp(&self) -> DetourOwnerCapabilitiesLikeCpp;
    fn near_point_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
        distance_2d: f32,
        absolute_angle: f32,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position;
    fn random_unit_snapshot_like_cpp(
        &self,
        has_los_to_destination: bool,
        path_result: RandomPathResult,
        distance_roll: f32,
        angle_roll: f32,
        next_wander_steps_roll: u8,
        pause_seconds_roll: i32,
        travel_time_ms: i32,
    ) -> RandomUnitSnapshot;
}

impl CreatureMovementQueriesLikeCpp for Creature {
    fn home_unit_snapshot_like_cpp(&self) -> wow_movement::HomeUnitSnapshot {
        wow_movement::HomeUnitSnapshot {
            owner_alive: self.is_alive(),
            owner_unit_state: self.unit().unit_state(),
            home_position: self.ai_ownership().home_position,
            move_spline_finalized: self
                .runtime_like_cpp()
                .active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            can_swim_out_of_combat: !self.is_missing_can_swim_flag_out_of_combat(),
            is_vehicle: false,
        }
    }

    fn chase_unit_snapshot_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
    ) -> wow_movement::ChaseUnitSnapshot {
        let unit = self.unit();
        let owner_combat_reach = unit.data().combat_reach.max(0.0);
        // C++ `Unit::GetMeleeRange`: reaches plus 4/3, floored at
        // `NOMINAL_MELEE_RANGE` (`Unit.cpp:664-668`).
        let owner_melee_range = (owner_combat_reach + target.combat_reach + 4.0 / 3.0)
            .max(NOMINAL_MELEE_RANGE_LIKE_CPP);
        let can_enter_water = self.can_enter_water_like_cpp();
        let can_walk = self.can_walk_like_cpp();
        let can_fly = self.can_fly_like_cpp();
        wow_movement::ChaseUnitSnapshot {
            owner_position: self.position(),
            target_position: target.position,
            owner_combat_reach,
            target_combat_reach: target.combat_reach,
            owner_melee_range,
            owner_alive: self.is_alive(),
            target_in_world: target.in_world,
            can_move: !unit.has_unit_state(UnitState::NOT_MOVE.bits()),
            movement_prevented_by_casting: unit.has_unit_state(UnitState::CASTING.bits()),
            owner_victim_is_target: self.ai_ownership().combat_target == Some(target.guid),
            owner_has_chase_move: unit.has_unit_state(UnitState::CHASE_MOVE.bits()),
            owner_movespline_finalized: self
                .runtime_like_cpp()
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
            creature_is_pet: self.unit().world().object().guid().is_pet(),
            creature_chase_walk: match self.chase_movement_type_like_cpp() {
                value if value == wow_constants::CreatureChaseMovementType::CanWalk as u8 => {
                    wow_movement::ChaseWalkMode::CanWalk
                }
                value if value == wow_constants::CreatureChaseMovementType::AlwaysWalk as u8 => {
                    wow_movement::ChaseWalkMode::AlwaysWalk
                }
                _ => wow_movement::ChaseWalkMode::Default,
            },
            owner_is_walking: self
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
        }
    }

    fn can_wander(&self) -> bool {
        self.can_ai_wander()
    }

    fn waypoint_unit_snapshot_like_cpp(&self) -> WaypointUnitSnapshot {
        let unit = self.unit();
        WaypointUnitSnapshot {
            owner_alive: self.is_alive(),
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

    fn should_wander(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::Idle
            && self.default_movement_type() == wow_entities::MovementGeneratorType::Random
            && self.can_wander()
            && self.ai_ownership().wander_radius > 0.0
            && self
                .runtime_elapsed_ms_like_cpp()
                .saturating_sub(self.ai_ownership().move_start_ms)
                >= self.ai_ownership().wander_delay_ms
    }

    fn movement_finished(&self) -> bool {
        if let Some(spline) = &self.runtime_like_cpp().active_move_spline {
            return spline.finalized();
        }
        self.ai_ownership()
            .move_target
            .map(|_| {
                self.runtime_elapsed_ms_like_cpp()
                    .saturating_sub(self.ai_ownership().move_start_ms)
                    >= u64::from(self.ai_ownership().move_duration_ms)
            })
            .unwrap_or(true)
    }

    fn interpolated_position(&self) -> Position {
        let Some(dst) = self.ai_ownership().move_target else {
            return self.position();
        };
        let elapsed = self
            .runtime_elapsed_ms_like_cpp()
            .saturating_sub(self.ai_ownership().move_start_ms) as f32;
        let total = self.ai_ownership().move_duration_ms as f32;
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

    fn random_movement_walk_like_cpp(&self) -> bool {
        match self.random_movement_type_like_cpp() {
            value if value == ConstantsCreatureRandomMovementType::CanRun as u8 => self
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
            value if value == ConstantsCreatureRandomMovementType::AlwaysRun as u8 => false,
            _ => true,
        }
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
    fn path_query_filter_context_like_cpp(&self) -> PathQueryFilterContext {
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
            .unit()
            .unit_flags_like_cpp()
            .contains(wow_constants::unit::UnitFlags::IN_COMBAT)
            || self.is_in_combat();
        PathQueryFilterContext::creature(
            self.can_walk_like_cpp(),
            self.can_enter_water_like_cpp(),
            in_combat,
            self.is_in_evade_mode_like_cpp(),
        )
    }

    fn allowed_position_z_caps_like_cpp(&self) -> AllowedPositionZCaps {
        let hover_offset = if self.movement_flags_like_cpp().contains(MovementFlag::HOVER) {
            self.unit().data().hover_height
        } else {
            0.0
        };
        AllowedPositionZCaps {
            on_transport: false,
            can_fly: self.can_fly_like_cpp(),
            can_swim: self.can_swim_like_cpp(),
            hover_offset,
        }
    }

    fn normalize_path_position_z_like_cpp(
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

    /// Owner capabilities `PathGenerator::BuildPolyPath` reads off `_source`
    /// when a position has no navmesh polygon: `Creature::CanFly()`
    /// (`Creature.h:126`), `Creature::CanSwim()` (`Creature.cpp:2912-2921`) and
    /// `Unit::IsFalling()` (`Unit.cpp:12173-12176`, movement flags **or** the
    /// active spline falling).
    fn detour_owner_capabilities_like_cpp(&self) -> DetourOwnerCapabilitiesLikeCpp {
        let spline_falling = self
            .runtime_like_cpp()
            .active_move_spline
            .as_ref()
            .is_some_and(|spline| spline.flags().contains(MoveSplineFlag::FALLING));
        DetourOwnerCapabilitiesLikeCpp {
            can_fly: self.can_fly_like_cpp(),
            can_swim: self.can_swim_like_cpp(),
            is_falling: self
                .movement_flags_like_cpp()
                .intersects(MovementFlag::FALLING | MovementFlag::FALLING_FAR)
                || spline_falling,
        }
    }

    /// C++ `WorldObject::GetNearPoint2D` + `GetNearPoint`
    /// (`Object.cpp:3379-3441`): a point `distance_2d` beyond the combined
    /// combat reaches, at `absolute_angle` around the target, with Z snapped by
    /// the searcher's `UpdateAllowedPositionZ`.
    ///
    /// Boundary: C++ also sweeps the angle in `M_PI/8` steps until the candidate
    /// is in line of sight when `CONFIG_DETECT_POS_COLLISION` is on. VMap line of
    /// sight is still a stub here, so the first candidate is taken.
    fn near_point_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
        distance_2d: f32,
        absolute_angle: f32,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position {
        let effective_reach = target.combat_reach + self.unit().data().combat_reach.max(0.0);
        let radius = effective_reach + distance_2d;
        let point = Position::new(
            target.position.x + radius * absolute_angle.cos(),
            target.position.y + radius * absolute_angle.sin(),
            target.position.z,
            0.0,
        );
        self.normalize_path_position_z_like_cpp(point, terrain)
    }

    fn random_unit_snapshot_like_cpp(
        &self,
        has_los_to_destination: bool,
        path_result: RandomPathResult,
        distance_roll: f32,
        angle_roll: f32,
        next_wander_steps_roll: u8,
        pause_seconds_roll: i32,
        travel_time_ms: i32,
    ) -> RandomUnitSnapshot {
        let random_type = match self.random_movement_type_like_cpp() {
            value if value == ConstantsCreatureRandomMovementType::CanRun as u8 => {
                MovementCreatureRandomMovementType::CanRun
            }
            value if value == ConstantsCreatureRandomMovementType::AlwaysRun as u8 => {
                MovementCreatureRandomMovementType::AlwaysRun
            }
            _ => MovementCreatureRandomMovementType::AlwaysWalk,
        };
        RandomUnitSnapshot {
            owner_position: self.position(),
            owner_alive: self.is_alive(),
            owner_unit_state: self.unit().unit_state(),
            movement_prevented_by_casting: self.unit().has_unit_state(UnitState::CASTING.bits()),
            move_spline_finalized: self
                .runtime_like_cpp()
                .active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            owner_wander_distance: self.ai_ownership().wander_radius,
            has_los_to_destination,
            path_result,
            movement_template: random_type,
            owner_is_walking: self
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
            travel_time_ms,
            distance_roll,
            angle_roll,
            next_wander_steps_roll,
            pause_seconds_roll,
            ai_enabled: true,
        }
    }
}
