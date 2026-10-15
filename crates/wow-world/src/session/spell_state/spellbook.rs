//! Known spells, learning and unlearning at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_cast_unstuck_enabled_like_cpp(&mut self, enabled: bool) {
        self.config.represented_cast_unstuck_enabled_like_cpp = enabled;
    }

    pub fn set_offhand_check_at_spell_unlearn_like_cpp(&mut self, enabled: bool) {
        self.spell_state
            .set_offhand_check_at_spell_unlearn_like_cpp(enabled);
    }
    pub(crate) fn apply_login_known_spell_proficiencies_like_cpp(
        &mut self,
        known_spells: &[i32],
    ) -> usize {
        if self.player_guid().is_none()
            || self
                .player_persistent_capability_state_snapshot_like_cpp()
                .is_none()
        {
            return 0;
        }
        let Some(spell_store) = self.spell_store().cloned() else {
            return 0;
        };

        let mut applied = 0usize;
        for &spell_id in known_spells {
            let Some(spell_info) = spell_store.get(spell_id) else {
                continue;
            };
            if spell_info.effect_type
                != wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY
                && !spell_info.has_effect_like_cpp(
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY,
                )
            {
                continue;
            }

            let Some(before) = self.player_persistent_capability_state_snapshot_like_cpp() else {
                return applied;
            };
            if wow_world_spell::apply_proficiency_effect_like_cpp(self, spell_id).is_ok()
                && self
                    .player_persistent_capability_state_snapshot_like_cpp()
                    .is_some_and(|after| {
                        after.weapon_proficiency != before.weapon_proficiency
                            || after.armor_proficiency != before.armor_proficiency
                    })
            {
                applied += 1;
            }
        }

        applied
    }
}
