//! Registration, insertion, relocation and removal of represented creatures on the map.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the realm ID for GUID creation.
    /// Register a creature through canonical map state when available, keeping
    /// the legacy per-session AI facade as a compatibility cache.
    pub(crate) fn register_world_creature(
        &mut self,
        map_id: u16,
        position: wow_core::Position,
        create_data: wow_packet::packets::update::CreatureCreateData,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        loot_id: u32,
        skin_loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) {
        self.register_world_creature_with_flags_extra_like_cpp(
            map_id,
            position,
            create_data,
            min_dmg,
            max_dmg,
            aggro_radius,
            loot_id,
            skin_loot_id,
            gold_min,
            gold_max,
            boss_id,
            dungeon_encounter_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
            0,
        );
    }
    pub(crate) fn register_world_creature_with_flags_extra_like_cpp(
        &mut self,
        map_id: u16,
        position: wow_core::Position,
        create_data: wow_packet::packets::update::CreatureCreateData,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        loot_id: u32,
        skin_loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
        flags_extra: u32,
    ) {
        self.register_world_creature_with_flags_extra_and_movement_like_cpp(
            map_id,
            position,
            create_data,
            min_dmg,
            max_dmg,
            aggro_radius,
            loot_id,
            skin_loot_id,
            gold_min,
            gold_max,
            wow_entities::DEFAULT_RESPAWN_DELAY_SECS,
            0,
            0,
            String::new(),
            None,
            None,
            boss_id,
            dungeon_encounter_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
            flags_extra,
            wow_constants::CreatureGroundMovementType::Run as u8,
            true,
            0,
        );
    }
    pub(crate) fn register_world_creature_with_flags_extra_and_movement_like_cpp(
        &mut self,
        map_id: u16,
        position: wow_core::Position,
        create_data: wow_packet::packets::update::CreatureCreateData,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        loot_id: u32,
        skin_loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        respawn_delay_secs: u32,
        selected_equipment_id: u8,
        original_equipment_id: i8,
        script_name: String,
        string_id: Option<String>,
        addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
        flags_extra: u32,
        ground_movement_type: u8,
        swim_allowed: bool,
        flight_movement_type: u8,
    ) {
        self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
            map_id,
            position,
            create_data,
            min_dmg,
            max_dmg,
            aggro_radius,
            loot_id,
            skin_loot_id,
            gold_min,
            gold_max,
            respawn_delay_secs,
            selected_equipment_id,
            original_equipment_id,
            script_name,
            string_id,
            addon,
            boss_id,
            dungeon_encounter_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
            flags_extra,
            ground_movement_type,
            swim_allowed,
            flight_movement_type,
            false,
            wow_constants::CreatureChaseMovementType::Run as u8,
            wow_constants::CreatureRandomMovementType::Walk as u8,
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            0.0,
            wow_entities::MovementGeneratorType::Idle,
            0,
        );
    }
    pub(crate) fn register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
        &mut self,
        map_id: u16,
        position: wow_core::Position,
        create_data: wow_packet::packets::update::CreatureCreateData,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        loot_id: u32,
        skin_loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        respawn_delay_secs: u32,
        selected_equipment_id: u8,
        original_equipment_id: i8,
        script_name: String,
        string_id: Option<String>,
        addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
        flags_extra: u32,
        ground_movement_type: u8,
        swim_allowed: bool,
        flight_movement_type: u8,
        rooted: bool,
        chase_movement_type: u8,
        random_movement_type: u8,
        interaction_pause_timer_ms: u32,
        wander_distance: f32,
        default_movement_type: wow_entities::MovementGeneratorType,
        waypoint_path_id: u32,
    ) {
        let guid = create_data.guid;
        let entry = create_data.entry;
        let hp = create_data.health.max(1) as u32;
        let level = create_data.level;
        let display_id = create_data.display_id;
        let faction = create_data.faction_template.max(0) as u32;
        let npc_flags = create_data.npc_flags as u32;
        let npc_flags2 = (create_data.npc_flags >> 32) as u32;
        let unit_flags = create_data.unit_flags;
        let unit_flags2 = create_data.unit_flags2;
        let unit_flags3 = create_data.unit_flags3;
        let damage_school = create_data.damage_school;
        let (db_phase_shift, validated_terrain_swap_map) = self.db_spawn_phase_shift_like_cpp(
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        );
        let mut canonical_creature = {
            let mut creature = wow_entities::Creature::new(false);
            creature.unit_mut().world_mut().object_mut().create(guid);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .set_entry(entry);
            let _ = creature
                .unit_mut()
                .world_mut()
                .set_map(u32::from(map_id), 0);
            creature.unit_mut().world_mut().relocate(position);
            *creature.unit_mut().world_mut().phase_shift_mut() = db_phase_shift.clone();
            creature.unit_mut().set_level(level);
            creature.unit_mut().set_max_health(u64::from(hp));
            creature.unit_mut().set_health(u64::from(hp));
            creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
            creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
            creature.set_unit_flags2_runtime_like_cpp(unit_flags2);
            creature.set_unit_flags3_runtime_like_cpp(unit_flags3);
            creature.set_melee_damage_school_like_cpp(damage_school);
            creature
                .unit_mut()
                .set_native_display_id_like_cpp(create_data.native_display_id);
            creature.unit_mut().set_display_scales_like_cpp(
                create_data.display_scale,
                create_data.native_x_display_scale,
            );
            creature
                .unit_mut()
                .set_bounding_radius(create_data.bounding_radius);
            creature
                .unit_mut()
                .set_combat_reach(create_data.combat_reach);
            creature
                .unit_mut()
                .set_hover_height_like_cpp(create_data.hover_height);
            let power_type = power_type_from_u8_like_cpp(create_data.display_power);
            // This legacy SQL path also becomes a typed canonical Creature. Keep
            // its display-power index and create mana coherent with the CREATE
            // arrays so later canonical reads and Unit power mutations address
            // the same slot as C++ `Creature::UpdateLevelDependantStats`.
            creature.set_power_type(power_type);
            creature
                .unit_mut()
                .set_create_mana_like_cpp(create_data.base_mana);
            creature
                .unit_mut()
                .replace_create_power_arrays_like_cpp(create_data.power, create_data.max_power);
            creature.unit_mut().set_base_attack_time_like_cpp(
                WeaponAttackType::BaseAttack,
                create_data.base_attack_time,
            );
            creature.unit_mut().set_base_attack_time_like_cpp(
                WeaponAttackType::OffAttack,
                create_data.base_attack_time,
            );
            creature.unit_mut().set_base_attack_time_like_cpp(
                WeaponAttackType::RangedAttack,
                create_data.ranged_attack_time,
            );
            creature
                .unit_mut()
                .set_mount_display_id(create_data.mount_display_id.max(0) as u32);
            creature
                .unit_mut()
                .set_stand_state_like_cpp(unit_stand_state_from_u8_like_cpp(
                    create_data.stand_state,
                ));
            creature
                .unit_mut()
                .replace_all_vis_flags_like_cpp(create_data.vis_flags);
            creature
                .unit_mut()
                .set_anim_tier_like_cpp(create_data.anim_tier);
            creature
                .unit_mut()
                .set_sheath_like_cpp(sheath_state_from_u8_like_cpp(create_data.sheathe_state));
            creature
                .unit_mut()
                .replace_all_pvp_flags_like_cpp(UnitPvpFlags::from_bits_retain(
                    create_data.pvp_flags,
                ));
            creature.set_flags_extra_runtime_like_cpp(flags_extra);
            creature.set_ground_movement_type_runtime_like_cpp(ground_movement_type);
            creature.set_swim_allowed_runtime_like_cpp(swim_allowed);
            creature.set_flight_movement_type_runtime_like_cpp(flight_movement_type);
            creature.set_template_rooted_like_cpp(rooted);
            creature.set_chase_movement_type_runtime_like_cpp(chase_movement_type);
            creature.set_random_movement_type_runtime_like_cpp(random_movement_type);
            creature.set_interaction_pause_timer_ms_runtime_like_cpp(interaction_pause_timer_ms);
            // This compatibility path has no DB CreatureTemplate. Real loaded-grid creatures carry
            // template RequiredExpansion through lifecycle metadata; legacy ad-hoc registrations
            // preserve the previous WotLK max-level behavior.
            creature.set_required_expansion_runtime_like_cpp(CURRENT_EXPANSION_LIKE_CPP);
            creature.set_default_movement_type_runtime_like_cpp(default_movement_type);
            creature.set_equipment_id_like_cpp(selected_equipment_id);
            creature.set_original_equipment_id_like_cpp(original_equipment_id);
            creature.set_ai_identity_names_runtime_like_cpp(String::new(), script_name);
            creature.set_spawn_string_id_runtime_like_cpp(string_id);
            creature.set_respawn_delay(respawn_delay_secs);
            if waypoint_path_id != 0 {
                creature.load_path_like_cpp(waypoint_path_id);
            }
            creature.apply_creatures_addon_lifecycle_like_cpp(addon.as_ref());
            let effective_waypoint_path_id = creature.waypoint_path_id_like_cpp();
            if effective_waypoint_path_id != 0 {
                creature.load_path_like_cpp(effective_waypoint_path_id);
            }
            creature.configure_ai_runtime(position, aggro_radius, wander_distance.max(0.0), 30);
            creature.ai_ownership_mut().respawn_time_secs = u64::from(respawn_delay_secs);
            creature.ai_ownership_mut().min_damage = min_dmg;
            creature.ai_ownership_mut().max_damage = max_dmg;
            creature.ai_ownership_mut().loot_id = loot_id;
            creature.ai_ownership_mut().skin_loot_id = skin_loot_id;
            creature.ai_ownership_mut().gold_min = gold_min;
            creature.ai_ownership_mut().gold_max = gold_max;
            creature.ai_ownership_mut().boss_id = boss_id;
            creature.ai_ownership_mut().dungeon_encounter_id = dungeon_encounter_id;
            creature.ai_ownership_mut().phase_use_flags = phase_use_flags;
            creature.ai_ownership_mut().phase_id = phase_id;
            creature.ai_ownership_mut().phase_group_id = phase_group_id;
            creature.ai_ownership_mut().terrain_swap_map = validated_terrain_swap_map;
            creature
        };
        canonical_creature.clear_data_changes();
        if let Some(authority) =
            self.insert_canonical_creature_map_object_like_cpp(map_id, canonical_creature.clone())
        {
            canonical_creature.rebind_loot_authority_like_cpp(authority);
        }
        let canonical_health_owner = self.canonical_map_manager.as_ref().and_then(|manager| {
            let manager = manager.lock().ok()?;
            manager
                .find_map(u32::from(map_id), 0)?
                .map()
                .with_creature_like_cpp(guid, |current| current.unit().clone())
        });
        if let Some(current_unit) = canonical_health_owner {
            // When a canonical object pre-exists (for example grid loading
            // racing legacy registration), seed the compatibility mirror from
            // that exact health timeline rather than a separately constructed
            // Unit with incomparable revisions.
            canonical_creature
                .unit_mut()
                .preserve_authoritative_health_state_for_snapshot_like_cpp(&current_unit);
        }

        if let Some(manager) = &self.map_manager {
            let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
            let waypoint_path_resolver = self.waypoint_path_resolver_like_cpp.clone();
            let mut world_creature = crate::map_manager::WorldCreature::from_canonical(
                canonical_creature,
                create_data.clone(),
            );
            if world_creature.creature.default_movement_type()
                == wow_entities::MovementGeneratorType::Waypoint
            {
                world_creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(
                    |path_id| {
                        waypoint_path_resolver
                            .as_ref()
                            .and_then(|resolver| resolver(path_id))
                    },
                );
            }
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if manager.find_creature(map_id, 0, guid).is_none() {
                manager.add_creature(map_id, 0, grid_x, grid_y, world_creature);
            }
        }
    }
    fn insert_canonical_creature_map_object_like_cpp(
        &mut self,
        map_id: u16,
        creature: wow_entities::Creature,
    ) -> Option<OwnedLootAuthority> {
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return None;
        };
        insert_canonical_creature_map_object_on_map_like_cpp(
            manager,
            u32::from(map_id),
            0,
            creature,
        )
    }
    pub(crate) fn remove_world_creature(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<crate::map_manager::WorldCreature> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let manager = self.map_manager.as_ref().cloned()?;
        let removed = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) {
                creature.creature.clear_loot_like_cpp();
            }
            manager.remove_creature_any(map_id, instance_id, guid)
        };
        if removed.is_some() {
            self.remove_canonical_creature_map_object_like_cpp(guid);
        }
        removed
    }
    fn remove_canonical_creature_map_object_like_cpp(&mut self, guid: ObjectGuid) {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        remove_canonical_creature_map_object_on_map_like_cpp(
            manager,
            u32::from(map_id),
            instance_id,
            guid,
        );
    }
    fn relocate_canonical_creature_map_object_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: wow_core::Position,
    ) {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        relocate_canonical_creature_map_object_on_map_like_cpp(
            manager,
            u32::from(map_id),
            instance_id,
            guid,
            position,
        );
    }
    pub(in crate::session) fn sync_canonical_creature_entity_like_cpp(
        &mut self,
        creature: wow_entities::Creature,
    ) {
        let guid = creature.guid();
        let expected_legacy_authority = creature.loot_authority_like_cpp().clone();
        let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let authority = sync_canonical_creature_entity_on_map_like_cpp(
            manager,
            u32::from(map_id),
            instance_id,
            creature,
        );
        if let Some(authority) = authority {
            let _ = self.rebind_legacy_creature_loot_authority_like_cpp(
                guid,
                &expected_legacy_authority,
                expected_legacy_stamp,
                authority,
            );
        }
    }
    pub(crate) fn mutate_world_creature<F, R>(&mut self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let mut f = Some(f);
        if let Some(manager) = self.map_manager.as_ref().cloned() {
            let result = {
                let mut manager = manager
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) {
                    let result = f.take().expect("creature mutator is called once")(creature);
                    Some((result, creature.creature.clone()))
                } else {
                    None
                }
            };
            if let Some((result, creature)) = result {
                self.sync_canonical_creature_entity_like_cpp(creature);
                return Some(result);
            }
        }

        None
    }
}
