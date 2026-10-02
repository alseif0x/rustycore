// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Rest progression: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, AreaTriggerDb2Store, PLAYER_FLAGS_RESTING_LIKE_CPP};
use super::{PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP, REST_FLAG_IN_TAVERN_LIKE_CPP};
use super::{RepresentedAuraEffectLikeCpp, WorldSession};

#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use wow_world_core::session::RestMgrTestFixtureLikeCpp;

impl WorldSession {
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
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_bonus_xp_like_cpp = state.rest_bonus_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_state_xp_like_cpp = state.rest_state_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_flag_mask_like_cpp = state.rest_flag_mask_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_location_initialized_like_cpp =
                state.is_location_initialized_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_defer_rest_flag_sync_like_cpp = state.defers_flag_sync_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_deferred_rest_flag_update_dirty_like_cpp =
                state.deferred_flag_update_dirty_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_inn_area_trigger_id_like_cpp = state.inn_trigger_id_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
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
        if self.core.player_handle_like_cpp.is_none() {
            return self.fixture_set_xp_rest_bonus_like_cpp(rest_bonus);
        }
        let at_max = crate::session::hub_ref(self).player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
            })
            .unwrap_or(0)
    }

    pub(crate) fn add_represented_xp_rest_bonus_like_cpp(&mut self, rest_bonus: f32) -> u8 {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            let Some(current) = crate::session::hub_ref(self).resolved_xp_rest_bonus_like_cpp()
            else {
                return 0;
            };
            return self.set_represented_xp_rest_bonus_like_cpp(current + rest_bonus);
        }
        let at_max = crate::session::hub_ref(self).player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.add_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
            })
            .unwrap_or(0)
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
            .catalogs
            .area_trigger_db2_store
            .clone()
            .unwrap_or_else(|| Arc::new(AreaTriggerDb2Store::from_entries([])));
        self.revalidate_represented_tavern_resting_with_catalog_like_cpp(db2.as_ref());
    }

    pub(crate) fn represented_player_has_flag_like_cpp(&self, flag: u32) -> bool {
        let canonical = self.player_guid().and_then(|guid| {
            self.core
                .canonical_player_has_player_flag_like_cpp(guid, flag)
        });
        if let Some(value) = canonical {
            return value;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .lifecycle
                .player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_like_cpp
                .is_some_and(|flags| (flags & flag) != 0);
        }
        false
    }

    pub(crate) fn represented_player_flags_value_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .lifecycle
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
        if self.core.player_handle_like_cpp.is_none() {
            return self.fixture_take_xp_rest_bonus_like_cpp(xp, victim);
        }
        let Some(pct) = crate::session::hub_ref(self)
            .resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
            )
        else {
            return (0, 0);
        };
        let at_max = crate::session::hub_ref(self).player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.take_xp_rest_bonus_like_cpp(xp, pct, at_max, raf)
            })
            .unwrap_or((0, 0))
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
        let canonical_flags_ex = self
            .core
            .with_owned_player_for_rest_like_cpp(|player| player.data().player_flags_ex);
        #[cfg(test)]
        let canonical_flags_ex =
            if canonical_flags_ex.is_none() && self.core.player_handle_like_cpp.is_none() {
                self.lifecycle
                    .player_flags_test_fixture_like_cpp
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
                    self.lifecycle
                        .player_flags_test_fixture_like_cpp
                        .represented_loaded_player_flags_like_cpp
                        .unwrap_or(0),
                    self.lifecycle
                        .player_flags_test_fixture_like_cpp
                        .represented_loaded_player_flags_ex_like_cpp
                        .unwrap_or(0),
                )
            })
    }

    pub(crate) fn current_played_time_values_like_cpp(&self) -> (u32, u32) {
        let session_secs: u32 = self
            .lifecycle
            .login_time
            .map(|time| time.elapsed().as_secs() as u32)
            .unwrap_or(0);
        (
            self.lifecycle
                .total_played_time
                .saturating_add(session_secs),
            self.lifecycle
                .level_played_time
                .saturating_add(session_secs),
        )
    }

    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        self.take_player_deferred_rest_flag_update_dirty_like_cpp()
    }
}


#[cfg(test)]
#[path = "../../unit_tests/session/rest_progression/f3_shims.rs"]
mod f3_shims;
