// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Object update packets, organised by entity domain.
//!
//! Issue #227 split the former 9,735-line `update.rs` by entity domain.
//! Every public type and byte contract is unchanged.

mod block;
mod game_object;
mod item;
mod movement;
mod player;
mod unit;

pub use block::*;
pub use game_object::*;
pub use item::*;
pub use movement::*;
pub use player::*;
pub use unit::*;

use std::collections::BTreeSet;

use wow_constants::ServerOpcodes;

use wow_core::guid::TypeId;

use wow_core::{ObjectGuid, Position};

use wow_movement::{MonsterMoveType, MoveSpline, MoveSplineFlag};

use crate::packets::movement::TransportInfo;

use crate::{ServerPacket, WorldPacket};

// ── UpdateType ──────────────────────────────────────────────────────

/// Write an empty packed GUID (2 zero mask bytes).
fn write_empty_guid(buf: &mut WorldPacket) {
    buf.write_packed_guid(&ObjectGuid::EMPTY);
}

fn write_object_data_create_like_cpp(
    buf: &mut WorldPacket,
    entry_id: u32,
    dynamic_flags: u32,
    scale: f32,
) {
    buf.write_int32(entry_id as i32);
    buf.write_uint32(dynamic_flags);
    buf.write_float(scale);
}

fn debug_create_header_len_like_cpp(
    update_type: UpdateType,
    guid: &ObjectGuid,
    type_id: TypeId,
) -> usize {
    let mut header = WorldPacket::new_empty();
    header.write_uint8(update_type as u8);
    header.write_packed_guid(guid);
    header.write_uint8(type_id as u8);
    header.into_data().len()
}

#[cfg(test)]
#[path = "../update_tests.rs"]
mod tests;
