//! Creature lifecycle and runtime state operations, part 1 of 3.
//!
//! The inherent `Creature` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Creature {
    pub fn new(is_world_object: bool) -> Self {
        let mut unit = Unit::new(is_world_object);
        unit.set_type(TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
        unit.set_power_index(PowerType::Mana, Some(0));
        unit.set_power_index(PowerType::ComboPoints, Some(2));
        unit.subsystems_mut()
            .combat
            .initialize_threat_list_capability(true);

        Self {
            unit,
            player_damage_req: 0,
            dont_clear_tap_list_on_evade: false,
            pickpocket_loot_restore: 0,
            corpse_remove_time: 0,
            respawn_time: 0,
            respawn_delay: DEFAULT_RESPAWN_DELAY_SECS,
            corpse_delay: DEFAULT_CORPSE_DELAY_SECS,
            ignore_corpse_decay_ratio: false,
            wander_distance: 0.0,
            boundary_check_time: DEFAULT_BOUNDARY_CHECK_TIME_MS,
            combat_pulse_time: 0,
            combat_pulse_delay: 0,
            react_state: ReactState::Aggressive,
            default_movement_type: MovementGeneratorType::Idle,
            waypoint_path_id: 0,
            spawn_id: 0,
            equipment_id: 0,
            original_equipment_id: 0,
            already_call_assistance: false,
            already_searched_assistance: false,
            cannot_reach_target: false,
            cannot_reach_timer: 0,
            melee_damage_school_mask: 0x1,
            original_entry: 0,
            trigger_just_appeared: true,
            respawn_compatibility_mode: false,
            last_damaged_time: 0,
            regenerate_health: true,
            is_missing_can_swim_flag_out_of_combat: false,
            unit_type_mask: 0,
            gossip_menu_id: 0,
            sparring_health_pct: 0.0,
            regen_timer: CREATURE_REGEN_INTERVAL_MS,
            spells: [0; MAX_CREATURE_SPELLS],
            disable_reputation_gain: false,
            sight_distance: DEFAULT_MONSTER_SIGHT_DISTANCE,
            combat_distance: 0.0,
            loot_mode: LOOT_MODE_DEFAULT,
            is_temp_world_object: false,
            grid_unload_cleanup_before_delete_count: 0,
            grid_unload_delete_requested: false,
            grid_unload_respawn_relocation_requested: false,
            owned_dynamic_objects: Vec::new(),
            removed_dynamic_objects_from_grid_unload: Vec::new(),
            owned_area_triggers: Vec::new(),
            removed_area_triggers_from_grid_unload: Vec::new(),
            lifecycle_metadata: CreatureLifecycleMetadata::default(),
            runtime_state: CreatureRuntimeState::default(),
            ai_ownership: CreatureAiOwnershipState::default(),
            tap_list: Vec::new(),
            attack_reputation_faction_id: None,
            is_contested_guard_faction: false,
            spell_focus: CreatureSpellFocusStateLikeCpp::default(),
            combat_log_stats: CreatureCombatLogStatsLikeCpp::default(),
            loot_lifecycle_revision: 0,
            loot_authority: OwnedLootAuthority::new(),
            shared_loot: None,
            personal_loot: HashMap::new(),
        }
    }
    pub fn create_from_lifecycle(record: CreatureCreateLifecycleRecord) -> Self {
        let mut creature = Self::new(false);
        creature.apply_create_lifecycle(record);
        creature
    }
    pub fn load_from_db_lifecycle(record: CreatureLoadFromDbLifecycleRecord) -> Self {
        let mut creature = Self::create_from_lifecycle(record.create);
        creature.apply_load_from_db_lifecycle(&record.spawn);
        creature
    }
    pub fn apply_create_lifecycle(&mut self, record: CreatureCreateLifecycleRecord) {
        let template = &record.template;
        let spawn = record.spawn.as_ref();
        let map_id = spawn.map(|spawn| spawn.map_id).unwrap_or(record.map_id);
        let instance_id = spawn
            .map(|spawn| spawn.instance_id)
            .unwrap_or(record.instance_id);
        let position = spawn.map(|spawn| spawn.position).unwrap_or(record.position);
        let home_position = spawn
            .map(|spawn| spawn.home_position)
            .unwrap_or(record.position);
        let equipment_id = spawn
            .and_then(|spawn| spawn.equipment_id)
            .unwrap_or(record.selected_equipment_id);
        let original_equipment_id = spawn
            .and_then(|spawn| spawn.original_equipment_id)
            .unwrap_or(record.selected_original_equipment_id);

        self.unit.world_mut().object_mut().create(record.guid);
        let _ = self.unit.world_mut().set_map(map_id, instance_id);
        self.unit.world_mut().relocate(position);
        self.unit.world_mut().set_name(template.name.clone());

        self.unit.world_mut().object_mut().set_entry(record.entry);
        self.original_entry = template.original_entry;
        self.unit.world_mut().object_mut().set_scale(template.scale);
        self.unit.set_race(0);
        self.unit.set_class(template.unit_class);
        self.set_faction(template.faction);
        self.set_display_id(
            record.selected_display_id,
            true,
            record
                .selected_model_dimensions
                .or(template.model_dimensions),
        );
        // C++ `Creature::InitEntry` sets walk/run from creature_template and
        // swim/flight to 1.0 immediately after haste/time-rate defaults
        // (`Creature.cpp:540-550`).
        self.unit.set_mod_casting_speed_like_cpp(1.0);
        self.unit.set_mod_spell_haste_like_cpp(1.0);
        self.unit.set_mod_haste_like_cpp(1.0);
        self.unit.set_mod_ranged_haste_like_cpp(1.0);
        self.unit.set_mod_haste_regen_like_cpp(1.0);
        self.unit.set_mod_time_rate_like_cpp(1.0);
        self.unit
            .set_speed_rate_like_cpp(UnitMoveType::Walk, template.speed_walk);
        self.unit
            .set_speed_rate_like_cpp(UnitMoveType::Run, template.speed_run);
        self.unit.set_speed_rate_like_cpp(UnitMoveType::Swim, 1.0);
        self.unit.set_speed_rate_like_cpp(UnitMoveType::Flight, 1.0);
        self.unit.set_can_dual_wield_like_cpp(
            CreatureFlagsExtra::from_bits_truncate(template.flags_extra)
                .contains(CreatureFlagsExtra::USE_OFFHAND_ATTACK),
        );
        self.spells = template.spells;
        self.equipment_id = equipment_id;
        self.original_equipment_id = original_equipment_id;
        for (index, &(item_id, item_appearance_mod_id, item_visual)) in
            record.selected_virtual_items.iter().enumerate()
        {
            let visible = (item_id != 0 || item_appearance_mod_id != 0 || item_visual != 0)
                .then_some(VisibleItemValues {
                    item_id,
                    item_appearance_mod_id,
                    item_visual,
                });
            self.unit.set_virtual_item(index, visible);
        }
        if record.vehicle_id.is_some() {
            // C++ `Creature::CreateFromProto` calls `CreateVehicleKit(vehId, entry, true)` here.
            // The bounded seam creates the local DB2 seat-backed `Vehicle` only when the caller
            // resolved a real `VehicleEntry`; a missing input preserves identity metadata but
            // represents `CreateVehicleKit` returning false.
            let create_input = record.vehicle_kit_create_input;
            let vehicle_id = create_input
                .as_ref()
                .map_or(record.vehicle_id, |input| Some(input.vehicle_id));
            let loading = create_input.as_ref().map_or(true, |input| input.loading);
            let creature_entry = create_input
                .as_ref()
                .map_or(record.entry, |input| input.creature_entry);
            let seat_defs = create_input.map(|input| input.seat_defs);
            let outcome = self
                .unit
                .subsystems_mut()
                .vehicle
                .create_vehicle_kit_like_cpp(
                    record.guid,
                    position,
                    vehicle_id,
                    creature_entry,
                    loading,
                    seat_defs,
                );
            if outcome.unit_type_mask_vehicle_represented {
                self.add_unit_type_mask_like_cpp(UNIT_MASK_VEHICLE);
            }
        }
        self.default_movement_type = spawn
            .map(|spawn| spawn.movement_type)
            .unwrap_or(template.movement_type);
        self.sync_motion_default_generator_like_cpp();
        self.set_corpse_delay(record.corpse_delay, record.ignore_corpse_decay_ratio);
        self.set_respawn_compatibility_mode(!record.dynamic);
        if let Some(spawn) = spawn {
            self.apply_spawn_lifecycle(spawn);
        }

        self.unit.set_level(record.selected_level);
        self.set_power_type(record.stats.power_type);
        self.unit.set_max_health(record.stats.max_health);
        self.unit.set_health(record.stats.health);
        self.unit.set_create_mana_like_cpp(record.stats.base_mana);
        self.unit
            .set_max_power(record.stats.power_type, record.stats.max_power);
        self.unit
            .set_power(record.stats.power_type, record.stats.power);
        self.unit.set_weapon_damage(
            WeaponAttackType::BaseAttack,
            record.stats.min_damage,
            record.stats.max_damage,
        );
        self.combat_log_stats = record.stats.combat_log;
        self.set_melee_damage_school_like_cpp(template.damage_school);
        self.ai_ownership.home_position = home_position;
        self.ai_ownership.move_target = None;
        self.ai_ownership.move_start_ms = 0;
        self.ai_ownership.move_duration_ms = 0;
        self.ai_ownership.state = CreatureAiState::Idle;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.ai_ownership.respawn_time_secs = spawn
            .map(|spawn| u64::from(spawn.respawn_delay))
            .unwrap_or(u64::from(DEFAULT_RESPAWN_DELAY_SECS));
        self.ai_ownership.wander_radius = spawn.map(|spawn| spawn.wander_distance).unwrap_or(0.0);
        self.ai_ownership.aggro_radius = DEFAULT_MONSTER_SIGHT_DISTANCE;
        self.ai_ownership.display_id = record.selected_display_id;
        self.ai_ownership.faction = template.faction;
        self.ai_ownership.npc_flags = template.npc_flags as u32;
        self.ai_ownership.npc_flags2 = (template.npc_flags >> 32) as u32;
        self.ai_ownership.trainer_class = template.trainer_class;
        self.ai_ownership.unit_flags = template.unit_flags;
        self.ai_ownership.unit_flags2 = template.unit_flags2;
        self.ai_ownership.unit_flags3 = template.unit_flags3;
        self.ai_ownership.min_damage = record.stats.min_damage.max(0.0) as u32;
        self.ai_ownership.max_damage = record.stats.max_damage.max(0.0) as u32;
        // C++ `Creature::GetLootId` first honors an object-local `m_lootId` override and then
        // reads the selected `CreatureDifficulty::LootID`; death/skinning generation reads
        // GoldMin/GoldMax/SkinLootID from that same difficulty (`Creature.cpp:1317-1323`,
        // `Unit.cpp:10545-10575,10675`). Keep those values on the runtime creature instead of
        // falling back to `CreatureAiOwnershipState`'s zero defaults.
        self.ai_ownership.loot_id = template.loot_id;
        self.ai_ownership.skin_loot_id = template.skin_loot_id;
        self.ai_ownership.gold_min = template.gold_min;
        self.ai_ownership.gold_max = template.gold_max;
        self.unit
            .set_npc_flags_like_cpp(self.ai_ownership.npc_flags);
        self.unit
            .set_npc_flags2_like_cpp(self.ai_ownership.npc_flags2);
        self.unit
            .set_unit_flags_like_cpp(UnitFlags::from_bits_truncate(template.unit_flags));
        self.unit
            .set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(template.unit_flags2));
        self.unit
            .set_unit_flags3_like_cpp(UnitFlags3::from_bits_truncate(template.unit_flags3));
        self.lifecycle_metadata.ground_movement_type =
            normalize_creature_ground_movement_type_like_cpp(template.ground_movement_type);
        self.load_creatures_addon_represented_like_cpp(record.addon.as_ref());

        let init_entry_static_flags = self.init_entry_static_flags_like_cpp(template);
        let init_entry_rooted = CreatureStaticFlags::from_bits_truncate(init_entry_static_flags[0])
            .contains(CreatureStaticFlags::SESSILE);

        self.lifecycle_metadata = CreatureLifecycleMetadata {
            template_entry: template.entry,
            original_entry: template.original_entry,
            difficulty_id: template.difficulty_id,
            ai_name: template.ai_name.clone(),
            script_name: template.script_name.clone(),
            required_expansion: template.required_expansion,
            unit_class: template.unit_class,
            trainer_class: template.trainer_class,
            classification: template.classification,
            damage_school: template.damage_school,
            flags_extra: template.flags_extra,
            static_flags: init_entry_static_flags,
            ground_movement_type: normalize_creature_ground_movement_type_like_cpp(
                template.ground_movement_type,
            ),
            swim_allowed: template.swim_allowed,
            flight_movement_type: normalize_creature_flight_movement_type_like_cpp(
                template.flight_movement_type,
            ),
            rooted: init_entry_rooted,
            chase_movement_type: normalize_creature_chase_movement_type_like_cpp(
                template.chase_movement_type,
            ),
            random_movement_type: normalize_creature_random_movement_type_like_cpp(
                template.random_movement_type,
            ),
            interaction_pause_timer_ms: template.interaction_pause_timer_ms,
            creature_type: template.creature_type,
            type_flags: template.type_flags,
            selected_level: record.selected_level,
            selected_display_id: record.selected_display_id,
            selected_model_dimensions: record
                .selected_model_dimensions
                .or(template.model_dimensions),
            spawn_health: Some(record.stats.health),
            spawn_mana: Some(record.stats.power),
            template_scale: template.scale,
            speed_walk: template.speed_walk,
            speed_run: template.speed_run,
            spawn_id: spawn.map(|spawn| spawn.spawn_id).unwrap_or(0),
            spawn_map_id: map_id,
            spawn_instance_id: instance_id,
            spawn_position: position,
            home_position,
            phase_id: spawn.and_then(|spawn| spawn.phase_id),
            phase_group: spawn.and_then(|spawn| spawn.phase_group),
            terrain_swap_map: spawn.and_then(|spawn| spawn.terrain_swap_map),
            spawn_group_id: spawn.and_then(|spawn| spawn.spawn_group_id),
            spawn_group_name: spawn.and_then(|spawn| spawn.spawn_group_name.clone()),
            pool_id: spawn.and_then(|spawn| spawn.pool_id),
            string_id: spawn.and_then(|spawn| spawn.string_id.clone()),
            is_spawn_active: spawn.map(|spawn| spawn.is_active).unwrap_or(true),
            inactive_by_spawn_group: spawn
                .map(|spawn| spawn.inactive_by_spawn_group)
                .unwrap_or(false),
            duplicate_spawn_found: spawn
                .map(|spawn| spawn.duplicate_spawn_found)
                .unwrap_or(false),
            add_to_map_requested: spawn.map(|spawn| spawn.add_to_map).unwrap_or(false),
            map_insertion_requested: spawn.map(|spawn| spawn.add_to_map).unwrap_or(false),
            dynamic_spawn: record.dynamic,
            is_summon_like_cpp: false,
            formation_info: None,
            vehicle_id: record.vehicle_id,
            add_to_world_vehicle_reset_context: record.add_to_world_vehicle_reset_context,
            equipment_id,
            original_equipment_id,
            addon: record.addon,
        };

        self.set_template_rooted_like_cpp(init_entry_rooted);

        // C++ `Creature::Create` (`Creature.cpp:1154-1155`) adds
        // `UNIT_STATE_IGNORE_PATHFINDING` for templates carrying
        // `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING` (0x20000000), and
        // `PathGenerator::CalculatePath` then skips the navmesh entirely for
        // them (`PathGenerator.cpp:80`).
        self.refresh_ignore_pathfinding_state_like_cpp();

        self.refresh_threat_list_capability_like_cpp();
        self.clear_data_changes();
    }
    pub(super) fn init_entry_static_flags_like_cpp(
        &self,
        template: &CreatureTemplateLifecycleRecord,
    ) -> [u32; 8] {
        let mut static_flags = template.static_flags;
        let flags_extra = CreatureFlagsExtra::from_bits_truncate(template.flags_extra);

        let mut primary = CreatureStaticFlags::from_bits_truncate(static_flags[0]);
        primary.set(
            CreatureStaticFlags::NO_XP,
            template.creature_type == CreatureType::Critter as u32
                || self.has_unit_type_mask_like_cpp(UNIT_MASK_PET)
                || self.has_unit_type_mask_like_cpp(UNIT_MASK_TOTEM)
                || flags_extra.contains(CreatureFlagsExtra::NO_XP),
        );
        static_flags[0] = primary.bits();

        let mut flags4 = CreatureStaticFlags4::from_bits_truncate(static_flags[3]);
        flags4.set(
            CreatureStaticFlags4::TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS,
            CreatureTypeFlags::from_bits_truncate(template.type_flags)
                .contains(CreatureTypeFlags::TREAT_AS_RAID_UNIT),
        );
        static_flags[3] = flags4.bits();

        static_flags
    }
    pub fn apply_load_from_db_lifecycle(&mut self, spawn: &CreatureSpawnLifecycleRecord) {
        self.apply_spawn_lifecycle(spawn);
        self.lifecycle_metadata.spawn_id = spawn.spawn_id;
        self.lifecycle_metadata.spawn_map_id = spawn.map_id;
        self.lifecycle_metadata.spawn_instance_id = spawn.instance_id;
        self.lifecycle_metadata.spawn_position = spawn.position;
        self.lifecycle_metadata.home_position = spawn.home_position;
        self.lifecycle_metadata.phase_id = spawn.phase_id;
        self.lifecycle_metadata.phase_group = spawn.phase_group;
        self.lifecycle_metadata.terrain_swap_map = spawn.terrain_swap_map;
        self.lifecycle_metadata.spawn_group_id = spawn.spawn_group_id;
        self.lifecycle_metadata.spawn_group_name = spawn.spawn_group_name.clone();
        self.lifecycle_metadata.pool_id = spawn.pool_id;
        self.lifecycle_metadata.string_id = spawn.string_id.clone();
        self.lifecycle_metadata.is_spawn_active = spawn.is_active;
        self.lifecycle_metadata.inactive_by_spawn_group = spawn.inactive_by_spawn_group;
        self.lifecycle_metadata.duplicate_spawn_found = spawn.duplicate_spawn_found;
        self.lifecycle_metadata.add_to_map_requested = spawn.add_to_map;
        self.lifecycle_metadata.map_insertion_requested = spawn.add_to_map;
        if let Some(equipment_id) = spawn.equipment_id {
            self.lifecycle_metadata.equipment_id = equipment_id;
        }
        if let Some(original_equipment_id) = spawn.original_equipment_id {
            self.lifecycle_metadata.original_equipment_id = original_equipment_id;
        }
        self.clear_data_changes();
    }
    pub(super) fn apply_spawn_lifecycle(&mut self, spawn: &CreatureSpawnLifecycleRecord) {
        self.set_spawn_id(spawn.spawn_id);
        self.set_respawn_compatibility_mode(spawn.respawn_compatibility_mode);
        self.wander_distance = spawn.wander_distance;
        self.set_respawn_delay(spawn.respawn_delay);
        self.set_respawn_time(spawn.respawn_time);
        self.default_movement_type = spawn.movement_type;
        self.sync_motion_default_generator_like_cpp();
        if let Some(equipment_id) = spawn.equipment_id {
            self.equipment_id = equipment_id;
        }
        if let Some(original_equipment_id) = spawn.original_equipment_id {
            self.original_equipment_id = original_equipment_id;
        }
        let _ = self
            .unit
            .world_mut()
            .set_map(spawn.map_id, spawn.instance_id);
        self.unit.world_mut().relocate(spawn.position);
        self.ai_ownership.home_position = spawn.home_position;
        self.ai_ownership.move_target = None;
        self.ai_ownership.respawn_time_secs = u64::from(spawn.respawn_delay);
        self.ai_ownership.wander_radius = spawn.wander_distance;
    }
    pub const fn lifecycle_metadata(&self) -> &CreatureLifecycleMetadata {
        &self.lifecycle_metadata
    }
    pub fn set_spawn_health_like_cpp(&mut self) {
        self.unit.set_health(
            self.lifecycle_metadata
                .spawn_health
                .unwrap_or(self.unit.data().max_health),
        );
        if let Some(spawn_mana) = self.lifecycle_metadata.spawn_mana {
            self.unit.set_power(PowerType::Mana, spawn_mana);
        }
    }
    pub fn set_required_expansion_runtime_like_cpp(&mut self, required_expansion: u8) {
        self.lifecycle_metadata.required_expansion = required_expansion;
    }
    pub fn is_world_boss_like_cpp(&self) -> bool {
        if self.is_summon_like_cpp() {
            return false;
        }

        CreatureTypeFlags::from_bits_truncate(self.lifecycle_metadata.type_flags)
            .contains(CreatureTypeFlags::BOSS_MOB)
    }
    pub fn set_type_flags_runtime_like_cpp(&mut self, type_flags: u32) {
        self.lifecycle_metadata.type_flags = type_flags;
    }
    pub fn is_civilian_like_cpp(&self) -> bool {
        CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
            .contains(CreatureFlagsExtra::CIVILIAN)
    }
    pub fn flight_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.flight_movement_type
    }
    /// C++ `Creature::CanWalk()` is true when the movement template allows
    /// ground movement (`Ground != None`).
    pub fn can_walk_like_cpp(&self) -> bool {
        self.lifecycle_metadata.ground_movement_type != CreatureGroundMovementType::None as u8
    }
    /// C++ `Creature::CanEnterWater()` returns true when `Unit::CanSwim()`,
    /// `IsPet()`, or the creature movement template allows swimming.
    pub fn can_enter_water_like_cpp(&self) -> bool {
        if self.can_swim_like_cpp() {
            return true;
        }
        if self.unit.world().object().guid().is_pet() {
            return true;
        }
        self.lifecycle_metadata.swim_allowed
    }
    /// Represented C++ `Unit::CanSwim()` for creature-backed units.
    pub fn can_swim_like_cpp(&self) -> bool {
        let unit_flags = self.unit.unit_flags_like_cpp();
        if unit_flags.contains(UnitFlags::CANT_SWIM) {
            return false;
        }
        if unit_flags.contains(UnitFlags::PLAYER_CONTROLLED) {
            return true;
        }
        if self
            .unit
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::AI_WILL_ONLY_SWIM_IF_TARGET_SWIMS)
        {
            return false;
        }
        if unit_flags.contains(UnitFlags::PET_IN_COMBAT) {
            return true;
        }
        unit_flags.intersects(UnitFlags::RENAME | UnitFlags::CAN_SWIM)
    }
    /// C++ `Creature::RefreshCanSwimFlag`: remember whether `UNIT_FLAG_CAN_SWIM`
    /// was absent out of combat, then add it while engaged when the movement
    /// template otherwise allows the creature to enter water.
    pub fn refresh_can_swim_flag_like_cpp(&mut self, recheck: bool) {
        if !self.is_missing_can_swim_flag_out_of_combat || recheck {
            self.is_missing_can_swim_flag_out_of_combat = !self
                .unit
                .unit_flags_like_cpp()
                .contains(UnitFlags::CAN_SWIM);
        }

        if self.is_missing_can_swim_flag_out_of_combat && self.can_enter_water_like_cpp() {
            let flags = self.unit.unit_flags_like_cpp() | UnitFlags::CAN_SWIM;
            self.unit.set_unit_flags_like_cpp(flags);
        }
    }
    pub fn restore_can_swim_flag_after_home_like_cpp(&mut self) {
        if self.is_missing_can_swim_flag_out_of_combat {
            let flags = self.unit.unit_flags_like_cpp() & !UnitFlags::CAN_SWIM;
            self.unit.set_unit_flags_like_cpp(flags);
        }
    }
    /// C++ `Creature::CanFly()` returns true when the movement template allows
    /// flight (`Flight != None`) or runtime movement flags say the unit is
    /// flying (`MOVEMENTFLAG_FLYING | MOVEMENTFLAG_DISABLE_GRAVITY`).
    pub fn can_fly_like_cpp(&self) -> bool {
        self.lifecycle_metadata.flight_movement_type != CreatureFlightMovementType::None as u8
            || self.is_flying_like_cpp()
    }
    pub fn is_flying_like_cpp(&self) -> bool {
        self.runtime_state
            .movement_flags
            .intersects(MovementFlag::FLYING | MovementFlag::DISABLE_GRAVITY)
    }
    pub fn can_hover_like_cpp(&self) -> bool {
        self.lifecycle_metadata.ground_movement_type == CreatureGroundMovementType::Hover as u8
            || self.is_hovering_like_cpp()
    }
    pub fn is_hovering_like_cpp(&self) -> bool {
        self.runtime_state
            .movement_flags
            .contains(MovementFlag::HOVER)
    }
    pub const fn movement_flags_like_cpp(&self) -> MovementFlag {
        self.runtime_state.movement_flags
    }
    pub fn set_movement_flags_runtime_like_cpp(&mut self, movement_flags: MovementFlag) {
        self.runtime_state.movement_flags = movement_flags;
    }
    pub fn add_to_world_vehicle_reset_context_like_cpp(
        &self,
    ) -> Option<&CreatureAddToWorldVehicleResetContextLikeCpp> {
        self.lifecycle_metadata
            .add_to_world_vehicle_reset_context
            .as_ref()
    }
    pub fn set_add_to_world_vehicle_reset_context_like_cpp(
        &mut self,
        context: Option<CreatureAddToWorldVehicleResetContextLikeCpp>,
    ) {
        self.lifecycle_metadata.add_to_world_vehicle_reset_context = context;
    }
    pub const fn is_summon_like_cpp(&self) -> bool {
        self.lifecycle_metadata.is_summon_like_cpp
    }
    pub const fn unit_type_mask_like_cpp(&self) -> u32 {
        self.unit_type_mask
    }
    pub const fn has_unit_type_mask_like_cpp(&self, mask: u32) -> bool {
        self.unit_type_mask & mask != 0
    }
    pub fn add_unit_type_mask_like_cpp(&mut self, mask: u32) {
        self.unit_type_mask |= mask;
        self.refresh_threat_list_capability_like_cpp();
    }
    pub fn remove_unit_type_mask_like_cpp(&mut self, mask: u32) {
        self.unit_type_mask &= !mask;
        self.refresh_threat_list_capability_like_cpp();
    }
    pub fn can_have_threat_list_like_cpp(&self) -> bool {
        if CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
            .contains(CreatureFlagsExtra::TRIGGER)
        {
            return false;
        }
        if self.has_unit_type_mask_like_cpp(UNIT_MASK_PET | UNIT_MASK_TOTEM) {
            return false;
        }
        if self.has_unit_type_mask_like_cpp(UNIT_MASK_MINION | UNIT_MASK_GUARDIAN) {
            return false;
        }
        true
    }
    pub(super) fn refresh_threat_list_capability_like_cpp(&mut self) {
        let can_have_threat_list = self.can_have_threat_list_like_cpp();
        self.unit
            .subsystems_mut()
            .combat
            .initialize_threat_list_capability(can_have_threat_list);
    }
    pub const fn is_totem_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_TOTEM)
    }
    pub const fn is_guardian_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_GUARDIAN)
    }
    pub const fn is_controlable_guardian_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_CONTROLABLE_GUARDIAN)
    }
    pub const fn is_vehicle_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_VEHICLE)
    }
    pub fn set_summon_like_cpp(&mut self, is_summon: bool) {
        self.lifecycle_metadata.is_summon_like_cpp = is_summon;
    }
    pub const fn formation_info_like_cpp(&self) -> Option<&CreatureFormationInfoLikeCpp> {
        self.lifecycle_metadata.formation_info.as_ref()
    }
    pub fn set_formation_info_like_cpp(&mut self, info: Option<CreatureFormationInfoLikeCpp>) {
        self.lifecycle_metadata.formation_info = info;
    }
    pub const fn spell_focus_state_like_cpp(&self) -> CreatureSpellFocusStateLikeCpp {
        self.spell_focus
    }
    pub fn set_represented_spell_focus_like_cpp(
        &mut self,
        spell_id: u32,
        target: ObjectGuid,
        orientation: f32,
        ai_does_not_face_target: bool,
    ) {
        self.spell_focus = CreatureSpellFocusStateLikeCpp {
            spell_id: Some(spell_id),
            delay_ms: 0,
            target: self.unit.data().target,
            orientation,
            ai_does_not_face_target,
        };
        if ai_does_not_face_target {
            self.unit.add_unit_state(UnitState::FOCUSING.bits());
        }
        let new_target = if ai_does_not_face_target {
            ObjectGuid::EMPTY
        } else {
            target
        };
        self.unit.set_target(new_target);
    }
    pub fn has_spell_focus_like_cpp(&self, focus_spell_id: Option<u32>) -> bool {
        if self.unit.is_dead() {
            return false;
        }

        match focus_spell_id {
            Some(focus_spell_id) => self.spell_focus.spell_id == Some(focus_spell_id),
            None => self.spell_focus.spell_id.is_some() || self.spell_focus.delay_ms != 0,
        }
    }
    pub fn set_target_like_cpp(&mut self, guid: ObjectGuid) {
        if self.has_spell_focus_like_cpp(None) {
            self.spell_focus.target = guid;
        } else {
            self.unit.set_target(guid);
        }
    }
    pub fn release_spell_focus_like_cpp(
        &mut self,
        focus_spell_id: Option<u32>,
        with_delay: bool,
        is_pet: bool,
        cannot_turn: bool,
    ) {
        let Some(active_spell_id) = self.spell_focus.spell_id else {
            return;
        };
        if focus_spell_id.is_some_and(|focus_spell_id| focus_spell_id != active_spell_id) {
            return;
        }

        if self.spell_focus.ai_does_not_face_target {
            self.unit.clear_unit_state(UnitState::FOCUSING.bits());
        }

        if is_pet {
            if !cannot_turn {
                self.reacquire_spell_focus_target_like_cpp(cannot_turn);
            }
        } else {
            self.spell_focus.delay_ms = if with_delay { 1000 } else { 1 };
        }
        self.spell_focus.spell_id = None;
    }
    pub fn reacquire_spell_focus_target_like_cpp(&mut self, cannot_turn: bool) {
        if !self.has_spell_focus_like_cpp(None) {
            return;
        }

        self.unit.set_target(self.spell_focus.target);
        if cannot_turn {
            // C++ skips target-facing/orientation restore when CannotTurn() is true.
        }
        self.spell_focus.delay_ms = 0;
    }
    pub fn do_not_reacquire_spell_focus_target_like_cpp(&mut self) {
        self.spell_focus.delay_ms = 0;
        self.spell_focus.spell_id = None;
    }
    /// Represented C++ `Creature::SearchFormation()` branch.
    ///
    /// C++ anchor: `Creature.cpp:379-389`. This only consumes explicit
    /// caller-provided `FormationInfo` evidence already stored on the creature.
    /// It does not query DB, scan spawn groups, or own a real `FormationMgr`.
    pub fn search_formation_like_cpp(&self) -> CreatureSearchFormationOutcomeLikeCpp {
        let spawn_id = self.spawn_id();
        let is_summon = self.is_summon_like_cpp();
        if is_summon {
            return CreatureSearchFormationOutcomeLikeCpp {
                spawn_id,
                is_summon,
                formation_info_found: self.lifecycle_metadata.formation_info.is_some(),
                leader_spawn_id: None,
                add_to_group_requested: false,
            };
        }

        if spawn_id == 0 {
            return CreatureSearchFormationOutcomeLikeCpp {
                spawn_id,
                is_summon,
                formation_info_found: self.lifecycle_metadata.formation_info.is_some(),
                leader_spawn_id: None,
                add_to_group_requested: false,
            };
        }

        let Some(formation_info) = self.lifecycle_metadata.formation_info else {
            return CreatureSearchFormationOutcomeLikeCpp {
                spawn_id,
                is_summon,
                formation_info_found: false,
                leader_spawn_id: None,
                add_to_group_requested: false,
            };
        };

        CreatureSearchFormationOutcomeLikeCpp {
            spawn_id,
            is_summon,
            formation_info_found: true,
            leader_spawn_id: Some(formation_info.leader_spawn_id),
            add_to_group_requested: true,
        }
    }
    /// Represented C++ `Creature::AIM_Initialize()` / `AIM_Create()` seam.
    ///
    /// C++ anchors: `Creature.cpp:1026-1044` (`AIM_Create`, `AIM_Initialize`)
    /// and `Creature.cpp:1046-1060` (`Motion_Initialize`). This records local
    /// evidence only: it does not instantiate real AI, run `InitializeAI`, call
    /// `CreatureGroup::FormationReset`, query `CreatureGroup::IsFormed`, move a
    /// `MotionMaster`, or reset a vehicle kit. The vehicle reset remains the
    /// following AddToMap seam representing `if (GetVehicleKit()) Reset()`.
    pub fn aim_initialize_like_cpp(&self) -> CreatureAimInitializeOutcomeLikeCpp {
        let spawn_id = self.spawn_id();
        let formation_info = self.formation_info_like_cpp();
        let formation_present = formation_info.is_some();
        let formation_leader = formation_info.is_some_and(|info| info.leader_spawn_id == spawn_id);
        let motion_initialize_requires_formed_state = formation_present && !formation_leader;

        CreatureAimInitializeOutcomeLikeCpp {
            guid: self.guid(),
            spawn_id,
            aim_create_represented: true,
            motion_initialize_represented: true,
            formation_present,
            formation_leader,
            // C++ non-leader formed groups call MoveIdle() and return, but this
            // represented seam has no real CreatureGroup::IsFormed() state yet.
            formation_move_idle_represented: false,
            motion_initialize_requires_formed_state,
            motion_master_initialize_represented: !motion_initialize_requires_formed_state,
            ai_selected_represented: true,
            ai_initialize_represented: true,
            vehicle_reset_expected: self.unit().subsystems().vehicle.kit.is_some(),
            succeeded: true,
        }
    }
    pub fn clear_data_changes(&mut self) {
        self.unit.clear_unit_data_changes();
        self.unit.world_mut().object_mut().clear_update_mask(false);
    }
    pub const fn unit(&self) -> &Unit {
        &self.unit
    }
    pub fn unit_mut(&mut self) -> &mut Unit {
        &mut self.unit
    }
    pub const fn ai_ownership(&self) -> &CreatureAiOwnershipState {
        &self.ai_ownership
    }
    pub fn ai_ownership_mut(&mut self) -> &mut CreatureAiOwnershipState {
        &mut self.ai_ownership
    }
    pub const fn ai_state(&self) -> CreatureAiState {
        self.ai_ownership.state
    }
    pub fn set_ai_state(&mut self, state: CreatureAiState) {
        self.ai_ownership.state = state;
    }
    pub const fn ai_home_position(&self) -> Position {
        self.ai_ownership.home_position
    }
    pub fn set_ai_home_position(&mut self, position: Position) {
        self.ai_ownership.home_position = position;
    }
    pub fn record_ai_movement_inform(&mut self, movement_type: u8, movement_id: u32) {
        self.ai_ownership.last_movement_inform = Some(CreatureMovementInform {
            movement_type,
            movement_id,
        });
    }
    pub fn take_ai_movement_inform(&mut self) -> Option<CreatureMovementInform> {
        self.ai_ownership.last_movement_inform.take()
    }
    /// Records C++ `CreatureAI::JustReachedHome()`
    /// (`HomeMovementGenerator.cpp:156`).
    pub fn record_ai_just_reached_home(&mut self) {
        self.ai_ownership.just_reached_home_pending = true;
    }
    pub fn take_ai_just_reached_home(&mut self) -> bool {
        std::mem::take(&mut self.ai_ownership.just_reached_home_pending)
    }
    pub fn record_ai_spell_click_inform(&mut self, clicker: ObjectGuid, spell_click_handled: bool) {
        self.ai_ownership.last_spell_click_inform = Some(CreatureSpellClickInform {
            clicker,
            spell_click_handled,
        });
    }
    pub fn take_ai_spell_click_inform(&mut self) -> Option<CreatureSpellClickInform> {
        self.ai_ownership.last_spell_click_inform.take()
    }
    pub const fn ai_position(&self) -> Position {
        self.unit.world().position()
    }
    pub fn set_ai_position(&mut self, position: Position) {
        self.unit.world_mut().relocate(position);
    }
    pub const fn ai_guid(&self) -> ObjectGuid {
        self.unit.world().object().guid()
    }
    pub const fn ai_entry(&self) -> u32 {
        self.unit.world().object().entry()
    }
    pub const fn guid(&self) -> ObjectGuid {
        self.ai_guid()
    }
    pub const fn entry(&self) -> u32 {
        self.ai_entry()
    }
    pub fn ai_level(&self) -> u8 {
        self.unit.data().level.clamp(0, u8::MAX as i32) as u8
    }
    pub const fn ai_current_health(&self) -> u64 {
        self.unit.data().health
    }
    pub const fn ai_max_health(&self) -> u64 {
        self.unit.data().max_health
    }
    pub fn ai_is_alive(&self) -> bool {
        self.unit.is_alive()
            && self.ai_current_health() > 0
            && self.ai_ownership.state != CreatureAiState::Dead
    }
    pub fn enter_ai_combat(&mut self, attacker: ObjectGuid) {
        // C++ `Creature::AtEngage` refreshes this before any chase spline is
        // built, so `MoveSplineInit` observes the effective combat capability.
        self.refresh_can_swim_flag_like_cpp(false);
        self.ai_ownership.state = CreatureAiState::InCombat;
        self.ai_ownership.combat_target = Some(attacker);
        self.ai_ownership.move_target = None;
        self.unit.set_attacking(Some(attacker));
    }
}
