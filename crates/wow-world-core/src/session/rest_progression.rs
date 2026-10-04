// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

impl crate::session::state::SessionCore {
    pub(crate) fn player_rest_state_snapshot_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture: &RestMgrTestFixtureLikeCpp,
    ) -> Option<wow_entities::PlayerRestState> {
        let canonical = self
            .with_owned_player_for_rest_like_cpp(|player| player.rest_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerRestState::from_represented_parts_like_cpp(
                fixture.represented_rest_state_xp_like_cpp,
                fixture.represented_rest_bonus_xp_like_cpp,
                fixture.represented_rest_flag_mask_like_cpp,
                fixture.represented_rest_location_initialized_like_cpp,
                fixture.represented_defer_rest_flag_sync_like_cpp,
                fixture.represented_deferred_rest_flag_update_dirty_like_cpp,
                fixture.represented_inn_area_trigger_id_like_cpp,
                fixture.represented_rest_time_secs_like_cpp,
            ));
        }
        canonical
    }
}

// C++ `RestMgr::SetRestBonus`: `float(next_level_xp) * 1.5f / 2`.
#[cfg(any(test, feature = "test-fixtures"))]
const REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP: f32 = 1.5 / 2.0;

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

impl crate::session::HubRef<'_> {
    /// C++ `Player::IsMaxLevel` reads `ActivePlayerData::MaxLevel`, which
    /// `InitStatsForLevel` derives from both the account's active expansion
    /// and CONFIG_MAX_PLAYER_LEVEL. RestMgr deliberately uses the config-only
    /// check above instead, so keep these two concepts separate.
    pub fn player_active_max_level_like_cpp(&self) -> u32 {
        self.core
            .player_active_max_level_with_config_like_cpp(self.config)
    }

    pub fn player_is_max_level_like_cpp(&self) -> bool {
        u32::from(self.player_level_like_cpp()) >= self.player_active_max_level_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn can_gain_represented_xp_rest_bonus_like_cpp(&self) -> Option<bool> {
        if self.player_is_at_configured_max_level_like_cpp() {
            return Some(false);
        }

        let next_level_xp = self.resolved_player_next_level_xp_like_cpp()?;
        Some(next_level_xp != 0 && next_level_xp != u32::MAX)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_xp_rest_bonus_cap_like_cpp(&self) -> Option<f32> {
        Some(
            self.resolved_player_next_level_xp_like_cpp()? as f32
                * REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn calc_represented_xp_rest_extra_per_sec_like_cpp(&self, bubble: f32) -> Option<f32> {
        if !self.can_gain_represented_xp_rest_bonus_like_cpp()? {
            return Some(0.0);
        }
        Some(self.resolved_player_next_level_xp_like_cpp()? as f32 / 72_000.0 * bubble)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_xp_rest_bonus_like_cpp(&self) -> f32 {
        self.resolved_xp_rest_bonus_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_xp_rest_state_like_cpp(&self) -> u8 {
        self.resolved_xp_rest_state_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_xp_rest_threshold_like_cpp(&self) -> u32 {
        self.resolved_xp_rest_threshold_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_is_resting_like_cpp(&self) -> bool {
        self.resolved_is_resting_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    pub fn represented_xp_rest_info_changed_since_like_cpp(
        &self,
        old_rest_bonus: f32,
        old_rest_state: u8,
    ) -> bool {
        self.resolved_xp_rest_bonus_like_cpp()
            .zip(self.resolved_xp_rest_state_like_cpp())
            .is_some_and(|(bonus, state)| {
                bonus.to_bits() != old_rest_bonus.to_bits() || state != old_rest_state
            })
    }

    pub fn represented_action_button_db_context_like_cpp(&self) -> Option<(u8, i32)> {
        // Trait-config-specific action bars are not represented yet. Both load and save must use
        // the same C++ fallback context so an autosave cannot mutate rows it never loaded.
        Some((self.represented_active_talent_group_like_cpp()?, 0))
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_rest_state_snapshot_like_cpp(&self) -> Option<wow_entities::PlayerRestState> {
        self.core.player_rest_state_snapshot_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.rest_mgr_test_fixture_like_cpp,
        )
    }

    pub fn resolved_xp_rest_bonus_like_cpp(&self) -> Option<f32> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_bonus_like_cpp())
    }

    pub fn resolved_xp_rest_state_like_cpp(&self) -> Option<u8> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_state_like_cpp())
    }

    pub fn resolved_xp_rest_threshold_like_cpp(&self) -> Option<u32> {
        Some(
            self.resolved_xp_rest_bonus_like_cpp()?
                .clamp(0.0, u32::MAX as f32) as u32,
        )
    }

    pub fn resolved_is_resting_like_cpp(&self) -> Option<bool> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.is_resting_by_flag_like_cpp())
    }
}
