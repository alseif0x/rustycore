//! Represented spell acquisition runtime wiring and its fixtures.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_spell_acquisition_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .player
            .player_spell_acquisition = Some(port);
    }
    /// Install the process-wide audited static authority consumed by the
    /// immutable acquisition planner. Absence remains fail-closed.
    pub fn set_spell_acquisition_static_authority_like_cpp(
        &mut self,
        safe_cast_spell_ids: impl IntoIterator<Item = u32>,
        valid_craft_spell_ids: impl IntoIterator<Item = u32>,
    ) {
        self.spell_acquisition_cast_authority_like_cpp = Some(Arc::new(
            crate::spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp::from_audited_rows_like_cpp(
                safe_cast_spell_ids,
                std::iter::empty(),
            ),
        ));
        self.spell_acquisition_craft_authority_like_cpp = Some(Arc::new(
            crate::spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp::from_audited_rows_like_cpp(
                valid_craft_spell_ids,
                std::iter::empty(),
            ),
        ));
    }
    /// Commit one trainer fee and its prepared spell/skill acquisition under
    /// the same per-character money exclusion. Runtime publication remains the
    /// caller's responsibility and must complete before the returned guard is
    /// dropped.
    pub(crate) async fn commit_exclusive_player_money_and_spell_acquisition_like_cpp(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        prepared: &crate::spell_acquisition::PreparedPlayerSpellAcquisitionLikeCpp,
        money_before: u64,
        money_after: u64,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        #[cfg(test)]
        if let Some(success) = self.loot_money_persistence_test_result_like_cpp {
            return success.then_some(money_persistence);
        }

        let port = self
            .persistence_ports_like_cpp
            .player
            .player_spell_acquisition
            .clone()?;
        let player_guid = self.player_guid()?;
        let guid_counter = player_guid.counter() as u64;
        let mut cancellation_fence = PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(
            &self.durable_loot_money_persistence_like_cpp,
        ));
        let mut operation_token = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut operation_token);
        let request =
            match crate::spell_acquisition::player_spell_acquisition_persistence_request_like_cpp(
                guid_counter,
                prepared,
                money_before,
                money_after,
                operation_token,
            ) {
                Ok(request) => request,
                Err(error) => {
                    cancellation_fence.disarm_like_cpp();
                    warn!(%error, "trainer purchase request was not persistence-safe");
                    return None;
                }
            };
        use crate::spell_acquisition::PlayerSpellAcquisitionPersistenceOutcomeLikeCpp as Outcome;
        match crate::spell_acquisition::persist_player_spell_acquisition_through_port_like_cpp(
            &*port, request,
        )
        .await
        {
            Outcome::Applied => {
                cancellation_fence.disarm_like_cpp();
                Some(money_persistence)
            }
            Outcome::DefinitelyRolledBack(reason) => {
                cancellation_fence.disarm_like_cpp();
                warn!(error = %reason, "trainer purchase transaction definitely rolled back");
                None
            }
            Outcome::ReconciledCommit(reason) => {
                cancellation_fence.disarm_like_cpp();
                warn!(error = %reason, "trainer COMMIT reply was lost but durable rows prove commit");
                Some(money_persistence)
            }
            Outcome::Indeterminate(reason) => {
                self.durable_loot_money_persistence_like_cpp
                    .mark_indeterminate_like_cpp();
                cancellation_fence.disarm_like_cpp();
                self.kick("trainer purchase COMMIT outcome is unknown; relog required");
                warn!(error = %reason, "trainer COMMIT outcome remains indeterminate; session quarantined");
                None
            }
        }
    }
    pub(in crate::session) fn invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp(
        &mut self,
    ) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);
        });
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_replace_loaded_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        complete: bool,
    ) -> bool {
        // A replacement can describe another logical PlayerSpellMap. Existing
        // trait/override mirrors must be re-authorized even when spell IDs happen
        // to overlap, otherwise residual state can be attributed to the new map.
        self.invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp();
        let mut exact_rows = BTreeMap::new();
        for row in rows {
            if row.spell_id <= 0 || exact_rows.insert(row.spell_id, row).is_some() {
                self.invalidate_represented_player_spell_rows_like_cpp();
                return false;
            }
        }

        // A coherent load must not erase a dirty base grant produced earlier
        // in this same Player lifetime while ancillary planner authority was
        // unavailable. Reconcile it against the newly authoritative row using
        // C++ `Player::LearnSpell`: disabled rows preserve active, other rows
        // become active, and favorite/dependent state comes from the load.
        let fallback_rows = self
            .player_spell_runtime_snapshot_like_cpp()
            .map(|runtime| runtime.fallback_rows)
            .unwrap_or_default();
        for (&spell_id, &fallback) in &fallback_rows {
            let reconciled = if let Some(loaded) = exact_rows.get(&spell_id).copied() {
                let active = if loaded.disabled { loaded.active } else { true };
                let dependent_promoted = fallback.dependent && !loaded.dependent;
                RepresentedPlayerSpellLikeCpp {
                    active,
                    disabled: false,
                    state: match loaded.state {
                        RepresentedPlayerSpellStateLikeCpp::New => {
                            RepresentedPlayerSpellStateLikeCpp::New
                        }
                        RepresentedPlayerSpellStateLikeCpp::Removed => {
                            RepresentedPlayerSpellStateLikeCpp::Changed
                        }
                        RepresentedPlayerSpellStateLikeCpp::Temporary => {
                            RepresentedPlayerSpellStateLikeCpp::New
                        }
                        _ if loaded.disabled || loaded.active != active || dependent_promoted => {
                            RepresentedPlayerSpellStateLikeCpp::Changed
                        }
                        _ => loaded.state,
                    },
                    dependent: loaded.dependent || fallback.dependent,
                    ..loaded
                }
            } else {
                fallback
            };
            exact_rows.insert(spell_id, reconciled);
        }

        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_rows_like_cpp(
                exact_rows
                    .into_iter()
                    .map(|(id, row)| (id, canonical_player_spell_record_like_cpp(row)))
                    .collect(),
                complete,
            );
        })
        .is_some()
    }
    /// Authorizes the current post-login spell, trait and override mirrors as one
    /// coherent acquisition snapshot. Call only after all represented `AddSpell`
    /// work for character login has completed.
    pub(crate) fn mark_represented_spell_acquisition_snapshot_complete_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.mark_override_spells_complete_like_cpp();
        });
    }
    /// Installs one validated spell-acquisition snapshot without an await or a
    /// second semantic walk. It accepts both the dirty post-`LearnSpell`
    /// snapshot and the normalized post-save snapshot. Inputs are validated
    /// into temporary maps first so a malformed prepared result cannot
    /// partially mutate the live player authority.
    pub(crate) fn replace_complete_spell_acquisition_runtime_like_cpp(
        &mut self,
        spell_rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        traits: impl IntoIterator<Item = (i32, i32)>,
        overrides: impl IntoIterator<Item = (i32, i32)>,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_skill_slots: u16,
        non_durable_skill_tombstones: BTreeSet<u16>,
    ) -> bool {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_replace_complete_spell_acquisition_runtime_like_cpp(
                spell_rows,
                traits,
                overrides,
                skill_records,
                occupied_skill_slots,
                non_durable_skill_tombstones,
            );
        }
        let Some(prepared) = wow_entities::PreparedPlayerSpellAcquisitionLikeCpp::try_new(
            spell_rows
                .into_iter()
                .map(canonical_player_spell_record_like_cpp),
            traits,
            overrides,
            skill_records
                .into_iter()
                .map(|(key, skill)| (key, canonical_player_skill_record_like_cpp(skill)))
                .collect(),
            occupied_skill_slots,
            non_durable_skill_tombstones,
        ) else {
            return false;
        };
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        if self
            .with_owned_player_mut_like_cpp(|player| {
                player.apply_prepared_spell_acquisition_like_cpp(prepared)
            })
            .is_none()
        {
            return false;
        }
        self.sync_player_registry_state_like_cpp();
        true
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_replace_complete_spell_acquisition_runtime_like_cpp(
        &mut self,
        spell_rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        traits: impl IntoIterator<Item = (i32, i32)>,
        overrides: impl IntoIterator<Item = (i32, i32)>,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_skill_slots: u16,
        non_durable_skill_tombstones: BTreeSet<u16>,
    ) -> bool {
        let mut exact_spells = BTreeMap::new();
        for spell in spell_rows {
            if spell.spell_id <= 0 || exact_spells.insert(spell.spell_id, spell).is_some() {
                return false;
            }
        }

        let mut exact_traits = HashMap::new();
        for (spell_id, trait_definition_id) in traits {
            if trait_definition_id <= 0
                || !exact_spells
                    .get(&spell_id)
                    .is_some_and(|spell| spell.state != RepresentedPlayerSpellStateLikeCpp::Removed)
                || exact_traits.insert(spell_id, trait_definition_id).is_some()
            {
                return false;
            }
        }

        let mut exact_overrides = HashMap::<i32, BTreeSet<i32>>::new();
        for (overridden_spell_id, overriding_spell_id) in overrides {
            if overridden_spell_id <= 0 || overriding_spell_id <= 0 {
                return false;
            }
            exact_overrides
                .entry(overridden_spell_id)
                .or_default()
                .insert(overriding_spell_id);
        }

        if usize::from(occupied_skill_slots) != skill_records.len()
            || occupied_skill_slots > 256
            || !skill_records.iter().all(|(skill_id, skill)| {
                *skill_id != 0
                    && *skill_id == skill.skill_id
                    && (skill.state != RepresentedPlayerSkillStateLikeCpp::Deleted
                        || (skill.step == 0
                            && skill.value == 0
                            && skill.max == 0
                            && skill.profession_slot == -1))
            })
            || !non_durable_skill_tombstones.iter().all(|skill_id| {
                skill_records
                    .get(skill_id)
                    .is_some_and(crate::session_rules::is_non_durable_skill_tombstone_like_cpp)
            })
        {
            return false;
        }

        let mut known_spells = exact_spells
            .values()
            .filter(|spell| {
                spell.state != RepresentedPlayerSpellStateLikeCpp::Removed && !spell.disabled
            })
            .map(|spell| spell.spell_id)
            .collect::<Vec<_>>();
        known_spells.sort_unstable();
        let dependent_spells = exact_spells
            .values()
            .filter(|spell| {
                spell.state != RepresentedPlayerSpellStateLikeCpp::Removed && spell.dependent
            })
            .map(|spell| spell.spell_id)
            .collect();
        let favorite_spells = exact_spells
            .values()
            .filter(|spell| {
                spell.state != RepresentedPlayerSpellStateLikeCpp::Removed && spell.favorite
            })
            .map(|spell| spell.spell_id)
            .collect();
        let removed_spells = exact_spells
            .values()
            .filter(|spell| spell.state == RepresentedPlayerSpellStateLikeCpp::Removed)
            .map(|spell| spell.spell_id)
            .collect();

        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        if self
            .mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.install_acquisition_snapshot_like_cpp(
                    wow_entities::PlayerSpellAcquisitionSnapshotLikeCpp {
                        known_spells,
                        rows: exact_spells
                            .into_iter()
                            .map(|(id, row)| (id, canonical_player_spell_record_like_cpp(row)))
                            .collect(),
                        dependent_known_spells: dependent_spells,
                        removed_known_spells: removed_spells,
                        favorite_known_spells: favorite_spells,
                        trait_definition_ids: exact_traits.into_iter().collect(),
                        override_spells: exact_overrides.into_iter().collect(),
                    },
                );
                // Fallback grants and trait-config source evidence are not part
                // of this prepared result; retain the current owner's values.
            })
            .is_none()
        {
            return false;
        }
        if !self.replace_player_skill_runtime_exact_like_cpp(
            skill_records,
            true,
            true,
            Some(occupied_skill_slots),
            non_durable_skill_tombstones,
        ) {
            return false;
        }
        // Cross-session consumers (notably disenchant roll eligibility) read
        // known spells and enchanting rank from the player registry. Publish
        // the committed snapshot there before any acquisition action packet.
        self.sync_player_registry_state_like_cpp();
        true
    }
    pub(crate) fn record_spell_acquisition_post_commit_action_like_cpp(
        &mut self,
        action: crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_spell_acquisition_post_commit_actions_like_cpp
            .push(action);
        #[cfg(not(test))]
        let _ = action;
    }
    pub(crate) fn begin_spell_acquisition_post_commit_action_batch_like_cpp(&mut self) {
        #[cfg(test)]
        self.represented_spell_acquisition_post_commit_actions_like_cpp
            .clear();
    }
    #[cfg(test)]
    pub(crate) fn represented_spell_acquisition_post_commit_actions_like_cpp(
        &self,
    ) -> &[crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp] {
        &self.represented_spell_acquisition_post_commit_actions_like_cpp
    }
    pub(crate) fn grant_dual_wield_after_spell_acquisition_like_cpp(&mut self) -> bool {
        self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_can_dual_wield_like_cpp(true);
        })
        .is_some()
    }
    pub(crate) fn has_canonical_player_for_spell_acquisition_like_cpp(&self) -> bool {
        self.canonical_player_snapshot_like_cpp(|_| ()).is_some()
    }
}
