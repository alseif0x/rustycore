//! Creature lifecycle and runtime state operations, part 2 of 3.
//!
//! The inherent `Creature` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Creature {
    pub fn reset_ai_combat(&mut self, now_ms: u64) {
        if self.ai_ownership.state == CreatureAiState::Dead
            || self.unit.is_dead()
            || self.unit.data().health == 0
        {
            self.advance_loot_lifecycle_revision_like_cpp();
        }
        self.ai_ownership.state = CreatureAiState::Returning;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = Some(self.ai_ownership.home_position);
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.unit.set_attacking(None);
        self.last_damaged_time = 0;
    }
    /// Apply damage and return `true` when this call killed the creature.
    pub fn take_ai_damage(&mut self, damage: u32, now_ms: u64) -> bool {
        self.take_ai_damage_at_game_time_like_cpp(damage, now_ms, game_time_secs_like_cpp())
    }
    pub fn take_ai_damage_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        now_ms: u64,
        game_time_secs: i64,
    ) -> bool {
        if self.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            damage,
            now_ms,
            game_time_secs,
        ) {
            self.mark_ai_dead_at_game_time_like_cpp(now_ms, game_time_secs);
            true
        } else {
            false
        }
    }
    pub fn apply_ai_damage_before_death_state_like_cpp(
        &mut self,
        damage: u32,
        now_ms: u64,
    ) -> bool {
        self.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            damage,
            now_ms,
            game_time_secs_like_cpp(),
        )
    }
    pub fn apply_ai_damage_before_death_state_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        now_ms: u64,
        game_time_secs: i64,
    ) -> bool {
        if !self.ai_is_alive() {
            return false;
        }

        let remaining = self.ai_current_health().saturating_sub(u64::from(damage));
        self.unit.set_health(remaining);
        if remaining > 0
            && damage > 0
            && !self
                .unit
                .subsystems()
                .control
                .owner_guid
                .is_some_and(|owner_guid| owner_guid.is_player())
        {
            self.last_damaged_time =
                game_time_secs.saturating_add(MAX_AGGRO_RESET_TIME_SECS_LIKE_CPP);
        }
        if remaining == 0 {
            self.advance_loot_lifecycle_revision_like_cpp();
            self.ai_ownership.state = CreatureAiState::Dead;
            self.ai_ownership.combat_target = None;
            self.ai_ownership.move_target = None;
            self.ai_ownership.death_time_ms = Some(now_ms);
            self.respawn_delay =
                self.ai_ownership.respawn_time_secs.min(u64::from(u32::MAX)) as u32;
            true
        } else {
            false
        }
    }
    pub fn mark_ai_dead(&mut self, now_ms: u64) {
        self.mark_ai_dead_at_game_time_like_cpp(now_ms, game_time_secs_like_cpp());
    }
    pub fn mark_ai_dead_at_game_time_like_cpp(&mut self, now_ms: u64, game_time_secs: i64) {
        if self.ai_ownership.state != CreatureAiState::Dead {
            self.advance_loot_lifecycle_revision_like_cpp();
        }
        self.ai_ownership.state = CreatureAiState::Dead;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = None;
        self.ai_ownership.death_time_ms = Some(now_ms);
        self.ai_ownership.corpse_despawn_at_ms =
            Some(now_ms.saturating_add(u64::from(self.corpse_delay).saturating_mul(1_000)));
        self.unit.set_health(0);
        self.respawn_delay = self.ai_ownership.respawn_time_secs.min(u64::from(u32::MAX)) as u32;
        self.set_death_state_runtime(DeathState::JustDied, game_time_secs);
        self.unit.set_health(0);
    }
    pub fn complete_ai_death_state_after_kill_hooks_like_cpp(
        &mut self,
        now_ms: u64,
        game_time_secs: i64,
    ) {
        if self.ai_ownership.state != CreatureAiState::Dead || self.unit.is_dead() {
            return;
        }
        self.ai_ownership.death_time_ms = Some(now_ms);
        self.ai_ownership.corpse_despawn_at_ms =
            Some(now_ms.saturating_add(u64::from(self.corpse_delay).saturating_mul(1_000)));
        self.set_death_state_runtime(DeathState::JustDied, game_time_secs);
        self.unit.set_health(0);
    }
    pub fn apply_corpse_loot_flags_after_death_state_like_cpp(
        &mut self,
        lootable: bool,
        can_skin: bool,
    ) {
        if lootable {
            self.unit
                .world_mut()
                .object_mut()
                .set_dynamic_flag(UnitDynFlags::Lootable as u32);
        }
        if can_skin {
            self.unit
                .world_mut()
                .object_mut()
                .set_dynamic_flag(UnitDynFlags::CanSkin as u32);
            let mut flags = self.unit.unit_flags_like_cpp();
            flags.insert(UnitFlags::SKINNABLE);
            self.unit.set_unit_flags_like_cpp(flags);
        }
    }
    pub fn respawn_ai(&mut self, now_ms: u64) {
        self.ai_ownership.state = CreatureAiState::Idle;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = None;
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.last_swing_ms = now_ms;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.ai_ownership.spline_id = self.ai_ownership.spline_id.saturating_add(1);
        self.unit.set_death_state(DeathState::Alive);
        self.unit.set_health(self.unit.data().max_health);
        self.unit
            .world_mut()
            .relocate(self.ai_ownership.home_position);
        self.unit.set_attacking(None);
        self.last_damaged_time = 0;
    }
    pub fn can_ai_wander(&self) -> bool {
        !self.is_template_rooted_like_cpp()
            && (self.ai_ownership.npc_flags == 0 || (self.ai_ownership.npc_flags & 0x80) == 0)
    }
    pub fn try_ai_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        self.try_ai_aggro_with_target_combat_reach_like_cpp(player_guid, player_pos, 0.0)
    }
    pub fn try_ai_aggro_with_target_combat_reach_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
    ) -> bool {
        self.try_ai_aggro_with_effective_range_like_cpp(
            player_guid,
            player_pos,
            player_combat_reach,
            self.ai_ownership.aggro_radius,
        )
    }
    pub fn try_ai_aggro_with_effective_range_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
        effective_aggro_range: f32,
    ) -> bool {
        if !self.ai_is_alive()
            || matches!(
                self.ai_ownership.state,
                CreatureAiState::InCombat | CreatureAiState::Returning | CreatureAiState::Dead
            )
        {
            return false;
        }

        if !self.has_react_state(ReactState::Aggressive) {
            return false;
        }

        if self.is_civilian_like_cpp() {
            return false;
        }

        if self.ai_ownership.aggro_radius <= 0.0 {
            return false;
        }

        if self
            .unit
            .unit_flags_like_cpp()
            .contains(UnitFlags::IMMUNE_TO_PC)
        {
            return false;
        }

        if !self.can_start_attack_z_range_like_cpp(player_pos, player_combat_reach) {
            return false;
        }

        if self.ai_position().distance(player_pos) <= effective_aggro_range.max(0.0) {
            self.enter_ai_combat(player_guid);
            true
        } else {
            false
        }
    }
    pub fn can_start_attack_z_range_like_cpp(
        &self,
        target_position: &Position,
        target_combat_reach: f32,
    ) -> bool {
        if self.can_fly_like_cpp() {
            return true;
        }

        let distance_z = ((self.ai_position().z - target_position.z).abs()
            - self.unit.world().combat_reach()
            - target_combat_reach.max(0.0))
        .max(0.0);
        distance_z <= CREATURE_Z_ATTACK_RANGE_LIKE_CPP + self.combat_distance.max(0.0)
    }
    pub fn should_ai_respawn(&self, now_ms: u64) -> bool {
        self.ai_ownership
            .death_time_ms
            .map(|death_ms| {
                now_ms
                    >= death_ms
                        .saturating_add(self.ai_ownership.respawn_time_secs.saturating_mul(1_000))
            })
            .unwrap_or(false)
    }
    pub fn set_ai_corpse_despawn_at(&mut self, corpse_despawn_at_ms: Option<u64>) {
        self.ai_ownership.corpse_despawn_at_ms = corpse_despawn_at_ms;
    }
    pub fn set_ai_identity_runtime(
        &mut self,
        display_id: u32,
        faction: u32,
        npc_flags: u32,
        unit_flags: u32,
    ) {
        self.ai_ownership.display_id = display_id;
        self.ai_ownership.faction = faction;
        self.ai_ownership.npc_flags = npc_flags;
        self.ai_ownership.unit_flags = unit_flags;
        self.unit.set_npc_flags_like_cpp(npc_flags);
        self.unit
            .set_unit_flags_like_cpp(UnitFlags::from_bits_truncate(unit_flags));
        self.set_display_id(display_id, true, None);
        self.set_faction(faction);
    }
    pub fn set_trainer_class_runtime_like_cpp(&mut self, trainer_class: u8) {
        self.ai_ownership.trainer_class = trainer_class;
        self.lifecycle_metadata.trainer_class = trainer_class;
    }
    pub const fn trainer_class_like_cpp(&self) -> u8 {
        self.ai_ownership.trainer_class
    }
    pub fn set_npc_flags2_runtime_like_cpp(&mut self, npc_flags2: u32) {
        self.ai_ownership.npc_flags2 = npc_flags2;
        self.unit.set_npc_flags2_like_cpp(npc_flags2);
    }
    pub fn set_unit_flags2_runtime_like_cpp(&mut self, unit_flags2: u32) {
        self.ai_ownership.unit_flags2 = unit_flags2;
        self.unit
            .set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(unit_flags2));
    }
    pub fn set_unit_flags3_runtime_like_cpp(&mut self, unit_flags3: u32) {
        self.ai_ownership.unit_flags3 = unit_flags3;
        self.unit
            .set_unit_flags3_like_cpp(UnitFlags3::from_bits_truncate(unit_flags3));
    }
    pub fn set_flags_extra_runtime_like_cpp(&mut self, flags_extra: u32) {
        self.lifecycle_metadata.flags_extra = flags_extra;
        // C++ derives `UNIT_STATE_IGNORE_PATHFINDING` from
        // `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING` once, in `Creature::Create`
        // (`Creature.cpp:1154-1155`). This setter is the runtime seam through
        // which legacy registrations apply template `flags_extra` without going
        // through the create lifecycle, so it has to reach the same state —
        // otherwise the same template would use the navmesh on one path and skip
        // it on the other.
        self.refresh_ignore_pathfinding_state_like_cpp();
    }
    /// Applies the `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING` →
    /// `UNIT_STATE_IGNORE_PATHFINDING` derivation of C++ `Creature::Create`
    /// (`Creature.cpp:1154-1155`).
    ///
    /// C++ only ever *adds* the state there, and deliberately excludes it from
    /// `UNIT_STATE_ALL_ERASABLE` so it survives respawn, so this never clears a
    /// state the flag does not ask for.
    pub(super) fn refresh_ignore_pathfinding_state_like_cpp(&mut self) {
        if CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
            .contains(CreatureFlagsExtra::IGNORE_PATHFINDING)
        {
            self.unit
                .add_unit_state(UnitState::IGNORE_PATHFINDING.bits());
        }
    }
    pub fn set_static_flags_runtime_like_cpp(&mut self, static_flags: [u32; 8]) {
        self.lifecycle_metadata.static_flags = static_flags;
    }
    pub fn is_template_rooted_like_cpp(&self) -> bool {
        CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0])
            .contains(CreatureStaticFlags::SESSILE)
    }
    /// Mirrors `Creature::CanGiveExperience`: critters, pets, totems, and
    /// templates carrying `CREATURE_FLAG_EXTRA_NO_XP` cannot reward kill XP.
    pub fn can_give_experience_like_cpp(&self) -> bool {
        !CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0])
            .contains(CreatureStaticFlags::NO_XP)
            && !CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
                .contains(CreatureFlagsExtra::NO_XP)
            && !self.has_unit_type_mask_like_cpp(UNIT_MASK_PET | UNIT_MASK_TOTEM)
    }
    pub fn set_template_rooted_like_cpp(&mut self, rooted: bool) {
        self.lifecycle_metadata.rooted = rooted;
        let mut flags =
            CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0]);
        flags.set(CreatureStaticFlags::SESSILE, rooted);
        self.lifecycle_metadata.static_flags[0] = flags.bits();
        if rooted {
            self.unit.add_unit_state(UnitState::ROOT.bits());
            self.runtime_state
                .movement_flags
                .remove(MovementFlag::MASK_MOVING);
            self.runtime_state.movement_flags.insert(MovementFlag::ROOT);
        } else {
            self.unit.clear_unit_state(UnitState::ROOT.bits());
            self.runtime_state.movement_flags.remove(MovementFlag::ROOT);
        }
    }
    pub fn can_melee_like_cpp(&self) -> bool {
        !CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0])
            .contains(CreatureStaticFlags::NO_MELEE_FLEE)
    }
    pub fn set_ai_identity_names_runtime_like_cpp(
        &mut self,
        ai_name: impl Into<String>,
        script_name: impl Into<String>,
    ) {
        self.lifecycle_metadata.ai_name = ai_name.into();
        self.lifecycle_metadata.script_name = script_name.into();
    }
    /// Represented seam for TrinityCore `Creature::LoadFromDB`, which stores
    /// `CreatureData::StringId` in `m_stringIds[1]` after spawn health/default
    /// movement initialization.
    pub fn set_spawn_string_id_runtime_like_cpp(&mut self, string_id: Option<String>) {
        self.lifecycle_metadata.string_id = string_id;
    }
    pub fn ground_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.ground_movement_type
    }
    pub fn set_ground_movement_type_runtime_like_cpp(&mut self, ground_movement_type: u8) {
        self.lifecycle_metadata.ground_movement_type =
            normalize_creature_ground_movement_type_like_cpp(ground_movement_type);
    }
    pub fn swim_allowed_like_cpp(&self) -> bool {
        self.lifecycle_metadata.swim_allowed
    }
    pub fn set_swim_allowed_runtime_like_cpp(&mut self, swim_allowed: bool) {
        self.lifecycle_metadata.swim_allowed = swim_allowed;
    }
    pub fn set_flight_movement_type_runtime_like_cpp(&mut self, flight_movement_type: u8) {
        self.lifecycle_metadata.flight_movement_type =
            normalize_creature_flight_movement_type_like_cpp(flight_movement_type);
    }
    pub const fn chase_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.chase_movement_type
    }
    pub fn set_chase_movement_type_runtime_like_cpp(&mut self, chase_movement_type: u8) {
        self.lifecycle_metadata.chase_movement_type =
            normalize_creature_chase_movement_type_like_cpp(chase_movement_type);
    }
    pub const fn random_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.random_movement_type
    }
    pub fn set_random_movement_type_runtime_like_cpp(&mut self, random_movement_type: u8) {
        self.lifecycle_metadata.random_movement_type =
            normalize_creature_random_movement_type_like_cpp(random_movement_type);
    }
    pub const fn interaction_pause_timer_ms_like_cpp(&self) -> u32 {
        self.lifecycle_metadata.interaction_pause_timer_ms
    }
    pub fn set_interaction_pause_timer_ms_runtime_like_cpp(
        &mut self,
        interaction_pause_timer_ms: u32,
    ) {
        self.lifecycle_metadata.interaction_pause_timer_ms = interaction_pause_timer_ms;
    }
    pub fn configure_ai_runtime(
        &mut self,
        home_position: Position,
        aggro_radius: f32,
        wander_radius: f32,
        respawn_time_secs: u64,
    ) {
        self.ai_ownership.home_position = home_position;
        self.ai_ownership.aggro_radius = aggro_radius;
        self.ai_ownership.wander_radius = wander_radius;
        self.ai_ownership.respawn_time_secs = respawn_time_secs;
    }
    pub fn begin_ai_move(&mut self, dst: Position, now_ms: u64) {
        let dist = self.ai_position().distance(&dst);
        let duration_ms = ((dist / 2.5) * 1000.0) as u32;
        self.ai_ownership.move_target = Some(dst);
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.move_duration_ms = duration_ms.max(500);
        self.ai_ownership.spline_id = self.ai_ownership.spline_id.saturating_add(1);
    }
    pub fn finish_ai_move(&mut self) {
        if let Some(dst) = self.ai_ownership.move_target.take() {
            self.unit.world_mut().relocate(dst);
        }
        self.ai_ownership.move_duration_ms = 0;
    }
    pub fn ai_movement_finished(&self, now_ms: u64) -> bool {
        self.ai_ownership.move_target.is_none()
            || now_ms.saturating_sub(self.ai_ownership.move_start_ms)
                >= u64::from(self.ai_ownership.move_duration_ms)
    }
    // Small compatibility aliases for callers that need canonical values without
    // reaching through Unit/WorldObject internals.
    pub fn level(&self) -> u8 {
        self.ai_level()
    }
    pub const fn current_health(&self) -> u64 {
        self.ai_current_health()
    }
    pub const fn max_health(&self) -> u64 {
        self.ai_max_health()
    }
    pub fn is_alive(&self) -> bool {
        self.ai_is_alive()
    }
    pub const fn position(&self) -> Position {
        self.ai_position()
    }
    pub const fn player_damage_req(&self) -> u32 {
        self.player_damage_req
    }
    pub const fn corpse_remove_time(&self) -> i64 {
        self.corpse_remove_time
    }
    pub const fn respawn_time(&self) -> i64 {
        self.respawn_time
    }
    pub fn set_respawn_time(&mut self, respawn_time: i64) {
        self.respawn_time = respawn_time;
    }
    pub const fn respawn_delay(&self) -> u32 {
        self.respawn_delay
    }
    pub fn set_respawn_delay(&mut self, delay: u32) {
        self.respawn_delay = delay;
    }
    pub const fn corpse_delay(&self) -> u32 {
        self.corpse_delay
    }
    pub fn set_corpse_delay(&mut self, delay: u32, ignore_corpse_decay_ratio: bool) {
        self.corpse_delay = delay;
        if ignore_corpse_decay_ratio {
            self.ignore_corpse_decay_ratio = true;
        }
    }
    pub const fn ignore_corpse_decay_ratio(&self) -> bool {
        self.ignore_corpse_decay_ratio
    }
    pub const fn wander_distance(&self) -> f32 {
        self.wander_distance
    }
    pub const fn boundary_check_time(&self) -> u32 {
        self.boundary_check_time
    }
    pub const fn combat_pulse_time(&self) -> u32 {
        self.combat_pulse_time
    }
    pub const fn combat_pulse_delay(&self) -> u32 {
        self.combat_pulse_delay
    }
    pub const fn react_state(&self) -> ReactState {
        self.react_state
    }
    pub fn set_react_state(&mut self, state: ReactState) {
        self.react_state = state;
    }
    pub fn has_react_state(&self, state: ReactState) -> bool {
        self.react_state == state
    }
    pub const fn default_movement_type(&self) -> MovementGeneratorType {
        self.default_movement_type
    }
    pub fn set_default_movement_type_runtime_like_cpp(
        &mut self,
        movement_type: MovementGeneratorType,
    ) {
        self.default_movement_type = movement_type;
        self.sync_motion_default_generator_like_cpp();
    }
    pub(super) fn sync_motion_default_generator_like_cpp(&mut self) {
        let kind = match self.default_movement_type {
            MovementGeneratorType::Idle => MovementGeneratorKind::Idle,
            MovementGeneratorType::Random => MovementGeneratorKind::Random,
            MovementGeneratorType::Waypoint => MovementGeneratorKind::Waypoint,
        };
        self.unit
            .subsystems_mut()
            .motion
            .initialize_default_generator_like_cpp(kind);
    }
    pub const fn waypoint_path_id_like_cpp(&self) -> u32 {
        self.waypoint_path_id
    }
    pub fn load_path_like_cpp(&mut self, path_id: u32) {
        self.waypoint_path_id = path_id;
    }
    pub fn apply_creatures_addon_lifecycle_like_cpp(
        &mut self,
        addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
    ) -> bool {
        self.load_creatures_addon_represented_like_cpp(addon)
    }
    pub const fn spawn_id(&self) -> u64 {
        self.spawn_id
    }
    pub fn set_spawn_id(&mut self, spawn_id: u64) {
        self.spawn_id = spawn_id;
        self.lifecycle_metadata.spawn_id = spawn_id;
    }
    pub const fn equipment_id(&self) -> u8 {
        self.equipment_id
    }
    /// Represented bounded seam for TrinityCore `Creature::LoadEquipment(id, true)` callers.
    ///
    /// This only records the selected equipment id on the canonical creature state and
    /// lifecycle metadata. It does not load `creature_equip_template` items, update
    /// visible item fields, or fan out values updates.
    pub fn set_equipment_id_like_cpp(&mut self, equipment_id: u8) {
        self.equipment_id = equipment_id;
        self.lifecycle_metadata.equipment_id = equipment_id;
    }
    pub const fn original_equipment_id(&self) -> i8 {
        self.original_equipment_id
    }
    /// Represented bounded seam for TrinityCore `Creature::InitEntry`.
    ///
    /// C++ stores `CreatureData::equipmentId` in `m_originalEquipmentId`
    /// before `LoadEquipment(id)` mutates the selected equipment id, notably
    /// when DB value `-1` means "pick a random equipment template".
    pub fn set_original_equipment_id_like_cpp(&mut self, original_equipment_id: i8) {
        self.original_equipment_id = original_equipment_id;
        self.lifecycle_metadata.original_equipment_id = original_equipment_id;
    }
    pub const fn already_call_assistance(&self) -> bool {
        self.already_call_assistance
    }
    pub const fn already_searched_assistance(&self) -> bool {
        self.already_searched_assistance
    }
    pub const fn cannot_reach_target(&self) -> bool {
        self.cannot_reach_target
    }
    pub fn set_cannot_reach_target_like_cpp(&mut self, cannot_reach: bool) {
        self.cannot_reach_target = cannot_reach;
        if !cannot_reach {
            self.cannot_reach_timer = 0;
        }
    }
    pub const fn cannot_reach_timer(&self) -> u32 {
        self.cannot_reach_timer
    }
    pub fn is_in_evade_mode_like_cpp(&self) -> bool {
        self.unit.has_unit_state(UnitState::EVADE.bits())
    }
    pub fn set_in_evade_mode_like_cpp(&mut self, in_evade_mode: bool) {
        if in_evade_mode {
            self.unit.add_unit_state(UnitState::EVADE.bits());
        } else {
            self.unit.clear_unit_state(UnitState::EVADE.bits());
        }
    }
    pub fn is_evading_attacks_like_cpp(&self) -> bool {
        self.is_in_evade_mode_like_cpp() || self.cannot_reach_target
    }
    pub const fn melee_damage_school_mask(&self) -> u32 {
        self.melee_damage_school_mask
    }
    pub fn melee_damage_school_like_cpp(&self) -> u8 {
        if self.melee_damage_school_mask == 0 {
            return wow_constants::spell::SpellSchools::Normal as u8;
        }
        self.melee_damage_school_mask
            .trailing_zeros()
            .min(u32::from(MAX_SPELL_SCHOOL_LIKE_CPP.saturating_sub(1))) as u8
    }
    pub fn set_melee_damage_school_like_cpp(&mut self, school: u8) {
        let school = school.min(MAX_SPELL_SCHOOL_LIKE_CPP.saturating_sub(1));
        self.melee_damage_school_mask = 1_u32 << school;
    }
    pub const fn original_entry(&self) -> u32 {
        self.original_entry
    }
    pub const fn trigger_just_appeared(&self) -> bool {
        self.trigger_just_appeared
    }
    pub const fn respawn_compatibility_mode(&self) -> bool {
        self.respawn_compatibility_mode
    }
    pub fn set_respawn_compatibility_mode(&mut self, enabled: bool) {
        self.respawn_compatibility_mode = enabled;
    }
    pub const fn last_damaged_time(&self) -> i64 {
        self.last_damaged_time
    }
    pub fn set_last_damaged_time_like_cpp(&mut self, last_damaged_time: i64) {
        self.last_damaged_time = last_damaged_time;
    }
    pub const fn regenerate_health(&self) -> bool {
        self.regenerate_health
    }
    pub const fn is_missing_can_swim_flag_out_of_combat(&self) -> bool {
        self.is_missing_can_swim_flag_out_of_combat
    }
    pub const fn gossip_menu_id(&self) -> u32 {
        self.gossip_menu_id
    }
    pub const fn sparring_health_pct(&self) -> f32 {
        self.sparring_health_pct
    }
    pub fn set_sparring_health_pct_like_cpp(&mut self, pct: f32) {
        self.sparring_health_pct = pct.clamp(0.0, 100.0);
    }
    pub fn is_charmed_owned_by_player_or_player_like_cpp(&self) -> bool {
        let control = &self.unit.subsystems().control;
        control
            .owner_guid
            .is_some_and(|owner_guid| owner_guid.is_player())
            || control.charmer_guid.is_some_and(|charmer_guid| {
                charmer_guid.is_player() || control.controlled_by_player
            })
    }
    /// C++ contrast: `Creature::CalculateDamageForSparring(Unit*, uint32)`.
    ///
    /// This helper keeps the current represented scope explicit: the caller
    /// tells us whether the attacker is a creature and whether the attacker is
    /// player-controlled. The victim-side player-control check is local.
    pub fn calculate_damage_for_sparring_like_cpp(
        &self,
        attacker_is_creature: bool,
        attacker_is_charmed_owned_by_player_or_player: bool,
        damage: u32,
    ) -> u32 {
        if self.sparring_health_pct <= 0.0
            || !attacker_is_creature
            || attacker_is_charmed_owned_by_player_or_player
            || self.is_charmed_owned_by_player_or_player_like_cpp()
        {
            return damage;
        }

        let health = self.unit.data().health;
        let max_health = self.unit.data().max_health;
        if max_health == 0
            || (health as f32 * 100.0 / max_health as f32) <= self.sparring_health_pct
        {
            return 0;
        }

        let sparring_health = (max_health as f32 * self.sparring_health_pct / 100.0) as u64;
        if health.saturating_sub(u64::from(damage)) <= sparring_health {
            return health
                .saturating_sub(sparring_health)
                .min(u64::from(u32::MAX)) as u32;
        }

        if u64::from(damage) >= health {
            return health.saturating_sub(1).min(u64::from(u32::MAX)) as u32;
        }

        damage
    }
    /// C++ contrast: `Creature::ShouldFakeDamageFrom(Unit*)`.
    pub fn should_fake_damage_from_like_cpp(
        &self,
        attacker_is_creature: bool,
        attacker_is_charmed_owned_by_player_or_player: bool,
    ) -> bool {
        if self.sparring_health_pct <= 0.0
            || !attacker_is_creature
            || attacker_is_charmed_owned_by_player_or_player
            || self.is_charmed_owned_by_player_or_player_like_cpp()
        {
            return false;
        }

        let max_health = self.unit.data().max_health;
        max_health != 0
            && (self.unit.data().health as f32 * 100.0 / max_health as f32)
                <= self.sparring_health_pct
    }
    pub const fn regen_timer(&self) -> u32 {
        self.regen_timer
    }
    pub const fn spells(&self) -> [u32; MAX_CREATURE_SPELLS] {
        self.spells
    }
    pub fn set_spell(&mut self, slot: usize, spell_id: u32) {
        if slot < MAX_CREATURE_SPELLS {
            self.spells[slot] = spell_id;
        }
    }
    pub const fn disable_reputation_gain(&self) -> bool {
        self.disable_reputation_gain
    }
    pub const fn sight_distance(&self) -> f32 {
        self.sight_distance
    }
    pub const fn combat_distance(&self) -> f32 {
        self.combat_distance
    }
    pub fn set_combat_distance_like_cpp(&mut self, combat_distance: f32) {
        self.combat_distance = combat_distance.max(0.0);
    }
    pub const fn loot_mode(&self) -> u16 {
        self.loot_mode
    }
    pub fn reset_loot_mode(&mut self) {
        self.loot_mode = LOOT_MODE_DEFAULT;
    }
    pub const fn is_temp_world_object(&self) -> bool {
        self.is_temp_world_object
    }
    /// C++ `Creature::m_isTempWorldObject` (`Creature.h:365`), toggled by
    /// `Map::SwitchGridContainers<Creature>` after moving between grid/world
    /// containers (`Map.cpp:294-305`).
    pub fn set_temp_world_object_like_cpp(&mut self, on: bool) {
        self.is_temp_world_object = on;
    }
    pub const fn cleanup_before_delete_count(&self) -> u32 {
        self.grid_unload_cleanup_before_delete_count
    }
    pub const fn grid_unload_delete_requested(&self) -> bool {
        self.grid_unload_delete_requested
    }
    pub const fn grid_unload_respawn_relocation_requested(&self) -> bool {
        self.grid_unload_respawn_relocation_requested
    }
    pub fn register_dynamic_object(&mut self, guid: ObjectGuid) {
        self.owned_dynamic_objects.push(guid);
    }
    pub fn unregister_dynamic_object(&mut self, guid: ObjectGuid) {
        self.owned_dynamic_objects
            .retain(|owned_guid| *owned_guid != guid);
    }
    pub fn dynamic_objects(&self) -> &[ObjectGuid] {
        &self.owned_dynamic_objects
    }
    pub fn removed_dynamic_objects_from_grid_unload(&self) -> &[ObjectGuid] {
        &self.removed_dynamic_objects_from_grid_unload
    }
    pub fn register_area_trigger(&mut self, guid: ObjectGuid) {
        self.owned_area_triggers.push(guid);
    }
    pub fn unregister_area_trigger(&mut self, guid: ObjectGuid) {
        self.owned_area_triggers
            .retain(|owned_guid| *owned_guid != guid);
    }
    pub fn area_triggers(&self) -> &[ObjectGuid] {
        &self.owned_area_triggers
    }
    pub fn removed_area_triggers_from_grid_unload(&self) -> &[ObjectGuid] {
        &self.removed_area_triggers_from_grid_unload
    }
    pub fn set_destroyed_object(&mut self, destroyed: bool) {
        self.unit
            .world_mut()
            .object_mut()
            .set_destroyed_object(destroyed);
    }
    pub fn remove_all_dyn_objects(&mut self) {
        self.removed_dynamic_objects_from_grid_unload
            .extend(self.owned_dynamic_objects.drain(..));
    }
    pub fn remove_all_area_triggers(&mut self) {
        self.removed_area_triggers_from_grid_unload
            .extend(self.owned_area_triggers.drain(..));
    }
    pub fn combat_stop(&mut self) {
        self.unit.set_attacking(None);
    }
    pub const fn is_in_combat(&self) -> bool {
        self.unit.attacking().is_some()
    }
    pub fn request_respawn_relocation_from_grid_unload(&mut self) {
        self.grid_unload_respawn_relocation_requested = true;
    }
    pub fn cleanup_before_delete(&mut self) {
        self.grid_unload_cleanup_before_delete_count = self
            .grid_unload_cleanup_before_delete_count
            .saturating_add(1);
    }
    pub fn request_delete_from_grid_unload(&mut self) {
        self.grid_unload_delete_requested = true;
        self.unit.world_mut().clear_current_cell();
    }
    pub fn get_power_index(&self, power: PowerType) -> Option<usize> {
        if power == self.power_type() {
            Some(0)
        } else if power == PowerType::ComboPoints {
            Some(2)
        } else {
            None
        }
    }
    pub fn power_type(&self) -> PowerType {
        power_type_from_u8(self.unit.data().display_power)
    }
    pub const fn combat_log_stats_like_cpp(&self) -> CreatureCombatLogStatsLikeCpp {
        self.combat_log_stats
    }
    /// Replace the live totals consumed by C++ `SpellCastLogData::Initialize`.
    ///
    /// Loaders and future represented UnitMods mutations must update this at
    /// the same boundary as the corresponding canonical Unit state.
    pub fn set_combat_log_stats_like_cpp(&mut self, stats: CreatureCombatLogStatsLikeCpp) {
        self.combat_log_stats = stats;
    }
    /// C++ `SpellCastLogData::Initialize` selects ranged AP only for the
    /// hunter unit class and base-attack AP for every other class.
    pub fn combat_log_attack_power_like_cpp(&self) -> i32 {
        if self.unit.data().class_id == Class::Hunter as u8 {
            self.combat_log_stats.ranged_attack_power
        } else {
            self.combat_log_stats.attack_power
        }
    }
    pub fn set_power_type(&mut self, power: PowerType) {
        let old_power = self.power_type();
        if old_power != PowerType::ComboPoints {
            self.unit.set_power_index(old_power, None);
        }
        self.unit.set_display_power(power);
        self.unit.set_power_index(power, Some(0));
        self.unit.set_power_index(PowerType::ComboPoints, Some(2));
    }
    pub fn set_display_id(
        &mut self,
        display_id: u32,
        set_native: bool,
        model: Option<CreatureModelDimensions>,
    ) {
        self.ai_ownership.display_id = display_id;
        self.unit.set_display_id(display_id, set_native);

        if let Some(model) = model {
            let scale = self.unit.world().object().scale() * self.unit.data().display_scale;
            self.unit.set_bounding_radius(model.bounding_radius * scale);
            self.unit.set_combat_reach(model.combat_reach * scale);
        }
    }
    pub fn set_faction(&mut self, faction: u32) {
        self.ai_ownership.faction = faction;
        self.unit.set_faction(faction);
    }
}
