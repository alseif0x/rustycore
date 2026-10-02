// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

/// Handle-less test fixture for RestMgr state and test-only rate configuration.
/// Production rest authority remains on the canonical `Player`.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct RestMgrTestFixtureLikeCpp {
    /// C++ `RestMgr::_restBonus[REST_TYPE_XP]`.
    pub represented_rest_bonus_xp_like_cpp: f32,
    /// C++ `UF::ActivePlayerData::RestInfo[REST_TYPE_XP].StateID`.
    pub represented_rest_state_xp_like_cpp: u8,
    /// C++ `RestMgr::_restFlagMask`.
    pub represented_rest_flag_mask_like_cpp: u32,
    /// Whether area and zone fixture updates initialized the rest flags.
    pub represented_rest_location_initialized_like_cpp: bool,
    /// Coalesce area and zone fixture changes into one visible flag transition.
    pub represented_defer_rest_flag_sync_like_cpp: bool,
    /// Deferred transition that marks `PLAYER_FLAGS_RESTING` dirty.
    pub represented_deferred_rest_flag_update_dirty_like_cpp: bool,
    /// C++ `RestMgr::_innAreaTriggerId`.
    pub represented_inn_area_trigger_id_like_cpp: u32,
    /// C++ `RestMgr::_restTime`, represented as Unix seconds.
    pub represented_rest_time_secs_like_cpp: u64,
    /// Test policy value corresponding to `RATE_REST_OFFLINE_IN_WILDERNESS`.
    pub rest_offline_wilderness_rate_like_cpp: f32,
    /// Test policy value corresponding to `RATE_REST_OFFLINE_IN_TAVERN_OR_CITY`.
    pub rest_offline_tavern_or_city_rate_like_cpp: f32,
    /// Test policy value corresponding to `RATE_REST_INGAME`.
    pub rest_ingame_rate_like_cpp: f32,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for RestMgrTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_rest_bonus_xp_like_cpp: 0.0,
            represented_rest_state_xp_like_cpp: wow_constants::rest::REST_STATE_NORMAL_LIKE_CPP,
            represented_rest_flag_mask_like_cpp: 0,
            represented_rest_location_initialized_like_cpp: false,
            represented_defer_rest_flag_sync_like_cpp: false,
            represented_deferred_rest_flag_update_dirty_like_cpp: false,
            represented_inn_area_trigger_id_like_cpp: 0,
            represented_rest_time_secs_like_cpp: 0,
            rest_offline_wilderness_rate_like_cpp: 1.0,
            rest_offline_tavern_or_city_rate_like_cpp: 1.0,
            rest_ingame_rate_like_cpp: 1.0,
        }
    }
}
