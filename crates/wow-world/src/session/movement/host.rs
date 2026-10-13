// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the movement bodies that moved to
//! `wow_world_entities` (#1363 slice 4b). Each method delegates to the
//! same-named session operation.

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
        wow_world_entities::update_visibility_with_catalogs_like_cpp(self, creature_spawn_catalogs)
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

    fn set_player_alive_like_cpp(&mut self, alive: bool) {
        WorldSession::set_player_alive_like_cpp(self, alive)
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
}
