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
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
            &mut hub,
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
            rooted,
            chase_movement_type,
            random_movement_type,
            interaction_pause_timer_ms,
            wander_distance,
            default_movement_type,
            waypoint_path_id,
        )
    }
    pub(crate) fn remove_world_creature(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<crate::map_manager::WorldCreature> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.remove_world_creature(&mut hub, guid)
    }
    fn relocate_canonical_creature_map_object_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: wow_core::Position,
    ) {
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
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
}



#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/creature_registry/f3_shims.rs"]
mod f3_shims;
