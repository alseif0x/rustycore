// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Two single-field Player transitions C++ also keeps on the object itself.
//!
//! Both are one-line assignments in Classic, and both were reached here by
//! opening the Player's gameplay state from the session. Giving them names
//! keeps the field private to its owner without pretending they are more than
//! the assignments they are.

use crate::{Player, PlayerTransportState};

impl Player {
    /// C++ `Player::SetChampioningFaction` (Player.h:2609).
    pub fn set_championing_faction_like_cpp(&mut self, faction_id: u32) {
        self.gameplay_state_mut().championing_faction_id = faction_id;
    }

    /// C++ `WorldObject::SetTransport` (Object.h:739), whose state this
    /// represented Player carries.
    pub fn set_transport_like_cpp(&mut self, transport: Option<PlayerTransportState>) {
        self.gameplay_state_mut().transport = transport;
    }
}
