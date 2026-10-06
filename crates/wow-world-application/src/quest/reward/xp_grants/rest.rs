// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Rested-XP consumption and the existing handle-less rest update seam.

use wow_entities::RepresentedAuraEffectLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{CoreXPGainAccessLikeCpp, SessionWorldConfig};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_social::SessionSocialLimits;

use super::super::QuestRewardCx;
#[cfg(any(test, feature = "test-fixtures"))]
use super::QuestXpGainFixtureRefsLikeCpp;

impl QuestRewardCx<'_> {
    pub fn take_represented_xp_rest_bonus_for_gain_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &mut QuestXpGainFixtureRefsLikeCpp<
            '_,
        >,
    ) -> (u32, u8) {
        if victim.is_empty() {
            return (0, 0);
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer
            && self
                .xp_gain_access_like_cpp()
                .owner_handle_absent_like_cpp()
        {
            let current_rest_bonus = {
                let player = self.xp_gain_access_like_cpp();
                let Some(bonus) = player
                    .resolved_xp_rest_bonus_like_cpp(fixtures.core.rest_mgr_fixture_like_cpp())
                else {
                    return (0, 0);
                };
                bonus
            };
            let rested_bonus = (current_rest_bonus as u32).min(xp);
            let rested_consumption_modifier = {
                let player = self.xp_gain_access_like_cpp();
                let Some(pct) = player.resolved_total_represented_aura_modifier_like_cpp(
                    RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
                    &fixtures.core,
                ) else {
                    return (0, 0);
                };
                pct
            };
            let rested_loss = wow_entities::apply_pct_modifier_to_u32_like_cpp(
                rested_bonus,
                rested_consumption_modifier,
            );
            let nested_mask = self.set_represented_xp_rest_bonus_for_fixture_like_cpp(
                current_rest_bonus - rested_loss as f32,
                fixtures,
            );
            return (rested_bonus, nested_mask);
        }

        let player = self.xp_gain_access_like_cpp();
        let Some(rested_consumption_modifier) = player
            .resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
                #[cfg(any(test, feature = "test-fixtures"))]
                &fixtures.core,
            )
        else {
            return (0, 0);
        };
        let at_max = player.player_is_at_configured_max_level_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.player_level,
        );
        let recruit_a_friend = self.gets_recruit_a_friend_xp_bonus_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        );
        player.take_xp_rest_bonus_like_cpp(
            xp,
            rested_consumption_modifier,
            at_max,
            recruit_a_friend,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_xp_rest_bonus_for_fixture_like_cpp(
        &mut self,
        rest_bonus: f32,
        fixtures: &mut QuestXpGainFixtureRefsLikeCpp<'_>,
    ) -> u8 {
        let (fixture_position, fixture_next_level_xp, fixture_rest) =
            fixtures.core.rest_transition_fixture_refs_like_cpp();
        Self::set_represented_xp_rest_bonus_from_selected_access_like_cpp(
            self.xp_gain_access_like_cpp(),
            self.social,
            self.config,
            self.world_test_consumer,
            &*self.player_level,
            fixture_position,
            fixture_next_level_xp,
            fixture_rest,
            rest_bonus,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    #[allow(clippy::too_many_arguments)]
    pub fn set_represented_xp_rest_bonus_from_selected_access_like_cpp(
        player: CoreXPGainAccessLikeCpp<'_>,
        social: &SessionSocialLimits,
        config: &SessionWorldConfig,
        world_test_consumer: bool,
        fixture_level: &u8,
        fixture_position: &Option<wow_core::Position>,
        fixture_next_level_xp: &u32,
        fixture_rest: &mut wow_world_core::session::RestMgrTestFixtureLikeCpp,
        rest_bonus: f32,
    ) -> u8 {
        let Some(old_threshold) = player.resolved_xp_rest_threshold_like_cpp(fixture_rest) else {
            return 0;
        };
        let Some(old_state) = player.resolved_xp_rest_state_like_cpp(fixture_rest) else {
            return 0;
        };
        let mut rest_bonus = wow_entities::sanitize_rest_bonus_like_cpp(rest_bonus);
        let Some(can_gain) = player.can_gain_represented_xp_rest_bonus_from_selected_refs_like_cpp(
            fixture_level,
            fixture_next_level_xp,
        ) else {
            return 0;
        };
        let Some(rest_bonus_cap) = player
            .represented_xp_rest_bonus_cap_from_selected_next_level_xp_like_cpp(
                fixture_next_level_xp,
            )
        else {
            return 0;
        };
        if !can_gain {
            rest_bonus = 0.0;
        }
        rest_bonus = rest_bonus.clamp(0.0, rest_bonus_cap);
        let recruit_a_friend = Self::gets_recruit_a_friend_bonus_from_access_like_cpp(
            &player,
            social,
            config,
            true,
            world_test_consumer,
            fixture_level,
            fixture_position,
        );
        let rest_state = if recruit_a_friend {
            wow_constants::rest::REST_STATE_RAF_LINKED_LIKE_CPP
        } else if rest_bonus >= 1.0 {
            wow_constants::rest::REST_STATE_RESTED_LIKE_CPP
        } else {
            wow_constants::rest::REST_STATE_NORMAL_LIKE_CPP
        };
        let replaced = if world_test_consumer && player.owner_handle_absent_like_cpp() {
            let Some(mut state) = player.player_rest_state_snapshot_like_cpp(fixture_rest) else {
                return 0;
            };
            state.install_loaded_rest_like_cpp(rest_state, rest_bonus);
            player.replace_rest_state_for_fixture_like_cpp(state, fixture_rest)
        } else {
            player
                .install_rest_state_like_cpp(rest_state, rest_bonus)
                .is_some()
        };
        if !replaced {
            return 0;
        }
        let Some(new_threshold) = player.resolved_xp_rest_threshold_like_cpp(fixture_rest) else {
            return 0;
        };
        let Some(new_state) = player.resolved_xp_rest_state_like_cpp(fixture_rest) else {
            return 0;
        };
        if old_threshold != new_threshold || old_state != new_state {
            0x07
        } else {
            0
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    #[allow(clippy::too_many_arguments)]
    pub fn take_represented_xp_rest_bonus_for_fixture_from_selected_access_like_cpp(
        player: CoreXPGainAccessLikeCpp<'_>,
        social: &SessionSocialLimits,
        config: &SessionWorldConfig,
        world_test_consumer: bool,
        fixture_level: &u8,
        fixture_position: &Option<wow_core::Position>,
        fixture_next_level_xp: &u32,
        fixture_rest: &mut wow_world_core::session::RestMgrTestFixtureLikeCpp,
        aura_authority_complete: &bool,
        aura_spell_hit_tombstoned: &bool,
        visible_auras: &std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        threat_aura_snapshots: &std::collections::HashMap<
            u8,
            wow_entities::AuraThreatSnapshotLikeCpp,
        >,
        xp: u32,
        victim: wow_core::ObjectGuid,
    ) -> (u32, u8) {
        if victim.is_empty() {
            return (0, 0);
        }
        let Some(current_rest_bonus) = player.resolved_xp_rest_bonus_like_cpp(fixture_rest) else {
            return (0, 0);
        };
        let rested_bonus = (current_rest_bonus as u32).min(xp);
        let Some(rested_consumption_modifier) = player
            .resolved_total_represented_aura_modifier_from_selected_refs_like_cpp(
                RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
                aura_authority_complete,
                aura_spell_hit_tombstoned,
                visible_auras,
                threat_aura_snapshots,
            )
        else {
            return (0, 0);
        };
        let rested_loss = wow_entities::apply_pct_modifier_to_u32_like_cpp(
            rested_bonus,
            rested_consumption_modifier,
        );
        let nested_mask = Self::set_represented_xp_rest_bonus_from_selected_access_like_cpp(
            player,
            social,
            config,
            world_test_consumer,
            fixture_level,
            fixture_position,
            fixture_next_level_xp,
            fixture_rest,
            current_rest_bonus - rested_loss as f32,
        );
        (rested_bonus, nested_mask)
    }
}
