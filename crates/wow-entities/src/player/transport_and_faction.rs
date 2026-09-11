// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Transport and championing-faction transitions C++ keeps on the object itself.
//!
//! All three were reached here by opening the Player's gameplay state from the
//! session. Giving them names keeps the fields private to their owner without
//! pretending they are more than the assignments they are.

use crate::{Player, PlayerTransportState};
use wow_core::Position;

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

    /// The `VehicleSeatEntry::Flags` and `ID` of the seat this Player
    /// occupies. C++ resolves the seat on demand through
    /// `Vehicle::GetSeatForPassenger` (Vehicle.h:68); RustyCore caches the two
    /// values it needs on the passenger they describe, so both move together
    /// and the cache can never describe half a seat.
    pub fn set_vehicle_seat_like_cpp(&mut self, flags: Option<i32>, seat_id: Option<u32>) {
        let state = self.gameplay_state_mut();
        state.vehicle_seat_flags = flags;
        state.vehicle_seat_id = seat_id;
    }

    /// The transport offset inside the Player's own `m_movementInfo.transport`,
    /// which C++ reassigns wholesale from the validated client status
    /// (`MovementHandler.cpp:119` after a worldport ack, `:405` on ordinary
    /// movement). Only the offset moves: guid, seat and timing stay as the
    /// transport set them, and a Player on no transport is left unchanged.
    pub fn set_transport_position_like_cpp(&mut self, position: Position) {
        let Some(transport) = self.gameplay_state_mut().transport.as_mut() else {
            return;
        };
        transport.x = position.x;
        transport.y = position.y;
        transport.z = position.z;
        transport.orientation = position.orientation;
    }
}
