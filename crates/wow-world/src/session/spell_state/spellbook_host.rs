//! The World session as the host of the spellbook bodies moved to
//! `wow_world_spell` (#1363 slice 9a). Each operation names the inherent
//! `WorldSession` method by path; the accessors are the field and the split
//! functions.

use super::*;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_spell::SessionSpellState;

impl wow_world_spell::SpellbookHostLikeCpp for WorldSession {
    fn spell_state(&self) -> &SessionSpellState {
        &self.spell_state
    }
    fn split_spell_state_ref(&self) -> (&SessionSpellState, HubRef<'_>) {
        crate::session::split_spell_state_ref(self)
    }
    fn split_spell_state_mut(&mut self) -> (&mut SessionSpellState, HubMut<'_>) {
        crate::session::split_spell_state_mut(self)
    }
    fn with_player_spell_runtime_like_cpp<R>(
        &self,
        query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        wow_world_spell::with_player_spell_runtime_like_cpp(self, query)
    }
    fn mutate_player_spell_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        WorldSession::mutate_player_spell_runtime_like_cpp(self, f)
    }
    fn represented_auto_unequip_offhand_if_need_like_cpp(&mut self, force: bool) -> bool {
        wow_world_inventory::represented_auto_unequip_offhand_if_need_like_cpp(self, force)
    }
    fn remove_represented_auras_due_to_spell_like_cpp(&mut self, spell_id: i32) -> usize {
        wow_world_spell::remove_represented_auras_due_to_spell_like_cpp(self, spell_id)
    }
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize {
        wow_world_spell::remove_auras_with_interrupt_flags_like_cpp(self, flags, flags2)
    }
}

impl wow_world_spell::SpellStateHostLikeCpp for WorldSession {
    fn interrupt_player_cast_like_cpp(&mut self, spell_id: Option<i32>) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        wow_world_application::interrupt_player_cast_like_cpp(&mut hub, state, spell_id)
    }
    fn player_persistent_capability_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerPersistentCapabilityStateLikeCpp> {
        WorldSession::player_persistent_capability_state_snapshot_like_cpp(self)
    }
}
