// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player taxi state.
//!
//! C++ `Player` owns `PlayerTaxi m_taxi` and performs the transitions on it:
//! `ClearTaxiDestinations` (`Entities/Player/PlayerTaxi.h:67`),
//! `AddTaxiDestination` (`:68`), `GetTaxiSource` (`:69`),
//! `GetTaxiDestination` (`:70`), `NextTaxiDestination` (`:74`), `empty`
//! (`:80`), `IsTaximaskNodeKnown` (`:44`) and `SetTaximaskNode` (`:50`), plus
//! the landing cleanup `Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`).
//!
//! Only the operations this Player's owners actually perform are modelled:
//! the route is appended and consumed by the flight path in `wow-world`, which
//! resolves taxi nodes from the catalog, so `AddTaxiDestination` (`:68`) and
//! `NextTaxiDestination` (`:74`) stay with that operation until it moves.
//!
//! Separated from `player/mod.rs` under #765, which also closed the fields to
//! this crate's Player module: reads keep named accessors and every write is a
//! named operation the owner checks.
//!
//! Two departures from C++ are preserved rather than introduced here. C++ keeps
//! no flight record on `PlayerTaxi` — the flight generator holds the path — so
//! the represented `flight` is RustyCore's own bookkeeping for the node the
//! Player is travelling to and the node that follows a teleport. And C++ keeps
//! the unit flags on `Unit` and the mount on the Player, where
//! `CleanupAfterTaxiFlight` reaches them; RustyCore mirrors the taxi-relevant
//! part of both beside the route, so the cleanup stays one transition.

use wow_core::Position;

/// One node of the represented flight path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerTaxiFlightNodeLikeCpp {
    pub map_id: u16,
    pub position: Position,
    pub teleport_flag: bool,
}

/// The node the Player is flying to, and the node that follows a teleport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerTaxiFlightStateLikeCpp {
    pub current_node: PlayerTaxiFlightNodeLikeCpp,
    pub node_after_teleport: Option<PlayerTaxiFlightNodeLikeCpp>,
}

/// Canonical owner of C++ `Player::m_taxi` (`PlayerTaxi.h:86-88`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerTaxiState {
    known_node_mask: Vec<u8>,
    known_node_mask_text: Option<String>,
    destinations: Vec<u32>,
    flight: Option<PlayerTaxiFlightStateLikeCpp>,
    unit_flags: u32,
    mounted: bool,
}

impl PlayerTaxiState {
    // ---- reads -------------------------------------------------------------

    /// The remaining route, as C++ `PlayerTaxi::GetPath` (`PlayerTaxi.h:79`)
    /// exposes `m_TaxiDestinations`.
    #[must_use]
    pub fn destinations_like_cpp(&self) -> &[u32] {
        &self.destinations
    }

    /// C++ `PlayerTaxi::GetTaxiDestination` (`:70`): the node after the source,
    /// which is `0` in C++ while the route holds fewer than two nodes.
    #[must_use]
    pub fn taxi_destination_like_cpp(&self) -> Option<u32> {
        self.destinations.get(1).copied()
    }

    /// C++ `PlayerTaxi::IsTaximaskNodeKnown` (`:44`), with the same field and
    /// submask arithmetic over one-based node indices. A node outside the
    /// loaded mask is unknown rather than a panic.
    #[must_use]
    pub fn is_taximask_node_known_like_cpp(&self, node_id: u32) -> bool {
        let Some(index) = node_id.checked_sub(1) else {
            return false;
        };
        let field = (index / 8) as usize;
        let submask = 1u8 << (index % 8);
        self.known_node_mask
            .get(field)
            .is_some_and(|value| (value & submask) == submask)
    }

    /// The loaded taxi mask, as C++ `PlayerTaxi::GetTaxiMask` (`:63`).
    #[must_use]
    pub fn known_node_mask_like_cpp(&self) -> &[u8] {
        &self.known_node_mask
    }

    /// The stored text the mask was loaded from (C++ `LoadTaxiMask`, `:42`).
    #[must_use]
    pub fn known_node_mask_text_like_cpp(&self) -> Option<&str> {
        self.known_node_mask_text.as_deref()
    }

    /// The represented flight, if the Player is on one.
    #[must_use]
    pub fn flight_like_cpp(&self) -> Option<PlayerTaxiFlightStateLikeCpp> {
        self.flight
    }

    /// Whether a represented flight is in progress.
    #[must_use]
    pub fn is_in_flight_like_cpp(&self) -> bool {
        self.flight.is_some()
    }

    /// The taxi-relevant unit flags mirrored beside the route.
    #[must_use]
    pub fn unit_flags_like_cpp(&self) -> u32 {
        self.unit_flags
    }

    /// Whether the taxi mount is up (C++ `Player::Dismount` clears it).
    #[must_use]
    pub fn mounted_like_cpp(&self) -> bool {
        self.mounted
    }

    // ---- transitions -------------------------------------------------------

    /// Build the state from one represented payload, as the login path hands
    /// the Player its loaded `PlayerTaxi`.
    #[must_use]
    pub fn from_represented_parts_like_cpp(
        destinations: Vec<u32>,
        flight: Option<PlayerTaxiFlightStateLikeCpp>,
        unit_flags: u32,
        mounted: bool,
    ) -> Self {
        Self {
            destinations,
            flight,
            unit_flags,
            mounted,
            ..Self::default()
        }
    }

    /// Install a whole route, as the load path replaces `m_TaxiDestinations`
    /// (C++ `PlayerTaxi::LoadTaxiDestinationsFromString`, `:65`).
    pub fn replace_destinations_like_cpp(&mut self, destinations: Vec<u32>) {
        self.destinations = destinations;
    }

    /// C++ `PlayerTaxi::SetTaximaskNode` (`:50`): set the node's bit and answer
    /// whether it was newly learned. A node beyond the loaded mask grows it,
    /// where C++ indexes a fixed-size `TaxiMask`.
    pub fn set_taximask_node_like_cpp(&mut self, node_id: u32) -> bool {
        let Some(index) = node_id.checked_sub(1) else {
            return false;
        };
        let field = (index / 8) as usize;
        let submask = 1u8 << (index % 8);
        if self.known_node_mask.len() <= field {
            self.known_node_mask.resize(field + 1, 0);
        }
        let value = &mut self.known_node_mask[field];
        if (*value & submask) == submask {
            return false;
        }
        *value |= submask;
        true
    }

    /// C++ `PlayerTaxi::LoadTaxiMask` (`:42`) installing the stored mask and
    /// keeping the text it came from.
    pub fn load_taxi_mask_like_cpp(&mut self, mask: Vec<u8>, text: Option<String>) {
        self.known_node_mask = mask;
        self.known_node_mask_text = text;
    }

    /// Begin the represented flight towards `current_node`.
    pub fn begin_taxi_flight_like_cpp(
        &mut self,
        current_node: PlayerTaxiFlightNodeLikeCpp,
        node_after_teleport: Option<PlayerTaxiFlightNodeLikeCpp>,
    ) {
        self.flight = Some(PlayerTaxiFlightStateLikeCpp {
            current_node,
            node_after_teleport,
        });
    }

    /// The node that follows a teleport becomes the current node, and nothing
    /// follows it any more. Answers the node taken, or `None` when there was no
    /// flight or no node after the teleport.
    pub fn advance_taxi_flight_after_teleport_like_cpp(
        &mut self,
    ) -> Option<PlayerTaxiFlightNodeLikeCpp> {
        let node = self.flight?.node_after_teleport?;
        self.flight = Some(PlayerTaxiFlightStateLikeCpp {
            current_node: node,
            node_after_teleport: None,
        });
        Some(node)
    }

    /// Mirror the taxi-relevant unit flags and mount state beside the route.
    pub fn set_taxi_cleanup_state_like_cpp(&mut self, unit_flags: u32, mounted: bool) {
        self.unit_flags = unit_flags;
        self.mounted = mounted;
    }

    /// C++ `Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`): clear the
    /// route, dismount and remove the taxi unit flags, together. The
    /// represented flight ends with them, because it describes the route being
    /// cleared.
    pub fn cleanup_after_taxi_flight_like_cpp(&mut self, taxi_unit_flags: u32) {
        self.destinations.clear();
        self.flight = None;
        self.mounted = false;
        self.unit_flags &= !taxi_unit_flags;
    }
}
