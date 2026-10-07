// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the movement handler context (#1263 F5).
//!
//! The application crate owns the C++ `MovementHandler.cpp` and
//! `VehicleHandler.cpp` bodies and their context; the session only lends its
//! hub, so no session reference crosses into the handler. The validated-ACK
//! steps stay here because they read the represented player shell the hub
//! context does not reach.

use wow_constants::ClientOpcodes;
use wow_packet::packets::movement::MovementAck;
use wow_world_application::{MovementHandlerCxLikeCpp, MovementHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl MovementHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn movement_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> MovementHandlerCxLikeCpp<'a> {
        MovementHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    fn apply_move_set_vehicle_rec_id_ack_like_cpp(&mut self, ack: &mut MovementAck) {
        // C++ `HandleMoveSetVehicleRecAck` has no session-visible side effect,
        // so the #142 wire-dispatch test counts this step to prove that the
        // registered thunk reached the C++ body's dependency point.
        #[cfg(test)]
        crate::handlers::movement::record_move_set_vehicle_rec_id_ack_handler_call_for_test();
        WorldSession::apply_move_set_vehicle_rec_id_ack_like_cpp(self, ack);
    }

    fn record_validated_movement_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut MovementAck,
        speed: Option<f32>,
    ) -> bool {
        WorldSession::record_validated_movement_ack_like_cpp(self, opcode, ack, speed)
    }
}
