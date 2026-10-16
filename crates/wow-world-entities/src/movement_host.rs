// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World operations the moved movement bodies still call (#1363 slice 4b).
//!
//! C++ `WorldSession::HandleMovementOpcode`, the movement ACK handlers
//! (`Handlers/MovementHandler.cpp`) and `Player::HandleFall` run on the Player.
//! Their Rust bodies now live in this crate; each method below is the
//! same-named `WorldSession` operation, called at the exact point the body
//! called it before the move. Aura queries, zone, exploration, visibility,
//! area-trigger, durability, vitals and registry updates stay with the host.
//! The former `#[cfg(test)]` fixture steps run through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`.

use std::collections::HashMap;
use std::future::Future;

use wow_entities::RepresentedAuraEffectLikeCpp;
use wow_world_core::session::{ProgressionCatalogsLikeCpp, SessionHubHostLikeCpp};

use crate::CreatureSpawnCatalogsLikeCpp;

pub trait PlayerMovementHostLikeCpp: SessionHubHostLikeCpp {
    /// The World area-trigger catalogs; their script dispatcher takes the
    /// session, so only the host can name them.
    type AreaTriggerCatalogsLikeCpp: Sync;

    fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool>;
    fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32>;
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize;
    fn update_zone_represented_like_cpp(&mut self, new_zone: u32, new_area: u32) -> bool;
    fn check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
        &mut self,
        progression: &ProgressionCatalogsLikeCpp,
        area_id: u32,
    ) -> impl Future<Output = bool> + Send;
    fn update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send;
    fn check_area_triggers_with_catalogs_like_cpp(
        &mut self,
        catalogs: &Self::AreaTriggerCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send;
    fn sync_player_registry_state_like_cpp(&self);
    fn apply_represented_durability_loss_all_like_cpp(
        &mut self,
        percent: f64,
        inventory: bool,
    ) -> usize;
    fn taxi_node_map_ids_like_cpp(&self) -> &HashMap<u32, u16>;
}
