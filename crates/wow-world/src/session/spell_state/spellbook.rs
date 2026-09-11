//! Known spells, learning and unlearning at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_offhand_check_at_spell_unlearn_like_cpp(&mut self, enabled: bool) {
        self.represented_offhand_check_at_spell_unlearn_like_cpp = enabled;
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
        crate::session_rules::represented_spell_valid_with_seen_like_cpp(
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
    pub(crate) fn apply_loaded_known_spell_dependencies_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let roots = known_spells.clone();
        self.apply_loaded_spell_dependencies_from_roots_like_cpp(&roots, known_spells)
    }
    pub(crate) fn deactivate_lower_rank_known_spells_for_send_like_cpp(
        &self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let Some(spell_chains) = self.spell_catalogs.spell_chain_store() else {
            return 0;
        };

        let known_set: HashSet<i32> = known_spells.iter().copied().collect();
        let before = known_spells.len();
        known_spells.retain(|spell_id| {
            let Ok(mut next_spell_id) = u32::try_from(*spell_id) else {
                return true;
            };

            loop {
                next_spell_id = spell_chains.next_spell_in_chain_like_cpp(next_spell_id);
                if next_spell_id == 0 {
                    return true;
                }

                if let Ok(next_spell_i32) = i32::try_from(next_spell_id)
                    && known_set.contains(&next_spell_i32)
                {
                    return false;
                }
            }
        });

        before - known_spells.len()
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
        let Some(mut spell_ids) = self
            .player_collection_state_snapshot_like_cpp()
            .map(|collections| collections.mounts.into_keys().collect::<Vec<_>>())
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
    pub(crate) fn apply_loaded_spell_learn_skills_like_cpp(&mut self, roots: &[i32]) -> bool {
        for &spell_id in roots {
            let Ok(spell_id) = u32::try_from(spell_id) else {
                return false;
            };
            let learned_skill = match self.spell_learn_skill_lookup_like_cpp(spell_id) {
                SpellLearnSkillLookupLikeCpp::Present(node) => *node,
                SpellLearnSkillLookupLikeCpp::CoveredWithoutNode => continue,
                SpellLearnSkillLookupLikeCpp::Indeterminate(_)
                | SpellLearnSkillLookupLikeCpp::MissingCoverage => return false,
            };
            let Some(mut value) = self.resolved_player_skill_value_like_cpp(learned_skill.skill)
            else {
                return false;
            };
            value = value.max(learned_skill.value);
            let Some(current_max) =
                self.resolved_player_skill_max_value_like_cpp(learned_skill.skill)
            else {
                return false;
            };
            let mut new_max = learned_skill.maxvalue;
            if new_max == 0 {
                let (Some(skills), Some(lines), Some(tiers)) = (
                    self.skill_store(),
                    self.skill_line_store(),
                    self.skill_tiers_store(),
                ) else {
                    return false;
                };
                let Some(rc_info) = skills.skill_race_class_info_like_cpp(
                    learned_skill.skill,
                    self.player_race_like_cpp(),
                    self.player_class_like_cpp(),
                ) else {
                    return false;
                };
                match skills.skill_range_type_like_cpp(rc_info, lines, tiers) {
                    SkillRangeTypeLikeCpp::Language => {
                        value = 300;
                        new_max = 300;
                    }
                    SkillRangeTypeLikeCpp::Level => {
                        new_max = self.max_skill_value_for_level_like_cpp();
                    }
                    SkillRangeTypeLikeCpp::Mono => new_max = 1,
                    SkillRangeTypeLikeCpp::Rank => {
                        let Some(tier) = u32::try_from(rc_info.skill_tier_id)
                            .ok()
                            .and_then(|id| tiers.get_skill_tier_like_cpp(id))
                        else {
                            return false;
                        };
                        new_max = tier
                            .get_value_for_tier_index_like_cpp(u32::from(
                                learned_skill.step.saturating_sub(1),
                            ))
                            .try_into()
                            .unwrap_or(u16::MAX);
                    }
                    SkillRangeTypeLikeCpp::None => return false,
                }
                if rc_info.flags & wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    value = new_max;
                }
            }
            self.set_represented_player_skill_like_cpp(
                learned_skill.skill,
                learned_skill.step,
                value,
                current_max.max(new_max),
            );
        }
        true
    }
    fn previous_spell_learn_skill_like_cpp(
        &self,
        mut prev_spell: u32,
    ) -> Option<SpellLearnSkillNodeLikeCpp> {
        let mut prev_skill = self.spell_learn_skill_like_cpp(prev_spell).copied();
        while prev_skill.is_none() && prev_spell != 0 {
            prev_spell = self.prev_spell_in_chain_like_cpp(prev_spell);
            let first_spell_id = self.first_spell_in_chain_like_cpp(prev_spell);
            prev_skill = self.spell_learn_skill_like_cpp(first_spell_id).copied();
        }
        prev_skill
    }
    fn downgrade_represented_spell_learn_skill_like_cpp(
        &mut self,
        learned_skill: SpellLearnSkillNodeLikeCpp,
        current_spell_id: u32,
    ) {
        let prev_spell = self.prev_spell_in_chain_like_cpp(current_spell_id);
        if prev_spell == 0 {
            self.set_represented_player_skill_like_cpp(learned_skill.skill, 0, 0, 0);
            return;
        }

        let Some(prev_skill) = self.previous_spell_learn_skill_like_cpp(prev_spell) else {
            self.set_represented_player_skill_like_cpp(learned_skill.skill, 0, 0, 0);
            return;
        };

        let Some(mut skill_value) = self.resolved_player_skill_value_like_cpp(prev_skill.skill)
        else {
            return;
        };
        let Some(mut skill_max_value) =
            self.resolved_player_skill_max_value_like_cpp(prev_skill.skill)
        else {
            return;
        };
        let mut new_skill_max_value = prev_skill.maxvalue;

        if new_skill_max_value == 0 {
            if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
                self.skill_store(),
                self.skill_line_store(),
                self.skill_tiers_store(),
            ) {
                if let Some(rc_info) = skill_store.skill_race_class_info_like_cpp(
                    prev_skill.skill,
                    self.player_race_like_cpp(),
                    self.player_class_like_cpp(),
                ) {
                    match skill_store.skill_range_type_like_cpp(
                        rc_info,
                        skill_line_store,
                        skill_tiers_store,
                    ) {
                        SkillRangeTypeLikeCpp::Language => {
                            skill_value = 300;
                            new_skill_max_value = 300;
                        }
                        SkillRangeTypeLikeCpp::Level => {
                            new_skill_max_value = self.max_skill_value_for_level_like_cpp();
                        }
                        SkillRangeTypeLikeCpp::Mono => {
                            new_skill_max_value = 1;
                        }
                        SkillRangeTypeLikeCpp::Rank => {
                            if let Some(tier) = u32::try_from(rc_info.skill_tier_id).ok().and_then(
                                |skill_tier_id| {
                                    skill_tiers_store.get_skill_tier_like_cpp(skill_tier_id)
                                },
                            ) {
                                new_skill_max_value = tier
                                    .get_value_for_tier_index_like_cpp(u32::from(
                                        prev_skill.step.saturating_sub(1),
                                    ))
                                    .try_into()
                                    .unwrap_or(u16::MAX);
                            }
                        }
                        SkillRangeTypeLikeCpp::None => {}
                    }

                    if rc_info.flags & wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                        skill_value = new_skill_max_value;
                    }
                }
            }
        } else if skill_value > prev_skill.value {
            skill_value = prev_skill.value;
        }

        if skill_max_value > new_skill_max_value {
            skill_max_value = new_skill_max_value;
        }
        if skill_value > new_skill_max_value {
            skill_value = new_skill_max_value;
        }

        self.set_represented_player_skill_like_cpp(
            prev_skill.skill,
            prev_skill.step,
            skill_value,
            skill_max_value,
        );
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
    pub(crate) fn remove_known_spell_like_cpp(&mut self, spell_id: i32) {
        self.remove_known_spell_with_suppress_messaging_like_cpp(spell_id, false);
    }
    pub(crate) fn remove_known_spell_with_suppress_messaging_like_cpp(
        &mut self,
        spell_id: i32,
        suppress_messaging: bool,
    ) {
        if self.known_spells_like_cpp().contains(&spell_id) {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        let mut seen = HashSet::new();
        self.remove_known_spell_with_seen_like_cpp(spell_id, true, suppress_messaging, &mut seen);
    }
    fn remove_known_spell_with_seen_like_cpp(
        &mut self,
        spell_id: i32,
        learn_low_rank: bool,
        suppress_messaging: bool,
        seen: &mut HashSet<i32>,
    ) {
        if !self.known_spells_like_cpp().contains(&spell_id) {
            return;
        }

        let preserve_complete = self
            .with_player_spell_runtime_like_cpp(|runtime| runtime.rows_complete_like_cpp())
            .unwrap_or(false);
        if !preserve_complete {
            self.invalidate_represented_player_spell_rows_like_cpp();
        }

        if !seen.insert(spell_id) {
            return;
        }

        if let Ok(current_spell_id) = u32::try_from(spell_id) {
            let next_spell_id = self.next_spell_in_chain_like_cpp(current_spell_id);
            if next_spell_id != 0 {
                if let Ok(next_known_spell_id) = i32::try_from(next_spell_id) {
                    let next_spell_is_talent = self
                        .spell_custom_attributes_for_difficulty_like_cpp(next_spell_id, 0)
                        & wow_data::SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
                        != 0;
                    if self.known_spells_like_cpp().contains(&next_known_spell_id)
                        && !next_spell_is_talent
                    {
                        self.remove_known_spell_with_seen_like_cpp(
                            next_known_spell_id,
                            false,
                            false,
                            seen,
                        );
                    }
                }
            }

            let spells_requiring_removed: Vec<i32> = self
                .spells_requiring_spell_like_cpp(current_spell_id)
                .iter()
                .filter_map(|spell| i32::try_from(*spell).ok())
                .collect();
            for requiring_spell_id in spells_requiring_removed {
                if self.known_spells_like_cpp().contains(&requiring_spell_id) {
                    self.remove_known_spell_with_seen_like_cpp(
                        requiring_spell_id,
                        true,
                        false,
                        seen,
                    );
                }
            }
        }

        let Some(was_dependent) = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            let forgotten = runtime.forget_known_spell_like_cpp(spell_id);
            let was_dependent = forgotten.was_dependent;
            if preserve_complete {
                if was_dependent {
                    runtime.remove_row_like_cpp(spell_id);
                } else if let Some(row) = runtime.row_mut_like_cpp(spell_id) {
                    row.active = false;
                    row.disabled = false;
                    row.dependent = false;
                    row.favorite = false;
                    row.state = wow_entities::PlayerSpellLoadState::Removed;
                }
            }
            was_dependent
        }) else {
            return;
        };

        let mut unlearned_spells_packet_like_cpp = None;

        if let Ok(current_spell_id) = u32::try_from(spell_id) {
            let mut prev_activate = false;

            if let Some(learned_skill) = self.spell_learn_skill_like_cpp(current_spell_id).copied()
            {
                self.downgrade_represented_spell_learn_skill_like_cpp(
                    learned_skill,
                    current_spell_id,
                );
            }

            let learned_spells: Vec<SpellLearnSpellNodeLikeCpp> = self
                .spell_learn_spell_map_bounds_like_cpp(current_spell_id)
                .to_vec();
            for learned_spell in learned_spells {
                if let Ok(learned_spell_id) = i32::try_from(learned_spell.spell) {
                    self.remove_known_spell_with_seen_like_cpp(learned_spell_id, true, false, seen);
                    if learned_spell.overrides_spell != 0 {
                        if let Ok(overrides_spell_id) = i32::try_from(learned_spell.overrides_spell)
                        {
                            self.remove_represented_override_spell_like_cpp(
                                overrides_spell_id,
                                learned_spell_id,
                            );
                        }
                    }
                }
            }

            if learn_low_rank {
                let prev_spell_id = self.prev_spell_in_chain_like_cpp(current_spell_id);
                if prev_spell_id != 0 {
                    if let Ok(prev_known_spell_id) = i32::try_from(prev_spell_id) {
                        let current_spell_is_ranked = self
                            .spell_catalogs
                            .spell_chain_store()
                            .and_then(|store| store.spell_chain_node_like_cpp(current_spell_id))
                            .is_some();
                        if current_spell_is_ranked
                            && self.known_spells_like_cpp().contains(&prev_known_spell_id)
                        {
                            if was_dependent {
                                self.learn_dependent_known_spell_like_cpp(prev_known_spell_id);
                            } else {
                                self.learn_known_spell_like_cpp(prev_known_spell_id);
                                let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                                    runtime.set_dependent_like_cpp(prev_known_spell_id, false);
                                });
                            }
                            self.send_packet(
                                &wow_packet::packets::trainer::SupercededSpells::single(
                                    spell_id,
                                    prev_known_spell_id,
                                ),
                            );
                            prev_activate = true;
                        }
                    }
                }
            }

            if !prev_activate {
                unlearned_spells_packet_like_cpp = Some((current_spell_id, suppress_messaging));
            }
        }

        let trait_definition_id = self
            .mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.remove_override_spell_entry_like_cpp(spell_id);
                runtime.take_trait_definition_id_like_cpp(spell_id)
            })
            .flatten();
        if let Some(trait_definition_id) = trait_definition_id {
            if let Ok(trait_definition_id) = u32::try_from(trait_definition_id) {
                let override_spell_id = self
                    .trait_definition_store()
                    .and_then(|store| store.get(trait_definition_id))
                    .map(|definition| definition.overrides_spell_id)
                    .unwrap_or(0);
                if override_spell_id > 0 {
                    self.remove_represented_override_spell_like_cpp(override_spell_id, spell_id);
                }
            }
        }
        self.cleanup_removed_spell_titan_grip_like_cpp(spell_id);
        self.cleanup_removed_spell_dual_wield_like_cpp(spell_id);
        if self.represented_offhand_check_at_spell_unlearn_like_cpp {
            self.represented_auto_unequip_offhand_if_need_like_cpp(false);
        }

        if let Some((spell_id, suppress_messaging)) = unlearned_spells_packet_like_cpp {
            self.send_packet(&wow_packet::packets::trainer::UnlearnedSpells::single(
                spell_id,
                suppress_messaging,
            ));
        }
    }
    pub(crate) fn resolved_known_spells_like_cpp(&self) -> Option<Vec<i32>> {
        self.with_player_spell_runtime_like_cpp(|runtime| runtime.known_spells_like_cpp().to_vec())
    }
    pub(crate) fn known_spells_like_cpp(&self) -> Vec<i32> {
        self.resolved_known_spells_like_cpp().unwrap_or_default()
    }
    #[cfg(test)]
    pub(crate) fn known_spells_fixture_like_cpp(&self) -> Vec<i32> {
        self.known_spells.clone()
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
