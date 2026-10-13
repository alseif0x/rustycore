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
    pub(crate) fn apply_loaded_known_spell_dependencies_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let roots = known_spells.clone();
        self.apply_loaded_spell_dependencies_from_roots_like_cpp(&roots, known_spells)
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
            if self.apply_proficiency_effect_like_cpp(spell_id).is_ok()
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
    /// Apply the non-aura combat-capability effects cast by C++ `Player::AddSpell`
    /// while `_LoadSpells` reconstructs known passive spells.
    ///
    /// `SPELL_EFFECT_PARRY` and `SPELL_EFFECT_BLOCK` set `m_canParry` /
    /// `m_canBlock` before `Player::UpdateAllStats`, so the first login
    /// projection must observe those flags as well.
    pub(crate) fn apply_login_known_spell_combat_capabilities_like_cpp(
        &mut self,
        known_spells: &[i32],
    ) -> usize {
        if self.player_guid().is_none() {
            return 0;
        }
        let Some(spell_store) = self.spell_store().cloned() else {
            return 0;
        };

        let before = self.core.canonical_player_parry_block_snapshot_like_cpp();
        for &spell_id in known_spells {
            if !spell_store.is_passive_like_cpp(spell_id) {
                continue;
            }
            let Some(spell_info) = spell_store.get(spell_id) else {
                continue;
            };

            if spell_info
                .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY)
            {
                let _ = self.apply_parry_effect_like_cpp();
            }
            if spell_info
                .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK)
            {
                let _ = self.apply_block_effect_like_cpp();
            }
        }

        let after = self.core.canonical_player_parry_block_snapshot_like_cpp();
        usize::from(!before.0 && after.0) + usize::from(!before.1 && after.1)
    }
    pub(crate) fn set_known_spells_like_cpp(&mut self, spells: Vec<i32>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        wow_world_spell::invalidate_represented_player_spell_rows_like_cpp(self);
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_known_spell_ids_like_cpp(spells);
        });
        wow_world_spell::learn_account_mount_spells_like_cpp(self);
    }
    pub(crate) fn set_represented_favorite_known_spells_like_cpp(
        &mut self,
        favorite_spells: HashSet<i32>,
    ) {
        let preserve_complete = self
            .with_player_spell_runtime_like_cpp(|runtime| runtime.rows_complete_like_cpp())
            .unwrap_or(false);
        if !preserve_complete {
            wow_world_spell::invalidate_represented_player_spell_rows_like_cpp(self);
        }
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_known_favorites_like_cpp(favorite_spells);
        });
    }
}
