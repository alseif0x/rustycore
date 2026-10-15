//! Remaining represented spell operations owned by this responsibility.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
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
    pub(in crate::session) fn reset_spells_notification_text_like_cpp(&self) -> String {
        let text = self.trinity_string_like_cpp(LANG_RESET_SPELLS_LIKE_CPP);
        if text == "<error>" {
            LANG_RESET_SPELLS_TEXT_LIKE_CPP.to_string()
        } else {
            text.to_string()
        }
    }

    pub(crate) fn player_spell_runtime_snapshot_like_cpp(
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
    pub(crate) fn with_player_spell_runtime_like_cpp<R>(
        &self,
        query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.with_player_spell_runtime_like_cpp(hub, cfg!(test), query)
    }
    /// Incarnation dispatch for one named spell-runtime transition (#754).
    ///
    /// The transitions themselves are the named operations on
    /// `PlayerSpellRuntimeState`. In production the only callers are this
    /// owner module (`session/spell_state`: the login spell application in
    /// `spellbook.rs` and `effects.rs`) and its `SpellbookHostLikeCpp` adapter
    /// (`spellbook_host.rs`), through which only the spellbook bodies in
    /// `wow_world_spell::spellbook_session` reach it, so no other module writes
    /// the Player's spell state directly. It is `pub(crate)` only for the
    /// fixtures in the `#![cfg(test)]` `handlers/spellbook_test_shims.rs`.
    pub(crate) fn mutate_player_spell_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_spell_state_mut(self);
        state.mutate_player_spell_runtime_like_cpp(hub.shared(), cfg!(test), f)
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/spell/f3_shims.rs"]
mod f3_shims;
