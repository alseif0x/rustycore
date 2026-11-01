// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the vehicle handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub and runs the deferred registry-state publication.
//!
//! `#1263 F5 remaining families`: the adapter also lends the four seat-change,
//! dismiss and ride-interact bodies, which moved to
//! `vehicle_session` in #1363 slice 4b (now `wow_world_entities`, slice A1);
//! the spell-click plan and raid check they still need are lent below.

use wow_handler::HandlerFuture;
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_world_entities::{VehicleHandlerCxLikeCpp, VehicleHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl VehicleHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn vehicle_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> VehicleHandlerCxLikeCpp<'a> {
        VehicleHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    fn sync_player_registry_state_after_vehicle_change_like_cpp(&mut self) {
        self.sync_player_registry_state_like_cpp();
    }

    fn handle_move_dismiss_vehicle<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            let mut pkt = pkt;
            match wow_packet::packets::vehicle::MoveDismissVehicle::read(&mut pkt) {
                Ok(packet) => wow_world_entities::handle_move_dismiss_vehicle(self, packet).await,
                Err(e) => tracing::warn!("Failed to read MoveDismissVehicle: {e}"),
            }
        })
    }

    fn handle_move_change_vehicle_seats<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            let mut pkt = pkt;
            match wow_packet::packets::vehicle::MoveChangeVehicleSeats::read(&mut pkt) {
                Ok(packet) => {
                    wow_world_entities::handle_move_change_vehicle_seats(self, packet).await
                }
                Err(e) => tracing::warn!("Failed to read MoveChangeVehicleSeats: {e}"),
            }
        })
    }

    fn handle_request_vehicle_switch_seat<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            let mut pkt = pkt;
            match wow_packet::packets::vehicle::RequestVehicleSwitchSeat::read(&mut pkt) {
                Ok(packet) => {
                    wow_world_entities::handle_request_vehicle_switch_seat(self, packet).await
                }
                Err(e) => tracing::warn!("Failed to read RequestVehicleSwitchSeat: {e}"),
            }
        })
    }

    fn handle_ride_vehicle_interact<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            let mut pkt = pkt;
            match wow_packet::packets::vehicle::RideVehicleInteract::read(&mut pkt) {
                Ok(packet) => wow_world_entities::handle_ride_vehicle_interact(self, packet).await,
                Err(e) => tracing::warn!("Failed to read RideVehicleInteract: {e}"),
            }
        })
    }
}

impl wow_world_entities::VehicleSessionHostLikeCpp for WorldSession {
    fn represented_handle_spell_click_plan_with_seat_like_cpp(
        &self,
        creature_guid: wow_core::ObjectGuid,
        seat_id: Option<i8>,
    ) -> wow_world_entities::spell_click_values::RepresentedSpellClickPlanLikeCpp {
        wow_world_spell::represented_handle_spell_click_plan_with_seat_like_cpp(
            self,
            creature_guid,
            seat_id,
        )
    }

    fn represented_player_is_same_raid_with_like_cpp(
        &self,
        player_guid: wow_core::ObjectGuid,
        owner_guid: wow_core::ObjectGuid,
    ) -> bool {
        wow_world_instances::represented_player_is_same_raid_with_like_cpp(
            self,
            player_guid,
            owner_guid,
        )
    }
}
