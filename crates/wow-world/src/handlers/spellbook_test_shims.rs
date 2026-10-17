// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the spellbook bodies moved to
//! `wow_world_spell` (#1363 slice 9a).
//!
//! The first impl block holds the spellbook's #1241 F3 test entry points
//! (from `unit_tests/session/spell_state/spellbook/f3_shims.rs`) and the
//! `#[cfg(test)]` methods of
//! `session/spell_state/{spellbook,spell,effects,acquisition}.rs`, verbatim
//! except that `pub(in crate::session)` reads `pub(crate)`: they were already
//! test-only. The second holds one-call method forwards for the moved
//! functions the scenarios and these fixtures call, with this session as the
//! host, so no scenario call site changes.

#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::session::*;
use wow_data::TalentTabStore;
use wow_world_spell::canonical_player_spell_record_like_cpp;

impl WorldSession {
    pub(crate) fn is_spell_learn_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.catalogs
            .spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .is_some_and(|store| store.is_spell_learn_spell_like_cpp(spell_id))
    }
    pub(crate) fn is_spell_learn_to_spell_like_cpp(&self, spell_id1: u32, spell_id2: u32) -> bool {
        self.catalogs
            .spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .is_some_and(|store| store.is_spell_learn_to_spell_like_cpp(spell_id1, spell_id2))
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
    #[cfg(test)]
    pub(crate) fn fixture_set_known_spells_like_cpp(&mut self, spells: Vec<i32>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        wow_world_spell::invalidate_represented_player_spell_rows_like_cpp(self);
        let known_spells = spells.clone();
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.replace_known_spells_and_prune_derived_like_cpp(spells);
        });
        wow_world_spell::learn_account_mount_spells_like_cpp(self);
    }
    #[cfg(test)]
    pub(crate) fn fixture_learn_known_spell_like_cpp(&mut self, spell_id: i32) {
        if !self.known_spells_like_cpp().contains(&spell_id) {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        wow_world_spell::invalidate_represented_player_spell_rows_like_cpp(self);
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.learn_known_spell_id_unless_known_like_cpp(spell_id);
        });
    }
    #[cfg(test)]
    pub(crate) fn fixture_learn_dependent_known_spell_like_cpp(&mut self, spell_id: i32) {
        self.fixture_learn_known_spell_like_cpp(spell_id);
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.mark_dependent_learned_spell_like_cpp(spell_id);
        });
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
    #[cfg(test)]
    pub(crate) fn fixture_set_complete_spell_trait_definition_ids_like_cpp(
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
    #[cfg(test)]
    pub(crate) fn represented_player_spell_rows_loaded_like_cpp(&self) -> bool {
        self.with_player_spell_runtime_like_cpp(|runtime| runtime.rows_loaded_like_cpp())
            .unwrap_or(false)
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
    #[cfg(test)]
    pub(crate) fn fixture_replace_loaded_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        complete: bool,
    ) -> bool {
        // A replacement can describe another logical PlayerSpellMap. Existing
        // trait/override mirrors must be re-authorized even when spell IDs happen
        // to overlap, otherwise residual state can be attributed to the new map.
        wow_world_spell::invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp(
            self,
        );
        let mut exact_rows = BTreeMap::new();
        for row in rows {
            if row.spell_id <= 0 || exact_rows.insert(row.spell_id, row).is_some() {
                wow_world_spell::invalidate_represented_player_spell_rows_like_cpp(self);
                return false;
            }
        }

        // A coherent load must not erase a dirty base grant produced earlier
        // in this same Player lifetime while ancillary planner authority was
        // unavailable. Reconcile it against the newly authoritative row using
        // C++ `Player::LearnSpell`: disabled rows preserve active, other rows
        // become active, and favorite/dependent state comes from the load.
        let fallback_rows = wow_world_spell::player_spell_runtime_snapshot_like_cpp(self)
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
    #[cfg(test)]
    pub(crate) fn fixture_replace_complete_spell_acquisition_runtime_like_cpp(
        &mut self,
        spell_rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        traits: impl IntoIterator<Item = (i32, i32)>,
        overrides: impl IntoIterator<Item = (i32, i32)>,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_skill_slots: u16,
        non_durable_skill_tombstones: BTreeSet<u16>,
    ) -> bool {
        let owner = self.core.player_acquisition_owner_access_like_cpp();
        let result = wow_world_application::install_represented_spell_acquisition_runtime_like_cpp(
            &owner,
            &mut self.spell_state,
            spell_rows,
            traits,
            overrides,
            skill_records,
            occupied_skill_slots,
            non_durable_skill_tombstones,
            true,
            #[cfg(any(test, feature = "test-fixtures"))]
            (
                &mut self.fixtures.progression.player_skill_test_fixture_like_cpp,
                &mut self.fixtures.progression.represented_enchanting_skill,
            ),
        );
        if result.is_ok() {
            self.sync_player_registry_state_like_cpp();
            true
        } else {
            false
        }
    }
}

impl WorldSession {
    pub(crate) fn learn_dependent_known_spell_like_cpp(&mut self, spell_id: i32) {
        wow_world_spell::learn_dependent_known_spell_like_cpp(self, spell_id)
    }

    pub(crate) fn resolved_known_spells_like_cpp(&self) -> Option<Vec<i32>> {
        wow_world_spell::resolved_known_spells_like_cpp(self)
    }

    pub(crate) fn known_spells_like_cpp(&self) -> Vec<i32> {
        wow_world_spell::known_spells_like_cpp(self)
    }

    pub(crate) fn set_complete_represented_player_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    ) -> bool {
        wow_world_spell::set_complete_represented_player_spell_rows_like_cpp(self, rows)
    }

    pub(crate) fn replace_loaded_represented_player_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        complete: bool,
    ) -> bool {
        wow_world_spell::replace_loaded_represented_player_spell_rows_like_cpp(self, rows, complete)
    }

    pub(crate) fn set_complete_represented_spell_trait_definition_ids_like_cpp(
        &mut self,
        traits: impl IntoIterator<Item = (i32, i32)>,
    ) -> bool {
        wow_world_spell::set_complete_represented_spell_trait_definition_ids_like_cpp(self, traits)
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
        wow_world_spell::load_represented_talent_row_with_spell_side_effects_like_cpp(
            self,
            talent_tabs,
            talent_id,
            rank,
            talent_group,
            known_spells,
            dependent_spells,
        )
    }
}
