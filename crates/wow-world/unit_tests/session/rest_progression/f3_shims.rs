// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn calc_represented_xp_rest_extra_per_sec_like_cpp(
        &self,
        bubble: f32,
    ) -> Option<f32> {
        crate::session::hub_ref(self).calc_represented_xp_rest_extra_per_sec_like_cpp(bubble)
    }
    #[cfg(test)]
    pub(crate) fn represented_xp_rest_bonus_like_cpp(&self) -> f32 {
        crate::session::hub_ref(self).represented_xp_rest_bonus_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_xp_rest_state_like_cpp(&self) -> u8 {
        crate::session::hub_ref(self).represented_xp_rest_state_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_xp_rest_threshold_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).represented_xp_rest_threshold_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_is_resting_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_is_resting_like_cpp()
    }
    pub(crate) fn represented_action_button_db_context_like_cpp(&self) -> Option<(u8, i32)> {
        crate::session::hub_ref(self).represented_action_button_db_context_like_cpp()
    }
    pub(in crate::session) fn represented_xp_rest_info_changed_since_like_cpp(
        &self,
        old_rest_bonus: f32,
        old_rest_state: u8,
    ) -> bool {
        crate::session::hub_ref(self)
            .represented_xp_rest_info_changed_since_like_cpp(old_rest_bonus, old_rest_state)
    }
    pub(crate) fn player_rest_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerRestState> {
        crate::session::hub_ref(self).player_rest_state_snapshot_like_cpp()
    }
}

impl WorldSession {
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
    pub(in crate::session) fn take_represented_xp_rest_bonus_for_gain_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
    ) -> (u32, u8) {
        if victim.is_empty() {
            return (0, 0);
        }
        #[cfg(test)]
        if self
            .core
            .with_owned_player_for_rest_like_cpp(|_| ())
            .is_none()
        {
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
        self.with_owned_player_mut_for_rest_like_cpp(|player| {
            player.take_xp_rest_bonus_like_cpp(xp, pct, at_max, raf)
        })
        .unwrap_or((0, 0))
    }
}
