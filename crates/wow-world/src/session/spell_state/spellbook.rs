//! Known spells, learning and unlearning at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

mod unlearn;
mod reconstruction;
mod learned_skills;

impl WorldSession {
    pub fn set_cast_unstuck_enabled_like_cpp(&mut self, enabled: bool) {
        self.represented_cast_unstuck_enabled_like_cpp = enabled;
    }

    pub fn set_offhand_check_at_spell_unlearn_like_cpp(&mut self, enabled: bool) {
        self.spell_state
            .represented_offhand_check_at_spell_unlearn_like_cpp = enabled;
    }
    pub(crate) fn spell_learn_skill_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&SpellLearnSkillNodeLikeCpp> {
        match self.spell_learn_skill_lookup_like_cpp(spell_id) {
            SpellLearnSkillLookupLikeCpp::Present(node) => Some(node),
            SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
            | SpellLearnSkillLookupLikeCpp::Indeterminate(_)
            | SpellLearnSkillLookupLikeCpp::MissingCoverage => None,
        }
    }
    pub(crate) fn spell_learn_skill_lookup_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellLearnSkillLookupLikeCpp<'_> {
        self.spell_catalogs
            .spell_learn_skill_store
            .as_ref()
            .map(|store| store.spell_learn_skill_lookup_like_cpp(spell_id))
            .unwrap_or(SpellLearnSkillLookupLikeCpp::MissingCoverage)
    }
    pub(crate) fn spell_learn_spell_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> &[SpellLearnSpellNodeLikeCpp] {
        self.spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .map(|store| store.get_spell_learn_spell_map_bounds_like_cpp(spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn is_spell_learn_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .is_some_and(|store| store.is_spell_learn_spell_like_cpp(spell_id))
    }
    pub(crate) fn is_spell_learn_to_spell_like_cpp(&self, spell_id1: u32, spell_id2: u32) -> bool {
        self.spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .is_some_and(|store| store.is_spell_learn_to_spell_like_cpp(spell_id1, spell_id2))
    }
    pub(in crate::session) fn represented_spell_valid_for_learning_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let Some(spell_store) = self.spell_store() else {
            return false;
        };
        wow_data::represented_spell_valid_with_seen_like_cpp(
            spell_store,
            spell_id,
            &mut HashSet::new(),
        )
    }
    pub(in crate::session) fn represented_direct_learn_spell_triggers_like_cpp(
        &self,
        spell_id: i32,
    ) -> Vec<i32> {
        self.spell_store()
            .and_then(|store| store.get(spell_id))
            .map(|spell_info| {
                spell_info
                    .effects()
                    .iter()
                    .filter(|effect| {
                        effect.effect
                            == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL
                            && effect.effect_trigger_spell > 0
                    })
                    .map(|effect| effect.effect_trigger_spell)
                    .collect()
            })
            .unwrap_or_default()
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

        let before = self.canonical_player_parry_block_snapshot_like_cpp();
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

        let after = self.canonical_player_parry_block_snapshot_like_cpp();
        usize::from(!before.0 && after.0) + usize::from(!before.1 && after.1)
    }
    pub(crate) fn set_known_spells_like_cpp(&mut self, spells: Vec<i32>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.invalidate_represented_player_spell_rows_like_cpp();
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_known_spell_ids_like_cpp(spells);
        });
        self.learn_account_mount_spells_like_cpp();
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_set_known_spells_like_cpp(&mut self, spells: Vec<i32>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.invalidate_represented_player_spell_rows_like_cpp();
        let known_spells = spells.clone();
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_known_spells_and_prune_derived_like_cpp(spells);
        });
        self.learn_account_mount_spells_like_cpp();
    }
    pub(in crate::session) fn learn_account_mount_spells_like_cpp(&mut self) -> usize {
        let Some(mut spell_ids) =
            self.player_collection_state_snapshot_like_cpp()
                .map(|collections| {
                    collections
                        .mounts_like_cpp()
                        .keys()
                        .copied()
                        .collect::<Vec<_>>()
                })
        else {
            return 0;
        };
        spell_ids.sort_unstable();
        spell_ids.dedup();

        let mut learned = 0usize;
        for spell_id in spell_ids {
            let Ok(spell_id_u32) = u32::try_from(spell_id) else {
                continue;
            };
            if !self.mount_store.as_ref().is_none_or(|store| {
                store
                    .get_by_source_spell_id_like_cpp(spell_id_u32)
                    .is_some()
            }) {
                continue;
            }
            let before = self.known_spells_like_cpp().len();
            self.learn_dependent_known_spell_like_cpp(spell_id);
            learned += usize::from(self.known_spells_like_cpp().len() != before);
        }
        learned
    }
    pub(crate) fn learn_known_spell_like_cpp(&mut self, spell_id: i32) {
        if !self.known_spells_like_cpp().contains(&spell_id) {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        // This low-level helper does not run the complete C++ AddSpell closure
        // (ranks, dependencies, skills, traits and overrides). Retaining exact
        // acquisition authority after it would make those mirrors stale.
        self.invalidate_represented_player_spell_rows_like_cpp();
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.learn_known_spell_id_like_cpp(spell_id);
        });
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_learn_known_spell_like_cpp(&mut self, spell_id: i32) {
        if !self.known_spells_like_cpp().contains(&spell_id) {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.invalidate_represented_player_spell_rows_like_cpp();
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.learn_known_spell_id_unless_known_like_cpp(spell_id);
        });
    }
    pub(crate) fn learn_dependent_known_spell_like_cpp(&mut self, spell_id: i32) {
        self.learn_known_spell_like_cpp(spell_id);
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.mark_known_spell_dependent_like_cpp(spell_id);
        });
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_learn_dependent_known_spell_like_cpp(
        &mut self,
        spell_id: i32,
    ) {
        self.fixture_learn_known_spell_like_cpp(spell_id);
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.mark_dependent_learned_spell_like_cpp(spell_id);
        });
    }
    pub(crate) fn resolved_known_spells_like_cpp(&self) -> Option<Vec<i32>> {
        self.with_player_spell_runtime_like_cpp(|runtime| runtime.known_spells_like_cpp().to_vec())
    }
    pub(crate) fn known_spells_like_cpp(&self) -> Vec<i32> {
        self.resolved_known_spells_like_cpp().unwrap_or_default()
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn known_spells_fixture_like_cpp(&self) -> Vec<i32> {
        self.player_spell_test_fixture_like_cpp.known_spells.clone()
    }
    pub(crate) fn represented_dependent_known_spells_like_cpp(&self) -> HashSet<i32> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .dependent_known_spells_like_cpp()
                .iter()
                .copied()
                .collect()
        })
        .unwrap_or_default()
    }
    pub(crate) fn set_represented_favorite_known_spells_like_cpp(
        &mut self,
        favorite_spells: HashSet<i32>,
    ) {
        let preserve_complete = self
            .with_player_spell_runtime_like_cpp(|runtime| runtime.rows_complete_like_cpp())
            .unwrap_or(false);
        if !preserve_complete {
            self.invalidate_represented_player_spell_rows_like_cpp();
        }
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_known_favorites_like_cpp(favorite_spells);
        });
    }
    pub(crate) fn represented_favorite_known_spells_like_cpp(&self) -> HashSet<i32> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .favorite_known_spells_like_cpp()
                .iter()
                .copied()
                .collect()
        })
        .unwrap_or_default()
    }
}
