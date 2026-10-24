// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player teleport and far transfer at the session boundary (#1363 slice 11a;
//! moved from the World `session/movement/{transfer,far_transfer}.rs`).
//!
//! C++ `Player::TeleportTo` (`Player.cpp`): the catalog, DisableMgr,
//! battleground and expansion gates, the same-map near teleport, the delayed
//! teleport after `Player::Update`, the far-transfer detach with
//! `SMSG_TRANSFER_PENDING`, the interruptible writer fence and
//! `SMSG_SUSPEND_TOKEN`, and `WorldSession::HandleMoveTeleportAck`'s near
//! state machine with the delayed resurrection. The far-transfer completion
//! (`HandleMoveWorldportAck`) stays with the World travel handler. Each host
//! method below is the same-named `WorldSession` operation (or field), called
//! at the exact point the body called it before the move; the former
//! `#[cfg(test)]` fixture steps run through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`.

use wow_core::Position;
use wow_world_core::session::movement_protocol::TeleportToOptionsLikeCpp;
use std::future::Future;

use crate::combat_session::CombatSessionHostLikeCpp;

mod far_transfer;
mod teleport;
mod teleport_ack;

pub use teleport::{
    pending_teleport_save_destination_like_cpp,
    process_represented_delayed_teleport_after_update_like_cpp,
    teleport_to_with_options, teleport_to_with_options_and_resurrection,
};
pub use teleport_ack::{
    handle_move_teleport_ack_like_cpp,
    process_represented_delayed_resurrection_after_teleport_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use teleport_ack::schedule_represented_resurrection_after_teleport;

pub const TELE_TO_NONE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x00;
pub const TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x04;
pub const TELE_TO_SPELL_LIKE_CPP: TeleportToOptionsLikeCpp = 0x10;
pub const TELE_REVIVE_AT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x40;
pub const TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP: u32 = 7;
pub const TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP: u32 = 9;
pub const TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP: u32 = 16;
pub const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
pub const DEATH_KNIGHT_START_MAP_LIKE_CPP: u16 = 609;
pub const DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP: i32 = 50977;

/// The World operations the moved teleport bodies still call; each is the
/// same-named `WorldSession` operation, or the named field.
pub trait PlayerTeleportHostLikeCpp: CombatSessionHostLikeCpp {
    /// The instances split's C++ `DisableMgr` map gate.
    fn is_map_disabled_for_player_like_cpp(&self, map_id: u32) -> bool;
    /// C++ `Map::PlayerCannotEnter` for the target map (instances).
    fn player_cannot_enter_target_map_like_cpp(&self, map_id: u32) -> Option<(u32, u8, i32)>;
    /// `wow_world_spell::known_spells_like_cpp`.
    fn known_spells_like_cpp(&self) -> Vec<i32>;
    fn interrupt_non_melee_spells_for_far_teleport_like_cpp(&mut self) -> bool;
    /// `wow_world_spell::remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp`.
    fn remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp(&mut self) -> usize;
    fn sync_current_player_session_visibility_detection_like_cpp(&mut self);
    fn pending_teleport_like_cpp(&self) -> Option<(u32, Position)>;
    fn set_pending_teleport_like_cpp(&mut self, destination: Option<(u32, Position)>) -> bool;
    /// The session view's `active_area_trigger` field.
    fn active_area_trigger_mut_like_cpp(&mut self) -> &mut Option<u32>;
    /// The lifecycle state's `player_logout_like_cpp`.
    fn player_logout_like_cpp(&self) -> bool;
    /// `wow_world_lifecycle::resummon_pet_temporary_unsummoned_like_cpp`.
    fn resummon_pet_temporary_unsummoned_like_cpp(&mut self);
    /// C++ `ResurrectPlayer` at `TELE_REVIVE_AT_TELEPORT`, using explicit
    /// catalogs while keeping corpse operations inside the World adapter.
    fn revive_for_teleport<'a>(
        &'a mut self,
        catalogs: crate::ResurrectionOperationCatalogs<'a>,
        map_id: u32,
        destination: Position,
    ) -> impl Future<Output = bool> + Send + 'a;
}
