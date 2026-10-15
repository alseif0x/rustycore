//! Represented cast-state helpers outside the #589 request lifecycle.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_login_passive_spell_cast_gate_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_login_passive_spell_cast_gate_like_cpp(hub, spell_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_can_duel_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedCanDuelSpellCastLikeCpp] {
        self.social
            .represented_can_duel_spell_casts_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_respec_visual_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecVisualSpellCastLikeCpp] {
        &self
            .fixtures
            .progression
            .represented_talent_respec_visual_spell_casts_like_cpp
    }
}
#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/cast/f3_shims.rs"]
mod f3_shims;
