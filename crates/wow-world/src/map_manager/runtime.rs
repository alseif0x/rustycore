// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Legacy creature runtime frame and its per-map orchestration.

use super::*;

impl WorldCreature {
    pub(super) fn runtime_default_generator_like_cpp(
        creature: &Creature,
    ) -> Box<dyn RuntimeMovementGenerator> {
        match creature.default_movement_type() {
            MovementGeneratorType::Idle => Box::new(IdleMovementGenerator::new()),
            MovementGeneratorType::Random => Box::new(RandomMovementGenerator::new(
                creature.ai_ownership().wander_radius,
                None,
            )),
            MovementGeneratorType::Waypoint => {
                Box::new(WaypointMovementGenerator::from_db_path_id(
                    creature.waypoint_path_id_like_cpp(),
                    true,
                ))
            }
        }
    }

    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        pos: Position,
        hp: u32,
        level: u8,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        display_id: u32,
        faction: u32,
        npc_flags: u32,
        unit_flags: u32,
    ) -> Self {
        let (min_dmg, max_dmg) = if min_dmg == 0 {
            let base = (level as u32) * 3 + 5;
            (base, base + base / 2)
        } else {
            (min_dmg, max_dmg)
        };

        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(entry);
        creature.set_ai_position(pos);
        creature.set_ai_home_position(pos);
        creature.unit_mut().set_level(level);
        creature.unit_mut().set_max_health(u64::from(hp));
        creature.unit_mut().set_health(u64::from(hp));
        creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
        creature.unit_mut().set_weapon_damage(
            WeaponAttackType::BaseAttack,
            min_dmg as f32,
            max_dmg as f32,
        );
        {
            let ai = creature.ai_ownership_mut();
            ai.aggro_radius = aggro_radius;
            // C++ `Creature::Creature` initializes `m_wanderDistance` to 0.0f and only
            // random movement spawns get a positive distance from CreatureData.
            ai.wander_radius = 0.0;
            ai.respawn_time_secs = 30;
            ai.min_damage = min_dmg;
            ai.max_damage = max_dmg;
        }

        let create_data = CreatureCreateData {
            guid,
            entry,
            display_id,
            native_display_id: display_id,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.5,
            health: hp as i64,
            max_health: hp as i64,
            level,
            faction_template: faction as i32,
            npc_flags: npc_flags as u64,
            unit_flags,
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: Self::health_aura_state_like_cpp(hp as u64, hp as u64, hp > 0),
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            display_power: 1,
            power: [0; 10],
            max_power: [0; 10],
            base_mana: 0,
            virtual_items: [(0, 0, 0); 3],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: 0,
            vehicle_id: 0,
            play_hover_anim: false,
            hover_height: 1.0,
            mount_display_id: 0,
            stand_state: 0,
            vis_flags: 0,
            anim_tier: 0,
            emote_state: 0,
            sheathe_state: wow_constants::unit::SheathState::Melee as u8,
            pvp_flags: 0,
            current_area_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };

        Self::from_canonical(creature, create_data)
    }

    pub fn from_canonical(mut creature: Creature, mut create_data: CreatureCreateData) -> Self {
        // This generic bridge carries no proof that every aura source was
        // hydrated. Preserve fail-closed semantics even if a caller passes a
        // clone that previously crossed a more authoritative boundary.
        creature
            .unit_mut()
            .subsystems_mut()
            .auras
            .invalidate_spell_hit_aura_authority_like_cpp();
        let ai = creature.ai_ownership();
        create_data.npc_flags = (u64::from(ai.npc_flags2) << 32) | u64::from(ai.npc_flags);
        create_data.unit_flags = ai.unit_flags;
        create_data.unit_flags2 = ai.unit_flags2;
        create_data.unit_flags3 = ai.unit_flags3;
        create_data.damage_school = creature.melee_damage_school_like_cpp();
        create_data.ai_anim_kit_id = creature.unit().ai_anim_kit_id_like_cpp();
        create_data.movement_anim_kit_id = creature.unit().movement_anim_kit_id_like_cpp();
        create_data.melee_anim_kit_id = creature.unit().melee_anim_kit_id_like_cpp();
        let _ = creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .add_to_world_like_cpp();
        let runtime_motion_master = Self::new_runtime_motion_master_like_cpp(&creature);
        Self {
            creature,
            create_data,
            active_move_spline: None,
            active_random_generator: None,
            active_random_path_poly_refs: Vec::new(),
            active_home_generator: None,
            active_chase_generator: None,
            active_chase_path_poly_refs: Vec::new(),
            active_waypoint_generator: None,
            active_waypoint_random_at_path_end: None,
            runtime_motion_master,
            runtime_chase_target: None,
            runtime_represented_active: None,
            pending_assistance_like_cpp: Vec::new(),
            assistance_called_like_cpp: false,
            active_taunts_like_cpp: Vec::new(),
            creature_spell_due_at_ms_like_cpp: [None; wow_entities::MAX_CREATURE_SPELLS],
            creature_spell_schedule_initialized_like_cpp: false,
            creature_spell_engagement_epoch_like_cpp: 0,
            home_health_restored_pending_like_cpp: false,
            runtime_motion_master_ticks: 0,
            runtime_rng_authority_complete_like_cpp: true,
            respawn_spell_hit_aura_source_authority_like_cpp: false,
            respawn_spell_cast_log_aura_source_authority_like_cpp: false,
            runtime_rng_like_cpp: StdRng::from_entropy(),
            clock_started_at: Instant::now(),
        }
    }

    pub fn create_data_from_canonical_like_cpp(creature: &Creature) -> CreatureCreateData {
        let unit = creature.unit();
        let data = unit.data();
        let object = unit.world().object();
        let npc_flags = unit.npc_flags_like_cpp();
        let attack_speed = unit.base_attack_speed();
        let speed_rate = unit.speed_rate();
        let vehicle_id = unit
            .subsystems()
            .vehicle
            .kit
            .as_ref()
            .map(|kit| kit.kit_id())
            .unwrap_or(0);

        CreatureCreateData {
            guid: creature.guid(),
            entry: creature.entry(),
            display_id: data.display_id.max(0) as u32,
            native_display_id: data.native_display_id.max(0) as u32,
            display_scale: data.display_scale,
            native_x_display_scale: data.native_display_scale,
            bounding_radius: data.bounding_radius,
            combat_reach: data.combat_reach,
            health: creature.current_health().min(i64::MAX as u64) as i64,
            max_health: creature.max_health().min(i64::MAX as u64) as i64,
            level: creature.level(),
            faction_template: data.faction_template,
            npc_flags: (u64::from(npc_flags[1]) << 32) | u64::from(npc_flags[0]),
            unit_flags: data.flags,
            unit_flags2: data.flags2,
            unit_flags3: data.flags3,
            aura_state: Self::health_aura_state_like_cpp(
                creature.current_health(),
                creature.max_health(),
                creature.current_health() > 0,
            ),
            damage_school: creature.melee_damage_school_like_cpp(),
            scale: object.scale(),
            unit_class: data.class_id,
            display_power: data.display_power,
            power: data.power,
            max_power: data.max_power,
            base_mana: data.base_mana,
            virtual_items: [
                (
                    data.virtual_items[0].item_id,
                    data.virtual_items[0].item_appearance_mod_id,
                    data.virtual_items[0].item_visual,
                ),
                (
                    data.virtual_items[1].item_id,
                    data.virtual_items[1].item_appearance_mod_id,
                    data.virtual_items[1].item_visual,
                ),
                (
                    data.virtual_items[2].item_id,
                    data.virtual_items[2].item_appearance_mod_id,
                    data.virtual_items[2].item_visual,
                ),
            ],
            // C++ guarantees UNIT_FIELD_BASEATTACKTIME is never 0: ObjectMgr.cpp:1100-1104
            // clamps creature_template BaseAttackTime/RangeAttackTime 0 -> BASE_ATTACK_TIME
            // (2000) at load. The 3.4.3 client divides by this on the first post-spawn unit
            // tick (swing-timer/attack-rate math), so a 0 here crashes the client a few
            // seconds after the create burst. Defense-in-depth clamp mirroring C++.
            base_attack_time: match attack_speed[WeaponAttackType::BaseAttack as usize] {
                0 => BASE_ATTACK_TIME_LIKE_CPP,
                t => t,
            },
            ranged_attack_time: match attack_speed[WeaponAttackType::RangedAttack as usize] {
                0 => BASE_ATTACK_TIME_LIKE_CPP,
                t => t,
            },
            movement_flags: creature.movement_flags_like_cpp().bits(),
            vehicle_id,
            play_hover_anim: false,
            hover_height: data.hover_height,
            mount_display_id: data.mount_display_id,
            stand_state: data.stand_state,
            vis_flags: data.vis_flags,
            anim_tier: data.anim_tier,
            emote_state: unit.emote_state_like_cpp() as i32,
            sheathe_state: data.sheathe_state,
            pvp_flags: data.pvp_flags,
            current_area_id: 0,
            speed_walk_rate: speed_rate[UnitMoveType::Walk as usize],
            speed_run_rate: speed_rate[UnitMoveType::Run as usize],
            ai_anim_kit_id: unit.ai_anim_kit_id_like_cpp(),
            movement_anim_kit_id: unit.movement_anim_kit_id_like_cpp(),
            melee_anim_kit_id: unit.melee_anim_kit_id_like_cpp(),
        }
    }

    pub fn from_loaded_grid_canonical_like_cpp(
        creature: Creature,
        mut waypoint_path_resolver: impl FnMut(u32) -> Option<WaypointPath>,
    ) -> Self {
        let create_data = Self::create_data_from_canonical_like_cpp(&creature);
        let mut world_creature = Self::from_canonical(creature, create_data);
        // The loaded-grid lifecycle receives a Creature only after the
        // DB-backed creature_addon/template_addon store has resolved and the
        // selected addon has been applied to its canonical AuraSubsystem.
        world_creature.restore_respawn_aura_source_authority_like_cpp(true, true);
        match world_creature.creature.default_movement_type() {
            wow_entities::MovementGeneratorType::Random => {
                world_creature.initialize_default_random_movement_like_cpp();
            }
            wow_entities::MovementGeneratorType::Waypoint => {
                world_creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(
                    |path_id| waypoint_path_resolver(path_id),
                );
            }
            wow_entities::MovementGeneratorType::Idle => {}
        }
        world_creature
    }

    pub fn visibility_range_like_cpp(&self) -> f32 {
        self.creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp()
            .unwrap_or(VISIBILITY_RADIUS)
    }

    pub(crate) fn now_ms(&self) -> u64 {
        self.clock_started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }

    #[cfg(test)]
    pub(crate) fn backdate_runtime_clock_for_test(&mut self, elapsed: Duration) {
        self.clock_started_at = Instant::now() - elapsed;
    }

    pub fn guid(&self) -> ObjectGuid {
        self.creature.ai_guid()
    }

    pub fn entry(&self) -> u32 {
        self.creature.ai_entry()
    }

    pub fn map_id(&self) -> u32 {
        self.creature.unit().world().map_id()
    }

    pub fn instance_id(&self) -> u32 {
        self.creature.unit().world().instance_id()
    }

    pub fn phase_shift(&self) -> &PhaseShift {
        self.creature.unit().world().phase_shift()
    }

    pub fn is_alive(&self) -> bool {
        self.creature.ai_is_alive()
    }

    pub fn current_hp(&self) -> u32 {
        self.creature.ai_current_health().min(u64::from(u32::MAX)) as u32
    }

    pub fn max_hp(&self) -> u32 {
        self.creature.ai_max_health().min(u64::from(u32::MAX)) as u32
    }

    pub fn level(&self) -> u8 {
        self.creature.ai_level()
    }

    pub fn npc_flags(&self) -> u32 {
        self.creature.ai_ownership().npc_flags
    }

    pub fn npc_flags2(&self) -> u32 {
        self.creature.ai_ownership().npc_flags2
    }

    pub fn unit_flags2_like_cpp(&self) -> UnitFlags2 {
        self.creature.unit().unit_flags2_like_cpp()
    }

    pub fn trainer_class_like_cpp(&self) -> u8 {
        self.creature.trainer_class_like_cpp()
    }

    pub fn npc_flags_mask_like_cpp(&self) -> u64 {
        (u64::from(self.npc_flags2()) << 32) | u64::from(self.npc_flags())
    }

    pub fn unit_flags(&self) -> u32 {
        self.creature.ai_ownership().unit_flags
    }

    pub fn display_id(&self) -> u32 {
        self.creature.ai_ownership().display_id
    }

    pub fn faction(&self) -> u32 {
        self.creature.ai_ownership().faction
    }

    pub fn min_dmg(&self) -> u32 {
        self.creature.ai_ownership().min_damage
    }

    pub fn max_dmg(&self) -> u32 {
        self.creature.ai_ownership().max_damage
    }

    pub fn loot_id(&self) -> u32 {
        self.creature.ai_ownership().loot_id
    }

    pub fn skin_loot_id(&self) -> u32 {
        self.creature.ai_ownership().skin_loot_id
    }

    pub fn gold_min(&self) -> u32 {
        self.creature.ai_ownership().gold_min
    }

    pub fn gold_max(&self) -> u32 {
        self.creature.ai_ownership().gold_max
    }

    pub fn boss_id(&self) -> Option<u32> {
        self.creature.ai_ownership().boss_id
    }

    pub fn dungeon_encounter_id(&self) -> u32 {
        self.creature.ai_ownership().dungeon_encounter_id
    }

    pub fn state(&self) -> CreatureAiState {
        self.creature.ai_state()
    }

    pub fn corpse_delay_secs_like_cpp(&self) -> u32 {
        self.creature.corpse_delay()
    }

    pub fn ignore_corpse_decay_ratio_like_cpp(&self) -> bool {
        self.creature.ignore_corpse_decay_ratio()
    }

    pub(super) fn finalize_runtime_represented_generator_like_cpp(
        &mut self,
        mut generator: MovementGeneratorRef,
    ) {
        match generator.kind {
            MovementGeneratorKind::Point => {
                let finalize = generator.finalize_point_like_cpp(true, true);
                if finalize.clear_roaming_move {
                    self.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                }
                if let Some(inform) = finalize.inform {
                    self.creature
                        .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            MovementGeneratorKind::Rotate => {
                if let Some(inform) = generator.finalize_rotate_like_cpp(true, true).inform {
                    self.creature
                        .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            MovementGeneratorKind::Distract => {
                let finalize = generator.finalize_distract_like_cpp(true, true);
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
            }
            MovementGeneratorKind::Effect => {
                if let Some(inform) = generator.finalize_generic_like_cpp(true) {
                    self.creature
                        .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            _ => {}
        }
    }

    pub fn apply_corpse_loot_flags_after_death_state_like_cpp(
        &mut self,
        lootable: bool,
        can_skin: bool,
    ) {
        self.creature
            .apply_corpse_loot_flags_after_death_state_like_cpp(lootable, can_skin);
    }

    pub fn force_dynamic_flags_update_like_cpp(&mut self) {
        self.creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .force_dynamic_flags_update_like_cpp();
    }

    pub fn has_lootable_dynamic_flag_like_cpp(&self) -> bool {
        self.creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    }

    pub fn die(&mut self) {
        self.creature.mark_ai_dead(self.now_ms());
    }

    pub(super) fn walk_speed_like_cpp(&self) -> f32 {
        (self.create_data.speed_walk_rate * 2.5).max(0.01)
    }

    pub(super) fn run_speed_like_cpp(&self) -> f32 {
        (self.create_data.speed_run_rate * 7.0).max(0.01)
    }

    /// Owner capabilities `PathGenerator::BuildPolyPath` reads off `_source`
    /// when a position has no navmesh polygon: `Creature::CanFly()`
    /// (`Creature.h:126`), `Creature::CanSwim()` (`Creature.cpp:2912-2921`) and
    /// `Unit::IsFalling()` (`Unit.cpp:12173-12176`, movement flags **or** the
    /// active spline falling).
    pub fn detour_owner_capabilities_like_cpp(&self) -> DetourOwnerCapabilitiesLikeCpp {
        let spline_falling = self
            .active_move_spline
            .as_ref()
            .is_some_and(|spline| spline.flags().contains(MoveSplineFlag::FALLING));
        DetourOwnerCapabilitiesLikeCpp {
            can_fly: self.creature.can_fly_like_cpp(),
            can_swim: self.creature.can_swim_like_cpp(),
            is_falling: self
                .creature
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
    pub(super) fn near_point_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
        distance_2d: f32,
        absolute_angle: f32,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position {
        let effective_reach =
            target.combat_reach + self.creature.unit().data().combat_reach.max(0.0);
        let radius = effective_reach + distance_2d;
        let point = Position::new(
            target.position.x + radius * absolute_angle.cos(),
            target.position.y + radius * absolute_angle.sin(),
            target.position.z,
            0.0,
        );
        self.normalize_path_position_z_like_cpp(point, terrain)
    }

    pub(super) fn random_unit_snapshot_like_cpp(
        &self,
        has_los_to_destination: bool,
        path_result: RandomPathResult,
        distance_roll: f32,
        angle_roll: f32,
        next_wander_steps_roll: u8,
        pause_seconds_roll: i32,
        travel_time_ms: i32,
    ) -> RandomUnitSnapshot {
        let random_type = match self.creature.random_movement_type_like_cpp() {
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
            owner_unit_state: self.creature.unit().unit_state(),
            movement_prevented_by_casting: self
                .creature
                .unit()
                .has_unit_state(UnitState::CASTING.bits()),
            move_spline_finalized: self
                .active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            owner_wander_distance: self.creature.ai_ownership().wander_radius,
            has_los_to_destination,
            path_result,
            movement_template: random_type,
            owner_is_walking: self
                .creature
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

    pub fn can_swing(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::InCombat
            && self
                .now_ms()
                .saturating_sub(self.creature.ai_ownership().last_swing_ms)
                >= self.creature.ai_ownership().swing_timer_ms
    }

    pub fn record_swing(&mut self) {
        let now_ms = self.now_ms();
        let base_attack_time = if self.create_data.base_attack_time > 0 {
            self.create_data.base_attack_time as u64
        } else {
            self.creature.ai_ownership().swing_timer_ms.max(1)
        };
        let ai = self.creature.ai_ownership_mut();
        ai.last_swing_ms = now_ms;
        ai.swing_timer_ms = base_attack_time;
    }

    pub fn record_failed_swing_retry_like_cpp(&mut self) {
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.last_swing_ms = now_ms;
        ai.swing_timer_ms = 100;
    }

    pub(crate) fn runtime_rng_authority_complete_like_cpp(&self) -> bool {
        self.runtime_rng_authority_complete_like_cpp
    }

    /// Permanently tombstone exact creature-spell RNG authority for this loaded
    /// creature. C++ keeps the same generator across combat resets, so neither
    /// a new target nor a new engagement epoch can restore a provable draw
    /// position. Existing transitional melee and movement continue to consume
    /// their best-effort stream so an unrepresented spell cannot freeze normal
    /// gameplay.
    pub(crate) fn invalidate_runtime_rng_authority_like_cpp(&mut self) {
        self.runtime_rng_authority_complete_like_cpp = false;
    }

    #[cfg(test)]
    pub fn seed_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }
}

impl MapInstance {
    pub fn new(map_id: u16, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
            grids: HashMap::new(),
            grid_unload_timeout: DEFAULT_GRID_UNLOAD_TIME,
            personal_phases: MultiPersonalPhaseTracker::default(),
            personal_phase_objects_to_remove: HashSet::new(),
            persisted_respawn_times: HashMap::new(),
            respawn_queue: Vec::new(),
        }
    }

    pub fn get_or_create_grid(&mut self, x: i16, y: i16) -> &mut Grid {
        let coord = GridCoord::new(x, y);
        if !self.grids.contains_key(&coord) {
            let grid = Grid::new(x, y);
            self.grids.insert(coord, grid);
            debug!(
                "Created new grid ({}, {}) for map {} instance {}",
                x, y, self.map_id, self.instance_id
            );
        }
        self.grids.get_mut(&coord).unwrap()
    }

    pub fn get_grid(&self, x: i16, y: i16) -> Option<&Grid> {
        self.grids.get(&GridCoord::new(x, y))
    }

    pub fn get_grid_mut(&mut self, x: i16, y: i16) -> Option<&mut Grid> {
        self.grids.get_mut(&GridCoord::new(x, y))
    }

    pub fn add_creature(&mut self, x: i16, y: i16, creature: WorldCreature) -> bool {
        self.get_or_create_grid(x, y).add_creature(creature)
    }

    pub fn get_creature(&self, x: i16, y: i16, guid: ObjectGuid) -> Option<&WorldCreature> {
        self.get_grid(x, y)?.get_creature(guid)
    }

    pub fn get_creature_mut(
        &mut self,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        self.get_grid_mut(x, y)?.get_creature_mut(guid)
    }

    pub fn unload_empty_grids(&mut self) {
        let to_remove: Vec<GridCoord> = self
            .grids
            .iter()
            .filter(|(_, grid)| grid.should_unload(self.grid_unload_timeout))
            .map(|(coord, _)| *coord)
            .collect();

        for coord in to_remove {
            info!(
                "Unloading grid {:?} from map {} (timeout)",
                coord, self.map_id
            );
            self.grids.remove(&coord);
            self.personal_phases
                .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
        }
    }

    pub fn creature_count(&self) -> usize {
        self.grids.values().map(|g| g.creature_count()).sum()
    }

    pub fn is_grid_loaded(&self, x: i16, y: i16) -> bool {
        self.get_grid(x, y).is_some()
    }

    pub fn min_height_like_cpp(&self, _x: f32, _y: f32) -> f32 {
        DEFAULT_MIN_HEIGHT_LIKE_CPP
    }

    pub fn load_personal_phase_grid_like_cpp(
        &mut self,
        phase_shift: &PhaseShift,
        x: i16,
        y: i16,
        has_personal_spawns: impl FnMut(u32) -> bool,
        load_phase: impl FnMut(ObjectGuid, u32),
    ) -> bool {
        self.get_or_create_grid(x, y);
        self.personal_phases.load_grid_like_cpp(
            phase_shift,
            GridCoord::new(x, y).personal_phase_grid_id_like_cpp(),
            has_personal_spawns,
            load_phase,
        )
    }

    pub fn update_personal_phases_for_owner_like_cpp(
        &mut self,
        phase_owner: ObjectGuid,
        phase_shift: &PhaseShift,
        grid: Option<GridCoord>,
        has_personal_spawns: impl FnMut(u32) -> bool,
        load_phase: impl FnMut(ObjectGuid, u32),
    ) -> bool {
        self.personal_phases.on_owner_phase_changed_like_cpp(
            phase_owner,
            phase_shift,
            grid.map(|coord| coord.personal_phase_grid_id_like_cpp()),
            has_personal_spawns,
            load_phase,
        )
    }

    pub fn register_personal_phase_object_like_cpp(
        &mut self,
        phase_id: u32,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phases
            .register_tracked_object_like_cpp(phase_id, phase_owner, object);
    }

    pub fn unregister_personal_phase_object_like_cpp(
        &mut self,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phases
            .unregister_tracked_object_like_cpp(phase_owner, object);
    }

    pub fn mark_personal_phases_for_deletion_like_cpp(&mut self, phase_owner: ObjectGuid) {
        self.personal_phases
            .mark_all_phases_for_deletion_like_cpp(phase_owner);
    }

    pub fn update_personal_phases_like_cpp(&mut self, diff: Duration) {
        let mut objects_to_remove = Vec::new();
        self.personal_phases
            .update_like_cpp(diff, |guid| objects_to_remove.push(guid));
        self.personal_phase_objects_to_remove
            .extend(objects_to_remove);
    }
}

impl MapManager {
    pub fn new() -> Self {
        let mut manager = Self {
            maps: HashMap::new(),
            free_instance_ids: Vec::new(),
            next_instance_id: 1,
            tick_owner: RuntimeTickOwner::Session,
            terrain: None,
        };
        manager.init_instance_ids_from_max(0);
        manager
    }

    /// Attach the shared, file-backed terrain height store (server startup).
    pub fn set_terrain(&mut self, terrain: Arc<LiveTerrainHeights>) {
        self.terrain = Some(terrain);
    }

    /// Shared terrain height store, if wired. Cloned so callers can use it while
    /// still holding `&mut self` for the spawn/respawn mutation.
    #[must_use]
    pub fn terrain(&self) -> Option<Arc<LiveTerrainHeights>> {
        self.terrain.clone()
    }

    /// Returns the current tick owner for this map manager.
    ///
    /// Returns a `Copy` value; the caller should read this once and release the
    /// lock before performing any tick work.
    pub fn tick_owner(&self) -> RuntimeTickOwner {
        self.tick_owner
    }

    /// Sets the tick owner.
    ///
    /// Production calls this exactly once, at startup
    /// (`crates/world-server/src/app.rs`), *before* the global legacy creature
    /// loop is spawned. Flipping it after the loop is running is the only
    /// window in which both the loop and a session can tick the same creature,
    /// so the single call site is asserted by a test rather than left to
    /// convention (#28).
    pub fn set_tick_owner(&mut self, owner: RuntimeTickOwner) {
        self.tick_owner = owner;
    }

    /// Returns the `(map_id, instance_id)` keys of all currently active map
    /// instances held by this manager.
    ///
    /// The key type matches `self.maps: HashMap<(u16, u32), MapInstance>` exactly.
    /// Order is unspecified (hash map iteration order).
    pub fn active_map_keys(&self) -> Vec<(u16, u32)> {
        self.maps.keys().copied().collect()
    }

    pub fn init_instance_ids_from_max(&mut self, max_existing_instance_id: u32) {
        self.next_instance_id = 1;
        self.free_instance_ids = vec![true; max_existing_instance_id.saturating_add(2) as usize];
        self.free_instance_ids[0] = false;
    }

    pub fn register_instance_id(&mut self, instance_id: u32) {
        let index = instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(2), true);
        }

        self.free_instance_ids[index] = false;

        if self.next_instance_id == instance_id {
            self.next_instance_id = self.next_instance_id.saturating_add(1);
        }
    }

    pub fn generate_instance_id(&mut self) -> Option<u32> {
        if self.next_instance_id == u32::MAX {
            return None;
        }

        let new_instance_id = self.next_instance_id;
        let index = new_instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(1), true);
        }
        self.free_instance_ids[index] = false;

        let search_start = self.next_instance_id.saturating_add(1) as usize;
        if let Some(next_free_offset) = self.free_instance_ids[search_start..]
            .iter()
            .position(|is_free| *is_free)
        {
            self.next_instance_id = (search_start + next_free_offset) as u32;
        } else {
            self.next_instance_id = self.free_instance_ids.len() as u32;
            self.free_instance_ids.push(true);
        }

        Some(new_instance_id)
    }

    pub fn free_instance_id(&mut self, instance_id: u32) {
        if instance_id == 0 {
            if self.free_instance_ids.is_empty() {
                self.init_instance_ids_from_max(0);
            } else {
                self.free_instance_ids[0] = false;
            }
            return;
        }

        let index = instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(2), true);
        }

        self.next_instance_id = self.next_instance_id.min(instance_id);
        self.free_instance_ids[index] = true;
        self.free_instance_ids[0] = false;
    }

    pub fn get_or_create_map(&mut self, map_id: u16, instance_id: u32) -> &mut MapInstance {
        let key = (map_id, instance_id);
        if !self.maps.contains_key(&key) {
            let instance = MapInstance::new(map_id, instance_id);
            self.maps.insert(key, instance);
            info!(
                "Created new map instance: map_id={}, instance_id={}",
                map_id, instance_id
            );
        }
        self.maps.get_mut(&key).unwrap()
    }

    pub fn get_map(&self, map_id: u16, instance_id: u32) -> Option<&MapInstance> {
        self.maps.get(&(map_id, instance_id))
    }

    pub fn get_map_mut(&mut self, map_id: u16, instance_id: u32) -> Option<&mut MapInstance> {
        self.maps.get_mut(&(map_id, instance_id))
    }

    // Convenience methods that delegate to MapInstance

    pub fn get_grid(&self, map_id: u16, instance_id: u32, x: i16, y: i16) -> Option<&Grid> {
        self.get_map(map_id, instance_id)?.get_grid(x, y)
    }

    pub fn get_grid_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
    ) -> Option<&mut Grid> {
        self.get_map_mut(map_id, instance_id)?.get_grid_mut(x, y)
    }

    pub fn get_or_create_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
    ) -> &mut Grid {
        self.get_or_create_map(map_id, instance_id)
            .get_or_create_grid(x, y)
    }

    pub fn add_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        mut creature: WorldCreature,
    ) -> bool {
        let _ = creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(u32::from(map_id), instance_id);
        creature
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        self.get_or_create_map(map_id, instance_id)
            .add_creature(x, y, creature)
    }

    pub fn get_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&WorldCreature> {
        self.get_map(map_id, instance_id)?.get_creature(x, y, guid)
    }

    pub fn get_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        self.get_map_mut(map_id, instance_id)?
            .get_creature_mut(x, y, guid)
    }

    pub fn find_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&WorldCreature> {
        let map = self.get_map(map_id, instance_id)?;
        map.grids.values().find_map(|grid| grid.get_creature(guid))
    }

    pub fn find_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.grids
            .values_mut()
            .find_map(|grid| grid.get_creature_mut(guid))
    }

    pub fn set_creature_anim_kit_id_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
        slot: CreatureAnimKitSlotLikeCpp,
        anim_kit_id: u16,
        anim_kit_exists: impl Fn(u16) -> bool,
    ) -> Option<RuntimeEvent> {
        use wow_packet::ServerPacket;

        let creature = self.find_creature_mut(map_id, instance_id, guid)?;
        if anim_kit_id != 0 && !anim_kit_exists(anim_kit_id) {
            return None;
        }

        let changed = match slot {
            CreatureAnimKitSlotLikeCpp::Ai => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_ai_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.ai_anim_kit_id = anim_kit_id;
                }
                changed
            }
            CreatureAnimKitSlotLikeCpp::Movement => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_movement_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.movement_anim_kit_id = anim_kit_id;
                }
                changed
            }
            CreatureAnimKitSlotLikeCpp::Melee => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_melee_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.melee_anim_kit_id = anim_kit_id;
                }
                changed
            }
        };
        if !changed {
            return None;
        }

        let packet_bytes = match slot {
            CreatureAnimKitSlotLikeCpp::Ai => wow_packet::packets::misc::SetAiAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
            CreatureAnimKitSlotLikeCpp::Movement => wow_packet::packets::misc::SetMovementAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
            CreatureAnimKitSlotLikeCpp::Melee => wow_packet::packets::misc::SetMeleeAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
        };
        let source_position = creature.position();
        let range = creature.visibility_range_like_cpp();
        Some(RuntimeEvent {
            source_guid: guid,
            recipients: RecipientRule::NearbyVisible {
                source_guid: guid,
                map_id,
                instance_id,
                source_position,
                range,
                required_3d: false,
            },
            packet_bytes,
        })
    }

    pub fn creature_guids(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid> {
        self.get_map(map_id, instance_id)
            .map(|map| {
                map.grids
                    .values()
                    .flat_map(|grid| grid.creatures.keys().copied())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn active_creature_guids_for_player_update_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        player_position: Position,
        player_phase_shift: &PhaseShift,
    ) -> Vec<ObjectGuid> {
        let Some(map) = self.get_map(map_id, instance_id) else {
            return Vec::new();
        };
        let (low, high) = calculate_cell_area_like_cpp(player_position, VISIBILITY_RADIUS);
        let mut guids = Vec::new();

        for grid in map.grids.values() {
            for creature in grid.creatures.values() {
                if !creature.creature.unit().world().object().is_in_world() {
                    continue;
                }
                if !player_phase_shift.can_see(creature.phase_shift()) {
                    continue;
                }
                let Some(cell) =
                    cell_area_contains_position_like_cpp(low, high, creature.position())
                else {
                    continue;
                };
                guids.push((cell, creature.guid()));
            }
        }

        guids.sort_by_key(|(cell, guid)| (cell.x, cell.y, guid.high_value(), guid.low_value()));
        guids.into_iter().map(|(_, guid)| guid).collect()
    }

    pub fn with_creature_mut<F, R>(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut WorldCreature) -> R,
    {
        self.get_map_mut(map_id, instance_id)?
            .get_grid_mut(x, y)?
            .get_creature_mut(guid)
            .map(f)
    }

    // ── Respawn queue delegates (Slice 4A.2a) ─────────────────────────────────

    pub fn player_enter_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        player_guid: ObjectGuid,
        _pos: Position,
    ) {
        let grid = self.get_or_create_grid(map_id, instance_id, x, y);
        grid.player_enter(player_guid);
        debug!(
            "Player {:?} entered grid ({}, {}) in map {}",
            player_guid, x, y, map_id
        );
    }

    pub fn player_leave_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        player_guid: ObjectGuid,
    ) {
        if let Some(grid) = self.get_grid_mut(map_id, instance_id, x, y) {
            grid.player_leave(player_guid);
            debug!(
                "Player {:?} left grid ({}, {}) in map {}",
                player_guid, x, y, map_id
            );
        }
    }

    pub fn get_visible_creatures(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        _z: f32,
    ) -> Vec<WorldCreature> {
        self.get_visible_creatures_in_phase(map_id, instance_id, x, y, _z, VISIBILITY_RADIUS, None)
    }

    pub fn get_visible_creatures_in_phase(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        z: f32,
        visibility_range: f32,
        seer_phase_shift: Option<&PhaseShift>,
    ) -> Vec<WorldCreature> {
        let center_x = world_to_grid_x(x);
        let center_y = world_to_grid_y(y);

        let mut creatures = Vec::new();

        // Get creatures from 3x3 grid area
        for dx in -1..=1 {
            for dy in -1..=1 {
                let grid_x = center_x + dx;
                let grid_y = center_y + dy;

                if let Some(grid) = self.get_grid(map_id, instance_id, grid_x, grid_y) {
                    for creature in grid.creatures.values() {
                        if let Some(seer_phase_shift) = seer_phase_shift
                            && !seer_phase_shift.can_see(creature.phase_shift())
                        {
                            continue;
                        }

                        // C++ `CanSeeOrDetect(..., distanceCheck=true)` uses
                        // `IsWithinDist(..., is3D=false)` for visibility
                        // (`Object.cpp:1609`). Keep the legacy map path aligned
                        // with the canonical map visibility path.
                        let dist = Position::new(x, y, z, 0.0).distance_2d(&creature.position());
                        if dist <= visibility_range {
                            creatures.push(creature.clone());
                        }
                    }
                }
            }
        }

        creatures
    }

    pub fn unload_distant_grids(
        &mut self,
        map_id: u16,
        instance_id: u32,
        center_x: i16,
        center_y: i16,
        range: i16,
    ) {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            let to_remove: Vec<GridCoord> = map
                .grids
                .keys()
                .filter(|coord| {
                    let dx = (coord.x - center_x).abs();
                    let dy = (coord.y - center_y).abs();
                    dx > range || dy > range
                })
                .copied()
                .collect();

            for coord in to_remove {
                if let Some(grid) = map.grids.get(&coord) {
                    if grid.should_unload(map.grid_unload_timeout) {
                        info!("Unloading distant grid {:?} from map {}", coord, map_id);
                        map.grids.remove(&coord);
                        map.personal_phases
                            .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
                    }
                }
            }
        }
    }

    pub fn is_grid_loaded(&self, map_id: u16, instance_id: u32, x: i16, y: i16) -> bool {
        self.get_map(map_id, instance_id)
            .map(|m| m.is_grid_loaded(x, y))
            .unwrap_or(false)
    }

    pub fn min_height_like_cpp(&self, map_id: u16, instance_id: u32, x: f32, y: f32) -> f32 {
        self.get_map(map_id, instance_id)
            .map(|m| m.min_height_like_cpp(x, y))
            .unwrap_or(DEFAULT_MIN_HEIGHT_LIKE_CPP)
    }

    pub fn create_grid(&mut self, map_id: u16, instance_id: u32, x: i16, y: i16) -> &mut Grid {
        self.get_or_create_grid(map_id, instance_id, x, y)
    }

    pub fn creature_count(&self) -> usize {
        self.maps.values().map(|m| m.creature_count()).sum()
    }
}
