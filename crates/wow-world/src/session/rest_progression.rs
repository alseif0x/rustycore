// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Rest progression: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, AreaTriggerDb2Store, PLAYER_FLAGS_RESTING_LIKE_CPP};
use super::{PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP, REST_FLAG_IN_TAVERN_LIKE_CPP};
#[cfg(test)]
use super::{REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP, REST_STATE_NORMAL_LIKE_CPP};
use super::{RepresentedAuraEffectLikeCpp, WorldSession, max_level_for_expansion_like_cpp};

/// Handle-less test fixture for RestMgr state and test-only rate configuration.
/// Production rest authority remains on the canonical `Player`.
#[cfg(test)]
pub(in crate::session) struct RestMgrTestFixtureLikeCpp {
    /// C++ `RestMgr::_restBonus[REST_TYPE_XP]`.
    pub(in crate::session) represented_rest_bonus_xp_like_cpp: f32,
    /// C++ `UF::ActivePlayerData::RestInfo[REST_TYPE_XP].StateID`.
    pub(in crate::session) represented_rest_state_xp_like_cpp: u8,
    /// C++ `RestMgr::_restFlagMask`.
    pub(in crate::session) represented_rest_flag_mask_like_cpp: u32,
    /// Whether area and zone fixture updates initialized the rest flags.
    pub(in crate::session) represented_rest_location_initialized_like_cpp: bool,
    /// Coalesce area and zone fixture changes into one visible flag transition.
    pub(in crate::session) represented_defer_rest_flag_sync_like_cpp: bool,
    /// Deferred transition that marks `PLAYER_FLAGS_RESTING` dirty.
    pub(in crate::session) represented_deferred_rest_flag_update_dirty_like_cpp: bool,
    /// C++ `RestMgr::_innAreaTriggerId`.
    pub(in crate::session) represented_inn_area_trigger_id_like_cpp: u32,
    /// C++ `RestMgr::_restTime`, represented as Unix seconds.
    pub(in crate::session) represented_rest_time_secs_like_cpp: u64,
    /// Test policy value corresponding to `RATE_REST_OFFLINE_IN_WILDERNESS`.
    pub(in crate::session) rest_offline_wilderness_rate_like_cpp: f32,
    /// Test policy value corresponding to `RATE_REST_OFFLINE_IN_TAVERN_OR_CITY`.
    pub(in crate::session) rest_offline_tavern_or_city_rate_like_cpp: f32,
    /// Test policy value corresponding to `RATE_REST_INGAME`.
    pub(in crate::session) rest_ingame_rate_like_cpp: f32,
}

#[cfg(test)]
impl Default for RestMgrTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_rest_bonus_xp_like_cpp: 0.0,
            represented_rest_state_xp_like_cpp: REST_STATE_NORMAL_LIKE_CPP,
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

impl WorldSession {
    /// C++ `Player::IsMaxLevel` reads `ActivePlayerData::MaxLevel`, which
    /// `InitStatsForLevel` derives from both the account's active expansion
    /// and CONFIG_MAX_PLAYER_LEVEL. RestMgr deliberately uses the config-only
    /// check above instead, so keep these two concepts separate.
    pub(crate) fn player_active_max_level_like_cpp(&self) -> u32 {
        let expansion_max = u32::from(max_level_for_expansion_like_cpp(self.expansion));
        let configured_max = self.max_player_level_config_like_cpp;
        if expansion_max == 80 || expansion_max >= configured_max {
            configured_max
        } else {
            expansion_max
        }
    }

    pub(in crate::session) fn player_is_max_level_like_cpp(&self) -> bool {
        u32::from(self.player_level_like_cpp()) >= self.player_active_max_level_like_cpp()
    }

    #[cfg(test)]
    pub(in crate::session) fn can_gain_represented_xp_rest_bonus_like_cpp(&self) -> Option<bool> {
        if self.player_is_at_configured_max_level_like_cpp() {
            return Some(false);
        }

        let next_level_xp = self.resolved_player_next_level_xp_like_cpp()?;
        Some(next_level_xp != 0 && next_level_xp != u32::MAX)
    }

    #[cfg(test)]
    pub(in crate::session) fn represented_xp_rest_bonus_cap_like_cpp(&self) -> Option<f32> {
        Some(
            self.resolved_player_next_level_xp_like_cpp()? as f32
                * REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP,
        )
    }

    pub(crate) fn player_rest_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerRestState> {
        let canonical =
            self.with_owned_player_for_rest_like_cpp(|player| player.rest_state_like_cpp().clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerRestState::from_represented_parts_like_cpp(
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_rest_state_xp_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_rest_bonus_xp_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_rest_flag_mask_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_rest_location_initialized_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_defer_rest_flag_sync_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_deferred_rest_flag_update_dirty_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_inn_area_trigger_id_like_cpp,
                    self.rest_mgr_test_fixture_like_cpp
                        .represented_rest_time_secs_like_cpp,
                ),
            );
        }
        canonical
    }

    #[cfg(test)]
    pub(in crate::session) fn replace_player_rest_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerRestState,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_for_rest_like_cpp(|player| {
                player.set_xp_rest_info_like_cpp(
                    state.rest_bonus_like_cpp().clamp(0.0, u32::MAX as f32) as u32,
                    state.rest_state_like_cpp(),
                );
                if state.is_location_initialized_like_cpp() {
                    if state.is_resting_by_flag_like_cpp() {
                        player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                    } else {
                        player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                    }
                }
                player.replace_rest_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.rest_mgr_test_fixture_like_cpp
                .represented_rest_bonus_xp_like_cpp = state.rest_bonus_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_rest_state_xp_like_cpp = state.rest_state_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_rest_flag_mask_like_cpp = state.rest_flag_mask_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_rest_location_initialized_like_cpp =
                state.is_location_initialized_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_defer_rest_flag_sync_like_cpp = state.defers_flag_sync_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_deferred_rest_flag_update_dirty_like_cpp =
                state.deferred_flag_update_dirty_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_inn_area_trigger_id_like_cpp = state.inn_trigger_id_like_cpp();
            self.rest_mgr_test_fixture_like_cpp
                .represented_rest_time_secs_like_cpp = state.rest_time_secs_like_cpp();
            return true;
        }
        canonical
    }

    #[cfg(test)]
    pub(in crate::session) fn set_represented_xp_rest_bonus_like_cpp(
        &mut self,
        rest_bonus: f32,
    ) -> u8 {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_set_xp_rest_bonus_like_cpp(rest_bonus);
        }
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.set_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
        })
        .unwrap_or(0)
    }

    pub(crate) fn add_represented_xp_rest_bonus_like_cpp(&mut self, rest_bonus: f32) -> u8 {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let Some(current) = self.resolved_xp_rest_bonus_like_cpp() else {
                return 0;
            };
            return self.set_represented_xp_rest_bonus_like_cpp(current + rest_bonus);
        }
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.add_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
        })
        .unwrap_or(0)
    }

    #[cfg(test)]
    pub(in crate::session) fn calc_represented_xp_rest_extra_per_sec_like_cpp(
        &self,
        bubble: f32,
    ) -> Option<f32> {
        if !self.can_gain_represented_xp_rest_bonus_like_cpp()? {
            return Some(0.0);
        }
        Some(self.resolved_player_next_level_xp_like_cpp()? as f32 / 72_000.0 * bubble)
    }

    #[cfg(test)]
    pub(crate) fn apply_offline_xp_rest_bonus_like_cpp(
        &mut self,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.apply_offline_xp_rest_bonus_with_policy_like_cpp(
            &policy,
            logout_time_secs,
            now_secs,
            was_logout_resting,
        )
    }

    #[cfg(test)]
    pub(in crate::session) fn update_represented_online_xp_rest_bonus_like_cpp(
        &mut self,
        now_secs: u64,
    ) -> (f32, u8) {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.update_represented_online_xp_rest_bonus_with_policy_like_cpp(&policy, now_secs)
    }

    #[cfg(test)]
    pub(in crate::session) fn tick_represented_online_xp_rest_bonus_with_roll_like_cpp(
        &mut self,
        now_secs: u64,
        update_roll_passed: bool,
    ) {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
            &policy,
            now_secs,
            update_roll_passed,
        );
    }

    #[cfg(test)]
    pub(in crate::session) fn revalidate_represented_tavern_resting_like_cpp(&mut self) {
        let db2 = self
            .area_trigger_db2_store
            .clone()
            .unwrap_or_else(|| Arc::new(AreaTriggerDb2Store::from_entries([])));
        self.revalidate_represented_tavern_resting_with_catalog_like_cpp(db2.as_ref());
    }

    pub(crate) fn represented_player_has_flag_like_cpp(&self, flag: u32) -> bool {
        let canonical = self
            .player_guid()
            .and_then(|guid| self.canonical_player_has_player_flag_like_cpp(guid, flag));
        if let Some(value) = canonical {
            return value;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_like_cpp
                .is_some_and(|flags| (flags & flag) != 0);
        }
        false
    }

    pub(crate) fn represented_player_flags_value_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.canonical_player_snapshot_like_cpp(|player| player.data().player_flags);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self
                .player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_like_cpp;
        }
        canonical
    }

    pub(crate) fn void_storage_is_unlocked_like_cpp(&self) -> bool {
        self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP)
    }

    pub(in crate::session) fn take_represented_xp_rest_bonus_for_gain_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
    ) -> (u32, u8) {
        if victim.is_empty() {
            return (0, 0);
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_take_xp_rest_bonus_like_cpp(xp, victim);
        }
        let Some(pct) = self.resolved_total_represented_aura_modifier_like_cpp(
            RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
        ) else {
            return (0, 0);
        };
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.take_xp_rest_bonus_like_cpp(xp, pct, at_max, raf)
        })
        .unwrap_or((0, 0))
    }

    pub(crate) fn resolved_xp_rest_bonus_like_cpp(&self) -> Option<f32> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_bonus_like_cpp())
    }

    pub(crate) fn resolved_xp_rest_state_like_cpp(&self) -> Option<u8> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_state_like_cpp())
    }

    pub(crate) fn resolved_xp_rest_threshold_like_cpp(&self) -> Option<u32> {
        Some(
            self.resolved_xp_rest_bonus_like_cpp()?
                .clamp(0.0, u32::MAX as f32) as u32,
        )
    }

    #[cfg(test)]
    pub(crate) fn represented_xp_rest_bonus_like_cpp(&self) -> f32 {
        self.resolved_xp_rest_bonus_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_xp_rest_state_like_cpp(&self) -> u8 {
        self.resolved_xp_rest_state_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_xp_rest_threshold_like_cpp(&self) -> u32 {
        self.resolved_xp_rest_threshold_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    pub(crate) fn resolved_is_resting_like_cpp(&self) -> Option<bool> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.is_resting_by_flag_like_cpp())
    }

    #[cfg(test)]
    pub(crate) fn represented_is_resting_like_cpp(&self) -> bool {
        self.resolved_is_resting_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    pub(crate) fn set_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        self.set_player_rest_flag_like_cpp(rest_flag, trigger_id)
    }

    pub(in crate::session) fn update_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        active: bool,
    ) -> bool {
        if active {
            self.set_represented_rest_flag_like_cpp(rest_flag, 0)
        } else {
            self.remove_represented_rest_flag_like_cpp(rest_flag)
        }
    }

    pub(crate) fn set_represented_tavern_resting_like_cpp(
        &mut self,
        trigger_id: u32,
        entered: bool,
    ) -> bool {
        if entered {
            self.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, trigger_id)
        } else {
            self.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP)
        }
    }

    pub(crate) fn resolved_player_flags_for_create_like_cpp(&self) -> Option<(u32, u32)> {
        let player_flags = self.resolved_player_flags_for_rest_state_save_like_cpp()?;
        let canonical_flags_ex =
            self.with_owned_player_for_rest_like_cpp(|player| player.data().player_flags_ex);
        #[cfg(test)]
        let canonical_flags_ex =
            if canonical_flags_ex.is_none() && self.player_handle_like_cpp.is_none() {
                self.player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_ex_like_cpp
                    .or(Some(0))
            } else {
                canonical_flags_ex
            };
        let player_flags_ex = canonical_flags_ex?;
        Some((player_flags, player_flags_ex))
    }

    #[cfg(test)]
    pub(crate) fn represented_player_flags_for_create_like_cpp(&self) -> (u32, u32) {
        self.resolved_player_flags_for_create_like_cpp()
            .unwrap_or_else(|| {
                (
                    self.player_flags_test_fixture_like_cpp
                        .represented_loaded_player_flags_like_cpp
                        .unwrap_or(0),
                    self.player_flags_test_fixture_like_cpp
                        .represented_loaded_player_flags_ex_like_cpp
                        .unwrap_or(0),
                )
            })
    }

    pub(in crate::session) fn represented_xp_rest_info_changed_since_like_cpp(
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

    pub(crate) fn current_played_time_values_like_cpp(&self) -> (u32, u32) {
        let session_secs: u32 = self
            .login_time
            .map(|time| time.elapsed().as_secs() as u32)
            .unwrap_or(0);
        (
            self.total_played_time.saturating_add(session_secs),
            self.level_played_time.saturating_add(session_secs),
        )
    }

    pub(crate) fn represented_action_button_db_context_like_cpp(&self) -> Option<(u8, i32)> {
        // Trait-config-specific action bars are not represented yet. Both load and save must use
        // the same C++ fallback context so an autosave cannot mutate rows it never loaded.
        Some((self.represented_active_talent_group_like_cpp()?, 0))
    }

    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        self.take_player_deferred_rest_flag_update_dirty_like_cpp()
    }
}
