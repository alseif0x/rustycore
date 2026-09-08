//! Shared represented preparation checks, repeated before launch for late failure.
//! None is an already-published rejection; Some(None) is an admitted cast
//! requiring no focus object. This does not expand the retained effect engine.

use super::*;

impl WorldSession {
    pub(in crate::session) fn check_represented_cast_preparation_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_visual: &wow_packet::packets::spell::SpellCastVisual,
        metadata: SpellCastMetadata,
    ) -> Result<Option<Option<RepresentedSpellFocusObjectLikeCpp>>, &'static str> {
        let spell_id = spell_info.spell_id;
        let has_represented_gameobject_summon_effect = spell_info.effects().iter().any(|effect| {
            effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD
                || spell_effect_is_represented_summon_object_slot_like_cpp(effect.effect)
        });
        let represented_spell_focus_aura_satisfies_check_cast =
            if spell_info.requires_spell_focus_like_cpp() {
                self.resolved_has_represented_aura_effect_with_misc_value_like_cpp(
                    RepresentedAuraEffectLikeCpp::ProvideSpellFocus,
                    i32::try_from(spell_info.requires_spell_focus).unwrap_or(i32::MAX),
                )
                .ok_or("Canonical Player aura owner unavailable")?
            } else {
                false
            };
        let represented_focus_object = if spell_info.requires_spell_focus_like_cpp()
            && has_represented_gameobject_summon_effect
            && !represented_spell_focus_aura_satisfies_check_cast
        {
            self.search_spell_focus_like_cpp(spell_info.requires_spell_focus)
        } else {
            None
        };
        if spell_info.requires_spell_focus_like_cpp()
            && has_represented_gameobject_summon_effect
            && !represented_spell_focus_aura_satisfies_check_cast
            && represented_focus_object.is_none()
        {
            // C++ `Spell::CheckCast` fails before `SMSG_SPELL_GO` when no
            // matching `SPELL_AURA_PROVIDE_SPELL_FOCUS` aura or focus object
            // exists. `Spell::SendCastResult` carries the SpellFocusObject id
            // in FailedArg1 for this failure reason.
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: spell_visual.clone(),
                reason: SpellCastResult::RequiresSpellFocus as i32,
                fail_arg1: i32::try_from(spell_info.requires_spell_focus).unwrap_or(i32::MAX),
                fail_arg2: 0,
            });
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                requires_spell_focus = spell_info.requires_spell_focus,
                "Failing live GameObject summon because C++ SearchSpellFocus found no represented focusObject"
            );
            return Ok(None);
        }

        if let Some(reason) =
            self.check_represented_battle_pet_spell_like_cpp(&spell_info, metadata)
        {
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: spell_visual.clone(),
                reason: reason as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                reason = reason as i32,
                "Failing represented battle-pet spell because C++ Spell::CheckCast rejected it"
            );
            return Ok(None);
        }

        if let Some(outcome) = self.check_represented_mount_spell_like_cpp(&spell_info) {
            match outcome {
                RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(reason) => {
                    self.send_packet(&wow_packet::packets::spell::CastFailed {
                        cast_id,
                        spell_id,
                        visual: spell_visual.clone(),
                        reason: reason as i32,
                        fail_arg1: 0,
                        fail_arg2: 0,
                    });
                    debug!(
                        account = self.account_id,
                        spell_id = spell_id,
                        reason = reason as i32,
                        "Failing represented mount spell because C++ Spell::CheckCast rejected it"
                    );
                }
                RepresentedMountSpellCheckOutcomeLikeCpp::DontReport => {
                    debug!(
                        account = self.account_id,
                        spell_id = spell_id,
                        "Failing represented mount spell with C++ SPELL_FAILED_DONT_REPORT"
                    );
                }
            }
            return Ok(None);
        }

        Ok(Some(represented_focus_object))
    }
}
