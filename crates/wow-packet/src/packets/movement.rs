// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet definitions.
//!
//! Handles all CMSG_MOVE_* client packets (player movement) and server-side
//! movement packets (MoveUpdate, OnMonsterMove).

use wow_constants::movement::{MovementFlag, MovementFlag2, MovementFlags3};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};
use wow_movement::{
    AnimTierTransition as MoveAnimTierTransition, MonsterMoveType, MoveSpline, MoveSplineFlag,
    SpellEffectExtraData as MoveSpellEffectExtraData,
};

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── MovementInfo ─────────────────────────────────────────────────

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "movement/tests/mod.rs"]
mod tests;

// ── ClientPlayerMovement (CMSG_MOVE_*) ───────────────────────────

/// Macro to implement ClientPacket for all movement opcodes.
macro_rules! impl_movement_client_packet {
    ($opcode:ident) => {
        // We can't use a macro for const OPCODE easily with multiple,
        // so we implement a shared read function instead.
    };
}

// ── Movement ACK client packets ──────────────────────────────────

// ── MoveTeleport (SMSG_MOVE_TELEPORT) ────────────────────────────

// ── MoveUpdateTeleport (SMSG_MOVE_UPDATE_TELEPORT) ───────────────

// ── MoveUpdate (SMSG_MOVE_UPDATE) ────────────────────────────────

// ── MonsterMove (SMSG_ON_MONSTER_MOVE) ───────────────────────────

// ── MonsterMoveStop ───────────────────────────────────────────────

// ── SetActiveMover (CMSG 0x3A3C) ──────────────────────────────────

// ── MoveInitActiveMoverComplete (CMSG 0x3A46) ─────────────────────
