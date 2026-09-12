// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player spell-runtime state.
//!
//! C++ `Player` owns `PlayerSpellMap m_spells` (`Player.h:2961`) and
//! `m_overrideSpells` (`:2962`) and performs every transition itself:
//! `AddSpell` (`Player.cpp:2741`), `LearnSpell` (`:3192`), `AddTemporarySpell`
//! (`:3140`), `RemoveSpell` (`:3236`), `AddOverrideSpell` (`:28581`) and
//! `RemoveOverrideSpell` (`:28586`), with `_LoadSpells` (`:18924`) and
//! `_SaveSpells` (`:20399`) at the persistence edges.
//!
//! Separated from the `player/mod.rs` root under #754, which also closed the
//! fields to this crate's Player module: reads keep named accessors, and every
//! write is a named operation the owner checks. The existing
//! `player::spellbook` operations keep their own C++ anchors. The completeness flags have no direct C++
//! member — they distinguish an authoritative empty load from an owner that was
//! never hydrated — and that meaning is preserved here.

use std::collections::{BTreeMap, BTreeSet};

use super::{PlayerKnownSpellRecord, PlayerTraitConfigDetails, PlayerTraitConfigState};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerSpellRuntimeState {
    pub(super) known_spells: Vec<i32>,
    pub(super) rows: BTreeMap<i32, PlayerKnownSpellRecord>,
    pub(super) rows_loaded: bool,
    pub(super) rows_complete: bool,
    pub(super) fallback_rows: BTreeMap<i32, PlayerKnownSpellRecord>,
    pub(super) dependent_known_spells: BTreeSet<i32>,
    pub(super) removed_known_spells: BTreeSet<i32>,
    pub(super) favorite_known_spells: BTreeSet<i32>,
    pub(super) trait_definition_ids: BTreeMap<i32, i32>,
    pub(super) trait_definition_ids_complete: bool,
    /// Exact C++ `TraitConfig` headers loaded for this Player, keyed by
    /// config ID. Completeness is explicit because an authoritative empty
    /// entry result is materially different from an owner that was not
    /// resolved during login.
    pub(super) trait_config_rows: BTreeMap<i32, PlayerTraitConfigState>,
    pub(super) trait_config_rows_complete: bool,
    pub(super) trait_entry_rows_complete: bool,
    pub(super) trait_entry_rows_empty: bool,
    pub(super) override_spells: BTreeMap<i32, BTreeSet<i32>>,
    pub(super) override_spells_complete: bool,
}

impl PlayerSpellRuntimeState {
    // ---- reads -------------------------------------------------------------

    /// C++ `Player::GetSpellMap()` keys the client sees as known.
    #[must_use]
    pub fn known_spells_like_cpp(&self) -> &[i32] {
        &self.known_spells
    }

    /// C++ `PlayerSpellMap m_spells` rows, keyed by spell id.
    #[must_use]
    pub fn rows_like_cpp(&self) -> &BTreeMap<i32, PlayerKnownSpellRecord> {
        &self.rows
    }

    /// Whether `Player::_LoadSpells` has run for this Player.
    #[must_use]
    pub fn rows_loaded_like_cpp(&self) -> bool {
        self.rows_loaded
    }

    /// Whether the loaded rows are the complete authoritative set.
    #[must_use]
    pub fn rows_complete_like_cpp(&self) -> bool {
        self.rows_complete
    }

    /// Rows retained for a Player that has no authoritative load yet.
    #[must_use]
    pub fn fallback_rows_like_cpp(&self) -> &BTreeMap<i32, PlayerKnownSpellRecord> {
        &self.fallback_rows
    }

    /// C++ `PlayerSpell::dependent` membership.
    #[must_use]
    pub fn dependent_known_spells_like_cpp(&self) -> &BTreeSet<i32> {
        &self.dependent_known_spells
    }

    /// Spells `Player::RemoveSpell` has taken away from this Player.
    #[must_use]
    pub fn removed_known_spells_like_cpp(&self) -> &BTreeSet<i32> {
        &self.removed_known_spells
    }

    /// C++ `PlayerSpell::favorite` membership.
    #[must_use]
    pub fn favorite_known_spells_like_cpp(&self) -> &BTreeSet<i32> {
        &self.favorite_known_spells
    }

    /// C++ `PlayerSpell::TraitDefinitionId`, keyed by spell id.
    #[must_use]
    pub fn trait_definition_ids_like_cpp(&self) -> &BTreeMap<i32, i32> {
        &self.trait_definition_ids
    }

    #[must_use]
    pub fn trait_definition_ids_complete_like_cpp(&self) -> bool {
        self.trait_definition_ids_complete
    }

    /// Exact C++ `TraitConfig` headers loaded for this Player.
    #[must_use]
    pub fn trait_config_rows_like_cpp(&self) -> &BTreeMap<i32, PlayerTraitConfigState> {
        &self.trait_config_rows
    }

    #[must_use]
    pub fn trait_config_rows_complete_like_cpp(&self) -> bool {
        self.trait_config_rows_complete
    }

    #[must_use]
    pub fn trait_entry_rows_complete_like_cpp(&self) -> bool {
        self.trait_entry_rows_complete
    }

    #[must_use]
    pub fn trait_entry_rows_empty_like_cpp(&self) -> bool {
        self.trait_entry_rows_empty
    }

    /// C++ `Player::m_overrideSpells`, keyed by the overridden spell id.
    #[must_use]
    pub fn override_spells_like_cpp(&self) -> &BTreeMap<i32, BTreeSet<i32>> {
        &self.override_spells
    }

    #[must_use]
    pub fn override_spells_complete_like_cpp(&self) -> bool {
        self.override_spells_complete
    }

    // ---- transitions -------------------------------------------------------

    /// Mark the auxiliary acquisition snapshots complete or stale together,
    /// as login and every invalidating edit do.
    pub fn set_acquisition_snapshot_completeness_like_cpp(
        &mut self,
        trait_definition_ids_complete: bool,
        override_spells_complete: bool,
    ) {
        self.trait_definition_ids_complete = trait_definition_ids_complete;
        self.override_spells_complete = override_spells_complete;
    }

    /// Mark the override snapshot authoritative once every represented
    /// `AddSpell` for login has run.
    pub fn mark_override_spells_complete_like_cpp(&mut self) {
        self.override_spells_complete = true;
    }

    /// Install the authoritative override map from its loader.
    pub fn replace_override_spells_like_cpp(
        &mut self,
        overrides: BTreeMap<i32, BTreeSet<i32>>,
        complete: bool,
    ) {
        self.override_spells = overrides;
        self.override_spells_complete = complete;
    }

    /// Drop every override and return to unhydrated, as login does before the
    /// next character's owner is built.
    pub fn clear_override_spells_like_cpp(&mut self) {
        self.override_spells.clear();
        self.override_spells_complete = false;
    }

    /// Install the authoritative spell rows from `Player::_LoadSpells`.
    pub fn replace_rows_like_cpp(
        &mut self,
        rows: BTreeMap<i32, PlayerKnownSpellRecord>,
        complete: bool,
    ) {
        self.rows = rows;
        self.rows_loaded = true;
        self.rows_complete = complete;
    }

    /// Mutable access to one identified spell row, as C++ writes
    /// `itr->second` after finding it in `m_spells`.
    pub fn row_mut_like_cpp(&mut self, spell_id: i32) -> Option<&mut PlayerKnownSpellRecord> {
        self.rows.get_mut(&spell_id)
    }

    /// Insert or replace one spell row (`Player::AddSpell`, `Player.cpp:2741`).
    pub fn insert_row_like_cpp(&mut self, spell_id: i32, row: PlayerKnownSpellRecord) {
        self.rows.insert(spell_id, row);
    }

    /// Remove one spell row (`Player::RemoveSpell`, `Player.cpp:3236`).
    pub fn remove_row_like_cpp(&mut self, spell_id: i32) -> Option<PlayerKnownSpellRecord> {
        self.rows.remove(&spell_id)
    }

    /// Drop every loaded row and return to unhydrated, as an invalidated
    /// acquisition snapshot does.
    pub fn clear_rows_like_cpp(&mut self) {
        self.rows.clear();
        self.rows_loaded = false;
        self.rows_complete = false;
    }

    /// C++ `Player::RemoveSpell` (`Player.cpp:3236`) forgetting one spell:
    /// it leaves the known list and its dependent/favorite membership, and is
    /// tracked as removed only when it was known and not dependent.
    pub fn forget_known_spell_like_cpp(&mut self, spell_id: i32) -> ForgottenKnownSpellLikeCpp {
        let was_known = self.known_spells.contains(&spell_id);
        let was_dependent = self.dependent_known_spells.contains(&spell_id);
        self.known_spells.retain(|known| *known != spell_id);
        self.dependent_known_spells.remove(&spell_id);
        self.favorite_known_spells.remove(&spell_id);
        if was_known && !was_dependent {
            self.removed_known_spells.insert(spell_id);
        }
        ForgottenKnownSpellLikeCpp {
            was_known,
            was_dependent,
        }
    }

    /// Keep only the favorites the Player actually knows, and mirror the
    /// result onto the loaded rows when they are authoritative.
    pub fn replace_known_favorites_like_cpp(
        &mut self,
        favorite_spells: impl IntoIterator<Item = i32>,
    ) {
        self.favorite_known_spells = favorite_spells
            .into_iter()
            .filter(|spell_id| self.known_spells.contains(spell_id))
            .collect();
        if self.rows_complete {
            for row in self.rows.values_mut() {
                row.favorite = self.favorite_known_spells.contains(&row.spell_id);
            }
        }
    }

    /// Drop the loaded trait-config headers and return them to unhydrated.
    pub fn clear_trait_config_rows_like_cpp(&mut self) {
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }

    /// Rebuild the known list and its authoritative rows together, as a
    /// fixture that installs one coherent spellbook snapshot does.
    pub fn replace_known_spells_and_rows_like_cpp(
        &mut self,
        known_spells: Vec<i32>,
        rows: BTreeMap<i32, PlayerKnownSpellRecord>,
    ) {
        self.known_spells = known_spells;
        self.rows = rows;
    }

    /// Drop every trait definition, config header and entry flag, as the
    /// trait-config load does before replacing them.
    pub fn begin_trait_authority_load_like_cpp(&mut self) {
        self.trait_definition_ids.clear();
        self.trait_definition_ids_complete = false;
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }

    /// Set the three trait-config authority facts exactly, for a fixture that
    /// must reproduce a recorded owner state including its incoherent
    /// combinations.
    pub fn set_trait_config_authority_for_fixture_like_cpp(
        &mut self,
        rows_complete: bool,
        entries_complete: bool,
        entries_empty: bool,
    ) {
        self.trait_config_rows_complete = rows_complete;
        self.trait_entry_rows_complete = entries_complete;
        self.trait_entry_rows_empty = entries_empty;
    }

    /// Mark the loaded trait-config entries authoritative on their own.
    pub fn mark_trait_entry_rows_complete_like_cpp(&mut self) {
        self.trait_entry_rows_complete = true;
    }

    /// C++ `Player::AddSpell` marking one spell dependent and no longer a
    /// favourite, mirroring both onto its row when the rows are authoritative.
    pub fn mark_dependent_learned_spell_like_cpp(&mut self, spell_id: i32) {
        self.dependent_known_spells.insert(spell_id);
        self.favorite_known_spells.remove(&spell_id);
        if self.rows_complete
            && let Some(row) = self.rows.get_mut(&spell_id)
        {
            row.dependent = true;
            row.favorite = false;
        }
    }

    /// Retain the current rows as the fallback set, as a Player that loses its
    /// authoritative load keeps them.
    pub fn retain_rows_as_fallback_like_cpp(&mut self) {
        self.fallback_rows = self.rows.clone();
    }

    /// Mark the trait-definition snapshot authoritative on its own.
    pub fn mark_trait_definition_ids_complete_like_cpp(&mut self) {
        self.trait_definition_ids_complete = true;
    }

    /// Mark the rows authoritative without changing whether they were loaded.
    pub fn set_rows_complete_like_cpp(&mut self, complete: bool) {
        self.rows_complete = complete;
    }

    /// Mark the rows loaded without changing whether they are complete.
    pub fn set_rows_loaded_like_cpp(&mut self, loaded: bool) {
        self.rows_loaded = loaded;
    }

    /// Mark the override snapshot authoritative or stale on its own.
    pub fn set_override_spells_complete_like_cpp(&mut self, complete: bool) {
        self.override_spells_complete = complete;
    }

    /// Track one spell as removed, as `Player::RemoveSpell` does.
    pub fn mark_removed_like_cpp(&mut self, spell_id: i32) {
        self.removed_known_spells.insert(spell_id);
    }

    /// Set the two row-authority facts directly.
    ///
    /// `Player::_LoadSpells` normally sets them together through
    /// [`Self::replace_rows_like_cpp`]. They stay independent facts because an
    /// authoritative empty load and an owner that was never hydrated are
    /// materially different, and a consumer must fail closed on any
    /// combination rather than assume one implies the other.
    pub fn set_row_authority_like_cpp(&mut self, loaded: bool, complete: bool) {
        self.rows_loaded = loaded;
        self.rows_complete = complete;
    }

    /// Mark the loaded trait-config headers and their entries authoritative,
    /// as the trait-config load's completion does.
    pub fn mark_trait_authority_complete_like_cpp(&mut self, entries_empty: bool) {
        self.trait_config_rows_complete = true;
        self.trait_entry_rows_complete = true;
        self.trait_entry_rows_empty = entries_empty;
    }

    /// Replace the known list and drop every derived entry the Player no
    /// longer knows, as a fixture installing a fresh spellbook does.
    pub fn replace_known_spells_and_prune_derived_like_cpp(&mut self, known_spells: Vec<i32>) {
        self.known_spells = known_spells;
        self.removed_known_spells.clear();
        let known = self.known_spells.clone();
        self.dependent_known_spells
            .retain(|spell_id| known.contains(spell_id));
        self.favorite_known_spells
            .retain(|spell_id| known.contains(spell_id));
        self.trait_definition_ids
            .retain(|spell_id, _| known.contains(spell_id));
    }

    /// C++ `Player::LearnSpell` (`Player.cpp:3192`) making one spell known
    /// again: it joins the known list and stops being tracked as removed.
    pub fn learn_known_spell_id_unless_known_like_cpp(&mut self, spell_id: i32) {
        if !self.known_spells.contains(&spell_id) {
            self.known_spells.push(spell_id);
        }
        self.removed_known_spells.remove(&spell_id);
    }

    /// Drop every trait definition id and mark the snapshot stale, as a
    /// rejected load does.
    pub fn clear_trait_definition_ids_like_cpp(&mut self) {
        self.trait_definition_ids.clear();
        self.trait_definition_ids_complete = false;
    }

    /// Install one validated post-login acquisition snapshot.
    ///
    /// This is the represented equivalent of the state C++ holds once
    /// `Player::_LoadSpells` (`Player.cpp:18924`) and the trait/override loads
    /// have all run: the rows, their derived sets and both auxiliary snapshots
    /// become authoritative together. The retained fallback rows and
    /// trait-config evidence are deliberately left untouched.
    pub fn install_acquisition_snapshot_like_cpp(
        &mut self,
        snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    ) {
        self.known_spells = snapshot.known_spells;
        self.rows = snapshot.rows;
        self.rows_loaded = true;
        self.rows_complete = true;
        self.dependent_known_spells = snapshot.dependent_known_spells;
        self.removed_known_spells = snapshot.removed_known_spells;
        self.favorite_known_spells = snapshot.favorite_known_spells;
        self.trait_definition_ids = snapshot.trait_definition_ids;
        self.trait_definition_ids_complete = true;
        self.override_spells = snapshot.override_spells;
        self.override_spells_complete = true;
    }

    /// A test fixture's equivalent of `Player::_SaveSpells` (`Player.cpp:20399`)
    /// completing: drop the removed rows, settle the rest to `Unchanged` and
    /// rebase every derived set on the rows that survived.
    pub fn rebase_onto_saved_rows_like_cpp(&mut self) {
        self.rows.retain(|_, spell| {
            if spell.state == crate::PlayerSpellLoadState::Removed {
                return false;
            }
            if spell.state != crate::PlayerSpellLoadState::Temporary {
                spell.state = crate::PlayerSpellLoadState::Unchanged;
            }
            true
        });
        self.removed_known_spells.clear();
        self.trait_definition_ids
            .retain(|spell_id, _| self.rows.contains_key(spell_id));
        self.dependent_known_spells = self
            .rows
            .values()
            .filter(|spell| spell.dependent)
            .map(|spell| spell.spell_id)
            .collect();
        self.favorite_known_spells = self
            .rows
            .values()
            .filter(|spell| spell.favorite)
            .map(|spell| spell.spell_id)
            .collect();
        self.known_spells = self
            .rows
            .values()
            .filter(|spell| !spell.disabled)
            .map(|spell| spell.spell_id)
            .collect();
    }

    /// Install the authoritative trait-config headers and their entry flags.
    pub fn complete_trait_authority_load_like_cpp(
        &mut self,
        rows: BTreeMap<i32, PlayerTraitConfigState>,
        entries_empty: bool,
    ) {
        self.trait_config_rows = rows;
        self.trait_config_rows_complete = true;
        self.trait_entry_rows_complete = true;
        self.trait_entry_rows_empty = entries_empty;
    }

    /// Take one spell's trait definition id, as `Player::RemoveSpell` does
    /// before dropping its override.
    pub fn take_trait_definition_id_like_cpp(&mut self, spell_id: i32) -> Option<i32> {
        self.trait_definition_ids.remove(&spell_id)
    }

    /// Drop every override registered under one overridden spell id.
    pub fn remove_override_spell_entry_like_cpp(&mut self, overridden_spell_id: i32) {
        self.override_spells.remove(&overridden_spell_id);
    }

    /// Forget the rows retained while no authoritative load exists.
    pub fn clear_fallback_rows_like_cpp(&mut self) {
        self.fallback_rows.clear();
    }

    /// Install the retained rows used while no authoritative load exists.
    pub fn replace_fallback_rows_like_cpp(&mut self, rows: BTreeMap<i32, PlayerKnownSpellRecord>) {
        self.fallback_rows = rows;
    }

    pub fn fallback_row_mut_like_cpp(
        &mut self,
        spell_id: i32,
    ) -> Option<&mut PlayerKnownSpellRecord> {
        self.fallback_rows.get_mut(&spell_id)
    }

    pub fn insert_fallback_row_like_cpp(&mut self, spell_id: i32, row: PlayerKnownSpellRecord) {
        self.fallback_rows.insert(spell_id, row);
    }

    pub fn remove_fallback_row_like_cpp(
        &mut self,
        spell_id: i32,
    ) -> Option<PlayerKnownSpellRecord> {
        self.fallback_rows.remove(&spell_id)
    }

    /// Replace the client-visible known list.
    pub fn replace_known_spells_like_cpp(&mut self, known_spells: Vec<i32>) {
        self.known_spells = known_spells;
    }

    /// Record one spell as known by the client.
    pub fn add_known_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        if self.known_spells.contains(&spell_id) {
            return false;
        }
        self.known_spells.push(spell_id);
        true
    }

    /// Drop one spell from the client-visible known list.
    pub fn remove_known_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        let before = self.known_spells.len();
        self.known_spells.retain(|known| *known != spell_id);
        self.known_spells.len() != before
    }

    /// C++ marks a spell dependent when it was learned through another
    /// (`Player::AddSpell`'s `dependent` argument).
    pub fn set_dependent_like_cpp(&mut self, spell_id: i32, dependent: bool) {
        if dependent {
            self.dependent_known_spells.insert(spell_id);
        } else {
            self.dependent_known_spells.remove(&spell_id);
        }
    }

    pub fn replace_dependent_known_spells_like_cpp(&mut self, spells: BTreeSet<i32>) {
        self.dependent_known_spells = spells;
    }

    /// Track a spell `Player::RemoveSpell` took away.
    pub fn set_removed_like_cpp(&mut self, spell_id: i32, removed: bool) {
        if removed {
            self.removed_known_spells.insert(spell_id);
        } else {
            self.removed_known_spells.remove(&spell_id);
        }
    }

    pub fn replace_removed_known_spells_like_cpp(&mut self, spells: BTreeSet<i32>) {
        self.removed_known_spells = spells;
    }

    /// C++ `PlayerSpell::favorite`.
    pub fn set_favorite_like_cpp(&mut self, spell_id: i32, favorite: bool) {
        if favorite {
            self.favorite_known_spells.insert(spell_id);
        } else {
            self.favorite_known_spells.remove(&spell_id);
        }
    }

    pub fn replace_favorite_known_spells_like_cpp(&mut self, spells: BTreeSet<i32>) {
        self.favorite_known_spells = spells;
    }

    /// C++ `PlayerSpell::TraitDefinitionId` for one spell.
    pub fn set_trait_definition_id_like_cpp(&mut self, spell_id: i32, definition_id: Option<i32>) {
        match definition_id {
            Some(definition_id) => {
                self.trait_definition_ids.insert(spell_id, definition_id);
            }
            None => {
                self.trait_definition_ids.remove(&spell_id);
            }
        }
    }

    pub fn replace_trait_definition_ids_like_cpp(
        &mut self,
        definition_ids: BTreeMap<i32, i32>,
        complete: bool,
    ) {
        self.trait_definition_ids = definition_ids;
        self.trait_definition_ids_complete = complete;
    }

    pub fn replace_trait_config_rows_like_cpp(
        &mut self,
        rows: BTreeMap<i32, PlayerTraitConfigState>,
        complete: bool,
    ) {
        self.trait_config_rows = rows;
        self.trait_config_rows_complete = complete;
    }

    pub fn insert_trait_config_row_like_cpp(
        &mut self,
        config_id: i32,
        row: PlayerTraitConfigState,
    ) {
        self.trait_config_rows.insert(config_id, row);
    }

    /// Install the loaded detail payload of every trait config at once, as the
    /// login path hands the Player the configs it read (C++
    /// `Player::AddTraitConfig`, `Player.h:1836`, with `GetTraitConfig` at
    /// `:1837` reading them back).
    ///
    /// The hydration is refused whole unless it describes exactly the rows this
    /// owner holds: both row sets must be authoritative, the incoming configs
    /// must match the stored rows one for one with unique ids, and each stored
    /// header — config type, specialization and combat flags — must equal the
    /// incoming one. A partial install would leave details describing rows that
    /// were never loaded.
    pub fn install_loaded_trait_config_details_like_cpp(
        &mut self,
        configs: &[(i32, (i32, i32, i32), PlayerTraitConfigDetails)],
    ) -> bool {
        if !self.trait_config_rows_complete || !self.trait_entry_rows_complete {
            return false;
        }
        if self.trait_config_rows.len() != configs.len() {
            return false;
        }
        let unique_ids = configs
            .iter()
            .map(|(config_id, _, _)| *config_id)
            .collect::<BTreeSet<_>>();
        if unique_ids.len() != configs.len() {
            return false;
        }
        if configs.iter().any(|(config_id, header, _)| {
            self.trait_config_rows
                .get(config_id)
                .is_none_or(|state| state.header != *header)
        }) {
            return false;
        }
        for (config_id, _, details) in configs {
            let Some(state) = self.trait_config_rows.get_mut(config_id) else {
                return false;
            };
            state.details = Some(details.clone());
        }
        true
    }

    pub fn set_trait_entry_rows_state_like_cpp(&mut self, complete: bool, empty: bool) {
        self.trait_entry_rows_complete = complete;
        self.trait_entry_rows_empty = empty;
    }

    /// Login rebuilds a fresh C++ Player: drop the trait and override edges so
    /// they cannot contaminate a coincident spell id on the next character.
    pub fn clear_trait_and_override_state_like_cpp(&mut self) {
        self.override_spells.clear();
        self.trait_definition_ids.clear();
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }
}

/// What one `Player::RemoveSpell` found before forgetting the spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgottenKnownSpellLikeCpp {
    pub was_known: bool,
    /// C++ keeps a dependent spell out of the removed set and deletes its row
    /// outright instead of tombstoning it.
    pub was_dependent: bool,
}

/// One validated post-login spell-acquisition snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerSpellAcquisitionSnapshotLikeCpp {
    pub known_spells: Vec<i32>,
    pub rows: BTreeMap<i32, PlayerKnownSpellRecord>,
    pub dependent_known_spells: BTreeSet<i32>,
    pub removed_known_spells: BTreeSet<i32>,
    pub favorite_known_spells: BTreeSet<i32>,
    pub trait_definition_ids: BTreeMap<i32, i32>,
    pub override_spells: BTreeMap<i32, BTreeSet<i32>>,
}
