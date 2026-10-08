//! Map key resolution used by the represented Session.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use std::sync::Arc;

use crate::map_manager::LegacyMapPresenceCaptureLikeCpp;
use crate::session::MMapRuntimeConfigLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::AdventureMapPoiStore;
use wow_data::{DungeonEncounterStore, MapStore};

impl crate::session::state::SessionCatalogs {
    /// Set the C++ AdventureMapPOI.db2 store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_adventure_map_poi_store(&mut self, store: Arc<AdventureMapPoiStore>) {
        self.adventure_map_poi_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn adventure_map_poi_store(&self) -> Option<&Arc<AdventureMapPoiStore>> {
        self.adventure_map_poi_store.as_ref()
    }

    pub fn dungeon_encounter_store(&self) -> Option<&Arc<DungeonEncounterStore>> {
        self.dungeon_encounter_store.as_ref()
    }

    pub fn map_store(&self) -> Option<&Arc<MapStore>> {
        self.maps.store.as_ref()
    }
}

impl crate::session::state::SessionWorldConfig {
    pub fn player_map_visibility_range_like_cpp(&self, map_id: u16) -> f32 {
        self.legacy_creature_aggro_config_like_cpp
            .map_visibility_range_like_cpp(map_id)
    }

    pub fn mmap_runtime_config_like_cpp(&self) -> &MMapRuntimeConfigLikeCpp {
        &self.mmap_runtime_config_like_cpp
    }
}

/// #1263 C2 capture record for one login-attach, detach or legacy-map-key
/// fallback decision.
///
/// Every field is a copy taken **while** the relevant guard is held; the event
/// itself is emitted by [`emit_legacy_runtime_location_capture_like_cpp`] only
/// **after** that guard is released, so no I/O ever runs inside a guard.
#[derive(Debug, Clone, Copy)]
pub struct LegacyRuntimeLocationCaptureLikeCpp {
    /// Stable capture identifier reported in the record.
    pub capture_id: &'static str,
    /// Production root that made the observation.
    pub root: &'static str,
    /// `login_attach` | `detach` | `map_key_fallback`.
    pub phase: &'static str,
    /// Character GUID counter, when a character identity is resolved.
    pub player_guid_counter: Option<u32>,
    /// Session state at the observation point.
    pub session_state: &'static str,
    /// Map id the caller asked about (login map, teleport destination or the
    /// represented `current_map_id` behind the fallback).
    pub requested_map_id: u16,
    /// Instance id the caller asked about, when it asked for a concrete one.
    pub requested_instance_id: Option<u32>,
    /// Whether `current_canonical_player_map_key_like_cpp` resolved a key.
    pub canonical_key_present: bool,
    /// The key actually returned to the caller by the production function.
    pub resolved_map_id: u16,
    pub resolved_instance_id: u32,
    /// Legacy runtime map presence for the resolved key.
    pub legacy_presence: LegacyMapPresenceCaptureLikeCpp,
    /// Legacy runtime map presence for the requested key, when different.
    pub requested_legacy_presence: LegacyMapPresenceCaptureLikeCpp,
    /// Free-form production detail tag for this observation.
    pub detail: &'static str,
}

pub const fn session_state_name_like_cpp(state: crate::session::SessionState) -> &'static str {
    match state {
        crate::session::SessionState::Authed => "Authed",
        crate::session::SessionState::LoggedIn => "LoggedIn",
        crate::session::SessionState::Transfer => "Transfer",
        crate::session::SessionState::Disconnecting => "Disconnecting",
    }
}

impl crate::session::state::SessionCore {
    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.current_map_id
    }

    /// The legacy map facade must follow the same map instance that owns the
    /// canonical Player. Instance `0` remains only the bootstrap fallback for
    /// tests/runtime phases where no canonical Player has been materialized.
    pub fn current_legacy_runtime_map_key_like_cpp(&self) -> (u16, u32) {
        let fallback_map_id = self.player_map_id_like_cpp();
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            // #1263 C2: the absent-canonical-key fallback is the decision this
            // capture exists for. The guard taken inside
            // `current_canonical_player_map_key_like_cpp` is already released
            // here, so nothing is held across the emission below.
            self.emit_legacy_runtime_location_capture_like_cpp(
                LegacyRuntimeLocationCaptureLikeCpp {
                    capture_id: "C2",
                    root: "SessionCore::current_legacy_runtime_map_key_like_cpp",
                    phase: "map_key_fallback",
                    player_guid_counter: self.player_guid.map(|guid| guid.counter() as u32),
                    session_state: session_state_name_like_cpp(self.state),
                    requested_map_id: fallback_map_id,
                    requested_instance_id: None,
                    canonical_key_present: false,
                    resolved_map_id: fallback_map_id,
                    resolved_instance_id: 0,
                    legacy_presence: self.legacy_map_presence_capture_like_cpp(fallback_map_id, 0),
                    requested_legacy_presence: LegacyMapPresenceCaptureLikeCpp::unobserved_like_cpp(
                    ),
                    detail: "canonical_player_map_key_absent",
                },
            );
            return (fallback_map_id, 0);
        };
        let Ok(map_id) = u16::try_from(map_key.map_id) else {
            return (fallback_map_id, 0);
        };
        (map_id, map_key.instance_id)
    }

    /// #1263 C2 capture: legacy runtime map presence for one exact key.
    ///
    /// `try_read` never blocks, so this can be called from any caller without
    /// participating in a lock order and without risking a deadlock if a caller
    /// already holds the legacy write guard. The global map loop holds that
    /// write guard for a large fraction of every tick, so a short bounded
    /// `yield_now` retry is used before giving up; a busy or absent manager is
    /// then reported as *not observed*, never as an absent map.
    pub fn legacy_map_presence_capture_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> LegacyMapPresenceCaptureLikeCpp {
        let Some(manager) = self.map_manager.as_ref() else {
            return LegacyMapPresenceCaptureLikeCpp {
                reason: "legacy_map_manager_absent",
                ..LegacyMapPresenceCaptureLikeCpp::unobserved_like_cpp()
            };
        };
        for _ in 0..64 {
            match manager.try_read() {
                Ok(manager) => {
                    return manager.legacy_map_presence_capture_like_cpp(map_id, instance_id);
                }
                Err(_) => std::thread::yield_now(),
            }
        }
        LegacyMapPresenceCaptureLikeCpp {
            reason: "legacy_map_manager_guard_busy",
            ..LegacyMapPresenceCaptureLikeCpp::unobserved_like_cpp()
        }
    }

    /// #1263 C2 capture: emit one already-copied location observation.
    ///
    /// This is the only place the C2 event is written, and it is always called
    /// after the guard that produced `capture` has been released.
    pub fn emit_legacy_runtime_location_capture_like_cpp(
        &self,
        capture: LegacyRuntimeLocationCaptureLikeCpp,
    ) {
        tracing::info!(
            target: "rustycore::capture::c2",
            capture_id = capture.capture_id,
            root = capture.root,
            phase = capture.phase,
            session_state = capture.session_state,
            player_guid_counter = capture.player_guid_counter.unwrap_or(0),
            player_guid_resolved = capture.player_guid_counter.is_some(),
            requested_map_id = capture.requested_map_id,
            requested_instance_id = capture.requested_instance_id.unwrap_or(u32::MAX),
            requested_instance_id_resolved = capture.requested_instance_id.is_some(),
            canonical_key_present = capture.canonical_key_present,
            resolved_map_id = capture.resolved_map_id,
            resolved_instance_id = capture.resolved_instance_id,
            legacy_presence_observed = capture.legacy_presence.observed,
            legacy_presence = capture.legacy_presence.present,
            legacy_presence_reason = capture.legacy_presence.reason,
            legacy_instance_id_is_zero = capture.legacy_presence.instance_id_is_zero,
            legacy_map_instance_count = capture.legacy_presence.map_instance_count,
            requested_legacy_presence_observed = capture.requested_legacy_presence.observed,
            requested_legacy_presence = capture.requested_legacy_presence.present,
            requested_legacy_presence_reason = capture.requested_legacy_presence.reason,
            account_id = self.account_id,
            detail = capture.detail,
            "RUSTYCORE_CAPTURE_C2 legacy runtime location observation"
        );
    }
}
