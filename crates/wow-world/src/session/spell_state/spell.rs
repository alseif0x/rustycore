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
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.search_spell_focus_like_cpp(hub, focus_id)
    }
    pub(crate) fn mark_represented_character_spell_charges_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mark_represented_character_spell_charges_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn record_loaded_character_spell_charge_like_cpp(
        &mut self,
        category_id: u32,
        recharge_start_unix_secs: i64,
        recharge_end_unix_secs: i64,
    ) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.record_loaded_character_spell_charge_like_cpp(
            &mut hub,
            category_id,
            recharge_start_unix_secs,
            recharge_end_unix_secs,
        )
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
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_state
            .install_spell_runtime_script_authority_like_cpp(
                exact_spell_ids,
                all_rank_root_spell_ids,
                legacy_spell_ids,
                rejected_linked_trigger_spell_ids,
            );
    }
    pub(in crate::session) fn player_target_spell_is_hit_inert_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.player_target_spell_is_hit_inert_like_cpp(hub, spell_id, difficulty_id)
    }
    pub(in crate::session) fn player_spell_hit_source_identity_complete_like_cpp(&self) -> bool {
        let Some(player_guid) = self.core.player_guid else {
            return false;
        };
        self.core
            .player_handle_like_cpp
            .is_some_and(|handle| handle.guid() == player_guid)
            || cfg!(test) && self.player_bootstrap_attached_for_test_like_cpp()
    }
    pub(crate) fn spell_linked_like_cpp(
        &self,
        link_type: SpellLinkedTypeLikeCpp,
        spell_id: u32,
    ) -> &[i32] {
        self.catalogs
            .spell_catalogs
            .spell_linked_store
            .as_ref()
            .and_then(|store| store.get_spell_linked_like_cpp(link_type, spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn spell_area_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_map_bounds_like_cpp(spell_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_area_map_bounds_like_cpp(
        &self,
        area_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_area_map_bounds_like_cpp(area_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_proc_entry_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&SpellProcEntryLikeCpp> {
        let store = self.catalogs.spell_catalogs.spell_proc_store.as_ref()?;
        store.spell_proc_entry_with_fallback_like_cpp(spell_id, difficulty, |current_difficulty| {
            self.catalogs
                .difficulty_store
                .as_ref()
                .and_then(|difficulties| difficulties.get(current_difficulty))
                .map(|difficulty| u32::from(difficulty.fallback_difficulty_id))
        })
    }
    pub(crate) fn spells_required_for_spell_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.catalogs
            .spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.spells_required_for_spell_like_cpp(spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn is_spell_requiring_spell_like_cpp(&self, spell_id: u32, req_spell: u32) -> bool {
        self.catalogs
            .spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.is_spell_requiring_spell_like_cpp(spell_id, req_spell))
            .unwrap_or(false)
    }
    pub(in crate::session) fn represented_talent_spell_id_like_cpp(
        &self,
        talent_id: u32,
        rank: u8,
    ) -> Option<i32> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_talent_spell_id_like_cpp(hub, talent_id, rank)
    }
    pub(in crate::session) fn represented_talent_override_spell_pair_like_cpp(
        &self,
        talent_id: u32,
    ) -> Option<(i32, i32)> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_talent_override_spell_pair_like_cpp(hub, talent_id)
    }
    pub(in crate::session) fn reset_spells_notification_text_like_cpp(&self) -> String {
        let text = self.trinity_string_like_cpp(LANG_RESET_SPELLS_LIKE_CPP);
        if text == "<error>" {
            LANG_RESET_SPELLS_TEXT_LIKE_CPP.to_string()
        } else {
            text.to_string()
        }
    }
    /// The owner-dispatch hook the canonical-ownership regressions drive.
    ///
    /// Production has no such caller: every transition goes through a named
    /// operation. This keeps the active/detached/replacement coverage able to
    /// exercise the dispatch itself (#754).
    #[cfg(test)]
    pub(crate) fn mutate_player_spell_runtime_for_test_like_cpp<R>(
        &mut self,
        apply: impl FnOnce(&mut wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        self.mutate_player_spell_runtime_like_cpp(apply)
    }

    /// Take one spell's trait definition id (C++ `Player::RemoveSpell`).
    pub(in crate::session) fn take_represented_trait_definition_id_like_cpp(
        &mut self,
        spell_id: i32,
    ) -> Option<i32> {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.take_trait_definition_id_like_cpp(spell_id)
        })
        .flatten()
    }

    /// Install one loaded spell row, keeping the authoritative set when it
    /// exists and otherwise retaining it as a fallback grant.
    pub(in crate::session) fn install_loaded_spell_row_like_cpp(
        &mut self,
        spell_id: i32,
        row: wow_entities::PlayerKnownSpellRecord,
        complete_rows: Option<
            std::collections::BTreeMap<i32, wow_entities::PlayerKnownSpellRecord>,
        >,
    ) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            if let Some(mut rows) = complete_rows {
                rows.insert(spell_id, row);
                runtime.replace_rows_like_cpp(rows, true);
            } else {
                runtime.insert_fallback_row_like_cpp(spell_id, row);
            }
        })
        .is_some()
    }

    /// Drop the trait definitions, config headers and entry flags before a
    /// trait-config load replaces them.
    pub(in crate::session) fn begin_represented_trait_authority_load_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.begin_trait_authority_load_like_cpp();
        })
        .is_some()
    }

    /// Drop the trait definitions and config headers a login reset discards.
    pub(in crate::session) fn begin_represented_trait_config_load_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(
            wow_entities::PlayerSpellRuntimeState::begin_trait_config_load_like_cpp,
        )
        .is_some()
    }

    /// Complete one represented trait-config load.
    pub(in crate::session) fn complete_represented_trait_config_rows_like_cpp(
        &mut self,
        configs: Vec<(i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> Option<bool> {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.complete_trait_config_load_like_cpp(configs, entries_empty)
        })
    }

    /// Install the authoritative trait-config headers and their entry flags.
    pub(in crate::session) fn install_represented_trait_authority_rows_like_cpp(
        &mut self,
        rows: std::collections::BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
        entries_empty: bool,
    ) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.complete_trait_authority_load_like_cpp(rows, entries_empty);
        })
        .is_some()
    }

    /// Drop every trait and override edge, as login does before rebuilding a
    /// fresh C++ Player.
    pub(in crate::session) fn clear_represented_trait_and_override_state_like_cpp(
        &mut self,
    ) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.clear_trait_and_override_state_like_cpp();
        })
        .is_some()
    }

    /// Drop the loaded trait-config headers and return them to unhydrated.
    pub(in crate::session) fn clear_represented_trait_config_rows_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.clear_trait_config_rows_like_cpp();
        })
        .is_some()
    }

    /// Forget the rows retained while no authoritative load exists.
    pub(in crate::session) fn clear_represented_fallback_spell_rows_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.clear_fallback_rows_like_cpp();
        })
        .is_some()
    }

    /// Settle the saved rows, as `Player::_SaveSpells` completing does.
    pub(in crate::session) fn mark_represented_spell_rows_saved_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(
            wow_entities::PlayerSpellRuntimeState::mark_spell_rows_saved_like_cpp,
        )
        .is_some()
    }

    /// Rebase every derived set on the rows a save left behind.
    pub(in crate::session) fn rebase_represented_spells_onto_saved_rows_like_cpp(
        &mut self,
    ) -> bool {
        self.mutate_player_spell_runtime_like_cpp(
            wow_entities::PlayerSpellRuntimeState::rebase_onto_saved_rows_like_cpp,
        )
        .is_some()
    }

    pub(in crate::session) fn invalidate_represented_player_spell_rows_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.clear_rows_like_cpp();
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
        if self.catalogs.mount_store.is_none() {
            return 0;
        }

        let mut added = 0usize;
        for &spell_id in spells {
            added +=
                usize::from(self.add_account_mount_with_faction_counterpart_like_cpp(spell_id, 0));
        }
        added
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
                self.core
                    .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
            }
            let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.set_trait_definition_id_like_cpp(spell_id, Some(trait_definition_id));
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

        let Some(penalty_spell_id) = self.core.mutate_canonical_player_like_cpp(|player| {
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

        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            player.set_can_titan_grip(false, 0);
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
                .override_spells_like_cpp()
                .iter()
                .map(|(&id, values)| (id, values.clone()))
                .collect()
        })
        .unwrap_or_default()
    }
    pub(crate) fn represented_spell_trait_definition_ids_like_cpp(&self) -> HashMap<i32, i32> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .trait_definition_ids_like_cpp()
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
            runtime.override_spells_complete_like_cpp().then(|| {
                runtime
                    .override_spells_like_cpp()
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
            runtime.trait_definition_ids_complete_like_cpp().then(|| {
                runtime
                    .trait_definition_ids_like_cpp()
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
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
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
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let mut exact_traits = HashMap::new();
        for (spell_id, trait_definition_id) in traits {
            if spell_id <= 0
                || trait_definition_id <= 0
                || exact_traits.insert(spell_id, trait_definition_id).is_some()
            {
                let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                    runtime.clear_trait_definition_ids_like_cpp();
                });
                return false;
            }
        }

        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_trait_definition_ids_like_cpp(exact_traits.into_iter().collect(), true);
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
                    runtime.clear_override_spells_like_cpp();
                });
                return false;
            }
            exact_overrides
                .entry(overridden_spell_id)
                .or_default()
                .insert(overriding_spell_id);
        }

        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_override_spells_like_cpp(exact_overrides.into_iter().collect(), true);
        })
        .is_some()
    }
    pub(in crate::session) fn player_spell_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerSpellRuntimeLikeCpp> {
        let canonical = self
            .core
            .owned_spell_acquisition_access_like_cpp()
            .with_player_spell_runtime_like_cpp(represented_player_spell_runtime_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.spell_state
                    .represented_spell_runtime_fixture_like_cpp(),
            );
        }
        canonical
    }
    pub(in crate::session) fn with_player_spell_runtime_like_cpp<R>(
        &self,
        query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            let runtime = canonical_player_spell_runtime_like_cpp(
                self.player_spell_runtime_snapshot_like_cpp()?,
            );
            return Some(query(&runtime));
        }
        // C++ Player::GetSpellMap returns the owner's map, not a Session copy.
        self.core
            .with_owned_player_like_cpp(|player| query(player.spell_runtime_like_cpp()))
    }
    /// Incarnation dispatch for one named spell-runtime transition.
    ///
    /// Scoped to this owner module: the transitions themselves are the named
    /// operations on `PlayerSpellRuntimeState` and the session wrappers below,
    /// so no other module writes the Player's spell state directly (#754).
    pub(in crate::session::spell_state) fn mutate_player_spell_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
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
        self.core
            .with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().spells))
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
        self.with_player_spell_runtime_like_cpp(|runtime| runtime.rows_loaded_like_cpp())
            .unwrap_or(false)
    }
    pub(crate) fn complete_represented_player_spell_rows_like_cpp(
        &self,
    ) -> Option<BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            (runtime.rows_loaded_like_cpp() && runtime.rows_complete_like_cpp()).then(|| {
                runtime
                    .rows_like_cpp()
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
            .catalogs
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
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .self_res_spells
                    .insert(spell_id)
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.spell_state
                .add_represented_self_res_spell_for_test_like_cpp(spell_id);
        }
    }
    pub(crate) fn has_represented_self_res_spell_like_cpp(&self, spell_id: i32) -> bool {
        self.player_resurrection_state_snapshot_like_cpp()
            .is_some_and(|state| state.self_res_spells.contains(&spell_id))
    }
    pub(crate) fn interrupt_non_melee_spells_for_far_teleport_like_cpp(&mut self) -> bool {
        let session_cast_interrupted = self.interrupt_player_cast_like_cpp(None);
        let canonical_spells_interrupted = self
            .core
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
            .core
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

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/spell/f3_shims.rs"]
mod f3_shims;
