// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::visibility` sub-state (#1241 F2): moved fields, no logic.

#[cfg(test)]
use super::*;

/// Per-client visibility and publication fences: visible transports, last visibility position and
/// farsight, delivered-update guards.
pub(crate) struct VisibilityState {
    /// C++ `Player::m_visibleTransports`, maintained by `Map::SendInitTransports`.
    pub(crate) client_visible_transports_like_cpp:
        crate::session::mailbox::SharedClientVisibleTransportsLikeCpp,
    /// Detached represented inputs used only by visibility tests.
    #[cfg(test)]
    pub(crate) visibility_test_fixture_like_cpp: VisibilityTestFixtureLikeCpp,
    /// Last canonical FarsightObject value observed by this Session's
    /// publication rail. This is a delivery fence, not gameplay authority:
    /// it lets the session emit the one explicit clear packet required when a
    /// map-owned viewpoint disappears between map ticks.
    pub(in crate::session) last_observed_farsight_object_like_cpp: wow_core::ObjectGuid,
    /// Session-local delivery guard for represented DynamicObject VALUES packets
    /// consumed from the last map-owned `Map::SendObjectUpdates` stable snapshot.
    /// Includes the map update generation so repeated `process_pending()` calls
    /// suppress the same snapshot without blocking later identical bytes.
    pub(in crate::session) represented_dynamic_object_values_updates_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid, u64)>,
    /// Session-local delivery guard for Player/Creature/Pet VALUES packets
    /// consumed from the canonical map's `Map::SendObjectUpdates` snapshot.
    pub(in crate::session) represented_player_unit_values_updates_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid, u64)>,
    /// Session-local delivery guard for represented GameObjectDespawn packets
    /// consumed from the last map-owned `GameObject::Update` summary.
    pub(in crate::session) represented_gameobject_visual_despawns_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid)>,
    /// Session-local delivery guard for represented CapturePointRemoved packets
    /// consumed from the last map-owned `GameObject::Delete` summary.
    pub(in crate::session) represented_capture_point_removed_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid)>,
    /// Position at which visibility was last fully recalculated.
    pub(crate) last_visibility_pos: Option<wow_core::Position>,
}
