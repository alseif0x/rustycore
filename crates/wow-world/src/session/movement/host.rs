// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the movement bodies that moved to
//! `wow_world_entities` (#1363 slice 4b). Each method delegates to the
//! same-named session operation; the fall-damage hook keeps the cfg(test)
//! fixture push the moved body ran before the move.

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
        WorldSession::remove_auras_with_interrupt_flags_like_cpp(self, flags, flags2)
    }

    fn request_jump_proc_like_cpp(&mut self) {
        WorldSession::request_jump_proc_like_cpp(self)
    }

    fn handle_under_map_like_cpp(
        &mut self,
        movement_info: &wow_packet::packets::movement::MovementInfo,
    ) -> Option<MovementUnderMapDamageEvent> {
        WorldSession::handle_under_map_like_cpp(self, movement_info)
    }

    fn update_zone_represented_like_cpp(&mut self, new_zone: u32, new_area: u32) -> bool {
        WorldSession::update_zone_represented_like_cpp(self, new_zone, new_area)
    }

    fn check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
        &mut self,
        progression: &ProgressionCatalogsLikeCpp,
        area_id: u32,
    ) -> impl Future<Output = bool> + Send {
        WorldSession::check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
            self,
            progression,
            area_id,
        )
    }

    fn update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::update_visibility_with_catalogs_like_cpp(self, creature_spawn_catalogs)
    }

    fn check_area_triggers_with_catalogs_like_cpp(
        &mut self,
        catalogs: &AreaTriggerCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::check_area_triggers_with_catalogs_like_cpp(self, catalogs)
    }

    fn sync_player_registry_state_like_cpp(&self) {
        WorldSession::sync_player_registry_state_like_cpp(self)
    }

    fn apply_represented_durability_loss_all_like_cpp(
        &mut self,
        percent: f64,
        inventory: bool,
    ) -> usize {
        WorldSession::apply_represented_durability_loss_all_like_cpp(self, percent, inventory)
    }

    fn taxi_node_map_ids_like_cpp(&self) -> &HashMap<u32, u16> {
        &self.view.taxi_node_map_ids_like_cpp
    }

    fn record_move_spline_done_taxi_event_like_cpp(
        &mut self,
        spline_id: i32,
        action: MoveSplineDoneTaxiActionLikeCpp,
        destination_node_id: Option<u32>,
        teleport_map_id: Option<u16>,
        teleport_position: Option<wow_core::Position>,
        honorless_target_cast: bool,
    ) -> MoveSplineDoneTaxiActionLikeCpp {
        WorldSession::record_move_spline_done_taxi_event_like_cpp(
            self,
            spline_id,
            action,
            destination_node_id,
            teleport_map_id,
            teleport_position,
            honorless_target_cast,
        )
    }

    fn record_movement_fall_damage_event_like_cpp(&mut self, event: MovementFallDamageEvent) {
        #[cfg(test)]
        self.fixtures
            .movement
            .fall_damage_events_like_cpp
            .push(event);
        let _ = event;
    }
}
