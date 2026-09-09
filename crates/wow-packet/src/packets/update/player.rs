// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! ActivePlayer and Player update blocks.

use super::*;

mod state_1;
mod state_2;
mod state_3;
mod state_4;
mod state_5;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;
pub(in crate::packets::update) use state_3::{
    MAX_ACTION_BUTTONS, VALUES_TYPE_ACTIVE_PLAYER, VALUES_TYPE_PLAYER,
    debug_player_create_values_len_like_cpp, write_active_player_movement_block,
    write_full_active_player_values_update_block, write_full_player_values_update_block,
    write_player_values_update_block,
};
#[allow(unused_imports)]
pub use state_4::*;
pub(in crate::packets::update) use state_5::write_active_player_data_values_update;
#[allow(unused_imports)]
pub use state_5::*;

// ── PlayerCombatStats ──────────────────────────────────────────────

// ── PlayerCreateData ────────────────────────────────────────────────

// ── Helpers ─────────────────────────────────────────────────────────
