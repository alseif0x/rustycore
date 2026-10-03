#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::BTreeMap;

use crate::RepresentedPendingBind;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::InstanceTestFixtureLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedAdventureMapStartQuestLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedAreaZoneCriteriaLikeCpp;

/// `WorldSession::instances` sub-state (#1241 F2): moved fields, no logic.
///
/// Instance binds and reset times, the instance fixture, exploration and area-zone criteria and
/// adventure-map quest starts.
pub struct InstanceState {
    /// Detached instance inputs used only by Session tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) instance_test_fixture_like_cpp: InstanceTestFixtureLikeCpp,
    /// Represented accepted Adventure Map quest starts until AddQuestAndCheckCompletion is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_adventure_map_start_quest_requests_like_cpp:
        Vec<RepresentedAdventureMapStartQuestLikeCpp>,
    /// C++ `Player::_instanceResetTimes`: instance id -> release time.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_instance_reset_times_like_cpp: BTreeMap<u32, u64>,
    /// C++ `ActivePlayerData::ExploredZones`, represented before the canonical Player owns persistence.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_explored_zones_like_cpp:
        [u64; wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    /// Represented `CriteriaType::RevealWorldMapOverlay` events from area discovery.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_reveal_world_map_overlay_criteria_like_cpp: Vec<u32>,
    /// Represented `Player::UpdateArea` / `Player::UpdateZone` criteria side effects.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_area_zone_criteria_like_cpp: Vec<RepresentedAreaZoneCriteriaLikeCpp>,
    /// C++ `Player::SetPendingBind` represented until `InstanceMap` owns real bind confirmation.
    pub(crate) pending_bind: Option<RepresentedPendingBind>,
    /// Confirmed pending bind ids observed by the represented `CMSG_INSTANCE_LOCK_RESPONSE`
    /// fixture. C++ keeps the real confirmation on Player/InstanceMap; this is test evidence,
    /// not production gameplay state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_confirmed_pending_binds: Vec<u32>,
}

impl InstanceState {
    pub fn new_like_cpp() -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            instance_test_fixture_like_cpp: InstanceTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_adventure_map_start_quest_requests_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_instance_reset_times_like_cpp: BTreeMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_explored_zones_like_cpp:
                [0; wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_reveal_world_map_overlay_criteria_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_area_zone_criteria_like_cpp: Vec::new(),
            pending_bind: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_confirmed_pending_binds: Vec::new(),
        }
    }
}
