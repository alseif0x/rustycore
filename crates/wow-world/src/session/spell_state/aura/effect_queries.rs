//! Canonical aura-effect queries and modifier projections.

use super::*;

impl WorldSession {
    pub(in crate::session) fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.resolved_has_represented_aura_effect_like_cpp(hub, effect)
    }

    #[cfg(test)]
    fn has_represented_aura_effect_like_cpp(&self, effect: RepresentedAuraEffectLikeCpp) -> bool {
        self.resolved_has_represented_aura_effect_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }

    pub(in crate::session) fn resolved_has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.resolved_has_represented_aura_effect_with_misc_value_like_cpp(hub, effect, misc_value)
    }

    pub(in crate::session) fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<i32> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
            hub, effect, misc_value,
        )
    }

    pub(in crate::session) fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.resolved_total_represented_aura_multiplier_like_cpp(hub, effect)
    }

    pub(in crate::session) fn aura_has_total_stat_percentage_effect_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.aura_has_total_stat_percentage_effect_like_cpp(hub, aura)
    }
}


#[cfg(test)]
#[path = "../../../../unit_tests/session/spell_state/aura/effect_queries/f3_shims.rs"]
mod f3_shims;
