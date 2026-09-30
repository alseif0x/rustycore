// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::teleport` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Near, far and delayed teleport state and acks, the pending teleport, the homebind and spline-
/// done taxi events.
pub(in crate::session) struct TeleportState {
    /// `MoveSplineDone` taxi decisions recorded until full Taxi/MotionMaster runtime exists.
    #[cfg(test)]
    pub(in crate::session) move_spline_done_taxi_events_like_cpp:
        Vec<MoveSplineDoneTaxiEventLikeCpp>,
    /// C++ `Player::m_bCanDelayTeleport`, represented around update-owned work.
    #[cfg(test)]
    pub(in crate::session) represented_can_delay_teleport_like_cpp: bool,
    /// C++ `Player::m_bHasDelayedTeleport`, represented for same-map near teleports.
    #[cfg(test)]
    pub(in crate::session) represented_has_delayed_teleport_like_cpp: bool,
    /// C++ `Player::mSemaphoreTeleport_Near` represented state.
    #[cfg(test)]
    pub(in crate::session) near_teleport_pending_like_cpp: bool,
    /// C++ `Player::mSemaphoreTeleport_Far` represented state.
    #[cfg(test)]
    pub(in crate::session) represented_far_teleport_pending_like_cpp: bool,
    /// C++ `Player::m_teleport_dest` represented state for near teleports.
    #[cfg(test)]
    pub(in crate::session) near_teleport_destination_like_cpp: Option<(u16, wow_core::Position)>,
    /// Saved `TeleportTo` arguments while `m_bHasDelayedTeleport` is set.
    #[cfg(test)]
    pub(in crate::session) represented_delayed_teleport_like_cpp:
        Option<(u32, wow_core::Position, TeleportToOptionsLikeCpp)>,
    /// Represented zone/area for the pending near-teleport destination.
    #[cfg(test)]
    pub(in crate::session) near_teleport_destination_zone_area_like_cpp: Option<(u32, u32)>,
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::m_homebind` lives on the canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_homebind_like_cpp: Option<RepresentedHomebindLikeCpp>,
    /// Near teleport ACK side-effect audit events.
    #[cfg(test)]
    pub(in crate::session) move_teleport_ack_events_like_cpp: Vec<MoveTeleportAckEventLikeCpp>,

    /// Ownerless legacy fixtures only; production uses Player's teleport state.
    #[cfg(test)]
    pub(in crate::session) pending_teleport: Option<(u32, wow_core::Position)>,
}
