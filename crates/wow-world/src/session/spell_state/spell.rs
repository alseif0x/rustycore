//! Remaining represented spell operations owned by this responsibility.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn search_spell_focus_like_cpp(
        &self,
        focus_id: u32,
    ) -> Option<RepresentedSpellFocusObjectLikeCpp> {
        if focus_id == 0 {
            return None;
        }
        let caster_position = self.player_position_like_cpp()?;
        let player_map_key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let managed = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
        let map = managed.map();
        let nearby = map.nearby_cell_guids_like_cpp(
            caster_position.x,
            caster_position.y,
            map.visibility_range(),
        );
        for guid in nearby.grid.gameobjects {
            let Some(gameobject) = map.get_typed_game_object(guid) else {
                continue;
            };
            let world = gameobject.world();
            if !world.object().is_in_world() {
                continue;
            }
            let Some(source) = gameobject.represented_spell_focus_use_source_like_cpp() else {
                continue;
            };
            if source.focus_type != focus_id {
                continue;
            }
            if !world
                .position()
                .is_within_dist(&caster_position, source.radius as f32)
            {
                continue;
            }
            return Some(RepresentedSpellFocusObjectLikeCpp {
                guid,
                map_key: player_map_key,
                position: world.position(),
                source,
            });
        }
        None
    }
    pub(crate) fn reset_represented_character_spell_charges_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_history_like_cpp(|history| {
            history.charges.clear();
            history.charges_loaded = false;
        });
    }
    pub(crate) fn mark_represented_character_spell_charges_loaded_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_history_like_cpp(|history| {
            history.charges_loaded = true;
        });
    }
    pub(crate) fn record_loaded_character_spell_charge_like_cpp(
        &mut self,
        category_id: u32,
        recharge_start_unix_secs: i64,
        recharge_end_unix_secs: i64,
    ) {
        let _ = self.mutate_player_spell_history_like_cpp(|history| {
            history.charges.entry(category_id).or_default().push_back(
                wow_entities::SpellChargeState {
                    recharge_start_ms: u64::try_from(recharge_start_unix_secs)
                        .unwrap_or(0)
                        .saturating_mul(1_000),
                    recharge_end_ms: u64::try_from(recharge_end_unix_secs)
                        .unwrap_or(0)
                        .saturating_mul(1_000),
                },
            );
        });
    }
    #[cfg(test)]
    pub fn set_start_all_spells_like_cpp(&mut self, enabled: bool) {
        self.start_all_spells_like_cpp = enabled;
    }
    #[cfg(test)]
    pub(crate) fn start_all_spells_like_cpp(&self) -> bool {
        self.start_all_spells_like_cpp
    }
    /// Install the complete process-wide C++ script binding audit.
    ///
    /// Positive and negative `spell_script_names` rows remain separate: a
    /// negative row applies to every represented rank, while a positive row
    /// applies only to its exact spell. Legacy `spell_scripts` IDs do not
    /// inherit through rank chains.
    pub fn set_spell_runtime_script_authority_like_cpp(
        &mut self,
        exact_spell_ids: Arc<BTreeSet<u32>>,
        all_rank_root_spell_ids: Arc<BTreeSet<u32>>,
        legacy_spell_ids: Arc<BTreeSet<u32>>,
        rejected_linked_trigger_spell_ids: Arc<BTreeSet<u32>>,
    ) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_script_exact_spell_ids_like_cpp = Some(exact_spell_ids);
        self.spell_script_all_rank_root_spell_ids_like_cpp = Some(all_rank_root_spell_ids);
        self.legacy_spell_script_spell_ids_like_cpp = Some(legacy_spell_ids);
        self.spell_linked_rejected_trigger_spell_ids_like_cpp =
            Some(rejected_linked_trigger_spell_ids);
    }
    /// Prove that applying/casting one spell cannot enter an unrepresented
    /// C++ spell script, legacy spell script, or linked-spell hook.
    ///
    /// Rank indeterminacy is not treated as absence: a negative
    /// `spell_script_names` binding can cover the whole C++ chain.
    pub(in crate::session) fn spell_has_no_unrepresented_runtime_hooks_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp(
            spell_id,
            self.spell_script_exact_spell_ids_like_cpp.as_deref(),
            self.spell_script_all_rank_root_spell_ids_like_cpp
                .as_deref(),
            self.legacy_spell_script_spell_ids_like_cpp.as_deref(),
            self.spell_linked_rejected_trigger_spell_ids_like_cpp
                .as_deref(),
            self.spell_catalogs.spell_chain_store.as_deref(),
            self.spell_catalogs.spell_linked_store.as_deref(),
        )
    }
    /// Prove that every effective effect and every world-table hook for one
    /// source spell is inert for the bounded rear physical/melee hit profile.
    pub(in crate::session) fn player_target_spell_is_hit_inert_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> bool {
        let Ok(spell_id_i32) = i32::try_from(spell_id) else {
            return false;
        };
        if !self.spell_has_no_unrepresented_runtime_hooks_like_cpp(spell_id) {
            return false;
        }
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        if spell_store.get(spell_id_i32).is_none() {
            return false;
        }
        spell_store
            .effects_for_difficulty_like_cpp(
                spell_id_i32,
                difficulty_id,
                self.difficulty_store.as_deref(),
            )
            .is_some_and(|effects| {
                effects
                    .iter()
                    .all(Self::player_target_spell_effect_is_hit_inert_like_cpp)
            })
    }
    pub(crate) fn next_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_catalogs
            .spell_chain_store
            .as_ref()
            .map(|store| store.next_spell_in_chain_like_cpp(spell_id))
            .unwrap_or(0)
    }
    pub(crate) fn first_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_catalogs
            .spell_chain_store
            .as_ref()
            .map(|store| store.first_spell_in_chain_like_cpp(spell_id))
            .unwrap_or(spell_id)
    }
    pub(crate) fn prev_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_catalogs
            .spell_chain_store
            .as_ref()
            .map(|store| store.prev_spell_in_chain_like_cpp(spell_id))
            .unwrap_or(0)
    }
    pub(in crate::session) fn player_spell_hit_source_identity_complete_like_cpp(&self) -> bool {
        let Some(player_guid) = self.player_guid else {
            return false;
        };
        self.player_handle_like_cpp
            .is_some_and(|handle| handle.guid() == player_guid)
            || cfg!(test) && self.player_bootstrap_attached_for_test_like_cpp()
    }
    pub(crate) fn spell_linked_like_cpp(
        &self,
        link_type: SpellLinkedTypeLikeCpp,
        spell_id: u32,
    ) -> &[i32] {
        self.spell_catalogs
            .spell_linked_store
            .as_ref()
            .and_then(|store| store.get_spell_linked_like_cpp(link_type, spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn spell_area_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_map_bounds_like_cpp(spell_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_area_map_bounds_like_cpp(
        &self,
        area_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_area_map_bounds_like_cpp(area_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_custom_attributes_for_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> u32 {
        self.spell_catalogs
            .spell_custom_attribute_store
            .as_ref()
            .map(|store| store.attributes_for_spell_difficulty_like_cpp(spell_id, difficulty))
            .unwrap_or(0)
    }
    #[cfg(test)]
    pub(crate) fn serverside_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&ServersideSpellInfoLikeCpp> {
        self.spell_catalogs
            .serverside_spell_store
            .as_ref()
            .and_then(|store| store.get_serverside_spell_like_cpp(spell_id, difficulty))
    }
    pub(crate) fn spell_proc_entry_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&SpellProcEntryLikeCpp> {
        let store = self.spell_catalogs.spell_proc_store.as_ref()?;
        store.spell_proc_entry_with_fallback_like_cpp(spell_id, difficulty, |current_difficulty| {
            self.difficulty_store
                .as_ref()
                .and_then(|difficulties| difficulties.get(current_difficulty))
                .map(|difficulty| u32::from(difficulty.fallback_difficulty_id))
        })
    }
    pub(crate) fn spells_required_for_spell_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.spells_required_for_spell_like_cpp(spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn spells_requiring_spell_like_cpp(&self, req_spell: u32) -> &[u32] {
        self.spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.spells_requiring_spell_like_cpp(req_spell))
            .unwrap_or(&[])
    }
    pub(crate) fn is_spell_requiring_spell_like_cpp(&self, spell_id: u32, req_spell: u32) -> bool {
        self.spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.is_spell_requiring_spell_like_cpp(spell_id, req_spell))
            .unwrap_or(false)
    }
    pub(in crate::session) fn spell_school_mask_for_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u8,
    ) -> u32 {
        self.spell_catalogs
            .spell_misc_store()
            .and_then(|store| {
                store.entry_for_spell_difficulty_with_fallback_like_cpp(
                    spell_id,
                    difficulty,
                    self.difficulty_store().map(AsRef::as_ref),
                )
            })
            .map_or(1, |entry| u32::from(entry.school_mask))
    }
    /// Represented C++ `SpellInfo::IsPositive` (`NegativeEffects.none()`).
    /// The current spell model does not yet persist C++'s calculated
    /// `NegativeEffects` bitset. This mirrors the represented C++ target-check,
    /// intrinsically harmful effect/aura, and sign-sensitive stat families;
    /// unknown non-enemy auras remain positive, as in C++'s default branch.
    pub(in crate::session) fn represented_spell_is_positive_like_cpp(
        spell_info: &wow_data::SpellInfo,
    ) -> bool {
        let effects: Vec<(u32, i32, i32, u32, u32)> = if spell_info.effects().is_empty() {
            vec![(
                spell_info.effect_type,
                spell_info.aura_type.unwrap_or(0),
                spell_info.effect_base_points,
                0,
                0,
            )]
        } else {
            spell_info
                .effects()
                .iter()
                .map(|effect| {
                    (
                        effect.effect,
                        effect.effect_aura,
                        effect.effect_base_points,
                        effect.implicit_target_1,
                        effect.implicit_target_2,
                    )
                })
                .collect()
        };

        const fn target_checks_enemy_like_cpp(target: u32) -> bool {
            matches!(
                target,
                2 | 6 | 15 | 16 | 24 | 28 | 53 | 54 | 93 | 104 | 108 | 115 | 116 | 129 | 134 | 151
            )
        }

        !effects.into_iter().any(|(effect, aura, amount, target_a, target_b)| {
            let targets_enemy =
                target_checks_enemy_like_cpp(target_a) || target_checks_enemy_like_cpp(target_b);
            effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT
                || (effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    && (targets_enemy
                        || matches!(
                            aura,
                            wow_data::spell::aura_types::SPELL_AURA_PERIODIC_DAMAGE
                                | wow_data::spell::aura_types::SPELL_AURA_PERIODIC_DAMAGE_PERCENT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_FEAR
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_STUN
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_ROOT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_SILENCE
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED
                                | wow_data::spell::aura_types::SPELL_AURA_SCHOOL_HEAL_ABSORB
                        )
                        || (matches!(
                            aura,
                            wow_data::spell::aura_types::SPELL_AURA_MOD_STAT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_HEALTH
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_HEALTH_PERCENT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE
                        ) && amount < 0)))
        })
    }
    pub(in crate::session) fn represented_spell_valid_for_talent_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let Some(spell_store) = self.spell_store() else {
            return true;
        };
        Self::represented_spell_valid_with_seen_like_cpp(spell_store, spell_id, &mut HashSet::new())
    }
    pub(in crate::session) fn represented_spell_valid_with_seen_like_cpp(
        spell_store: &wow_data::SpellStore,
        spell_id: i32,
        seen: &mut HashSet<i32>,
    ) -> bool {
        if !seen.insert(spell_id) {
            return true;
        }

        let Some(spell_info) = spell_store.get(spell_id) else {
            return false;
        };

        spell_info.effects().iter().all(|effect| {
            if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL {
                return true;
            }
            if effect.effect_trigger_spell <= 0 {
                return false;
            }
            Self::represented_spell_valid_with_seen_like_cpp(
                spell_store,
                effect.effect_trigger_spell,
                seen,
            )
        })
    }
    pub(in crate::session) fn represented_talent_spell_id_like_cpp(
        &self,
        talent_id: u32,
        rank: u8,
    ) -> Option<i32> {
        self.talent_store()?
            .get(talent_id)?
            .spell_rank
            .get(usize::from(rank))
            .copied()
            .filter(|spell_id| *spell_id > 0)
    }
    pub(in crate::session) fn represented_talent_override_spell_pair_like_cpp(
        &self,
        talent_id: u32,
    ) -> Option<(i32, i32)> {
        let talent = self.talent_store()?.get(talent_id)?;
        (talent.overrides_spell_id > 0 && talent.spell_id > 0)
            .then_some((talent.overrides_spell_id, talent.spell_id))
    }
    pub(in crate::session) fn represented_mount_capability_mod_spell_like_cpp(
        &self,
        mount_capability_id: i32,
    ) -> Option<i32> {
        u32::try_from(mount_capability_id)
            .ok()
            .and_then(|id| self.mount_capability_store.as_ref()?.get(id))
            .map(|capability| capability.mod_spell_aura_id)
            .filter(|spell_id| *spell_id > 0)
    }
    pub(in crate::session) fn reset_spells_notification_text_like_cpp(&self) -> String {
        let text = self.trinity_string_like_cpp(LANG_RESET_SPELLS_LIKE_CPP);
        if text == "<error>" {
            LANG_RESET_SPELLS_TEXT_LIKE_CPP.to_string()
        } else {
            text.to_string()
        }
    }
    pub(in crate::session) fn invalidate_represented_player_spell_rows_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.rows.clear();
            runtime.rows_loaded = false;
            runtime.rows_complete = false;
        });
        self.invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp();
    }
    pub(crate) fn set_complete_represented_player_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    ) -> bool {
        self.replace_loaded_represented_player_spell_rows_like_cpp(rows, true)
    }
    pub(crate) fn replace_loaded_represented_player_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        complete: bool,
    ) -> bool {
        self.invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp();
        let mut exact_rows = BTreeMap::new();
        for row in rows {
            if row.spell_id <= 0
                || exact_rows
                    .insert(row.spell_id, canonical_player_spell_record_like_cpp(row))
                    .is_some()
            {
                self.invalidate_represented_player_spell_rows_like_cpp();
                return false;
            }
        }
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_loaded_spell_rows_like_cpp(exact_rows, complete);
        })
        .is_some()
    }
    pub(crate) fn promote_loaded_character_mount_spells_like_cpp(
        &mut self,
        spells: &[i32],
    ) -> usize {
        if self.mount_store.is_none() {
            return 0;
        }

        let mut added = 0usize;
        for &spell_id in spells {
            added +=
                usize::from(self.add_account_mount_with_faction_counterpart_like_cpp(spell_id, 0));
        }
        added
    }
    pub(crate) const fn account_mount_spells_are_session_dependent_like_cpp() -> bool {
        true
    }
    #[cfg(test)]
    pub(crate) fn set_represented_spell_trait_definition_id_like_cpp(
        &mut self,
        spell_id: i32,
        trait_definition_id: i32,
    ) {
        if self.known_spells_like_cpp().contains(&spell_id) && trait_definition_id > 0 {
            if self
                .represented_spell_trait_definition_ids_like_cpp()
                .get(&spell_id)
                .copied()
                != Some(trait_definition_id)
            {
                self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
            }
            let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime
                    .trait_definition_ids
                    .insert(spell_id, trait_definition_id);
            });
        }
    }
    pub(in crate::session) fn cleanup_removed_spell_titan_grip_like_cpp(&mut self, spell_id: i32) {
        let Some(spell_store) = self.spell_store() else {
            return;
        };
        let Some(spell_info) = spell_store.get(spell_id) else {
            return;
        };
        if !spell_store.is_passive_like_cpp(spell_id)
            || !spell_info
                .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP)
        {
            return;
        }

        let Some(penalty_spell_id) = self.mutate_canonical_player_like_cpp(|player| {
            player
                .can_titan_grip()
                .then_some(player.titan_grip_penalty_spell_id())
        }) else {
            return;
        };
        let Some(penalty_spell_id) = penalty_spell_id else {
            return;
        };

        if penalty_spell_id > 0 {
            let _ = self.remove_represented_auras_due_to_spell_like_cpp(
                i32::try_from(penalty_spell_id).unwrap_or(i32::MAX),
            );
        }

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.set_can_titan_grip(false, 0);
        });
    }
    pub(in crate::session) fn cleanup_removed_spell_dual_wield_like_cpp(&mut self, spell_id: i32) {
        let Some(spell_store) = self.spell_store() else {
            return;
        };
        let Some(spell_info) = spell_store.get(spell_id) else {
            return;
        };
        if !spell_store.is_passive_like_cpp(spell_id)
            || !spell_info
                .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD)
        {
            return;
        }

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            if player.unit().can_dual_wield_like_cpp() {
                player.unit_mut().set_can_dual_wield_like_cpp(false);
            }
        });
    }
    pub(crate) fn add_represented_override_spell_like_cpp(
        &mut self,
        overriden_spell_id: i32,
        new_spell_id: i32,
    ) {
        if overriden_spell_id <= 0 || new_spell_id <= 0 {
            return;
        }
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.add_override_spell_like_cpp(overriden_spell_id, new_spell_id);
        });
    }
    pub(crate) fn remove_represented_override_spell_like_cpp(
        &mut self,
        overriden_spell_id: i32,
        new_spell_id: i32,
    ) {
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.remove_override_spell_like_cpp(overriden_spell_id, new_spell_id);
        });
    }
    pub(crate) fn represented_override_spells_like_cpp(&self) -> HashMap<i32, BTreeSet<i32>> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .override_spells
                .iter()
                .map(|(&id, values)| (id, values.clone()))
                .collect()
        })
        .unwrap_or_default()
    }
    pub(crate) fn represented_spell_trait_definition_ids_like_cpp(&self) -> HashMap<i32, i32> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .trait_definition_ids
                .iter()
                .map(|(&id, &value)| (id, value))
                .collect()
        })
        .unwrap_or_default()
    }
    pub(crate) fn complete_represented_override_spells_like_cpp(
        &self,
    ) -> Option<HashMap<i32, BTreeSet<i32>>> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime.override_spells_complete.then(|| {
                runtime
                    .override_spells
                    .iter()
                    .map(|(&id, values)| (id, values.clone()))
                    .collect()
            })
        })
        .flatten()
    }
    pub(crate) fn complete_represented_spell_trait_definition_ids_like_cpp(
        &self,
    ) -> Option<HashMap<i32, i32>> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime.trait_definition_ids_complete.then(|| {
                runtime
                    .trait_definition_ids
                    .iter()
                    .map(|(&id, &value)| (id, value))
                    .collect()
            })
        })
        .flatten()
    }
    pub(crate) fn set_complete_represented_spell_trait_definition_ids_like_cpp(
        &mut self,
        traits: impl IntoIterator<Item = (i32, i32)>,
    ) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        // Keep caller-provided iteration outside the owner guard. The login
        // loader supplies its already materialized exact-traits Vec here.
        let traits = traits.into_iter().collect();
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_complete_trait_definition_ids_like_cpp(traits)
        })
        .unwrap_or(false)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_set_complete_spell_trait_definition_ids_like_cpp(
        &mut self,
        traits: impl IntoIterator<Item = (i32, i32)>,
    ) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let mut exact_traits = HashMap::new();
        for (spell_id, trait_definition_id) in traits {
            if spell_id <= 0
                || trait_definition_id <= 0
                || exact_traits.insert(spell_id, trait_definition_id).is_some()
            {
                let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                    runtime.trait_definition_ids.clear();
                    runtime.trait_definition_ids_complete = false;
                });
                return false;
            }
        }

        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.trait_definition_ids = exact_traits.into_iter().collect();
            runtime.trait_definition_ids_complete = true;
        })
        .is_some()
    }
    #[cfg(test)]
    pub(crate) fn set_complete_represented_override_spells_like_cpp(
        &mut self,
        overrides: impl IntoIterator<Item = (i32, i32)>,
    ) -> bool {
        let mut exact_overrides = HashMap::<i32, BTreeSet<i32>>::new();
        for (overridden_spell_id, overriding_spell_id) in overrides {
            if overridden_spell_id <= 0 || overriding_spell_id <= 0 {
                let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                    runtime.override_spells.clear();
                    runtime.override_spells_complete = false;
                });
                return false;
            }
            exact_overrides
                .entry(overridden_spell_id)
                .or_default()
                .insert(overriding_spell_id);
        }

        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.override_spells = exact_overrides.into_iter().collect();
            runtime.override_spells_complete = true;
        })
        .is_some()
    }
    pub(in crate::session) fn player_spell_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerSpellRuntimeLikeCpp> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            represented_player_spell_runtime_like_cpp(player.spell_runtime_like_cpp())
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(RepresentedPlayerSpellRuntimeLikeCpp {
                known_spells: self.known_spells.clone(),
                rows: self.represented_player_spell_rows_like_cpp.clone(),
                rows_loaded: self.represented_player_spell_rows_loaded_like_cpp,
                rows_complete: self.represented_player_spell_rows_complete_like_cpp,
                fallback_rows: self.represented_fallback_player_spell_rows_like_cpp.clone(),
                dependent_known_spells: self.represented_dependent_known_spells_like_cpp.clone(),
                removed_known_spells: self.represented_removed_known_spells_like_cpp.clone(),
                favorite_known_spells: self.represented_favorite_known_spells_like_cpp.clone(),
                trait_definition_ids: self.represented_spell_trait_definition_ids_like_cpp.clone(),
                trait_definition_ids_complete: self
                    .represented_spell_trait_definition_ids_complete_like_cpp,
                trait_config_rows: self.represented_trait_config_rows_like_cpp.clone(),
                trait_config_rows_complete: self.represented_trait_config_rows_complete_like_cpp,
                trait_entry_rows_complete: self.represented_trait_entry_rows_complete_like_cpp,
                trait_entry_rows_empty: self.represented_trait_entry_rows_empty_like_cpp,
                override_spells: self.represented_override_spells_like_cpp.clone(),
                override_spells_complete: self.represented_override_spells_complete_like_cpp,
            });
        }
        canonical
    }
    pub(in crate::session) fn with_player_spell_runtime_like_cpp<R>(
        &self,
        query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let runtime = canonical_player_spell_runtime_like_cpp(
                self.player_spell_runtime_snapshot_like_cpp()?,
            );
            return Some(query(&runtime));
        }
        // C++ Player::GetSpellMap returns the owner's map, not a Session copy.
        self.with_owned_player_like_cpp(|player| query(player.spell_runtime_like_cpp()))
    }
    pub(in crate::session) fn mutate_player_spell_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut runtime = canonical_player_spell_runtime_like_cpp(
                self.player_spell_runtime_snapshot_like_cpp()?,
            );
            let result = f(&mut runtime);
            return self
                .store_player_spell_runtime_fixture_like_cpp(
                    represented_player_spell_runtime_like_cpp(&runtime),
                )
                .then_some(result);
        }
        // Player::AddSpell/RemoveSpell change the Player's own spell map.
        // Keep this callback inside the one generation-checked owner access.
        self.with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().spells))
    }
    #[allow(dead_code)]
    pub(crate) fn represented_player_spell_rows_like_cpp(
        &self,
    ) -> Vec<RepresentedPlayerSpellLikeCpp> {
        let Some(runtime) = self.player_spell_runtime_snapshot_like_cpp() else {
            return Vec::new();
        };
        let mut rows = runtime
            .known_spells
            .iter()
            .copied()
            .map(|spell_id| RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: runtime.dependent_known_spells.contains(&spell_id),
                favorite: runtime.favorite_known_spells.contains(&spell_id),
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            })
            .collect::<Vec<_>>();

        rows.extend(
            runtime
                .removed_known_spells
                .iter()
                .copied()
                .map(|spell_id| RepresentedPlayerSpellLikeCpp {
                    spell_id,
                    active: false,
                    disabled: false,
                    dependent: false,
                    favorite: false,
                    state: RepresentedPlayerSpellStateLikeCpp::Removed,
                }),
        );
        rows.sort_by_key(|spell| spell.spell_id);
        rows
    }
    #[cfg(test)]
    pub(crate) fn represented_player_spell_rows_loaded_like_cpp(&self) -> bool {
        self.with_player_spell_runtime_like_cpp(|runtime| runtime.rows_loaded)
            .unwrap_or(false)
    }
    pub(crate) fn complete_represented_player_spell_rows_like_cpp(
        &self,
    ) -> Option<BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            (runtime.rows_loaded && runtime.rows_complete).then(|| {
                runtime
                    .rows
                    .iter()
                    .map(|(&id, row)| (id, represented_player_spell_record_like_cpp(row)))
                    .collect()
            })
        })
        .flatten()
    }
    #[allow(dead_code)]
    pub(crate) fn represented_mount_source_spell_usable_like_cpp(&self, spell_id: u32) -> bool {
        let Some(mount) = self
            .mount_store
            .as_ref()
            .and_then(|store| store.get_by_source_spell_id_like_cpp(spell_id))
        else {
            return false;
        };

        self.represented_meets_player_condition_id_like_cpp(mount.player_condition_id)
    }
    pub(crate) fn add_represented_self_res_spell_like_cpp(&mut self, spell_id: i32) {
        if spell_id == 0 {
            return;
        }
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .self_res_spells
                    .insert(spell_id)
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.represented_self_res_spells_like_cpp.insert(spell_id);
        }
    }
    pub(crate) fn remove_represented_self_res_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player
                .resurrection_state_mut_like_cpp()
                .self_res_spells
                .remove(&spell_id)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.represented_self_res_spells_like_cpp.remove(&spell_id);
        }
        canonical.unwrap_or(false)
    }
    pub(crate) fn has_represented_self_res_spell_like_cpp(&self, spell_id: i32) -> bool {
        self.player_resurrection_state_snapshot_like_cpp()
            .is_some_and(|state| state.self_res_spells.contains(&spell_id))
    }
    pub(crate) fn interrupt_non_melee_spells_for_far_teleport_like_cpp(&mut self) -> bool {
        let session_cast_interrupted = self.interrupt_player_cast_like_cpp(None);
        let canonical_spells_interrupted = self
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                if !unit.is_non_melee_spell_cast_like_cpp(true, false, false, true) {
                    return false;
                }
                !unit.interrupt_non_melee_spells(None, true, true).is_empty()
            })
            .unwrap_or(false);

        session_cast_interrupted || canonical_spells_interrupted
    }
    pub(crate) fn interrupt_current_channeled_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return false;
        };
        if spell_id == 0 {
            return false;
        }

        let interrupted = self
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                if unit
                    .current_spell(wow_entities::CurrentSpellSlot::Channeled)
                    .is_none_or(|current| current.spell_id != spell_id)
                {
                    return false;
                }
                unit.interrupt_spell(wow_entities::CurrentSpellSlot::Channeled, true, true)
                    .is_some()
            })
            .unwrap_or(false);

        if interrupted {
            let _ = self.interrupt_player_cast_like_cpp(Some(spell_id as i32));
        }

        interrupted
    }
}
