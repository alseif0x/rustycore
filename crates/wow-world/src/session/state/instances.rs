// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::instances` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Instance binds and reset times, the instance fixture, exploration and area-zone criteria and
/// adventure-map quest starts.
pub(crate) struct InstanceState {
    /// Detached instance inputs used only by Session tests.
    #[cfg(test)]
    pub(crate) instance_test_fixture_like_cpp: InstanceTestFixtureLikeCpp,
    /// Represented accepted Adventure Map quest starts until AddQuestAndCheckCompletion is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_adventure_map_start_quest_requests_like_cpp:
        Vec<RepresentedAdventureMapStartQuestLikeCpp>,
    /// C++ `Player::_instanceResetTimes`: instance id -> release time.
    #[cfg(test)]
    pub(crate) represented_instance_reset_times_like_cpp: BTreeMap<u32, u64>,

    /// C++ `ActivePlayerData::ExploredZones`, represented before the canonical Player owns persistence.
    #[cfg(test)]
    pub(in crate::session) represented_explored_zones_like_cpp:
        [u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    /// Represented `CriteriaType::RevealWorldMapOverlay` events from area discovery.
    #[cfg(test)]
    pub(in crate::session) represented_reveal_world_map_overlay_criteria_like_cpp: Vec<u32>,
    /// Represented `Player::UpdateArea` / `Player::UpdateZone` criteria side effects.
    #[cfg(test)]
    pub(in crate::session) represented_area_zone_criteria_like_cpp:
        Vec<RepresentedAreaZoneCriteriaLikeCpp>,
    /// C++ `Player::SetPendingBind` represented until `InstanceMap` owns real bind confirmation.
    pub(crate) pending_bind: Option<RepresentedPendingBind>,
    /// Confirmed pending bind ids observed by the represented `CMSG_INSTANCE_LOCK_RESPONSE`
    /// fixture. C++ keeps the real confirmation on Player/InstanceMap; this is test evidence,
    /// not production gameplay state.
    #[cfg(test)]
    pub(crate) represented_confirmed_pending_binds: Vec<u32>,
}
