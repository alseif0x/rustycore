//! Creature packets.
//!
//! Separated from runtime.rs under #701.

use super::*;

impl WorldCreature {
    pub(in crate::map_manager) fn runtime_default_generator_like_cpp(
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
            runtime_elapsed_ms_like_cpp: 0,
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

    pub(crate) const fn runtime_elapsed_ms_like_cpp(&self) -> u64 {
        self.runtime_elapsed_ms_like_cpp
    }

    pub(crate) fn advance_runtime_clock_like_cpp(&mut self, diff_ms: u32) {
        self.runtime_elapsed_ms_like_cpp = self
            .runtime_elapsed_ms_like_cpp
            .saturating_add(u64::from(diff_ms));
    }

    #[cfg(test)]
    pub(crate) fn backdate_runtime_clock_for_test(&mut self, elapsed: Duration) {
        self.runtime_elapsed_ms_like_cpp = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
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

    pub(in crate::map_manager) fn finalize_runtime_represented_generator_like_cpp(
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
        self.creature
            .mark_ai_dead(self.runtime_elapsed_ms_like_cpp());
    }

    pub(in crate::map_manager) fn walk_speed_like_cpp(&self) -> f32 {
        (self.create_data.speed_walk_rate * 2.5).max(0.01)
    }

    pub(in crate::map_manager) fn run_speed_like_cpp(&self) -> f32 {
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
    pub(in crate::map_manager) fn near_point_like_cpp(
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

    pub(in crate::map_manager) fn random_unit_snapshot_like_cpp(
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
                .runtime_elapsed_ms_like_cpp()
                .saturating_sub(self.creature.ai_ownership().last_swing_ms)
                >= self.creature.ai_ownership().swing_timer_ms
    }

    pub fn record_swing(&mut self) {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
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
        let now_ms = self.runtime_elapsed_ms_like_cpp();
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

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn seed_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }
}
