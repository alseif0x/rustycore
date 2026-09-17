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
        let boost_ids = self.represented_shapeshift_boost_spell_ids_like_cpp(form_id);
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
                || self.player_has_visible_aura_spell_like_cpp(spell_id) == Some(true)
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
        let slots: Vec<u8> = self
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

    /// C++ `AuraEffect::HandleShapeshiftBoosts` hardcoded form boost ids
    /// (`SpellAuraEffects.cpp:1332-1392`), including the two glyph-gated
    /// choices. `FORM_DIRE_BEAR_FORM` deliberately has none, matching C++.
    fn represented_shapeshift_boost_spell_ids_like_cpp(&self, form_id: u32) -> Vec<i32> {
        const FORM_CAT_LIKE_CPP: u32 = 1;
        const FORM_TREE_OF_LIFE_LIKE_CPP: u32 = 2;
        const FORM_TRAVEL_LIKE_CPP: u32 = 3;
        const FORM_AQUATIC_LIKE_CPP: u32 = 4;
        const FORM_BEAR_LIKE_CPP: u32 = 5;
        const FORM_GHOST_WOLF_LIKE_CPP: u32 = 16;
        const FORM_FLIGHT_EPIC_LIKE_CPP: u32 = 27;
        const FORM_SHADOWFORM_LIKE_CPP: u32 = 28;
        const FORM_FLIGHT_LIKE_CPP: u32 = 29;
        const FORM_SPIRIT_OF_REDEMPTION_LIKE_CPP: u32 = 32;
        const GLYPH_OF_SHADOW_LIKE_CPP: i32 = 107_906;
        const GLYPH_OF_SHADOWY_FRIENDS_LIKE_CPP: i32 = 126_745;
        const GLYPH_OF_SPECTRAL_WOLF_LIKE_CPP: i32 = 58_135;

        match form_id {
            FORM_CAT_LIKE_CPP => vec![3_025, 48_629, 106_840, 113_636],
            FORM_TREE_OF_LIFE_LIKE_CPP => vec![5_420, 81_097],
            FORM_TRAVEL_LIKE_CPP => vec![5_419],
            FORM_AQUATIC_LIKE_CPP => vec![5_421],
            FORM_BEAR_LIKE_CPP => vec![1_178, 21_178, 106_829, 106_899],
            FORM_GHOST_WOLF_LIKE_CPP => {
                if self.player_has_visible_aura_spell_like_cpp(GLYPH_OF_SPECTRAL_WOLF_LIKE_CPP)
                    == Some(true)
                {
                    vec![160_942]
                } else {
                    Vec::new()
                }
            }
            FORM_FLIGHT_EPIC_LIKE_CPP => vec![40_122, 40_121],
            FORM_SHADOWFORM_LIKE_CPP => {
                if self.player_has_visible_aura_spell_like_cpp(GLYPH_OF_SHADOW_LIKE_CPP)
                    == Some(true)
                {
                    vec![107_904]
                } else if self
                    .player_has_visible_aura_spell_like_cpp(GLYPH_OF_SHADOWY_FRIENDS_LIKE_CPP)
                    == Some(true)
                {
                    vec![142_024]
                } else {
                    vec![107_903]
                }
            }
            FORM_FLIGHT_LIKE_CPP => vec![33_948, 34_764],
            FORM_SPIRIT_OF_REDEMPTION_LIKE_CPP => vec![27_792, 27_795, 62_371],
            _ => Vec::new(),
        }
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
        let slots: Vec<u8> = self
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
