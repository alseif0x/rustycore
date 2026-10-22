// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player corpse Map outcomes (#43); the Map methods live in
//! `map/player_corpses.rs`.

use wow_core::ObjectGuid;

/// C++ `CORPSE_FLAG_BONES` (`Corpse.h:43`).
pub const CORPSE_FLAG_BONES_LIKE_CPP: u32 = 0x01;

/// The Map half of C++ `Map::ConvertCorpseToBones`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvertCorpseToBonesOutcomeLikeCpp {
    /// The removed resurrectable corpse.
    pub corpse_guid: ObjectGuid,
    /// The bones created in its place, when bones were allowed and the
    /// corpse's grid was not being removed.
    pub bones_guid: Option<ObjectGuid>,
}
