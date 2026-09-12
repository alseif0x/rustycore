// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player world-local state: where the Player is and what that
//! location makes true of them.
//!
//! C++ writes these on the Player as it moves: `Player::UpdateZone` and
//! `Player::UpdateArea` (`Entities/Player/Player.cpp`) set the zone and area,
//! `Player::UpdatePvPState` maintains `pvpInfo.IsHostile` and
//! `pvpInfo.EndTimer`, `Player::UpdateContestedPvP` maintains
//! `m_contestedPvPTimer`, and `WorldObject::IsOutdoors()` answers from the
//! terrain the position refresh established.
//!
//! Separated from `player/mod.rs` under #781, which also closed the members.
//! The terrain lookups, the zone catalog and the packets stay in `wow-world`:
//! this owner holds what the Player knows about its location, not how that
//! location is resolved.
//!
//! Two facts here are RustyCore's own and are recorded rather than presented as
//! C++ members. The zone/area authority flag says whether terrain actually
//! produced the pair, so a stale pair is never read as established; C++ has no
//! equivalent because it only asks the map while the Player is in world. And
//! the outdoors value is tri-state: `None` means terrain or VMAP has not
//! established it for the current position, where C++ `IsOutdoors()` answers a
//! plain bool from data it always has.

/// C++ `Player` state updated by `UpdateZone`, `UpdateArea`, `UpdatePvPState`
/// and `UpdateContestedPvP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerWorldLocalState {
    pub(super) zone_id: u32,
    pub(super) area_id: u32,
    /// Rust extraction fidelity: true only when terrain produced zone/area.
    pub(super) zone_area_authority_complete: bool,
    /// C++ `Player::pvpInfo.IsHostile`.
    pub(super) pvp_hostile: bool,
    /// C++ `Player::pvpInfo.EndTimer`; `None` mirrors C++ zero.
    pub(super) pvp_end_timer: Option<i64>,
    /// C++ `Player::m_contestedPvPTimer`.
    pub(super) contested_pvp_timer: u32,
    /// C++ `WorldObject::IsOutdoors()` result; `None` means terrain/VMAP has
    /// not established authority for the current position.
    pub(super) is_outdoors: Option<bool>,
}

impl PlayerWorldLocalState {
    // ---- reads -------------------------------------------------------------

    /// C++ `Player::GetZoneId()`.
    #[must_use]
    pub fn zone_id_like_cpp(&self) -> u32 {
        self.zone_id
    }

    /// C++ `Player::GetAreaId()`.
    #[must_use]
    pub fn area_id_like_cpp(&self) -> u32 {
        self.area_id
    }

    /// The zone and area together, as every caller that acts on the location
    /// needs both.
    #[must_use]
    pub fn zone_area_like_cpp(&self) -> (u32, u32) {
        (self.zone_id, self.area_id)
    }

    /// Whether terrain actually produced the current zone and area pair.
    #[must_use]
    pub fn has_zone_area_authority_like_cpp(&self) -> bool {
        self.zone_area_authority_complete
    }

    /// C++ `Player::pvpInfo.IsHostile`.
    #[must_use]
    pub fn is_pvp_hostile_like_cpp(&self) -> bool {
        self.pvp_hostile
    }

    /// C++ `Player::pvpInfo.EndTimer`; `None` is the C++ zero.
    #[must_use]
    pub fn pvp_end_timer_like_cpp(&self) -> Option<i64> {
        self.pvp_end_timer
    }

    /// C++ `Player::m_contestedPvPTimer`.
    #[must_use]
    pub fn contested_pvp_timer_like_cpp(&self) -> u32 {
        self.contested_pvp_timer
    }

    /// C++ `WorldObject::IsOutdoors()`; `None` means terrain or VMAP has not
    /// established it for the current position.
    #[must_use]
    pub fn is_outdoors_like_cpp(&self) -> Option<bool> {
        self.is_outdoors
    }

    /// Rebuild the world-local state from one represented payload, for the
    /// handle-less mirror that has no Player to borrow. The zone and area
    /// arrive with the authority flag that vouches for them, and the PvP facts
    /// with the timer they belong to.
    #[must_use]
    pub fn from_represented_parts_like_cpp(
        zone_id: u32,
        area_id: u32,
        zone_area_authority_complete: bool,
        pvp_hostile: bool,
        pvp_end_timer: Option<i64>,
        contested_pvp_timer: u32,
        is_outdoors: Option<bool>,
    ) -> Self {
        Self {
            zone_id,
            area_id,
            zone_area_authority_complete,
            pvp_hostile,
            pvp_end_timer,
            contested_pvp_timer,
            is_outdoors,
        }
    }

    // ---- transitions -------------------------------------------------------

    /// C++ `Player::UpdateZone` (`Player.cpp`) storing the zone it moved into.
    /// A zone that differs from the stored one drops the authority flag,
    /// because the pair the flag vouched for is no longer the current one.
    pub fn set_zone_id_like_cpp(&mut self, zone_id: u32) {
        if self.zone_id != zone_id {
            self.zone_area_authority_complete = false;
        }
        self.zone_id = zone_id;
    }

    /// C++ `Player::UpdateArea` (`Player.cpp`) storing the area it moved into,
    /// with the same authority rule as the zone.
    pub fn set_area_id_like_cpp(&mut self, area_id: u32) {
        if self.area_id != area_id {
            self.zone_area_authority_complete = false;
        }
        self.area_id = area_id;
    }

    /// Store a zone and area the terrain resolved together, dropping the
    /// authority flag when either changed.
    pub fn set_zone_area_like_cpp(&mut self, zone_id: u32, area_id: u32) {
        if self.zone_id != zone_id || self.area_id != area_id {
            self.zone_area_authority_complete = false;
        }
        self.zone_id = zone_id;
        self.area_id = area_id;
    }

    /// Record whether terrain established the stored zone and area pair.
    pub fn set_zone_area_authority_like_cpp(&mut self, complete: bool) {
        self.zone_area_authority_complete = complete;
    }

    /// C++ `Player::UpdatePvPState` writing `pvpInfo.IsHostile`.
    pub fn set_pvp_hostile_like_cpp(&mut self, hostile: bool) {
        self.pvp_hostile = hostile;
    }

    /// C++ `Player::UpdatePvPState` writing `pvpInfo.EndTimer`; `None` is the
    /// C++ zero that means no timer is running.
    pub fn set_pvp_end_timer_like_cpp(&mut self, end_timer: Option<i64>) {
        self.pvp_end_timer = end_timer;
    }

    /// C++ `Player::UpdateContestedPvP` writing `m_contestedPvPTimer`.
    pub fn set_contested_pvp_timer_like_cpp(&mut self, timer: u32) {
        self.contested_pvp_timer = timer;
    }

    /// Record the `WorldObject::IsOutdoors()` result terrain established for
    /// the current position.
    pub fn set_is_outdoors_like_cpp(&mut self, is_outdoors: Option<bool>) {
        self.is_outdoors = is_outdoors;
    }
}
