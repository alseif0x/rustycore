// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::identity` sub-state (#1241 F2): moved fields, no logic.

/// Player identity fixtures: race, class, level, gender, name, create mode, faction template, scale
/// and zone/area state.
pub(in crate::session) struct PlayerIdentityState {
    /// Detached fixture identity; production identity belongs to Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_race: u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_class: u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_level: u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_gender: u8,
    /// C++ `Player::m_createMode`, loaded from `characters.createMode`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_create_mode_like_cpp: u8,

    /// Cached character name for chat messages.
    /// Detached fixture identity; production name belongs to the canonical Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_name: Option<String>,
    /// Represented `UnitData::ScaleDuration` for movement collision-height packets.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_scale_duration_like_cpp: i32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_faction_template_like_cpp: Option<u32>,
    /// Current represented zone/area ids until Map/Terrain runtime can calculate them.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_zone_id_like_cpp: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_area_id_like_cpp: u32,
    /// True only when the current zone/area came from an extracted C++ terrain
    /// tile, rather than the DB-seeded or map-wide fallback.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_zone_area_authority_complete_like_cpp: bool,
    /// Represented C++ `WorldObject::IsOutdoors()` result until VMAP owns it.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) represented_is_outdoors_like_cpp: Option<bool>,
}
