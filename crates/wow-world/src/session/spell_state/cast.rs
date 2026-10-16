//! Represented cast-state helpers outside the #589 request lifecycle.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The login passive-spell cast
//! gate moved to `wow_world_spell` (#1363 slice 12b3).

use super::*;

impl WorldSession {
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
