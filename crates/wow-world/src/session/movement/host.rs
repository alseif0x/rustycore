// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapters for the movement bodies (#1363 slice 4b) and the
//! teleport bodies (slice 11a) that moved to `wow_world_entities`. Each
//! method delegates to the same-named session operation (or field).

use std::collections::HashMap;
use std::future::Future;

use wow_world_entities::PlayerMovementHostLikeCpp;

use super::*;

impl PlayerMovementHostLikeCpp for WorldSession {
    type AreaTriggerCatalogsLikeCpp = AreaTriggerCatalogsLikeCpp;

    fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool> {
        WorldSession::resolved_has_represented_aura_effect_like_cpp(self, effect)
    }

    fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        WorldSession::resolved_total_represented_aura_multiplier_like_cpp(self, effect)
    }

    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize {
        wow_world_spell::remove_auras_with_interrupt_flags_like_cpp(self, flags, flags2)
    }

    fn update_zone_represented_like_cpp(&mut self, new_zone: u32, new_area: u32) -> bool {
        wow_world_instances::update_zone_represented_like_cpp(self, new_zone, new_area)
    }

    fn check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
        &mut self,
        progression: &ProgressionCatalogsLikeCpp,
        area_id: u32,
    ) -> impl Future<Output = bool> + Send {
        wow_world_instances::check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
            self,
            progression,
            area_id,
        )
    }

    fn update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        wow_world_entities::update_visibility_with_catalogs_like_cpp(self, creature_spawn_catalogs)
    }

    fn check_area_triggers_with_catalogs_like_cpp(
        &mut self,
        catalogs: &AreaTriggerCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        wow_world_instances::check_area_triggers_with_catalogs_like_cpp(self, catalogs)
    }

    fn sync_player_registry_state_like_cpp(&self) {
        WorldSession::sync_player_registry_state_like_cpp(self)
    }

    fn set_player_alive_like_cpp(&mut self, alive: bool) {
        WorldSession::set_player_alive_like_cpp(self, alive)
    }

    fn apply_represented_durability_loss_all_like_cpp(
        &mut self,
        percent: f64,
        inventory: bool,
    ) -> usize {
        wow_world_inventory::apply_represented_durability_loss_all_like_cpp(
            self, percent, inventory,
        )
    }

    fn taxi_node_map_ids_like_cpp(&self) -> &HashMap<u32, u16> {
        &self.view.taxi_node_map_ids_like_cpp
    }
}

impl wow_world_entities::PlayerTeleportHostLikeCpp for WorldSession {
    fn is_map_disabled_for_player_like_cpp(&self, map_id: u32) -> bool {
        WorldSession::is_map_disabled_for_player_like_cpp(self, map_id)
    }

    fn player_cannot_enter_target_map_like_cpp(&self, map_id: u32) -> Option<(u32, u8, i32)> {
        wow_world_instances::player_cannot_enter_target_map_like_cpp(self, map_id)
    }

    fn known_spells_like_cpp(&self) -> Vec<i32> {
        wow_world_spell::known_spells_like_cpp(self)
    }

    fn interrupt_non_melee_spells_for_far_teleport_like_cpp(&mut self) -> bool {
        WorldSession::interrupt_non_melee_spells_for_far_teleport_like_cpp(self)
    }

    fn remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp(&mut self) -> usize {
        wow_world_spell::remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp(self)
    }

    fn request_represented_battleground_leave_like_cpp(&mut self) {
        WorldSession::request_represented_battleground_leave_like_cpp(self)
    }

    fn sync_current_player_session_visibility_detection_like_cpp(&mut self) {
        WorldSession::sync_current_player_session_visibility_detection_like_cpp(self)
    }

    fn pending_teleport_like_cpp(&self) -> Option<(u32, Position)> {
        WorldSession::pending_teleport_like_cpp(self)
    }

    fn set_pending_teleport_like_cpp(&mut self, destination: Option<(u32, Position)>) -> bool {
        WorldSession::set_pending_teleport_like_cpp(self, destination)
    }

    fn active_area_trigger_mut_like_cpp(&mut self) -> &mut Option<u32> {
        &mut self.view.active_area_trigger
    }

    fn player_logout_like_cpp(&self) -> bool {
        self.lifecycle.player_logout_like_cpp()
    }

    fn resummon_pet_temporary_unsummoned_like_cpp(&mut self) {
        wow_world_lifecycle::resummon_pet_temporary_unsummoned_like_cpp(self)
    }
}
