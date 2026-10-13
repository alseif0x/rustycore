//! Represented spell effect application seen from the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn apply_represented_login_spell_reset_if_needed_like_cpp(&mut self) -> bool {
        const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

        if !self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_SPELLS_LIKE_CPP) != 0)
        {
            return false;
        }

        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP, true);
        let spells = wow_world_spell::known_spells_like_cpp(self).to_vec();
        for spell_id in spells {
            wow_world_spell::remove_known_spell_like_cpp(self, spell_id);
        }
        self.apply_represented_quest_rewarded_spells_like_cpp();
        self.core
            .send_notification_like_cpp(self.reset_spells_notification_text_like_cpp());
        true
    }
    /// Represented C++ `Player::LearnCustomSpells` / `CONFIG_START_ALL_SPELLS`.
    ///
    /// C++ calls `AddSpell` while the player is not in world, so the full port
    /// still needs character_spell persistence semantics. This represented slice
    /// mirrors the login spell snapshot: configured custom spells are included in
    /// `INITIAL_SPELLS` without duplicating spells already loaded from DB/DBC.
    pub(crate) fn apply_represented_start_all_spells_with_catalogs_like_cpp(
        &mut self,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        if !player_bootstrap.start_all_spells {
            return 0;
        }

        let spells = player_bootstrap
            .custom_spells
            .custom_spells_like_cpp(
                crate::session::hub_ref(self).player_race_like_cpp(),
                crate::session::hub_ref(self).player_class_like_cpp(),
            )
            .to_vec();

        let mut applied = 0usize;
        for spell_id in spells {
            let Ok(spell_id) = i32::try_from(spell_id) else {
                continue;
            };
            if !known_spells.contains(&spell_id) {
                known_spells.push(spell_id);
                applied += 1;
            }
            let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.set_dependent_like_cpp(spell_id, true);
            });
        }

        applied
    }
    pub(crate) fn apply_loaded_spell_dependency_skills_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
        loaded_spell_side_effect_spells: &mut Vec<i32>,
    ) -> (usize, bool) {
        // C++ AddSpell applies SpellLearnSkill and recursively AddSpell-s every
        // non-auto spell_learn_spell target. Expanding the complete closure
        // first is final-state equivalent only if every reached target then
        // participates in the SpellLearnSkill pass before skill authority is
        // published.
        let dependent_spell_count =
            self.apply_loaded_known_spell_dependencies_like_cpp(known_spells);
        for &spell_id in known_spells.iter() {
            if !loaded_spell_side_effect_spells.contains(&spell_id) {
                loaded_spell_side_effect_spells.push(spell_id);
            }
        }
        let skills_complete = {
            let (s, mut h) = crate::session::split_spell_state_mut(self);
            s.apply_loaded_spell_learn_skills_like_cpp(&mut h, loaded_spell_side_effect_spells)
        };
        (dependent_spell_count, skills_complete)
    }
    pub(crate) fn apply_loaded_spell_dependencies_from_roots_like_cpp(
        &mut self,
        roots: &[i32],
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let mut added = 0usize;
        let mut pending = roots.to_vec();
        let mut index = 0usize;
        while index < pending.len() {
            let spell_id = pending[index];
            index += 1;

            let Ok(spell_id_u32) = u32::try_from(spell_id) else {
                continue;
            };
            let learned_spells = self
                .catalogs
                .spell_learn_spell_map_bounds_like_cpp(spell_id_u32)
                .to_vec();
            for learned_spell in learned_spells {
                let Ok(learned_spell_id) = i32::try_from(learned_spell.spell) else {
                    continue;
                };
                if !learned_spell.auto_learned && learned_spell.active {
                    let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                        runtime.set_dependent_like_cpp(learned_spell_id, true);
                        runtime.set_favorite_like_cpp(learned_spell_id, false);
                    });
                    if !known_spells.contains(&learned_spell_id) {
                        known_spells.push(learned_spell_id);
                        pending.push(learned_spell_id);
                        added += 1;
                    }
                }

                // C++ AddSpell rebuilds this edge for every active dependency,
                // including auto-learned and already-known spells.
                if learned_spell.active && learned_spell.overrides_spell != 0 {
                    if let Ok(overrides_spell_id) = i32::try_from(learned_spell.overrides_spell) {
                        wow_world_spell::add_represented_override_spell_like_cpp(
                            self,
                            overrides_spell_id,
                            learned_spell_id,
                        );
                    }
                }
            }
        }

        added
    }
}
