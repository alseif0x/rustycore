// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player combat at the session boundary (#1363 slice 7a): the regeneration
//! tick, the combat stop and PvP flag transitions, the melee absorb and power
//! publication, and the map-owned creature combat command receivers.
//!
//! C++ `Player::RegenerateAll`, `Unit::CombatStop`, `Unit::SetPvpFlag`,
//! `Unit::CalcAbsorbResist` and `Unit::SetPower` run on the Player. Their Rust
//! bodies now live in this crate; each host method below is the same-named
//! `WorldSession` operation, called at the exact point the body called it
//! before the move. The melee health presentation, the death durability loss,
//! the resting and polymorph reads, the realm flag and the visible-set fanout
//! stay with the host. The former `#[cfg(test)]` fixture steps run through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`.

use crate::movement_host::PlayerMovementHostLikeCpp;
use crate::world_entities_host::WorldEntitiesHostLikeCpp;

mod commands;
mod damage;
mod pvp_state;
mod regeneration;

pub use commands::{
    handle_apply_creature_melee_damage_like_cpp_command_like_cpp,
    handle_creature_attack_start_like_cpp_command_like_cpp,
    handle_creature_attack_stop_like_cpp_command_like_cpp,
    handle_reconcile_pvp_combat_expiry_like_cpp,
};
use damage::publish_melee_absorb_consumption_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use damage::send_player_power_update_like_cpp;
#[cfg(not(any(test, feature = "test-fixtures")))]
use damage::send_player_power_update_like_cpp;
pub use pvp_state::{
    apply_represented_ffa_pvp_login_state_like_cpp, combat_stop_like_cpp,
    reset_contested_pvp_like_cpp, set_represented_ffa_pvp_flag_like_cpp, update_pvp_flag_like_cpp,
};
pub use regeneration::tick_player_regeneration_like_cpp;

pub trait CombatSessionHostLikeCpp: WorldEntitiesHostLikeCpp + PlayerMovementHostLikeCpp {
    fn broadcast_player_packet_to_visible_set_realm_like_cpp(&self, bytes: Vec<u8>);
    fn present_committed_creature_melee_health_like_cpp(
        &mut self,
        committed_revision: u64,
    ) -> Option<u64>;
    fn publish_creature_melee_death_durability_loss_like_cpp(&mut self, over_damage: i32);
    fn resolved_visible_resting_like_cpp(&self) -> Option<bool>;
    fn represented_player_is_polymorphed_like_cpp(&self) -> Option<bool>;
    /// The World realm flag `SessionWorldView::is_ffa_pvp_realm_like_cpp`.
    fn is_ffa_pvp_realm_like_cpp(&self) -> bool;
}
