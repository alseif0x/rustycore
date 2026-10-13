// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the combat bodies moved to
//! `wow_world_entities::combat_session` (#1363 slice 7a). Every method is a
//! pure forward to the same-named inherent `WorldSession` method or field.

use super::WorldSession;

impl wow_world_entities::CombatSessionHostLikeCpp for WorldSession {
    fn broadcast_player_packet_to_visible_set_realm_like_cpp(&self, bytes: Vec<u8>) {
        WorldSession::broadcast_player_packet_to_visible_set_realm_like_cpp(self, bytes)
    }
    fn present_committed_creature_melee_health_like_cpp(
        &mut self,
        committed_revision: u64,
    ) -> Option<u64> {
        WorldSession::present_committed_creature_melee_health_like_cpp(self, committed_revision)
    }
    fn publish_creature_melee_death_durability_loss_like_cpp(&mut self, over_damage: i32) {
        WorldSession::publish_creature_melee_death_durability_loss_like_cpp(self, over_damage)
    }
    fn resolved_visible_resting_like_cpp(&self) -> Option<bool> {
        WorldSession::resolved_visible_resting_like_cpp(self)
    }
    fn represented_player_is_polymorphed_like_cpp(&self) -> Option<bool> {
        WorldSession::represented_player_is_polymorphed_like_cpp(self)
    }
    fn is_ffa_pvp_realm_like_cpp(&self) -> bool {
        self.view.is_ffa_pvp_realm_like_cpp
    }
}
