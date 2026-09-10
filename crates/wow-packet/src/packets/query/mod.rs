// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Query packets: QueryCreature, QueryGameObject and their responses.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};

use crate::world_packet::PacketError;
use crate::{ClientPacket, ServerPacket, WorldPacket};

// ── Constants ────────────────────────────────────────────────────────

mod corpse;
mod creature;
mod game_object;
mod names;
mod quest;
mod text;

pub use corpse::*;
pub use creature::*;
pub use game_object::*;
pub use names::*;
pub use quest::*;
pub use text::*;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "quest_poi_tests.rs"]
mod quest_poi_tests;

#[cfg(test)]
#[path = "query_quest_completion_tests.rs"]
mod query_quest_completion_tests;
