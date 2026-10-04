//! Represented shapeshift form boosts.
//!
//! C++ `AuraEffect::HandleShapeshiftBoosts` (`SpellAuraEffects.cpp:1325-1464`)
//! owns the form's bonus spells and the stance-gated self-aura sweep. Form
//! ownership itself lives in `super::aura`.

use super::*;

impl WorldSession {
    /// C++ `AuraEffect::HandleShapeshiftBoosts` apply branch
    /// (`SpellAuraEffects.cpp:1394-1432`): cast the form's hardcoded boost
    /// spells on the player and every known passive (or
    /// `SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG`) spell whose
    /// `Stances` mask admits the new form.
    pub(crate) fn apply_represented_shapeshift_boosts_like_cpp(&mut self, form_id: u32) -> usize {
        self.player_aura_application_cx_like_cpp().apply_shapeshift_boosts_like_cpp(form_id)
    }

    /// C++ `AuraEffect::HandleShapeshiftBoosts` remove branch
    /// (`SpellAuraEffects.cpp:1434-1464`): drop the removed form's owned boost
    /// auras and then every self-cast aura whose spell declares `Stances` that
    /// the resulting form no longer admits.
    pub(crate) fn remove_represented_shapeshift_boosts_like_cpp(
        &mut self,
        removed_form: u32,
        new_form: u32,
    ) -> usize {
        self.player_aura_application_cx_like_cpp().remove_shapeshift_boosts_like_cpp(removed_form, new_form)
    }

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
