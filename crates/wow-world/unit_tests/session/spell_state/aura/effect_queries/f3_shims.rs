// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.has_represented_aura_effect_with_misc_value_like_cpp(hub, effect, misc_value)
    }
    #[cfg(test)]
    pub(in crate::session) fn total_represented_aura_modifier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.total_represented_aura_modifier_like_cpp(hub, effect)
    }
    #[cfg(test)]
    pub(in crate::session) fn total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> f32 {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.total_represented_aura_multiplier_like_cpp(hub, effect)
    }
}
