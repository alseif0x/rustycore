// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social, group, guild, mail, auction and trade packets.

use super::*;

mod state_1;
mod state_2;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;

// ── FarSight (CMSG 0x34e8) ──────────────────────────────────────────

// ── Object update recovery (CMSG 0x3183 / 0x3184) ───────────────────────────

// ── ActiveGlyphs (SMSG 0x2c51) ──────────────────────────────────────

// ── PhaseShiftChange (SMSG 0x2578) ───────────────────────────────────────────
//
// Sent after AddToMap so the client knows which phases the player is in.
// Without this, the client may not render any world objects.
//
// C++ ref: `PhasingHandler::SendToPlayer` + `MiscPackets.cpp::PhaseShiftChange::Write`.
// Format:
//   WritePackedGuid(Client)         — player GUID
//   Phaseshift.Write():
//     WriteUInt32(PhaseShiftFlags)  — 0x08 = Unphased (default, no special phase)
//     WriteUInt32(Phases.Count)     — 0
//     WritePackedGuid(PersonalGUID) — empty
//   WriteUInt32(VisibleMapIDs * 2)  — size in bytes, followed by u16 ids
//   WriteUInt32(PreloadMapIDs * 2)  — size in bytes, followed by u16 ids
//   WriteUInt32(UiMapPhaseIDs * 2)  — size in bytes, followed by u16 ids

// ── NpcInteractionOpenResult ──────────────────────────────────────────────────

// ── QueryTimeResponse ────────────────────────────────────────────────────────

// ── LFG list status ──────────────────────────────────────────────────────────
