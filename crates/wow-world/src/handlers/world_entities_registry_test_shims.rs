// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only world-entity entry points (#1363 slice 10b): the `#[cfg(test)]`
//! methods that were nested in the production impls of
//! `session/world_entities/{creature_registry,creature,creature_kill,aggro}.rs`
//! (the fixture creature registration and removal, the talk-to and kill
//! objective entries without a generator, and the two canonical threat
//! readers), moved verbatim so the classifier counts them as test code, and
//! one-call method forwards to the creature kill reputation and death-state
//! completion moved to `wow_world_entities` (#1363 slice 10b), so no scenario
//! call site changes. The
//! two threat readers widen from `pub(in crate::session)` to `pub(crate)`,
//! because this file is outside the session tree. Mounted from
//! `handlers/mod.rs`, so the session hotspot's test lines do not grow; the
//! creature-registry F3 shim is mounted here for the same reason.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;
use wow_entities::CreatureAddonLifecycleRecordLikeCpp;

/// The #1241 F3 entry point of the creature registry, mounted here instead of
/// from the deleted `session/world_entities/creature_registry.rs`.
#[path = "../../unit_tests/session/world_entities/creature_registry/f3_shims.rs"]
mod creature_registry_f3_shims;

impl WorldSession {
    /// Register a creature through the canonical map (test fixtures; the
    /// production spawn path is `register_materialized_creature_spawn_like_cpp`).
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
    ) -> Option<wow_entities::Creature> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.remove_world_creature(&mut hub, guid)
    }
    pub(crate) async fn talked_to_creature_like_cpp(
        &mut self,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.talked_to_creature_with_generator_like_cpp(
            generator.as_ref(),
            creature_entry,
            creature_guid,
        )
        .await;
    }
    pub(crate) async fn on_creature_killed(
        &mut self,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.on_creature_killed_with_generator_like_cpp(
            generator.as_ref(),
            creature_entry,
            creature_guid,
        )
        .await;
    }
    pub(crate) fn canonical_creature_threat_value_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
    ) -> Option<f32> {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.canonical_creature_threat_value_like_cpp(hub, creature_guid, attacker_guid)
    }
    pub(crate) fn mirror_canonical_creature_threat_from_attacker_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        threat_value: f32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.mirror_canonical_creature_threat_from_attacker_like_cpp(
            &mut hub,
            creature_guid,
            attacker_guid,
            threat_value,
        )
    }
    pub(crate) fn reward_reputation_from_creature_kill_like_cpp(
        &mut self,
        creature_entry: u32,
        creature_guid: ObjectGuid,
        creature_level: u8,
        kill_reward_rate: f32,
    ) {
        wow_world_entities::reward_reputation_from_creature_kill_like_cpp(
            self,
            creature_entry,
            creature_guid,
            creature_level,
            kill_reward_rate,
        )
    }
    pub(crate) fn complete_represented_creature_death_state_after_kill_hooks_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    ) -> Option<wow_entities::UnitValuesUpdate> {
        wow_world_entities::complete_represented_creature_death_state_after_kill_hooks_like_cpp(
            self,
            attacker_guid,
            creature_guid,
        )
    }
}
