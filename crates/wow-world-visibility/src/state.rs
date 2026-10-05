// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::visibility` sub-state (#1241 F2): moved fields, no logic.

use std::collections::HashSet;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::fixtures::VisibilityTestFixtureLikeCpp;
use wow_core::{ObjectGuid, Position};
use wow_world_core::session::mailbox::SharedClientVisibleTransportsLikeCpp;

/// Per-client visibility and publication fences: visible transports, last visibility position and
/// farsight, delivered-update guards.
pub struct VisibilityState {
    /// C++ `Player::m_visibleTransports`, maintained by `Map::SendInitTransports`.
    pub(crate) client_visible_transports_like_cpp: SharedClientVisibleTransportsLikeCpp,
    /// Detached represented inputs used only by visibility tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) visibility_test_fixture_like_cpp: VisibilityTestFixtureLikeCpp,
    /// Last canonical FarsightObject value observed by this Session's
    /// publication rail. This is a delivery fence, not gameplay authority:
    /// it lets the session emit the one explicit clear packet required when a
    /// map-owned viewpoint disappears between map ticks.
    pub(crate) last_observed_farsight_object_like_cpp: ObjectGuid,
    /// Session-local delivery guard for represented DynamicObject VALUES packets
    /// consumed from the last map-owned `Map::SendObjectUpdates` stable snapshot.
    /// Includes the map update generation so repeated `process_pending()` calls
    /// suppress the same snapshot without blocking later identical bytes.
    pub(crate) represented_dynamic_object_values_updates_delivered_like_cpp:
        HashSet<(u32, u32, u64, ObjectGuid, u64)>,
    /// Session-local delivery guard for Player/Creature/Pet VALUES packets
    /// consumed from the canonical map's `Map::SendObjectUpdates` snapshot.
    pub(crate) represented_player_unit_values_updates_delivered_like_cpp:
        HashSet<(u32, u32, u64, ObjectGuid, u64)>,
    /// Session-local delivery guard for represented GameObjectDespawn packets
    /// consumed from the last map-owned `GameObject::Update` summary.
    pub(crate) represented_gameobject_visual_despawns_delivered_like_cpp:
        HashSet<(u32, u32, u64, ObjectGuid)>,
    /// Session-local delivery guard for represented CapturePointRemoved packets
    /// consumed from the last map-owned `GameObject::Delete` summary.
    pub(crate) represented_capture_point_removed_delivered_like_cpp:
        HashSet<(u32, u32, u64, ObjectGuid)>,
    /// Position at which visibility was last fully recalculated.
    pub(crate) last_visibility_pos: Option<Position>,
}

impl VisibilityState {
    /// Construct the per-session visibility state with its original field order and defaults.
    pub fn new_like_cpp() -> Self {
        Self {
            client_visible_transports_like_cpp: Default::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            visibility_test_fixture_like_cpp: VisibilityTestFixtureLikeCpp::default(),
            last_observed_farsight_object_like_cpp: ObjectGuid::EMPTY,
            represented_dynamic_object_values_updates_delivered_like_cpp: HashSet::new(),
            represented_player_unit_values_updates_delivered_like_cpp: HashSet::new(),
            represented_gameobject_visual_despawns_delivered_like_cpp: HashSet::new(),
            represented_capture_point_removed_delivered_like_cpp: HashSet::new(),
            last_visibility_pos: None,
        }
    }

    pub fn client_visible_transports_like_cpp(&self) -> &SharedClientVisibleTransportsLikeCpp {
        &self.client_visible_transports_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_seer_guid_fixture_like_cpp(&self) -> Option<ObjectGuid> {
        self.visibility_test_fixture_like_cpp
            .represented_seer_guid_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_seer_guid_fixture_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        self.visibility_test_fixture_like_cpp
            .represented_seer_guid_like_cpp = guid;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_player_phase_shift_fixture_like_cpp(&self) -> wow_entities::PhaseShift {
        self.visibility_test_fixture_like_cpp
            .represented_player_phase_shift
            .clone()
    }

    pub fn clear_client_visible_transports_like_cpp(&self) {
        self.client_visible_transports_like_cpp.clear();
    }

    pub fn insert_client_visible_transport_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.client_visible_transports_like_cpp.insert(guid)
    }

    pub fn contains_client_visible_transport_like_cpp(&self, guid: &ObjectGuid) -> bool {
        self.client_visible_transports_like_cpp.contains(guid)
    }

    pub fn snapshot_client_visible_transports_like_cpp(&self) -> HashSet<ObjectGuid> {
        self.client_visible_transports_like_cpp.snapshot_like_cpp()
    }

    pub fn extend_client_visible_transports_like_cpp(
        &self,
        guids: impl IntoIterator<Item = ObjectGuid>,
    ) {
        self.client_visible_transports_like_cpp.extend(guids);
    }

    pub fn last_visibility_pos_like_cpp(&self) -> Option<Position> {
        self.last_visibility_pos
    }

    pub fn set_last_visibility_pos_like_cpp(&mut self, position: Position) {
        self.last_visibility_pos = Some(position);
    }

    pub fn clear_last_visibility_pos_like_cpp(&mut self) {
        self.last_visibility_pos = None;
    }

    pub fn observe_farsight_object_like_cpp(&mut self, guid: ObjectGuid) {
        self.last_observed_farsight_object_like_cpp = guid;
    }

    pub fn clear_observed_farsight_object_like_cpp(&mut self) {
        self.last_observed_farsight_object_like_cpp = ObjectGuid::EMPTY;
    }

    pub fn admit_dynamic_object_values_update_delivery_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
        update_generation: u64,
        guid: ObjectGuid,
        fingerprint: u64,
    ) -> bool {
        self.represented_dynamic_object_values_updates_delivered_like_cpp
            .insert((map_id, instance_id, update_generation, guid, fingerprint))
    }

    pub fn admit_gameobject_visual_despawn_delivery_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
        update_generation: u64,
        guid: ObjectGuid,
    ) -> bool {
        self.represented_gameobject_visual_despawns_delivered_like_cpp
            .insert((map_id, instance_id, update_generation, guid))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_gameobject_visual_despawn_delivery_count_like_cpp(&self) -> usize {
        self.represented_gameobject_visual_despawns_delivered_like_cpp
            .len()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_gameobject_visual_despawn_delivery_is_empty_like_cpp(&self) -> bool {
        self.represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    }

    pub fn admit_capture_point_removed_delivery_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
        update_generation: u64,
        guid: ObjectGuid,
    ) -> bool {
        self.represented_capture_point_removed_delivered_like_cpp
            .insert((map_id, instance_id, update_generation, guid))
    }
}
