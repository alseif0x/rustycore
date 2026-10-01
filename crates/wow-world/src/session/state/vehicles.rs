// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::vehicles` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Taxi flight and mount/vehicle kit state: destinations, seat state, vehicle requests, transport
/// attach and mount counters.
pub(crate) struct TaxiVehicleState {
    /// Represented `PlayerTaxi::m_TaxiDestinations` until PlayerTaxi/MotionMaster runtime is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) taxi_destinations_like_cpp: Vec<u32>,
    /// Represented accepted `CMSG_ACTIVATE_TAXI` requests until TaxiPathGraph/MotionMaster are canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) represented_activate_taxi_requests_like_cpp:
        Vec<RepresentedActivateTaxiLikeCpp>,
    /// Represented active `FlightPathMovementGenerator`, if any.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) taxi_flight_state_like_cpp: Option<RepresentedTaxiFlightStateLikeCpp>,
    /// Represented unit flags touched by `CleanupAfterTaxiFlight`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) taxi_unit_flags_like_cpp: UnitFlags,
    /// Represented mount state touched by `CleanupAfterTaxiFlight`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) taxi_mounted_like_cpp: bool,
    /// Handle-less fixture for C++ `Unit::GetMountDisplayId()`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_mount_display_id_like_cpp: i32,
    /// Represented vehicle id selected from mount creature template until VehicleKit exists.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_mount_vehicle_id_like_cpp: u32,
    /// Legacy handle-less test fixture for C++ `Unit::m_vehicleKit`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_mount_vehicle_kit_like_cpp: Option<Vehicle>,
    /// Vehicle accessory rows selected by C++ `Vehicle::InstallAllAccessories(false)`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_mount_vehicle_accessories_like_cpp: Vec<VehicleAccessory>,
    /// Represented number of VehicleSeat rows installed by C++ `Vehicle` constructor.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_mount_vehicle_seat_count_like_cpp: u8,
    /// Represented C++ `Vehicle::UsableSeatNum`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_mount_vehicle_usable_seat_count_like_cpp: u8,
    /// Legacy handle-less test fixture for current `VehicleSeatEntry::Flags`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_vehicle_seat_flags_like_cpp: Option<i32>,
    /// Legacy handle-less test fixture for current `VehicleSeatEntry::ID`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_vehicle_seat_id_like_cpp: Option<u32>,
    /// Represented `Player::ChangeSeat(seatId, next)` requests until live vehicle ownership exists.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) represented_vehicle_seat_change_requests_like_cpp:
        Vec<RepresentedVehicleSeatChangeRequestLikeCpp>,
    /// Represented cross-vehicle `HandleSpellClick(player, seatId)` requests from vehicle switching.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) represented_vehicle_seat_spell_click_requests_like_cpp:
        Vec<RepresentedVehicleSeatSpellClickRequestLikeCpp>,
    /// Represented `Player::EnterVehicle(targetPlayer)` requests from `CMSG_RIDE_VEHICLE_INTERACT`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) represented_vehicle_enter_requests_like_cpp:
        Vec<RepresentedVehicleEnterRequestLikeCpp>,
    /// Count of C++ `CreateVehicleKit` mount side effects represented until Vehicle runtime sends packets.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) mount_vehicle_create_requests_like_cpp: u32,
    /// Count of C++ `RemoveVehicleKit` mount side effects represented until Vehicle runtime sends packets.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) mount_vehicle_remove_requests_like_cpp: u32,
    /// Count of C++ `SendOnCancelExpectedVehicleRideAura` packets emitted after vehicle-kit creation.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) mount_cancel_expected_vehicle_aura_packets_like_cpp: u32,
    /// Count of C++ mount collision-height updates represented until movement packets are canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) mount_collision_height_update_requests_like_cpp: u32,
    /// Handle-less fixture for C++ `UNIT_FLAG_MOUNT`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_mounted_like_cpp: bool,
    /// Represented transport guard for speed ACK anticheat; C++ skips speed mismatch while on transport.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_on_transport_like_cpp: bool,
    /// Current C++ `m_movementInfo.transport.guid`, used to exclude the
    /// player's own transport from `Map::SendInitTransports`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_transport_login_state_like_cpp:
        Option<Box<PlayerTransportLoginStateLikeCpp>>,
}
