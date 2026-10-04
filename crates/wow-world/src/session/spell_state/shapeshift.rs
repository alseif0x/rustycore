//! Represented shapeshift form boosts.
//!
//! C++ `AuraEffect::HandleShapeshiftBoosts` (`SpellAuraEffects.cpp:1325-1464`)
//! owns the form's bonus spells and the stance-gated self-aura sweep. Form
//! ownership itself lives in `super::aura`.

use super::*;

impl WorldSession {


    pub(crate) fn sync_represented_display_power_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.sync_represented_display_power_like_cpp(&mut hub)
    }

    pub(crate) fn represented_spell_has_power_display_effect_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_spell_has_power_display_effect_like_cpp(hub, spell_id)
    }

}
