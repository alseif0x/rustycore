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
        let boost_ids = {
            let (s, h) = crate::session::split_spell_state_ref(self);
            s.represented_shapeshift_boost_spell_ids_like_cpp(h, form_id)
        };
        let mut applied = 0usize;
        for spell_id in &boost_ids {
            if self.represented_apply_owned_aura_spell_like_cpp(*spell_id) {
                applied += 1;
            }
        }
        let Some(spell_store) = self.spell_store().cloned() else {
            return applied;
        };
        let Some(stance_mask) = represented_stance_mask_like_cpp(form_id) else {
            return applied;
        };
        for spell_id in self.known_spells_like_cpp().to_vec() {
            if spell_id <= 0
                || boost_ids.contains(&spell_id)
                || crate::session::hub_ref(self).player_has_visible_aura_spell_like_cpp(spell_id)
                    == Some(true)
            {
                continue;
            }
            let (stances, _) = spell_store.shapeshift_masks_like_cpp(spell_id);
            if stances == 0 || stances & stance_mask == 0 {
                continue;
            }
            // C++ `SpellInfo::IsPassive() || HasAttribute(
            // SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG)`.
            if !spell_store.is_passive_like_cpp(spell_id)
                && !self.represented_spell_has_attribute_like_cpp(
                    spell_id,
                    0,
                    wow_data::spell::attributes::
                        SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG,
                )
            {
                continue;
            }
            if self.represented_apply_owned_aura_spell_like_cpp(spell_id) {
                applied += 1;
            }
        }
        applied
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
        let mut removed = 0usize;
        for spell_id in self.represented_shapeshift_boost_spell_ids_like_cpp(removed_form) {
            removed += self.represented_remove_owned_aura_spell_like_cpp(spell_id);
        }
        let Some(player_guid) = self.player_guid() else {
            return removed;
        };
        let Some(spell_store) = self.spell_store().cloned() else {
            return removed;
        };
        let new_stance = represented_stance_mask_like_cpp(new_form).unwrap_or(0);
        let slots: Vec<u8> = crate::session::hub_ref(self)
            .resolved_player_visible_auras_like_cpp()
            .unwrap_or_default()
            .values()
            .filter(|aura| aura.caster_guid == player_guid)
            .filter(|aura| {
                let (stances, _) = spell_store.shapeshift_masks_like_cpp(aura.spell_id);
                // C++ `Aura::IsRemovedOnShapeLost`.
                stances != 0
                    && !self.represented_spell_has_attribute_like_cpp(
                        aura.spell_id,
                        2,
                        wow_data::spell::attributes::
                            SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM,
                    )
                    && !self.represented_spell_has_attribute_like_cpp(
                        aura.spell_id,
                        0,
                        wow_data::spell::attributes::SPELL_ATTR0_NOT_SHAPESHIFTED,
                    )
                    && stances & new_stance == 0
            })
            .map(|aura| aura.slot)
            .collect();
        for slot in slots {
            if self.remove_aura(slot).is_ok() {
                removed += 1;
            }
        }
        removed
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

    fn represented_shapeshift_boost_spell_ids_like_cpp(&self, form_id: u32) -> Vec<i32> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_shapeshift_boost_spell_ids_like_cpp(hub, form_id)
    }

    /// C++ `Unit::CastSpell(target, spellId, this)` for a represented self-cast
    /// passive boost: install the spell's owned aura on the session player.
    fn represented_apply_owned_aura_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(spell_info) = self
            .spell_store()
            .and_then(|store| store.get(spell_id))
            .cloned()
        else {
            return false;
        };
        let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
        if effect_mask == 0 {
            return false;
        }
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id,
            player_guid,
            0,
            AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
            effect_mask,
        )
        .is_ok()
    }

    /// C++ `Unit::RemoveOwnedAura(spellId, target->GetGUID())`: remove every
    /// active aura of the spell whose caster is the session player.
    fn represented_remove_owned_aura_spell_like_cpp(&mut self, spell_id: i32) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let slots: Vec<u8> = crate::session::hub_ref(self)
            .resolved_player_visible_auras_like_cpp()
            .unwrap_or_default()
            .values()
            .filter(|aura| aura.spell_id == spell_id && aura.caster_guid == player_guid)
            .map(|aura| aura.slot)
            .collect();
        let mut removed = 0usize;
        for slot in slots {
            if self.remove_aura(slot).is_ok() {
                removed += 1;
            }
        }
        removed
    }
}


/// C++ `UI64LIT(1) << (form - 1)`: the `SpellInfo::Stances` bit a form selects.
fn represented_stance_mask_like_cpp(form_id: u32) -> Option<u64> {
    form_id
        .checked_sub(1)
        .and_then(|shift| 1_u64.checked_shl(shift))
}
