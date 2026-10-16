//! Remaining represented spell operations owned by this responsibility.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The hit-inert, self-res and
//! far-teleport interrupt operations moved to `wow_world_spell` (#1363 slice
//! 12b3); the spell-row and mount-source reads, test-only, moved to the test
//! shim.

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
    pub(in crate::session) fn player_spell_hit_source_identity_complete_like_cpp(&self) -> bool {
        let Some(player_guid) = self.core.player_guid else {
            return false;
        };
        self.core
            .player_handle_like_cpp
            .is_some_and(|handle| handle.guid() == player_guid)
            || cfg!(test) && self.player_bootstrap_attached_for_test_like_cpp()
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/spell/f3_shims.rs"]
mod f3_shims;
