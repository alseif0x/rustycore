// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Rested-XP projection and the handle-less rest transition inputs.

use super::CoreXPGainAccessLikeCpp;

impl CoreXPGainAccessLikeCpp<'_> {
    pub fn player_rest_state_snapshot_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &crate::session::RestMgrTestFixtureLikeCpp,
    ) -> Option<wow_entities::PlayerRestState> {
        self.core.player_rest_state_snapshot_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_rest,
        )
    }

    pub fn resolved_xp_rest_bonus_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &crate::session::RestMgrTestFixtureLikeCpp,
    ) -> Option<f32> {
        self.player_rest_state_snapshot_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_rest,
        )
        .map(|state| state.rest_bonus_like_cpp())
    }

    pub fn resolved_xp_rest_state_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &crate::session::RestMgrTestFixtureLikeCpp,
    ) -> Option<u8> {
        self.player_rest_state_snapshot_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_rest,
        )
        .map(|state| state.rest_state_like_cpp())
    }

    pub fn resolved_xp_rest_threshold_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &crate::session::RestMgrTestFixtureLikeCpp,
    ) -> Option<u32> {
        Some(
            self.resolved_xp_rest_bonus_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_rest,
            )?
            .clamp(0.0, u32::MAX as f32) as u32,
        )
    }

    pub fn represented_xp_rest_info_changed_since_like_cpp(
        &self,
        old_rest_bonus: f32,
        old_rest_state: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &crate::session::RestMgrTestFixtureLikeCpp,
    ) -> bool {
        self.resolved_xp_rest_bonus_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_rest,
        )
        .zip(self.resolved_xp_rest_state_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_rest,
        ))
        .is_some_and(|(bonus, state)| {
            bonus.to_bits() != old_rest_bonus.to_bits() || state != old_rest_state
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn can_gain_represented_xp_rest_bonus_like_cpp(
        &self,
        fixture_level: &u8,
        fixtures: &super::CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> Option<bool> {
        if self.player_is_at_configured_max_level_like_cpp(fixture_level) {
            return Some(false);
        }
        let next_level_xp = self.resolved_player_next_level_xp_like_cpp(fixtures)?;
        can_gain_rest_bonus_from_next_level_xp_like_cpp(Some(next_level_xp))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn can_gain_represented_xp_rest_bonus_from_selected_refs_like_cpp(
        &self,
        fixture_level: &u8,
        fixture_next_level_xp: &u32,
    ) -> Option<bool> {
        if self.player_is_at_configured_max_level_like_cpp(fixture_level) {
            return Some(false);
        }
        let next_level_xp = self
            .core
            .resolved_player_next_level_xp_with_fixture_like_cpp(fixture_next_level_xp)?;
        can_gain_rest_bonus_from_next_level_xp_like_cpp(Some(next_level_xp))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_xp_rest_bonus_cap_like_cpp(
        &self,
        fixtures: &super::CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> Option<f32> {
        rest_bonus_cap_from_next_level_xp_like_cpp(
            self.resolved_player_next_level_xp_like_cpp(fixtures),
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn resolved_total_represented_aura_modifier_from_selected_refs_like_cpp(
        &self,
        effect: wow_entities::RepresentedAuraEffectLikeCpp,
        aura_authority_complete: &bool,
        aura_spell_hit_tombstoned: &bool,
        visible_auras: &std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        threat_aura_snapshots:
            &std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    ) -> Option<i32> {
        super::represented_total_aura_modifier_from_snapshot_like_cpp(
            self.core.player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
                aura_authority_complete,
                aura_spell_hit_tombstoned,
                visible_auras,
                threat_aura_snapshots,
            ),
            effect,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_xp_rest_bonus_cap_from_selected_next_level_xp_like_cpp(
        &self,
        fixture_next_level_xp: &u32,
    ) -> Option<f32> {
        let next_level_xp = self
            .core
            .resolved_player_next_level_xp_with_fixture_like_cpp(fixture_next_level_xp)?;
        rest_bonus_cap_from_next_level_xp_like_cpp(Some(next_level_xp))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_rest_state_for_fixture_like_cpp(
        &self,
        state: wow_entities::PlayerRestState,
        fixture_rest: &mut crate::session::RestMgrTestFixtureLikeCpp,
    ) -> bool {
        if self.core.player_handle_like_cpp.is_some() {
            return false;
        }
        fixture_rest.represented_rest_bonus_xp_like_cpp = state.rest_bonus_like_cpp();
        fixture_rest.represented_rest_state_xp_like_cpp = state.rest_state_like_cpp();
        fixture_rest.represented_rest_flag_mask_like_cpp = state.rest_flag_mask_like_cpp();
        fixture_rest.represented_rest_location_initialized_like_cpp =
            state.is_location_initialized_like_cpp();
        fixture_rest.represented_defer_rest_flag_sync_like_cpp =
            state.defers_flag_sync_like_cpp();
        fixture_rest.represented_deferred_rest_flag_update_dirty_like_cpp =
            state.deferred_flag_update_dirty_like_cpp();
        fixture_rest.represented_inn_area_trigger_id_like_cpp = state.inn_trigger_id_like_cpp();
        fixture_rest.represented_rest_time_secs_like_cpp = state.rest_time_secs_like_cpp();
        true
    }

    pub fn install_rest_state_like_cpp(&self, rest_state: u8, rest_bonus: f32) -> Option<()> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.mutate_rest_state_like_cpp(|state| {
                state.install_loaded_rest_like_cpp(rest_state, rest_bonus);
            });
        })
    }
}

fn can_gain_rest_bonus_from_next_level_xp_like_cpp(
    next_level_xp: Option<u32>,
) -> Option<bool> {
    let next_level_xp = next_level_xp?;
    Some(next_level_xp != 0 && next_level_xp != u32::MAX)
}

fn rest_bonus_cap_from_next_level_xp_like_cpp(next_level_xp: Option<u32>) -> Option<f32> {
    Some(next_level_xp? as f32 * (1.5 / 2.0))
}
