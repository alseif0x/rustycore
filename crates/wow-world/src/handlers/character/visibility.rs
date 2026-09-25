// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character visibility, nearby object creation and phase updates.

use super::*;

#[path = "visibility/creatures.rs"]
mod creatures;
#[path = "visibility/gameobjects.rs"]
mod gameobjects;

impl WorldSession {
    pub(in crate::handlers::character) fn creature_addon_create_fields_like_cpp(
        addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
    ) -> CreatureAddonCreateFieldsLikeCpp {
        let Some(addon) = addon else {
            // C++ Creature::UpdateEntry calls SetSheath(SHEATH_STATE_MELEE)
            // when no addon row exists; addon rows then own the exact value.
            return CreatureAddonCreateFieldsLikeCpp {
                stand_state: UnitStandStateType::Stand as u8,
                sheathe_state: SheathState::Melee as u8,
                ..CreatureAddonCreateFieldsLikeCpp::default()
            };
        };

        CreatureAddonCreateFieldsLikeCpp {
            has_addon: true,
            mount_display_id: addon.mount_display_id as i32,
            stand_state: addon.stand_state as u8,
            vis_flags: addon.vis_flags,
            anim_tier: addon.anim_tier,
            sheathe_state: addon.sheath_state as u8,
            pvp_flags: addon.pvp_flags.bits(),
            emote_state: addon.emote as i32,
            ai_anim_kit_id: addon.ai_anim_kit_id,
            movement_anim_kit_id: addon.movement_anim_kit_id,
            melee_anim_kit_id: addon.melee_anim_kit_id,
        }
    }

    pub(super) fn materialize_creature_spawn_row_with_catalogs_like_cpp(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        map_id: u16,
        row: &wow_persistence::CreatureVisibilityPersistenceRowLikeCpp,
        viewer_position: &Position,
        visibility_range: f32,
    ) -> Option<MaterializedCreatureSpawnLikeCpp> {
        // C++ ObjectMgr::LoadCreatures materializes CreatureData once, then
        // Creature::LoadFromDB consumes that data for create/update fields.
        // Rust still queries lazily by visibility; keep a single row reader so
        // login and movement refresh cannot drift from each other.
        let spawn_guid = row.spawn_guid;
        let entry = row.entry;
        let [pos_x, pos_y, pos_z, orientation] = row.position;
        if !is_within_2d_visibility_range_like_cpp(viewer_position, pos_x, pos_y, visibility_range)
        {
            return None;
        }
        let spawn_difficulties = &row.spawn_difficulties;
        if !spawn_difficulties_contains_spawn_mode_like_cpp(
            spawn_difficulties,
            self.current_map_difficulty_id_like_cpp(),
        ) {
            return None;
        }

        let cur_health = row.current_health;
        let cur_mana = row.current_mana;
        let model_id = row.model_id;
        let min_level = row.min_level;
        let faction = row.faction;
        let template_npc_flags = row.template_npc_flags;
        let [
            template_unit_flags,
            template_unit_flags2,
            template_unit_flags3,
        ] = row.template_unit_flags;
        let speed_walk = normalize_creature_template_speed_walk_like_cpp(row.speed_walk);
        let speed_run = normalize_creature_template_speed_run_like_cpp(row.speed_run);
        let scale = row.scale;
        let unit_class = row.unit_class;
        let flags_extra = row.flags_extra;
        let (npc_flags, unit_flags, unit_flags2, unit_flags3) = choose_creature_flags_like_cpp(
            template_npc_flags,
            template_unit_flags,
            template_unit_flags2,
            template_unit_flags3,
            row.spawn_npc_flags_override,
            row.spawn_unit_flags_override[0],
            row.spawn_unit_flags_override[1],
            row.spawn_unit_flags_override[2],
            flags_extra,
        );
        let classification = row.classification;
        let regen_health = row.regen_health;
        let [base_attack_time, ranged_attack_time] = row.attack_time;
        let template_display_id = row.template_display_id;
        let template_display_scale = row.template_display_scale;
        let loot_id = row.loot_id;
        let skin_loot_id = row.skin_loot_id;
        let [gold_min, gold_max] = row.gold;
        let respawn_delay_secs = row.respawn_delay_secs;
        let script_name = row.script_name.clone();
        let string_id = row.string_id.clone();
        let vehicle_id = row.vehicle_id;
        let phase_use_flags = row.phase_use_flags;
        let phase_id = row.phase_id;
        let phase_group_id = row.phase_group_id;
        let terrain_swap_map = row.terrain_swap_map;
        let ground_movement_type = row.ground_movement_type;
        let swim_allowed = row.swim_allowed;
        let flight_movement_type = row.flight_movement_type;
        let rooted = row.rooted;
        let chase_movement_type =
            normalize_creature_chase_movement_type_like_cpp(row.chase_movement_type);
        let random_movement_type =
            normalize_creature_random_movement_type_like_cpp(row.random_movement_type);
        let interaction_pause_timer_ms = row.interaction_pause_timer_ms;
        let wander_distance = row.wander_distance;
        let default_movement_type = creature_movement_generator_type_from_db_like_cpp(
            row.effective_movement_type,
            wander_distance,
        );
        let wander_distance =
            normalized_creature_wander_distance_like_cpp(default_movement_type, wander_distance);
        let waypoint_path_id = row.waypoint_path_id;

        let Some(display_selection) = self.choose_creature_display_like_cpp(
            entry,
            model_id,
            flags_extra,
            template_display_id,
            template_display_scale,
        ) else {
            return None;
        };
        let display_id = display_selection.display_id;
        let Some(model_scalars) = self.creature_create_model_scalars_like_cpp(
            display_id,
            scale,
            display_selection.display_scale,
        ) else {
            warn!(
                "Skipping creature entry={} spawn={} display={} because creature_model_info is missing, matching C++ CreateFromProto failure",
                entry, spawn_guid, display_id
            );
            return None;
        };

        let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        );
        if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
            return None;
        }

        let creature_stats = self.creature_create_stats_with_catalogs_like_cpp(
            catalogs,
            entry,
            min_level,
            unit_class,
            classification,
            regen_health,
            cur_health,
            cur_mana,
        );
        let addon = catalogs.addons.get_for_creature_like_cpp(spawn_guid, entry);
        let addon_fields = WorldSession::creature_addon_create_fields_like_cpp(addon.as_ref());
        let equipment_fields = self.creature_virtual_items_from_row_with_catalogs_like_cpp(
            catalogs,
            entry,
            row.equipment_id,
        );

        let guid = if vehicle_id != 0 {
            ObjectGuid::create_vehicle_like_cpp(self.realm_id(), map_id, entry, spawn_guid as i64)
        } else {
            ObjectGuid::create_creature_like_cpp(self.realm_id(), map_id, entry, spawn_guid as i64)
        };
        let movement_flags = creature_create_movement_flags_like_cpp(ground_movement_type, rooted);
        let position = creature_create_position_after_hover_offset_like_cpp(
            Position::new(pos_x, pos_y, pos_z, orientation),
            movement_flags,
            model_scalars.hover_height,
        );
        let create_data = CreatureCreateData {
            guid,
            entry,
            display_id,
            native_display_id: display_id,
            display_scale: model_scalars.display_scale,
            native_x_display_scale: model_scalars.native_x_display_scale,
            bounding_radius: model_scalars.bounding_radius,
            combat_reach: model_scalars.combat_reach,
            health: creature_stats.health,
            max_health: creature_stats.max_health,
            level: min_level,
            faction_template: faction,
            npc_flags,
            unit_flags,
            unit_flags2,
            unit_flags3,
            aura_state: crate::map_manager::WorldCreature::health_aura_state_like_cpp(
                creature_stats.health.max(0) as u64,
                creature_stats.max_health.max(0) as u64,
                creature_stats.health > 0,
            ),
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale,
            unit_class,
            display_power: creature_stats.power_type as u8,
            power: {
                let mut power = [0; 10];
                power[0] = creature_stats.power;
                power
            },
            max_power: {
                let mut max_power = [0; 10];
                max_power[0] = creature_stats.max_power;
                max_power
            },
            base_mana: creature_stats.base_mana,
            virtual_items: equipment_fields.virtual_items,
            base_attack_time,
            ranged_attack_time,
            movement_flags,
            vehicle_id,
            play_hover_anim: false,
            hover_height: model_scalars.hover_height,
            mount_display_id: addon_fields.mount_display_id,
            stand_state: addon_fields.stand_state,
            vis_flags: addon_fields.vis_flags,
            anim_tier: addon_fields.anim_tier,
            emote_state: addon_fields.emote_state,
            sheathe_state: addon_fields.sheathe_state,
            pvp_flags: addon_fields.pvp_flags,
            current_area_id: 0,
            speed_walk_rate: speed_walk,
            speed_run_rate: speed_run,
            ai_anim_kit_id: addon_fields.ai_anim_kit_id,
            movement_anim_kit_id: addon_fields.movement_anim_kit_id,
            melee_anim_kit_id: addon_fields.melee_anim_kit_id,
        };

        let aggro_radius =
            self.creature_aggro_radius_for_faction_template_like_cpp(faction.max(0) as u32, 15.0);
        let min_damage = (min_level as u32).saturating_sub(1) * 3 + 5;
        let max_damage = min_damage + min_damage / 2;

        Some(MaterializedCreatureSpawnLikeCpp {
            guid,
            position,
            create_data,
            min_damage,
            max_damage,
            aggro_radius,
            loot_id,
            skin_loot_id,
            gold_min,
            gold_max,
            respawn_delay_secs,
            selected_equipment_id: equipment_fields.selected_equipment_id,
            original_equipment_id: equipment_fields.original_equipment_id,
            script_name,
            string_id,
            addon,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
            flags_extra,
            ground_movement_type,
            swim_allowed,
            flight_movement_type,
            rooted,
            chase_movement_type,
            random_movement_type,
            interaction_pause_timer_ms,
            wander_distance,
            default_movement_type,
            waypoint_path_id,
        })
    }

    #[cfg(test)]
    pub(super) fn materialize_creature_spawn_row_like_cpp(
        &mut self,
        map_id: u16,
        row: &wow_persistence::CreatureVisibilityPersistenceRowLikeCpp,
        viewer_position: &Position,
        visibility_range: f32,
    ) -> Option<MaterializedCreatureSpawnLikeCpp> {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.materialize_creature_spawn_row_with_catalogs_like_cpp(
            &catalogs,
            map_id,
            row,
            viewer_position,
            visibility_range,
        )
    }

    pub(super) fn register_materialized_creature_spawn_like_cpp(
        &mut self,
        map_id: u16,
        spawn: &MaterializedCreatureSpawnLikeCpp,
    ) {
        self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
            map_id,
            spawn.position,
            spawn.create_data.clone(),
            spawn.min_damage,
            spawn.max_damage,
            spawn.aggro_radius,
            spawn.loot_id,
            spawn.skin_loot_id,
            spawn.gold_min,
            spawn.gold_max,
            spawn.respawn_delay_secs,
            spawn.selected_equipment_id,
            spawn.original_equipment_id,
            spawn.script_name.clone(),
            spawn.string_id.clone(),
            spawn.addon.clone(),
            None,
            0,
            spawn.phase_use_flags,
            spawn.phase_id,
            spawn.phase_group_id,
            spawn.terrain_swap_map,
            spawn.flags_extra,
            spawn.ground_movement_type,
            spawn.swim_allowed,
            spawn.flight_movement_type,
            spawn.rooted,
            spawn.chase_movement_type,
            spawn.random_movement_type,
            spawn.interaction_pause_timer_ms,
            spawn.wander_distance,
            spawn.default_movement_type,
            spawn.waypoint_path_id,
        );
    }
}
