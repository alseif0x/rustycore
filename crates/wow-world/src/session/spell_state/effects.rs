//! Represented spell effect application seen from the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Whether one effective spell effect is inert for the deliberately
    /// narrow Creature -> Player physical melee hit profile while the Creature
    /// is behind the Player.
    ///
    /// This is not a general aura safety list. In particular total stats may
    /// alter avoidance, but C++ disables Player dodge/parry/block from behind
    /// unless aura type 288 is present; that type is intentionally absent
    /// here. The remaining accepted aura types affect visibility, outgoing
    /// damage, reputation, or the victim's outgoing expertise.
    pub(in crate::session) fn player_target_spell_effect_is_hit_inert_like_cpp(
        effect: &wow_data::spell::SpellEffectInfo,
    ) -> bool {
        use wow_data::spell::{aura_types, spell_effect_types};

        if effect.effect == spell_effect_types::SPELL_EFFECT_NONE
            || spell_effect_types::is_cpp_null_or_unused_noop(effect.effect)
        {
            return true;
        }
        if effect.effect_trigger_spell != 0 {
            return false;
        }

        match effect.effect {
            spell_effect_types::SPELL_EFFECT_APPLY_AURA => matches!(
                effect.effect_aura,
                aura_types::SPELL_AURA_MOD_STEALTH_DETECT
                    | aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
                    | aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                    | aura_types::SPELL_AURA_MOD_REPUTATION_GAIN
                    | aura_types::SPELL_AURA_MOD_XP_PCT
                    | aura_types::SPELL_AURA_MOD_EXPERTISE
            ),
            // These login-time capability effects either change only the
            // Player's outgoing attacks/equipment use or the defensive
            // parry/block gates that C++ disables for a rear attacker.
            spell_effect_types::SPELL_EFFECT_PARRY
            | spell_effect_types::SPELL_EFFECT_BLOCK
            | spell_effect_types::SPELL_EFFECT_DUAL_WIELD
            | spell_effect_types::SPELL_EFFECT_PROFICIENCY => true,
            _ => false,
        }
    }
    pub(crate) fn apply_represented_login_spell_reset_if_needed_like_cpp(&mut self) -> bool {
        const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

        if !self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_SPELLS_LIKE_CPP) != 0)
        {
            return false;
        }

        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP, true);
        let spells = self.known_spells_like_cpp().to_vec();
        for spell_id in spells {
            self.remove_known_spell_like_cpp(spell_id);
        }
        self.apply_represented_quest_rewarded_spells_like_cpp();
        self.send_notification_like_cpp(self.reset_spells_notification_text_like_cpp());
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
            .custom_spells_like_cpp(self.player_race_like_cpp(), self.player_class_like_cpp())
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
                runtime.dependent_known_spells.insert(spell_id);
            });
        }

        applied
    }
    #[cfg(test)]
    pub(crate) fn apply_represented_start_all_spells_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_start_all_spells_with_catalogs_like_cpp(
            &player_bootstrap,
            known_spells,
        )
    }
    pub(crate) fn load_represented_talent_row_with_spell_side_effects_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        rank: u8,
        talent_group: u8,
        known_spells: &mut Vec<i32>,
        dependent_spells: &mut HashSet<i32>,
    ) -> bool {
        let previous_rank = self
            .player_talent_runtime_snapshot_like_cpp()
            .and_then(|runtime| {
                runtime
                    .talent_groups
                    .get(usize::from(talent_group))
                    .and_then(|talents| talents.get(&talent_id).copied())
            });

        if !self.load_represented_talent_row_like_cpp(talent_tabs, talent_id, rank, talent_group) {
            return false;
        }

        if self.represented_active_talent_group_like_cpp() != Some(talent_group) {
            return true;
        }

        if let Some(previous_rank) = previous_rank {
            if let Some(previous_spell_id) =
                self.represented_talent_spell_id_like_cpp(talent_id, previous_rank)
            {
                known_spells.retain(|known_spell| *known_spell != previous_spell_id);
                dependent_spells.remove(&previous_spell_id);
                for trigger_spell in
                    self.represented_direct_learn_spell_triggers_like_cpp(previous_spell_id)
                {
                    known_spells.retain(|known_spell| *known_spell != trigger_spell);
                    dependent_spells.remove(&trigger_spell);
                }
            }
        }

        if let Some(spell_id) = self.represented_talent_spell_id_like_cpp(talent_id, rank) {
            if !known_spells.contains(&spell_id) {
                known_spells.push(spell_id);
            }
            dependent_spells.insert(spell_id);
            for trigger_spell in self.represented_direct_learn_spell_triggers_like_cpp(spell_id) {
                if !known_spells.contains(&trigger_spell) {
                    known_spells.push(trigger_spell);
                }
                dependent_spells.insert(trigger_spell);
            }
        }

        if let Some((overriden_spell_id, new_spell_id)) =
            self.represented_talent_override_spell_pair_like_cpp(talent_id)
        {
            self.add_represented_override_spell_like_cpp(overriden_spell_id, new_spell_id);
        }

        true
    }
    pub(in crate::session) fn apply_represented_active_talent_spell_side_effects_like_cpp(
        &mut self,
        talent_id: u32,
        previous_rank: Option<u8>,
        rank: u8,
        talent_group: u8,
    ) {
        if self.represented_active_talent_group_like_cpp() != Some(talent_group) {
            return;
        }

        if let Some(previous_rank) = previous_rank {
            if let Some(previous_spell_id) =
                self.represented_talent_spell_id_like_cpp(talent_id, previous_rank)
            {
                self.remove_known_spell_like_cpp(previous_spell_id);
                for trigger_spell in
                    self.represented_direct_learn_spell_triggers_like_cpp(previous_spell_id)
                {
                    self.remove_known_spell_like_cpp(trigger_spell);
                }
            }
        }

        if let Some(spell_id) = self.represented_talent_spell_id_like_cpp(talent_id, rank) {
            self.learn_known_spell_like_cpp(spell_id);
            for trigger_spell in self.represented_direct_learn_spell_triggers_like_cpp(spell_id) {
                self.learn_known_spell_like_cpp(trigger_spell);
            }
        }

        if let Some((overriden_spell_id, new_spell_id)) =
            self.represented_talent_override_spell_pair_like_cpp(talent_id)
        {
            self.add_represented_override_spell_like_cpp(overriden_spell_id, new_spell_id);
        }

        self.remove_auras_with_interrupt_flags_like_cpp(
            0,
            SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP,
        );
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
        let skills_complete =
            self.apply_loaded_spell_learn_skills_like_cpp(loaded_spell_side_effect_spells);
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
                .spell_learn_spell_map_bounds_like_cpp(spell_id_u32)
                .to_vec();
            for learned_spell in learned_spells {
                let Ok(learned_spell_id) = i32::try_from(learned_spell.spell) else {
                    continue;
                };
                if !learned_spell.auto_learned && learned_spell.active {
                    let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                        runtime.dependent_known_spells.insert(learned_spell_id);
                        runtime.favorite_known_spells.remove(&learned_spell_id);
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
                        self.add_represented_override_spell_like_cpp(
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
